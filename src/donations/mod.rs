//! Donation ingestion from the service character's wallet, ported from
//! the legacy `GetWalletJournalCommand` → `GetWalletJournalJob` →
//! `CreateDonationAction` chain: fetch the ESI wallet journal, keep the
//! incoming `player_donation` entries, record each once (keyed by the
//! journal entry id), and on first sight credit premium through the
//! ported `PremiumService` and queue the confirmation mail.
//!
//! The legacy wrapped creation in `DB::transaction`; each donation here
//! runs in its own Postgres transaction, so a crash can never leave a
//! recorded donation without its premium credit. Like legacy, only a
//! donation created by this run credits premium (`wasRecentlyCreated`),
//! so re-runs never double-credit.

use sqlx::PgPool;

use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiClient, EsiError, EsiWalletJournalEntry};
use crate::notifications::format_isk;
use crate::premium::{self, PremiumCosts, PremiumUpdate};

/// The wallet-journal ref type of incoming ISK gifts, the legacy
/// `TransactionType::PlayerDonation`.
pub const PLAYER_DONATION_REF_TYPE: &str = "player_donation";

/// Outbox kind of the confirmation mail.
pub const DONATION_RECEIVED_KIND: &str = "donation-received";

/// The legacy `DonationReceivedNotification::getSubject`.
pub const DONATION_RECEIVED_SUBJECT: &str = "Donation Received - Thank You!";

#[derive(Debug, Default, Clone, Copy)]
pub struct WalletSyncStats {
    /// Journal entries fetched across all pages.
    pub entries: usize,
    /// Incoming player donations among them.
    pub donations: usize,
    /// Donations recorded (and premium-credited) by this run.
    pub created: usize,
}

#[derive(Debug)]
pub enum DonationSyncError {
    /// The service character holds no token with the wallet scope.
    NoToken,
    Token(TokenError),
    Esi(EsiError),
    Db(sqlx::Error),
    /// A player donation without a sender id would have crashed the
    /// legacy job on the not-null `character_id` column the same way.
    MissingSender(i64),
}

impl std::fmt::Display for DonationSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DonationSyncError::NoToken => write!(f, "no token with the wallet scope"),
            DonationSyncError::Token(error) => write!(f, "token: {error}"),
            DonationSyncError::Esi(error) => write!(f, "wallet journal fetch failed: {error:?}"),
            DonationSyncError::Db(error) => write!(f, "database error: {error}"),
            DonationSyncError::MissingSender(journal_id) => {
                write!(f, "journal entry {journal_id} has no first_party_id")
            }
        }
    }
}

impl std::error::Error for DonationSyncError {}

impl From<TokenError> for DonationSyncError {
    fn from(error: TokenError) -> Self {
        DonationSyncError::Token(error)
    }
}

impl From<sqlx::Error> for DonationSyncError {
    fn from(error: sqlx::Error) -> Self {
        DonationSyncError::Db(error)
    }
}

/// The whole legacy job for one run: fetch every journal page as the
/// service character and feed the incoming donations through
/// [`create_donation`].
pub async fn sync_wallet_donations(
    pool: &PgPool,
    esi: &EsiClient,
    sso: &SsoClient,
    character_id: i64,
) -> Result<WalletSyncStats, DonationSyncError> {
    let costs = PremiumCosts::from_env();
    let token = tokens::valid_access_token(pool, sso, character_id, scopes::READ_WALLET)
        .await?
        .ok_or(DonationSyncError::NoToken)?;

    let mut entries: Vec<EsiWalletJournalEntry> = Vec::new();
    let mut page = 1;
    loop {
        let (mut batch, pages) =
            match esi.wallet_journal(&token.access_token, character_id, page).await {
                Ok(result) => result,
                Err(error) => {
                    // ESI rejecting the token deletes it, like the
                    // legacy connector.
                    if matches!(error, EsiError::Forbidden(_)) {
                        tokens::delete_token(pool, token.token_id).await?;
                    }
                    return Err(DonationSyncError::Esi(error));
                }
            };
        entries.append(&mut batch);
        if page >= pages {
            break;
        }
        page += 1;
    }

    let mut stats = WalletSyncStats { entries: entries.len(), ..Default::default() };
    for entry in &entries {
        // The legacy filter: incoming player donations only (a null
        // amount is not `> 0` in PHP either).
        if entry.ref_type != PLAYER_DONATION_REF_TYPE
            || !entry.amount.is_some_and(|amount| amount > 0.0)
        {
            continue;
        }
        stats.donations += 1;
        if create_donation(pool, entry, costs).await? {
            stats.created += 1;
        }
    }

    Ok(stats)
}

/// The legacy `CreateDonationAction`: character stubs, firstOrCreate by
/// journal id, and on first creation the premium credit, the
/// confirmation flag and the queued mail — atomically.
///
/// Returns whether the donation was created by this call.
pub async fn create_donation(
    pool: &PgPool,
    entry: &EsiWalletJournalEntry,
    costs: PremiumCosts,
) -> Result<bool, DonationSyncError> {
    let donor_id = entry.first_party_id.ok_or(DonationSyncError::MissingSender(entry.id))?;
    let amount = entry.amount.unwrap_or(0.0);

    let mut tx = pool.begin().await?;

    // The legacy `Character::insertByIds([first, second])` stubs.
    let mut party_ids: Vec<i64> =
        [entry.first_party_id, entry.second_party_id].into_iter().flatten().collect();
    party_ids.sort_unstable();
    party_ids.dedup();
    sqlx::query("insert into characters (id) select unnest($1::bigint[]) on conflict do nothing")
        .bind(&party_ids)
        .execute(tx.as_mut())
        .await?;

    // firstOrCreate by journal_id: an existing entry ends the story, and
    // only a row created right now credits premium (`wasRecentlyCreated`).
    let existing: Option<i64> = sqlx::query_scalar("select id from donations where journal_id = $1")
        .bind(entry.id)
        .fetch_optional(tx.as_mut())
        .await?;
    if existing.is_some() {
        tx.commit().await?;
        return Ok(false);
    }

    let donation_id: i64 = sqlx::query_scalar(
        "insert into donations (journal_id, character_id, amount, date)
         values ($1, $2, $3, $4::timestamptz) returning id",
    )
    .bind(entry.id)
    .bind(donor_id)
    .bind(amount)
    .bind(&entry.date)
    .fetch_one(tx.as_mut())
    .await?;

    let update = premium::add_premium_to_character(tx.as_mut(), donor_id, amount, costs).await?;

    sqlx::query("update donations set confirmation_sent = true, updated_at = now() where id = $1")
        .bind(donation_id)
        .execute(tx.as_mut())
        .await?;

    // The legacy `$donation->character->user?->notify(...)`: only donors
    // with an account get the mail.
    let (donor_name, donor_user_id): (String, Option<i64>) =
        sqlx::query_as("select name, user_id from characters where id = $1")
            .bind(donor_id)
            .fetch_one(tx.as_mut())
            .await?;
    if let Some(user_id) = donor_user_id {
        let (subject, body) = donation_received_mail(&donor_name, amount, &update, costs);
        crate::notifications::queue_on(
            tx.as_mut(),
            user_id,
            DONATION_RECEIVED_KIND,
            &subject,
            &body,
            serde_json::json!({ "donation_id": donation_id, "journal_id": entry.id }),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(true)
}

/// The `mails/donation_received` blade template.
pub fn donation_received_mail(
    character_name: &str,
    amount: f64,
    update: &PremiumUpdate,
    costs: PremiumCosts,
) -> (String, String) {
    let middle = match update.paid_until {
        // The extended branch (`didExtendPremium`).
        Some(paid_until) => {
            let months = update.months_paid;
            let month_word = if months == 1 { "month" } else { "months" };
            let mut text = format!(
                "We've extended your premium status by {months} {month_word} until {}.",
                premium::format_ymd(paid_until),
            );
            if update.rest_amount > 0.0 {
                text.push_str(&format!(
                    "\n\nWe're holding {} ISK on your account toward your next month.",
                    format_isk(update.rest_amount),
                ));
            }
            text
        }
        None => format!(
            "Your donation has been saved to your account balance, which is now {} ISK.\n\n\
             Our premium plans are:\n\
             - Monthly: {} ISK/month\n\
             - Yearly: {} ISK/year (save 2 months!)\n\n\
             You need an additional {} ISK to unlock your next month of premium.",
            format_isk(update.rest_amount),
            format_isk(costs.monthly),
            format_isk(costs.yearly),
            format_isk(update.amount_needed_for_next_month),
        ),
    };

    let body = format!(
        "Hello {character_name},\n\n\
         Thank you for your donation of {} ISK!\n\n\
         {middle}\n\n\
         We appreciate your support!\n\
         The MutaMarket Team",
        format_isk(amount),
    );
    (DONATION_RECEIVED_SUBJECT.to_owned(), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::premium::{DEFAULT_MONTHLY_COST, DEFAULT_YEARLY_COST};

    const COSTS: PremiumCosts =
        PremiumCosts { monthly: DEFAULT_MONTHLY_COST, yearly: DEFAULT_YEARLY_COST };

    #[test]
    fn the_extended_mail_reports_months_and_held_rest() {
        let update = PremiumUpdate {
            months_paid: 1,
            rest_amount: 50_000_000.0,
            amount_needed_for_next_month: 0.0,
            // 2026-09-28 00:00:00 UTC.
            paid_until: Some(1_790_553_600),
        };
        let (subject, body) = donation_received_mail("Donor", 150_000_000.0, &update, COSTS);
        assert_eq!(subject, "Donation Received - Thank You!");
        assert_eq!(
            body,
            "Hello Donor,\n\n\
             Thank you for your donation of 150,000,000 ISK!\n\n\
             We've extended your premium status by 1 month until 2026-09-28.\n\n\
             We're holding 50,000,000 ISK on your account toward your next month.\n\n\
             We appreciate your support!\n\
             The MutaMarket Team",
        );
    }

    #[test]
    fn the_saved_up_mail_lists_the_plans_and_missing_amount() {
        let update = PremiumUpdate {
            months_paid: 0,
            rest_amount: 40_000_000.0,
            amount_needed_for_next_month: 60_000_000.0,
            paid_until: None,
        };
        let (_, body) = donation_received_mail("Donor", 40_000_000.0, &update, COSTS);
        assert_eq!(
            body,
            "Hello Donor,\n\n\
             Thank you for your donation of 40,000,000 ISK!\n\n\
             Your donation has been saved to your account balance, which is now 40,000,000 ISK.\n\n\
             Our premium plans are:\n\
             - Monthly: 100,000,000 ISK/month\n\
             - Yearly: 1,000,000,000 ISK/year (save 2 months!)\n\n\
             You need an additional 60,000,000 ISK to unlock your next month of premium.\n\n\
             We appreciate your support!\n\
             The MutaMarket Team",
        );
    }

    #[test]
    fn plural_months_read_naturally() {
        let update = PremiumUpdate {
            months_paid: 12,
            rest_amount: 0.0,
            amount_needed_for_next_month: 0.0,
            paid_until: Some(1_790_553_600),
        };
        let (_, body) = donation_received_mail("Donor", 1_000_000_000.0, &update, COSTS);
        assert!(body.contains("by 12 months until 2026-09-28."));
        assert!(!body.contains("We're holding"), "no rest line when nothing is held");
    }
}
