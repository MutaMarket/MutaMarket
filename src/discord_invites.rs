//! Discord invite member counts, the legacy `DiscordWidgetService` plus
//! the `DiscordInvites` shared-prop middleware. Counts come from
//! Discord's invite API (`/invites/{code}?with_counts=true`,
//! `approximate_member_count`) and every failure reads as null, like
//! the legacy nullable fetch.
//!
//! Deliberate divergence: the legacy cached each count per request for
//! 24 hours (`Cache::remember('discord_member_count:{code}')`); the
//! rewrite never calls external services during request handling, so
//! the scheduler's `discord-member-counts` job refreshes the counts on
//! the same 24-hour cadence and persists them in `app_settings` under
//! the legacy cache key. The sidebar payload only reads those rows.

use sqlx::PgPool;

use crate::app_settings;

/// The invite URL env vars, the legacy `config/services.php` names:
/// `services.abyssal_trading.invite`, `services.discord.invite` and
/// `services.ectrade.invite`.
pub const ABYSSAL_TRADING_INVITE_ENV: &str = "ABYSSAL_TRADING_INVITE";
pub const DISCORD_INVITE_ENV: &str = "DISCORD_INVITE";
pub const ECTRADE_INVITE_ENV: &str = "ECTRADE_INVITE";

/// The `app_settings` key prefix for a stored count, mirroring the
/// legacy cache key `discord_member_count:{invite_code}`.
pub const MEMBER_COUNT_KEY_PREFIX: &str = "discord_member_count:";

/// One partner Discord of the legacy `DiscordInvites` middleware.
pub struct InviteDefinition {
    pub name: &'static str,
    pub url_env: &'static str,
    pub image: Option<&'static str>,
}

/// The legacy middleware's three invites, in its order.
pub const INVITES: [InviteDefinition; 3] = [
    InviteDefinition {
        name: "Abyssal Trading",
        url_env: ABYSSAL_TRADING_INVITE_ENV,
        image: Some("/img/at.webp"),
    },
    InviteDefinition { name: "MutaMarket", url_env: DISCORD_INVITE_ENV, image: None },
    InviteDefinition {
        name: "EC Trade",
        url_env: ECTRADE_INVITE_ENV,
        image: Some("/img/ectrade.png"),
    },
];

/// A configured invite URL; unset or empty env reads as unconfigured
/// (the legacy `env()` null).
pub fn invite_url(definition: &InviteDefinition) -> Option<String> {
    std::env::var(definition.url_env).ok().filter(|url| !url.is_empty())
}

/// The invite code of a URL, the legacy `extractInviteCode`: the
/// alphanumeric run after `discord.gg/` anywhere in the string, or the
/// string itself when it is entirely alphanumeric.
pub fn extract_invite_code(url: &str) -> Option<String> {
    const MARKER: &str = "discord.gg/";
    for (index, _) in url.match_indices(MARKER) {
        let code: String = url[index + MARKER.len()..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if !code.is_empty() {
            return Some(code);
        }
    }
    (!url.is_empty() && url.chars().all(|character| character.is_ascii_alphanumeric()))
        .then(|| url.to_owned())
}

/// The stored member count for an invite URL; null when the code never
/// resolved or the last refresh found no count.
pub async fn stored_member_count(pool: &PgPool, url: &str) -> sqlx::Result<Option<i64>> {
    let Some(code) = extract_invite_code(url) else {
        return Ok(None);
    };
    let value = app_settings::get(pool, &format!("{MEMBER_COUNT_KEY_PREFIX}{code}")).await?;
    Ok(value.and_then(|value| value.parse().ok()))
}

/// One count from the invite API; any failure is null, like the legacy
/// silently-failing `fetchMemberCount`.
async fn fetch_member_count(api_base_url: &str, code: &str) -> Option<i64> {
    let url = format!("{api_base_url}/invites/{code}?with_counts=true");
    let response = reqwest::get(url).await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    body.get("approximate_member_count")?.as_i64()
}

pub struct RefreshStats {
    pub stored: usize,
    pub unavailable: usize,
}

/// Refreshes the stored count of every given invite URL. A null fetch
/// clears the stored row, like the legacy caching the null result: the
/// count disappears until a refresh succeeds again.
pub async fn refresh_member_counts(
    pool: &PgPool,
    api_base_url: &str,
    invite_urls: &[String],
) -> sqlx::Result<RefreshStats> {
    let mut stats = RefreshStats { stored: 0, unavailable: 0 };
    for url in invite_urls {
        let Some(code) = extract_invite_code(url) else {
            stats.unavailable += 1;
            continue;
        };
        let key = format!("{MEMBER_COUNT_KEY_PREFIX}{code}");
        match fetch_member_count(api_base_url, &code).await {
            Some(count) => {
                app_settings::set(pool, &key, &count.to_string()).await?;
                stats.stored += 1;
            }
            None => {
                app_settings::remove(pool, &key).await?;
                stats.unavailable += 1;
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::extract_invite_code;

    #[test]
    fn invite_codes_extract_like_the_legacy_regexes() {
        assert_eq!(extract_invite_code("https://discord.gg/abc123").as_deref(), Some("abc123"));
        assert_eq!(extract_invite_code("https://discord.gg/abc123?x=1").as_deref(), Some("abc123"));
        assert_eq!(extract_invite_code("abc123").as_deref(), Some("abc123"));
        assert_eq!(extract_invite_code("https://discord.gg/"), None);
        assert_eq!(extract_invite_code("https://example.com/abc"), None);
        assert_eq!(extract_invite_code(""), None);
    }
}
