//! Admin-configured application settings, a small key/value store. The
//! first setting is the service character: the ESI character the admin
//! authorized through `/eve/admin`, used by structure resolution (and
//! by donation/wallet processing once those features are ported).

use sqlx::PgPool;

/// The service character's id, set by the admin authorize flow.
pub const SERVICE_CHARACTER_KEY: &str = "service_character_id";

pub async fn get(pool: &PgPool, key: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("select value from app_settings where key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
}

pub async fn set(pool: &PgPool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query(
        "insert into app_settings (key, value) values ($1, $2)
         on conflict (key) do update set value = excluded.value, updated_at = now()",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove(pool: &PgPool, key: &str) -> sqlx::Result<()> {
    sqlx::query("delete from app_settings where key = $1")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

/// The service character: the admin-authorized setting, falling back to
/// the legacy `EVE_STRUCTURES_CHARACTER_ID` env configuration.
pub async fn service_character_id(pool: &PgPool) -> sqlx::Result<Option<i64>> {
    if let Some(value) = get(pool, SERVICE_CHARACTER_KEY).await?
        && let Ok(id) = value.parse()
    {
        return Ok(Some(id));
    }
    Ok(std::env::var("EVE_STRUCTURES_CHARACTER_ID")
        .ok()
        .and_then(|value| value.parse().ok()))
}
