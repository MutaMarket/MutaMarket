//! The SDE bootstrap bookkeeping: the recorded build lets repeated
//! `sde_import` runs (docker compose up) skip an unchanged SDE.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use mutamarket::db;

#[tokio::test]
async fn the_seeded_sde_build_records_and_overwrites() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    sqlx::query("delete from sde_meta")
        .execute(&pool)
        .await
        .expect("clean meta");

    assert_eq!(db::seeded_sde_build(&pool).await.expect("read"), None);

    db::record_sde_build(&pool, "20260825")
        .await
        .expect("record");
    assert_eq!(
        db::seeded_sde_build(&pool).await.expect("read"),
        Some("20260825".to_owned())
    );

    db::record_sde_build(&pool, "20260901")
        .await
        .expect("overwrite");
    assert_eq!(
        db::seeded_sde_build(&pool).await.expect("read"),
        Some("20260901".to_owned())
    );
}
