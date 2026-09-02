//! Server-side sessions in Postgres, addressed by a random token in an
//! HttpOnly cookie. The table holds the token's SHA-256, so a database
//! read or backup never yields a usable cookie.

use axum::http::HeaderMap;
use rand::Rng;
use sqlx::{PgPool, Row};

pub const SESSION_COOKIE: &str = "mm_session";
pub const OAUTH_STATE_COOKIE: &str = "mm_oauth_state";

/// Sessions live this long, like the legacy "remember me" login.
const SESSION_LIFETIME_DAYS: i32 = 30;

/// The OAuth state cookie only needs to survive the SSO round trip.
const OAUTH_STATE_LIFETIME_SECONDS: i64 = 600;

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: i64,
    pub active_character_id: Option<i64>,
}

/// 32 random bytes, hex encoded.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The stored form of a session token.
pub fn token_hash(token: &str) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub async fn create_session(
    pool: &PgPool,
    user_id: i64,
    active_character_id: Option<i64>,
) -> sqlx::Result<String> {
    let token = random_token();

    // Expired sessions are dead tokens; every login sweeps them.
    sqlx::query("delete from sessions where expires_at < now()")
        .execute(pool)
        .await?;

    sqlx::query(
        "insert into sessions (token, user_id, active_character_id, expires_at)
         values ($1, $2, $3, now() + make_interval(days => $4))",
    )
    .bind(token_hash(&token))
    .bind(user_id)
    .bind(active_character_id)
    .bind(SESSION_LIFETIME_DAYS)
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn session_by_token(pool: &PgPool, token: &str) -> sqlx::Result<Option<Session>> {
    let row = sqlx::query(
        "select token, user_id, active_character_id
         from sessions
         where token = $1 and expires_at > now()",
    )
    .bind(token_hash(token))
    .fetch_optional(pool)
    .await?;

    let session = row.map(|row| Session {
        token: row.get("token"),
        user_id: row.get("user_id"),
        active_character_id: row.get("active_character_id"),
    });

    if let Some(session) = &session {
        touch_activity(pool, session.user_id).await?;
    }

    Ok(session)
}

/// How stale `users.last_active_at` may get before a request refreshes
/// it. The legacy `MonitorUserActivity` wrote on every request; its only
/// reader is the raffle draw's multi-day activity window, so throttling
/// keeps that answer identical without a write per request.
const ACTIVITY_REFRESH_INTERVAL: &str = "5 minutes";

async fn touch_activity(pool: &PgPool, user_id: i64) -> sqlx::Result<()> {
    sqlx::query(&format!(
        "update users set last_active_at = now()
         where id = $1
           and (last_active_at is null
                or last_active_at < now() - interval '{ACTIVITY_REFRESH_INTERVAL}')",
    ))
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_session(pool: &PgPool, token: &str) -> sqlx::Result<()> {
    sqlx::query("delete from sessions where token = $1")
        .bind(token_hash(token))
        .execute(pool)
        .await?;

    Ok(())
}

/// The session of the request's cookie, if there is a live one.
pub async fn session_from_headers(
    pool: &PgPool,
    headers: &HeaderMap,
) -> sqlx::Result<Option<Session>> {
    match cookie_value(headers, SESSION_COOKIE) {
        Some(token) => session_by_token(pool, &token).await,
        None => Ok(None),
    }
}

/// The user behind a session token, without the `last_active_at` write
/// `session_by_token` does. The activity middleware runs on every
/// request and must not turn a read into a write.
pub async fn session_user_id(pool: &PgPool, token: &str) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar("select user_id from sessions where token = $1 and expires_at > now()")
        .bind(token_hash(token))
        .fetch_optional(pool)
        .await
}

/// Reads a cookie from the request headers.
pub fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;

    cookies.split(';').find_map(|pair| {
        let (cookie_name, value) = pair.trim().split_once('=')?;
        (cookie_name == name).then(|| value.to_owned())
    })
}

/// `; Secure` outside a local environment: production is https-only, so
/// the browser must never send the token over plain http first.
pub fn secure_flag() -> &'static str {
    if crate::environment::is_local() {
        ""
    } else {
        "; Secure"
    }
}

pub fn session_cookie(token: &str) -> String {
    let max_age = i64::from(SESSION_LIFETIME_DAYS) * 24 * 60 * 60;
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",
        secure_flag()
    )
}

pub fn oauth_state_cookie(state: &str) -> String {
    format!(
        "{OAUTH_STATE_COOKIE}={state}; Path=/; HttpOnly; SameSite=Lax; Max-Age={OAUTH_STATE_LIFETIME_SECONDS}{}",
        secure_flag()
    )
}

pub fn clear_cookie(name: &str) -> String {
    format!(
        "{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        secure_flag()
    )
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, header};

    use super::cookie_value;

    #[test]
    fn cookies_parse_out_of_the_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "other=1; mm_session=abc123; last=x"
                .parse()
                .expect("header"),
        );

        assert_eq!(
            cookie_value(&headers, "mm_session"),
            Some("abc123".to_owned())
        );
        assert_eq!(cookie_value(&headers, "missing"), None);
    }
}
