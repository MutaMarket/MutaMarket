//! Postgres access: connection pool and migrations.

pub mod reference;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// The local docker-compose Postgres (see `docker-compose.yml`).
pub const DEFAULT_DATABASE_URL: &str = "postgres://mutamarket:mutamarket@127.0.0.1:5433/mutamarket";

/// Enough for the web handlers plus background work without exhausting
/// Postgres' default connection limit alongside tests and tooling.
const MAX_POOL_CONNECTIONS: u32 = 10;

pub fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned())
}

pub async fn connect() -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(&database_url())
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}

/// The `sde_meta` key recording the seeded SDE build number.
const SDE_BUILD_KEY: &str = "seeded_build";

/// The SDE build the database was last seeded from, if any.
pub async fn seeded_sde_build(pool: &PgPool) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("select value from sde_meta where key = $1")
        .bind(SDE_BUILD_KEY)
        .fetch_optional(pool)
        .await
}

/// Records the seeded SDE build, so unchanged bootstraps can skip.
pub async fn record_sde_build(pool: &PgPool, build: &str) -> sqlx::Result<()> {
    sqlx::query(
        "insert into sde_meta (key, value) values ($1, $2)
         on conflict (key) do update set value = excluded.value",
    )
    .bind(SDE_BUILD_KEY)
    .bind(build)
    .execute(pool)
    .await?;

    Ok(())
}

/// Where the integration tests make their databases: they connect to this
/// server and clone a template per test, so test seeding never touches
/// development data.
pub const DEFAULT_TEST_DATABASE_URL: &str =
    "postgres://mutamarket:mutamarket@127.0.0.1:5433/mutamarket_test";

pub fn test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_TEST_DATABASE_URL.to_owned())
}
