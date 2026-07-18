//! Postgres access: connection pool and migrations.

pub mod reference;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// The local docker-compose Postgres (see `docker-compose.yml`).
pub const DEFAULT_DATABASE_URL: &str = "postgres://mutamarket:mutamarket@127.0.0.1:5433/mutamarket";

pub fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned())
}

pub async fn connect() -> sqlx::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url())
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}
