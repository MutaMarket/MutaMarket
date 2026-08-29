//! The Patreon subscriber sync, ported from the legacy
//! `GetPatreonSubscribers` command + `UpdateSubscribedPatreonMembers`
//! action: walk every campaign of the creator access token, fetch each
//! member's currently entitled tiers, and flag the users whose linked
//! `patreon_id` sits in a premium tier — everyone else flagged loses the
//! flag. The flag only feeds the Patreon badge (`is_premium` in the
//! legacy `PatreonDetailsResource`); like legacy, it does not touch
//! character premium.
//!
//! Legacy quirks ported faithfully:
//! - Only the first members page (up to [`MEMBERS_PAGE_SIZE`]) is
//!   fetched; the legacy command never followed the cursor.
//! - The campaign detail request is made just to re-read the campaign id
//!   the members listing uses.
//! - A member without a linked user in the API response crashed the
//!   legacy command (TypeError past the campaign-list try/catch); here it
//!   fails the run.

use serde_json::Value;
use sqlx::PgPool;

/// The Patreon v2 API base the legacy `patreon/patreon` library used
/// (the `www` host, unlike the OAuth identity endpoint).
pub const DEFAULT_CAMPAIGN_API_BASE_URL: &str = "https://www.patreon.com/api/oauth2/v2";

/// Members fetched per campaign: the legacy command asked for one page
/// of 1000 and never paged further.
pub const MEMBERS_PAGE_SIZE: u32 = 1000;

/// The creator access token, the legacy `services.patreon.access_token`.
pub const ACCESS_TOKEN_ENV: &str = "PATREON_ACCESS_TOKEN";

/// Comma-separated premium tier ids, the legacy
/// `services.patreon.premium_tiers` (`PATREON_PREMIUM_TIERS`).
pub const PREMIUM_TIERS_ENV: &str = "PATREON_PREMIUM_TIERS";

#[derive(Debug)]
pub enum PatreonError {
    Http(reqwest::Error),
    UnexpectedStatus(reqwest::StatusCode),
    /// A field the legacy code read unconditionally is missing.
    Malformed(&'static str),
    Db(sqlx::Error),
}

impl std::fmt::Display for PatreonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatreonError::Http(error) => write!(f, "Patreon request failed: {error}"),
            PatreonError::UnexpectedStatus(status) => {
                write!(f, "unexpected Patreon status: {status}")
            }
            PatreonError::Malformed(what) => write!(f, "malformed Patreon response: {what}"),
            PatreonError::Db(error) => write!(f, "database error: {error}"),
        }
    }
}

impl std::error::Error for PatreonError {}

impl From<reqwest::Error> for PatreonError {
    fn from(error: reqwest::Error) -> Self {
        PatreonError::Http(error)
    }
}

impl From<sqlx::Error> for PatreonError {
    fn from(error: sqlx::Error) -> Self {
        PatreonError::Db(error)
    }
}

/// One campaign member with what the sync reads: their entitled tier ids
/// and their Patreon user id (the legacy `MemberData`).
#[derive(Debug, Clone)]
pub struct PatreonMember {
    pub tier_ids: Vec<i64>,
    pub user_id: i64,
}

/// The campaign API client authenticated with the creator access token
/// (the legacy `Patreon\API` wrapper).
#[derive(Clone)]
pub struct PatreonCampaignClient {
    http: reqwest::Client,
    base_url: String,
    access_token: String,
}

impl PatreonCampaignClient {
    pub fn new(base_url: &str, access_token: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            access_token: access_token.to_owned(),
        }
    }

    /// The production client; `None` without a configured access token
    /// (the scheduler job then skips, like an unconfigured legacy env).
    pub fn from_env() -> Option<Self> {
        let access_token = std::env::var(ACCESS_TOKEN_ENV)
            .ok()
            .filter(|token| !token.is_empty())?;
        Some(Self::new(DEFAULT_CAMPAIGN_API_BASE_URL, &access_token))
    }

    async fn get(&self, suffix: &str, endpoint: &'static str) -> Result<Value, PatreonError> {
        let response = self
            .http
            .get(format!("{}/{suffix}", self.base_url))
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        if !response.status().is_success() {
            tracing::warn!("patreon: {endpoint} answered {}", response.status());
            return Err(PatreonError::UnexpectedStatus(response.status()));
        }
        Ok(response.json().await?)
    }

    /// The creator's campaign ids (`fetch_campaigns`, ids cast to int
    /// like the legacy `CampaignList::fromArray`).
    pub async fn campaigns(&self) -> Result<Vec<i64>, PatreonError> {
        let body = self.get("campaigns", "campaigns").await?;
        body.get("data")
            .and_then(Value::as_array)
            .ok_or(PatreonError::Malformed("campaigns without data"))?
            .iter()
            .map(|campaign| id_as_int(campaign).ok_or(PatreonError::Malformed("campaign id")))
            .collect()
    }

    /// The campaign detail fetch, returning the string id the members
    /// listing is keyed by (`fetch_campaign_details`).
    pub async fn campaign_id(&self, campaign_id: i64) -> Result<String, PatreonError> {
        let body = self
            .get(
                &format!("campaigns/{campaign_id}?include=benefits,creator,goals,tiers"),
                "campaign details",
            )
            .await?;
        body.get("data")
            .and_then(|data| data.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(PatreonError::Malformed("campaign details without id"))
    }

    /// The first page of member ids
    /// (`fetch_page_of_members_from_campaign`, no cursor follow-up).
    pub async fn member_ids(&self, campaign_id: &str) -> Result<Vec<String>, PatreonError> {
        let body = self
            .get(
                &format!("campaigns/{campaign_id}/members?page%5Bcount%5D={MEMBERS_PAGE_SIZE}"),
                "campaign members",
            )
            .await?;
        body.get("data")
            .and_then(Value::as_array)
            .ok_or(PatreonError::Malformed("members without data"))?
            .iter()
            .map(|member| {
                member
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or(PatreonError::Malformed("member id"))
            })
            .collect()
    }

    /// One member's entitled tiers and user (`fetch_member_details`).
    pub async fn member(&self, member_id: &str) -> Result<PatreonMember, PatreonError> {
        let body = self
            .get(
                &format!(
                    "members/{member_id}?include=address,campaign,user,currently_entitled_tiers"
                ),
                "member details",
            )
            .await?;
        let data = body
            .get("data")
            .ok_or(PatreonError::Malformed("member details without data"))?;

        // The legacy fluent `collect(...)`: a missing tier list is empty.
        let tier_ids = data
            .pointer("/relationships/currently_entitled_tiers/data")
            .and_then(Value::as_array)
            .map(|tiers| tiers.iter().filter_map(id_as_int).collect())
            .unwrap_or_default();

        let user_id = data
            .pointer("/relationships/user/data")
            .and_then(id_as_int)
            .ok_or(PatreonError::Malformed("member without user"))?;

        Ok(PatreonMember { tier_ids, user_id })
    }
}

/// The PHP `(int)` cast the legacy DTOs applied to the JSON:API string
/// ids.
fn id_as_int(value: &Value) -> Option<i64> {
    match value.get("id") {
        Some(Value::String(id)) => id.parse().ok(),
        Some(Value::Number(id)) => id.as_i64(),
        _ => None,
    }
}

/// The premium tier ids from the env, the legacy
/// `explode(',', PATREON_PREMIUM_TIERS)` (loose string-to-int matching
/// means non-numeric pieces can never match a tier; they are dropped).
pub fn parse_premium_tiers(raw: &str) -> Vec<i64> {
    raw.split(',')
        .filter_map(|piece| piece.trim().parse().ok())
        .collect()
}

pub fn premium_tiers_from_env() -> Vec<i64> {
    parse_premium_tiers(&std::env::var(PREMIUM_TIERS_ENV).unwrap_or_default())
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PatreonSyncStats {
    pub campaigns: usize,
    pub members: usize,
    pub premium_members: usize,
}

/// The whole legacy command for one run: collect the premium members'
/// Patreon user ids across every campaign, then flip the flags in one
/// transaction (the legacy `UpdateSubscribedPatreonMembers`).
pub async fn sync_patreon_subscribers(
    pool: &PgPool,
    client: &PatreonCampaignClient,
    premium_tiers: &[i64],
) -> Result<PatreonSyncStats, PatreonError> {
    let campaigns = client.campaigns().await?;

    let mut stats = PatreonSyncStats {
        campaigns: campaigns.len(),
        ..Default::default()
    };
    let mut premium_user_ids: Vec<i64> = Vec::new();
    for campaign in campaigns {
        let campaign_id = client.campaign_id(campaign).await?;
        for member_id in client.member_ids(&campaign_id).await? {
            let member = client.member(&member_id).await?;
            stats.members += 1;
            if member
                .tier_ids
                .iter()
                .any(|tier| premium_tiers.contains(tier))
            {
                premium_user_ids.push(member.user_id);
            }
        }
    }
    stats.premium_members = premium_user_ids.len();

    let mut tx = pool.begin().await?;
    // `<> all` mirrors the legacy `whereNotIn` on both edges: a null
    // patreon_id compares to null against a non-empty list (the user
    // keeps the flag), while an empty list is vacuously true and
    // unflags everyone — exactly like Laravel's no-op empty NOT IN.
    // No updated_at bumps: the legacy query-builder updates bypassed
    // Eloquent's timestamps.
    sqlx::query(
        "update users set is_patreon_member = false
         where is_patreon_member and patreon_id <> all($1)",
    )
    .bind(&premium_user_ids)
    .execute(tx.as_mut())
    .await?;
    sqlx::query("update users set is_patreon_member = true where patreon_id = any($1)")
        .bind(&premium_user_ids)
        .execute(tx.as_mut())
        .await?;
    tx.commit().await?;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::parse_premium_tiers;

    #[test]
    fn premium_tiers_parse_like_the_legacy_explode() {
        assert_eq!(parse_premium_tiers("42,7"), vec![42, 7]);
        assert_eq!(parse_premium_tiers(""), Vec::<i64>::new());
        // Non-numeric pieces could never loosely equal a tier id.
        assert_eq!(parse_premium_tiers("42,,gold, 7 "), vec![42, 7]);
    }
}
