//! Schema-level settings that no request path would reveal: the per-table
//! autovacuum thresholds the hot tables carry.
//!
//! Needs the local database: `docker compose up -d postgres`.

use mutamarket::db;
use sqlx::Row;

/// The tables tuned in 20260911000000_tune_autovacuum.sql, with the
/// options they must carry after a migrate. Stock autovacuum let them
/// reach a fifth dead rows, which keeps index-only scans off the table.
const TUNED: [&str; 4] = ["assets", "modules", "characters", "character_contracts"];

#[tokio::test]
async fn the_churning_tables_vacuum_earlier_than_the_cluster_default() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    for table in TUNED {
        let options: Option<Vec<String>> =
            sqlx::query("select reloptions from pg_class where relname = $1")
                .bind(table)
                .fetch_one(&pool)
                .await
                .expect("read reloptions")
                .get("reloptions");
        let mut options = options.unwrap_or_default();
        options.sort();

        assert_eq!(
            options,
            [
                "autovacuum_analyze_scale_factor=0.02",
                "autovacuum_vacuum_scale_factor=0.05",
            ],
            "{table} lost its autovacuum thresholds",
        );
    }
}
