//! EVE mail ingestion, the legacy `app:get-mails` chain
//! (`GetEveMailsJob` → `CreateMailsAction` → `GetEveMailJob` →
//! `CreateMailAction`/`UpdateMailAction`/`SendModuleMailsAction`): the
//! service character's inbox is scanned every thirty seconds, mails are
//! stored with their involved characters, abyssal module links in new
//! mail bodies are imported and linked, the mail is marked read
//! in-game, and a "Modules processed" appraisal reply is queued for the
//! sender — the mail-based appraisal flow.
//!
//! Divergences, deliberate and local:
//! - Legacy fetched a mail's detail when its row was newly created and
//!   relied on queue retries; here a mail is processed while its stored
//!   body is null, so an interrupted run simply retries (an ESI body of
//!   null is stored as '' to terminate). Legacy replies were sent
//!   inline in production only; ours queue through the notification
//!   outbox, whose delivery job simulates outside production.
//! - A module link ESI cannot resolve is logged and skipped instead of
//!   failing the mail (the legacy dispatchSync threw, burned the queue
//!   retries and left the mail permanently unprocessed).
//! - The mail scopes are part of the admin login set (see
//!   `auth::scopes`; an earlier claim that CCP retired them was wrong).
//!   The sync still reports itself skipped while the service character
//!   holds no token carrying the read scope.

use sqlx::PgPool;

use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiClient, EsiError};
use crate::estimator::Estimator;
use crate::modules::ingest::import_module;
use crate::mutation::reference::ReferenceData;

/// Outbox kind of the appraisal replies.
pub const MAIL_REPLY_KIND: &str = "modules-processed";

/// Modules per reply mail, the legacy `SendModuleMailsAction`
/// `chunkById(10)`.
const REPLY_CHUNK: usize = 10;

#[derive(Debug, Default, Clone, Copy)]
pub struct MailSyncStats {
    /// Headers in the inbox scan.
    pub mails: usize,
    /// Mails processed this run (stored without a body yet).
    pub new: usize,
    /// Modules linked from processed mail bodies.
    pub modules: usize,
    /// Appraisal replies queued into the outbox.
    pub replies: usize,
    /// Mails whose detail fetch failed (retried next run).
    pub failed: usize,
}

#[derive(Debug)]
pub enum MailSyncError {
    Esi(EsiError),
    Db(sqlx::Error),
    Token(TokenError),
}

impl std::fmt::Display for MailSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailSyncError::Esi(error) => write!(f, "ESI: {error}"),
            MailSyncError::Db(error) => write!(f, "database: {error}"),
            MailSyncError::Token(error) => write!(f, "token: {error:?}"),
        }
    }
}

impl std::error::Error for MailSyncError {}

impl From<EsiError> for MailSyncError {
    fn from(error: EsiError) -> Self {
        MailSyncError::Esi(error)
    }
}

impl From<sqlx::Error> for MailSyncError {
    fn from(error: sqlx::Error) -> Self {
        MailSyncError::Db(error)
    }
}

/// One inbox scan for the service character. `Ok(None)` when the
/// character holds no token with the mail read scope (the job reports
/// itself skipped).
pub async fn sync_eve_mails(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    estimator: &Estimator,
    character_id: i64,
    mut progress: impl FnMut(String),
) -> Result<Option<MailSyncStats>, MailSyncError> {
    let Some(token) = tokens::valid_access_token(pool, sso, character_id, scopes::READ_MAIL)
        .await
        .map_err(MailSyncError::Token)?
    else {
        return Ok(None);
    };

    let headers = esi.mail_headers(&token.access_token, character_id).await?;
    let mut stats = MailSyncStats { mails: headers.len(), ..Default::default() };

    // The abyssal module types a link may point at, the legacy
    // `JobCacheService::getAbyssalTypeIds`.
    let abyssal_types: Vec<i64> =
        sqlx::query_scalar("select distinct output_type_id from mutaplasmids")
            .fetch_all(pool)
            .await?;

    for (index, header) in headers.iter().enumerate() {
        progress(format!(
            "mail {}/{} (id {}): {} modules so far",
            index + 1,
            stats.mails,
            header.mail_id,
            stats.modules,
        ));

        // The involved characters: character-type recipients plus the
        // sender, all as stub rows (CreateMailsAction).
        let mut involved: Vec<i64> = header
            .recipients
            .iter()
            .filter(|recipient| recipient.recipient_type == "character")
            .map(|recipient| recipient.recipient_id)
            .collect();
        involved.push(header.from);
        for involved_id in &involved {
            sqlx::query(
                "insert into characters (id, name) values ($1, '') on conflict (id) do nothing",
            )
            .bind(involved_id)
            .execute(pool)
            .await?;
        }

        // The header upsert stores subject/timestamp/sender only, like
        // the legacy updateOrCreate; body and is_read arrive with the
        // detail. A null body marks the mail as not yet processed.
        let unprocessed: bool = sqlx::query_scalar(
            "insert into eve_mails (id, subject, timestamp, character_id)
             values ($1, $2, $3::timestamptz, $4)
             on conflict (id) do update set
                 subject = excluded.subject,
                 timestamp = excluded.timestamp,
                 character_id = excluded.character_id,
                 updated_at = now()
             returning body is null",
        )
        .bind(header.mail_id)
        .bind(&header.subject)
        .bind(&header.timestamp)
        .bind(header.from)
        .fetch_one(pool)
        .await?;

        // The recipients pivot sync (detaches stale rows, like sync()).
        sqlx::query(
            "delete from eve_mail_recipients
             where eve_mail_id = $1 and character_id != all($2)",
        )
        .bind(header.mail_id)
        .bind(&involved)
        .execute(pool)
        .await?;
        for involved_id in &involved {
            sqlx::query(
                "insert into eve_mail_recipients (eve_mail_id, character_id)
                 values ($1, $2) on conflict (eve_mail_id, character_id) do nothing",
            )
            .bind(header.mail_id)
            .bind(involved_id)
            .execute(pool)
            .await?;
        }

        if !unprocessed {
            continue;
        }

        match process_mail(
            pool,
            reference,
            esi,
            sso,
            estimator,
            &token.access_token,
            character_id,
            header.mail_id,
            &abyssal_types,
        )
        .await
        {
            Ok(processed) => {
                stats.new += 1;
                stats.modules += processed.modules;
                stats.replies += processed.replies;
            }
            Err(error) => {
                stats.failed += 1;
                tracing::warn!("eve mail {} failed: {error}", header.mail_id);
            }
        }
    }

    Ok(Some(stats))
}

struct ProcessedMail {
    modules: usize,
    replies: usize,
}

/// The legacy `GetEveMailJob` for one mail: store the full detail, import
/// and link the abyssal modules in the body, mark the mail read (locally
/// and in-game) and queue the appraisal replies.
#[allow(clippy::too_many_arguments)]
async fn process_mail(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    estimator: &Estimator,
    access_token: &str,
    character_id: i64,
    mail_id: i64,
    abyssal_types: &[i64],
) -> Result<ProcessedMail, MailSyncError> {
    let detail = esi.mail(access_token, character_id, mail_id).await?;
    let body = detail.body.clone().unwrap_or_default();

    sqlx::query(
        "insert into characters (id, name) values ($1, '') on conflict (id) do nothing",
    )
    .bind(detail.from)
    .execute(pool)
    .await?;
    sqlx::query(
        "update eve_mails
         set subject = $2, timestamp = $3::timestamptz, is_read = $4, character_id = $5,
             body = $6, updated_at = now()
         where id = $1",
    )
    .bind(mail_id)
    .bind(&detail.subject)
    .bind(&detail.timestamp)
    .bind(detail.read)
    .bind(detail.from)
    .bind(&body)
    .execute(pool)
    .await?;

    // Import the linked abyssal modules; unresolvable links are skipped
    // (see the module divergence note).
    let mut linked: Vec<i64> = Vec::new();
    for (type_id, item_id) in module_links(&body) {
        if !abyssal_types.contains(&type_id) {
            continue;
        }
        match import_module(pool, reference, esi, estimator, type_id, item_id).await {
            Ok(()) => {
                if !linked.contains(&item_id) {
                    linked.push(item_id);
                }
            }
            Err(error) => {
                tracing::warn!("mail {mail_id} module {type_id}//{item_id} skipped: {error}");
            }
        }
    }

    sqlx::query("delete from eve_mail_module where eve_mail_id = $1 and module_id != all($2)")
        .bind(mail_id)
        .bind(&linked)
        .execute(pool)
        .await?;
    for module_id in &linked {
        sqlx::query(
            "insert into eve_mail_module (eve_mail_id, module_id)
             values ($1, $2) on conflict (eve_mail_id, module_id) do nothing",
        )
        .bind(mail_id)
        .bind(module_id)
        .execute(pool)
        .await?;
    }

    // A mail already read in-game gets no reply (the legacy fresh
    // is_read early return).
    if detail.read {
        return Ok(ProcessedMail { modules: linked.len(), replies: 0 });
    }

    // Mark it read locally and in-game; the ESI failure is only logged,
    // like the legacy UpdateMailAction.
    sqlx::query("update eve_mails set is_read = true, updated_at = now() where id = $1")
        .bind(mail_id)
        .execute(pool)
        .await?;
    match tokens::valid_access_token(pool, sso, character_id, scopes::ORGANIZE_MAIL).await {
        Ok(Some(organize)) => {
            if let Err(error) =
                esi.set_mail_read(&organize.access_token, character_id, mail_id).await
            {
                tracing::warn!("marking mail {mail_id} read on ESI failed: {error}");
            }
        }
        Ok(None) => tracing::warn!("no organize-mail token; mail {mail_id} stays unread on ESI"),
        Err(error) => tracing::warn!("organize-mail token for mail {mail_id} failed: {error:?}"),
    }

    // The appraisal replies, chunked like the legacy chunkById(10). No
    // modules, no mail.
    let sender_name: String = sqlx::query_scalar("select name from characters where id = $1")
        .bind(detail.from)
        .fetch_one(pool)
        .await?;
    let modules: Vec<(i64, i64, String, Option<f64>)> = sqlx::query_as(
        "select m.id, m.type_id, t.name, m.estimated_value
         from modules m join types t on t.id = m.type_id
         where m.id = any($1) order by m.id",
    )
    .bind(&linked)
    .fetch_all(pool)
    .await?;

    let mut replies = 0usize;
    for chunk in modules.chunks(REPLY_CHUNK) {
        let (subject, reply_body) = modules_processed_mail(&sender_name, chunk);
        crate::notifications::queue_for_character(
            pool,
            detail.from,
            MAIL_REPLY_KIND,
            &subject,
            &reply_body,
            serde_json::json!({ "mail_id": mail_id }),
        )
        .await?;
        replies += 1;
    }

    Ok(ProcessedMail { modules: linked.len(), replies })
}

/// Every in-game module link in a text, the legacy `ModuleLink::allFrom`
/// pattern `showinfo:{type_id}//{item_id}`.
pub fn module_links(text: &str) -> Vec<(i64, i64)> {
    const MARKER: &str = "showinfo:";
    let mut links = Vec::new();
    let mut rest = text;
    while let Some(position) = rest.find(MARKER) {
        rest = &rest[position + MARKER.len()..];
        let Some((type_id, after_type)) = leading_i64(rest) else { continue };
        let Some(after_slashes) = after_type.strip_prefix("//") else { continue };
        let Some((item_id, _)) = leading_i64(after_slashes) else { continue };
        links.push((type_id, item_id));
    }
    links
}

fn leading_i64(text: &str) -> Option<(i64, &str)> {
    let end = text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    text[..end].parse().ok().map(|value| (value, &text[end..]))
}

/// The `mails/eve_mail_processed` blade template: the greeting, one
/// three-line entry per module (in-game link, site link, value line),
/// and the sign-off.
pub fn modules_processed_mail(
    sender_name: &str,
    modules: &[(i64, i64, String, Option<f64>)],
) -> (String, String) {
    let subject = "Modules processed".to_owned();
    let list = modules
        .iter()
        .map(|(module_id, type_id, type_name, estimated_value)| {
            let value_line = match estimated_value {
                // PHP truthiness: a zero estimate reads as no value.
                Some(value) if *value != 0.0 => format!("{} ISK", number_for_humans(*value)),
                _ => "No estimated value available".to_owned(),
            };
            format!(
                "<a href=\"showinfo:{type_id}//{module_id}\">{type_name}</a>\n\
                 <a href=\"https://mutamarket.com/modules/{module_id}\">[View on MutaMarket]</a>\n\
                 {value_line}\n",
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "Hello {sender_name},\n\n\
         We successfully processed your mail with the following modules:\n\n\
         {list}\n\
         Sincerely,\nThe MutaMarket Team",
    );
    (subject, body)
}

/// Laravel's `Number::forHumans` at the template's default precision 0:
/// the value scaled to its largest named power of a thousand, rounded
/// and thousands-grouped (999999 renders as "1,000 thousand", exactly
/// like the original).
pub fn number_for_humans(value: f64) -> String {
    const UNITS: [(i32, &str); 5] = [
        (15, "quadrillion"),
        (12, "trillion"),
        (9, "billion"),
        (6, "million"),
        (3, "thousand"),
    ];
    if value == 0.0 {
        return "0".to_owned();
    }
    let exponent = value.abs().log10().floor() as i32;
    match UNITS.into_iter().find(|(power, _)| exponent >= *power) {
        Some((power, unit)) => {
            format!("{} {unit}", crate::notifications::format_isk(value / 10f64.powi(power)))
        }
        None => crate::notifications::format_isk(value),
    }
}

#[cfg(test)]
mod tests {
    use super::{module_links, number_for_humans};

    #[test]
    fn links_parse_like_the_legacy_pattern() {
        assert_eq!(
            module_links("look: <a href=\"showinfo:47736//1035000000001\">mod</a> and showinfo:47740//1035000000002"),
            vec![(47736, 1035000000001), (47740, 1035000000002)],
        );
        assert_eq!(module_links("showinfo:123/456 showinfo:x//1 showinfo:12//"), vec![]);
        assert_eq!(
            module_links("showinfo:1//2showinfo:1//2"),
            vec![(1, 2), (1, 2)],
            "duplicates survive, like preg_match_all",
        );
    }

    #[test]
    fn for_humans_matches_laravel() {
        assert_eq!(number_for_humans(0.0), "0");
        assert_eq!(number_for_humans(489.0), "489");
        assert_eq!(number_for_humans(1_230_000.0), "1 million");
        assert_eq!(number_for_humans(1_500_000_000.0), "2 billion");
        assert_eq!(number_for_humans(999_999.0), "1,000 thousand");
        assert_eq!(number_for_humans(45_000_000_000_000.0), "45 trillion");
    }
}
