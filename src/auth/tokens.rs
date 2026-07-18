//! Access-token acquisition for authenticated ESI calls, ported from the
//! legacy `NicolasKion\Esi` connector: pick the character's newest stored
//! token carrying the required scope, refresh it lazily through the SSO
//! when it is (about to be) expired, persist the rotated tokens, and treat
//! a rejected refresh as a dead token by deleting the row.

use std::fmt;

use sqlx::{PgPool, Row};

use super::sso::{RefreshError, SsoClient};

/// A token counts as expired this long before its real `expires_at`, like
/// the legacy `EsiToken::isExpired` five-minute buffer.
const EXPIRY_BUFFER_MINUTES: i32 = 5;

/// A stored, currently valid access token.
#[derive(Debug, Clone)]
pub struct AccessToken {
    /// The `esi_tokens` row backing this token; ESI 401/403 responses
    /// delete it, like the legacy connector's `handleFailedResponse`.
    pub token_id: i64,
    pub access_token: String,
}

#[derive(Debug)]
pub enum TokenError {
    /// The SSO rejected the refresh token (revoked or invalid); the stored
    /// token has been deleted, like the legacy connector does.
    RefreshRejected {
        status: reqwest::StatusCode,
        body: String,
    },
    /// The SSO could not be reached; the stored token was left alone.
    Network(reqwest::Error),
    Db(sqlx::Error),
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::RefreshRejected { status, body } => {
                write!(f, "refresh token rejected ({status}): {body}")
            }
            TokenError::Network(error) => write!(f, "SSO unreachable: {error}"),
            TokenError::Db(error) => write!(f, "database error: {error}"),
        }
    }
}

impl std::error::Error for TokenError {}

impl From<sqlx::Error> for TokenError {
    fn from(error: sqlx::Error) -> Self {
        TokenError::Db(error)
    }
}

/// A valid access token for the character and scope, refreshed and
/// persisted on the way if the stored one is (nearly) expired — the
/// equivalent of the legacy `getEsiTokenWithScope` + connector refresh.
///
/// `Ok(None)` means the character holds no token with that scope (callers
/// skip the character, like the legacy jobs). A rejected refresh deletes
/// the stored row and errors: the character needs a fresh SSO login.
pub async fn valid_access_token(
    pool: &PgPool,
    sso: &SsoClient,
    character_id: i64,
    scope: &str,
) -> Result<Option<AccessToken>, TokenError> {
    // The newest token carrying the scope wins, like the legacy
    // `esiTokens()->whereHas(scope)->latest()->first()`.
    let row = sqlx::query(
        "select id, access_token, refresh_token,
                expires_at <= now() + make_interval(mins => $3) as expiring
         from esi_tokens
         where character_id = $1 and $2 = any(scopes)
         order by created_at desc, id desc
         limit 1",
    )
    .bind(character_id)
    .bind(scope)
    .bind(EXPIRY_BUFFER_MINUTES)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let token_id: i64 = row.get("id");
    if !row.get::<bool, _>("expiring") {
        return Ok(Some(AccessToken {
            token_id,
            access_token: row.get("access_token"),
        }));
    }

    match sso.refresh(row.get("refresh_token")).await {
        Ok(tokens) => {
            // EVE rotates the refresh token on every grant; persist both,
            // like the legacy connector's update (token_type is left as
            // stored, mirroring legacy).
            sqlx::query(
                "update esi_tokens
                 set access_token = $1, refresh_token = $2,
                     expires_at = now() + make_interval(secs => $3)
                 where id = $4",
            )
            .bind(&tokens.access_token)
            .bind(&tokens.refresh_token)
            .bind(tokens.expires_in as f64)
            .bind(token_id)
            .execute(pool)
            .await?;

            Ok(Some(AccessToken {
                token_id,
                access_token: tokens.access_token,
            }))
        }
        // Any failure status means the refresh token is dead: the legacy
        // connector hard-deletes the row so the character simply has no
        // token anymore until the next SSO login.
        Err(RefreshError::Rejected { status, body }) => {
            delete_token(pool, token_id).await?;
            Err(TokenError::RefreshRejected { status, body })
        }
        // Network failures do not condemn the token.
        Err(RefreshError::Http(error)) => Err(TokenError::Network(error)),
    }
}

/// Deletes a stored token, the reaction to ESI answering 401/403 with it
/// (legacy `Connector::handleFailedResponse`) or to a rejected refresh.
pub async fn delete_token(pool: &PgPool, token_id: i64) -> sqlx::Result<()> {
    sqlx::query("delete from esi_tokens where id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Every character holding a token with the given scope, oldest first —
/// the fan-out set of the character-scoped sync jobs (legacy
/// `Character::hasEsiTokenWithScope` filtering).
pub async fn characters_with_scope(pool: &PgPool, scope: &str) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select distinct character_id from esi_tokens where $1 = any(scopes) order by character_id",
    )
    .bind(scope)
    .fetch_all(pool)
    .await
}
