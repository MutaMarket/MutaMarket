//! The admin-scope check, ported from the legacy hourly
//! `app:check-admin-scopes` command: verify the service character holds
//! tokens covering every scope of [`crate::auth::scopes::ADMIN_LOGIN`]
//! and alert the Discord webhook about missing ones. Without a webhook
//! the check still reports its result, like the legacy nullable
//! `services.discord.alert_webhook` config.
//!
//! Divergence, documented: the legacy alert listed the PHP enum case
//! names (`ReadWallet`); the rewrite keeps scopes as their ESI
//! identifiers, so those are what the alert lists.

use sqlx::PgPool;

use crate::auth::scopes;

/// Env var naming the Discord webhook the alert posts to, the legacy
/// `services.discord.alert_webhook` (`DISCORD_ALERT_WEBHOOK`). Unset
/// means check-only.
pub const ALERT_WEBHOOK_ENV: &str = "DISCORD_ALERT_WEBHOOK";

/// Absolute origin for the alert's grant link (the legacy
/// `route('auth.eve.admin')` rendered absolute on `app.url`), like the
/// sitemap origin.
const SITE_ORIGIN: &str = "https://mutamarket.com";

/// The alert embed's sidebar color, the legacy `hexdec('EF4444')`
/// (Tailwind red-500).
const ALERT_COLOR: u32 = 0xEF4444;

#[derive(Debug)]
pub enum ScopeCheckError {
    /// The configured service character has no character row, the
    /// legacy "Admin character not found" failure.
    CharacterNotFound,
    Db(sqlx::Error),
    Webhook(reqwest::Error),
}

impl std::fmt::Display for ScopeCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeCheckError::CharacterNotFound => write!(f, "Admin character not found"),
            ScopeCheckError::Db(error) => write!(f, "database: {error}"),
            ScopeCheckError::Webhook(error) => write!(f, "webhook: {error}"),
        }
    }
}

impl std::error::Error for ScopeCheckError {}

impl From<sqlx::Error> for ScopeCheckError {
    fn from(error: sqlx::Error) -> Self {
        ScopeCheckError::Db(error)
    }
}

/// What one check found (and did).
#[derive(Debug, PartialEq, Eq)]
pub struct ScopeCheckOutcome {
    /// Admin scopes no token of the character carries, in
    /// `ADMIN_LOGIN` order.
    pub missing: Vec<&'static str>,
    /// Whether a Discord alert was posted (scopes missing and a
    /// webhook configured).
    pub alerted: bool,
}

/// The whole legacy command for one run against the given character and
/// webhook (the scheduler job reads both from their config sources).
pub async fn check_admin_scopes(
    pool: &PgPool,
    character_id: i64,
    webhook_url: Option<&str>,
) -> Result<ScopeCheckOutcome, ScopeCheckError> {
    let exists: bool = sqlx::query_scalar("select exists (select 1 from characters where id = $1)")
        .bind(character_id)
        .fetch_one(pool)
        .await?;
    if !exists {
        return Err(ScopeCheckError::CharacterNotFound);
    }

    let held: Vec<String> = sqlx::query_scalar(
        "select distinct unnest(scopes) from esi_tokens where character_id = $1",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    let missing: Vec<&'static str> = scopes::ADMIN_LOGIN
        .into_iter()
        .filter(|scope| !held.iter().any(|name| name == scope))
        .collect();
    if missing.is_empty() {
        return Ok(ScopeCheckOutcome {
            missing,
            alerted: false,
        });
    }

    let Some(webhook_url) = webhook_url else {
        return Ok(ScopeCheckOutcome {
            missing,
            alerted: false,
        });
    };

    // The legacy `now()->toIso8601String()` embed timestamp, from
    // Postgres like the rest of the codebase's clock reads.
    let timestamp: String = sqlx::query_scalar(
        "select to_char(now() at time zone 'utc', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')",
    )
    .fetch_one(pool)
    .await?;
    post_alert(webhook_url, &missing, &timestamp)
        .await
        .map_err(ScopeCheckError::Webhook)?;

    Ok(ScopeCheckOutcome {
        missing,
        alerted: true,
    })
}

/// The legacy `Http::post` payload, verbatim except for the scope
/// identifiers (see the module doc).
async fn post_alert(
    webhook_url: &str,
    missing: &[&'static str],
    timestamp: &str,
) -> Result<(), reqwest::Error> {
    let bullets = missing.join("\n• ");
    let body = serde_json::json!({
        "content": "@everyone Admin character is missing ESI scopes!",
        "embeds": [{
            "title": "Missing Admin ESI Scopes",
            "description": format!(
                "The admin character is missing required ESI scopes.\n\n**Missing Scopes:**\n• {bullets}"
            ),
            "color": ALERT_COLOR,
            "fields": [{
                "name": "Action Required",
                "value": format!("[Grant Scopes]({SITE_ORIGIN}/eve/admin)"),
            }],
            "timestamp": timestamp,
        }],
    });

    reqwest::Client::new()
        .post(webhook_url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
