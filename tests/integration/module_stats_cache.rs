//! Behavior tests for the cached market-wide statistics: a reading is
//! reused within its window, the two `unlisted` variants are held apart,
//! and a fresh cache sees the current numbers.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::Arc;

use mutamarket::db;
use mutamarket::modules::stats::{ModuleStatsCache, cached_all_modules_stats};

/// A type and a module of this suite's own, so the counts move by
/// exactly one.
const TYPE_ID: i64 = 990_010_100;
const MODULE_ID: i64 = 990_010_000;

#[tokio::test]
async fn the_market_statistics_are_computed_once_per_window() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    sqlx::query("delete from modules where id = $1")
        .bind(MODULE_ID)
        .execute(&pool)
        .await
        .expect("clean module");
    sqlx::query(
        "insert into types (id, name, published) values ($1, 'Stats Cache Fixture', false)
         on conflict (id) do nothing",
    )
    .bind(TYPE_ID)
    .execute(&pool)
    .await
    .expect("seed type");

    let cache = Arc::new(ModuleStatsCache::default());
    let first = cached_all_modules_stats(&pool, &cache, false)
        .await
        .expect("stats read");

    sqlx::query("insert into modules (id, type_id) values ($1, $2)")
        .bind(MODULE_ID)
        .bind(TYPE_ID)
        .execute(&pool)
        .await
        .expect("seed module");

    // Within the window the held reading answers, module or no module.
    assert_eq!(
        cached_all_modules_stats(&pool, &cache, false)
            .await
            .expect("stats read"),
        first,
    );

    // The all-modules variant is its own reading, so it counts the new
    // module the moment it is asked.
    let unlisted = cached_all_modules_stats(&pool, &cache, true)
        .await
        .expect("stats read");
    assert_eq!(unlisted.total_count, first.total_count + 1);

    // A restart (a cache of its own) reads the current numbers.
    let fresh = Arc::new(ModuleStatsCache::default());
    let after = cached_all_modules_stats(&pool, &fresh, false)
        .await
        .expect("stats read");
    assert_eq!(after.total_count, first.total_count + 1);

    sqlx::query("delete from modules where id = $1")
        .bind(MODULE_ID)
        .execute(&pool)
        .await
        .expect("clean module");
}
