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
        let (mut batch, pages) = match esi
            .wallet_journal(&token.access_token, character_id, page)
            .await
        {
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

    let mut stats = WalletSyncStats {
        entries: entries.len(),
        ..Default::default()
    };
    // The legacy filter: incoming player donations only (a null amount
    // is not `> 0` in PHP either).
    let donations: Vec<&EsiWalletJournalEntry> = entries
        .iter()
        .filter(|entry| {
            entry.ref_type == PLAYER_DONATION_REF_TYPE
                && entry.amount.is_some_and(|amount| amount > 0.0)
        })
        .collect();
    stats.donations = donations.len();

    // Already-stored journal entries skip the per-entry firstOrCreate
    // transaction, so the minutely 30-day rescan costs one select in
    // steady state (like the mails pre-filter). A skipped entry is
    // exactly the existing-row early return of `create_donation`.
    let journal_ids: Vec<i64> = donations.iter().map(|entry| entry.id).collect();
    let known_ids: Vec<i64> =
        sqlx::query_scalar("select journal_id from donations where journal_id = any($1)")
            .bind(&journal_ids)
            .fetch_all(pool)
            .await?;

    for entry in donations {
        if known_ids.contains(&entry.id) {
            continue;
        }
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
    let donor_id = entry
        .first_party_id
        .ok_or(DonationSyncError::MissingSender(entry.id))?;
    let amount = entry.amount.unwrap_or(0.0);

    let mut tx = pool.begin().await?;

    // The legacy `Character::insertByIds([first, second])` stubs.
    let mut party_ids: Vec<i64> = [entry.first_party_id, entry.second_party_id]
        .into_iter()
        .flatten()
        .collect();
    party_ids.sort_unstable();
    party_ids.dedup();
    sqlx::query("insert into characters (id) select unnest($1::bigint[]) on conflict do nothing")
        .bind(&party_ids)
        .execute(tx.as_mut())
        .await?;

    // firstOrCreate by journal_id: an existing entry ends the story, and
    // only a row created right now credits premium (`wasRecentlyCreated`).
    let existing: Option<i64> =
        sqlx::query_scalar("select id from donations where journal_id = $1")
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

/// Latest-donations floor: only gifts over 10M ISK make the recent
/// activity list (the legacy `getLatestDonations` where-amount).
pub const LATEST_MIN_AMOUNT: f64 = 10_000_000.0;

/// Rows in the latest-donations list.
pub const LATEST_LIMIT: i64 = 5;

/// Rows in the all-time and 14-day top-donor lists.
pub const TOP_DONORS_LIMIT: i64 = 10;

/// The rolling window of the recent top-donor list, in days.
pub const RECENT_WINDOW_DAYS: i32 = 14;

/// The base-query filter of the legacy shared `Donations` middleware:
/// admin donations are hidden, ownerless characters pass.
const NON_ADMIN_FILTER: &str = "(c.user_id is null
    or exists (select 1 from users u where u.id = c.user_id and not u.is_admin))";

/// The `{latest, highest, recent}` lists of the legacy shared
/// `donations` prop, serialized with the exact `DonationResource` key
/// sets (aggregated rows carry no `date` unless selected, like
/// `whenHas`). The legacy 300-second cache is deliberately not ported:
/// three small indexed queries per sidebar load are fine for Postgres.
pub async fn donation_lists(pool: &PgPool) -> sqlx::Result<serde_json::Value> {
    type LatestRow = (
        i64,
        f64,
        String,
        i64,
        i64,
        String,
        Option<String>,
        bool,
        Option<i64>,
    );
    let latest: Vec<LatestRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select d.id, d.amount, d.date::text,
                (select count(*) from donations d2 where d2.character_id = d.character_id),
                c.id, c.name, c.description,
                (c.premium_paid_until is not null and c.premium_paid_until > now()),
                c.corporation_id
         from donations d
         join characters c on c.id = d.character_id
         where d.amount > $1 and {NON_ADMIN_FILTER}
         order by d.date desc
         limit $2",
    )))
    .bind(LATEST_MIN_AMOUNT)
    .bind(LATEST_LIMIT)
    .fetch_all(pool)
    .await?;

    type TopRow = (
        i64,
        f64,
        Option<String>,
        i64,
        i64,
        String,
        Option<String>,
        bool,
        Option<i64>,
    );
    let highest: Vec<TopRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select max(d.id), sum(d.amount)::double precision, null::text, count(*),
                c.id, c.name, c.description,
                (c.premium_paid_until is not null and c.premium_paid_until > now()),
                c.corporation_id
         from donations d
         join characters c on c.id = d.character_id
         where {NON_ADMIN_FILTER}
         group by c.id
         order by sum(d.amount) desc
         limit $1",
    )))
    .bind(TOP_DONORS_LIMIT)
    .fetch_all(pool)
    .await?;
    let recent: Vec<TopRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select max(d.id), sum(d.amount)::double precision, max(d.date)::text, count(*),
                c.id, c.name, c.description,
                (c.premium_paid_until is not null and c.premium_paid_until > now()),
                c.corporation_id
         from donations d
         join characters c on c.id = d.character_id
         where {NON_ADMIN_FILTER} and d.date >= now() - make_interval(days => $2)
         group by c.id
         order by sum(d.amount) desc
         limit $1",
    )))
    .bind(TOP_DONORS_LIMIT)
    .bind(RECENT_WINDOW_DAYS)
    .fetch_all(pool)
    .await?;

    let character_json = |id: i64,
                          name: &str,
                          description: &Option<String>,
                          has_premium: bool,
                          corporation_id: Option<i64>| {
        serde_json::json!({
            "id": id,
            "slug": crate::modules::view::module_slug(name, id),
            "name": name,
            "description": description,
            "has_premium": has_premium,
            "corporation_id": corporation_id,
        })
    };

    let latest: Vec<serde_json::Value> = latest
        .iter()
        .map(
            |(id, amount, date, count, cid, name, description, premium, corporation)| {
                serde_json::json!({
                    "id": id,
                    "amount": amount,
                    "date": date,
                    "character": character_json(*cid, name, description, *premium, *corporation),
                    "donation_count": count,
                })
            },
        )
        .collect();
    let top_json = |rows: &[TopRow]| -> Vec<serde_json::Value> {
        rows.iter()
            .map(|(id, amount, date, count, cid, name, description, premium, corporation)| {
                let mut entry = serde_json::json!({
                    "id": id,
                    "amount": amount,
                    "character": character_json(*cid, name, description, *premium, *corporation),
                    "donation_count": count,
                });
                // The recent list selects MAX(date); the all-time list
                // does not, so `whenHas` drops the key there.
                if let Some(date) = date {
                    entry["date"] = serde_json::json!(date);
                }
                entry
            })
            .collect()
    };

    Ok(serde_json::json!({
        "latest": latest,
        "highest": top_json(&highest),
        "recent": top_json(&recent),
    }))
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

    const COSTS: PremiumCosts = PremiumCosts {
        monthly: DEFAULT_MONTHLY_COST,
        yearly: DEFAULT_YEARLY_COST,
    };

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
        assert!(
            !body.contains("We're holding"),
            "no rest line when nothing is held"
        );
    }
}
