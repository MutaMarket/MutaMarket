//! Postgres roundtrip test for the reference data: migrate, seed from the
//! fixture dumps, read everything back, and require the DB-loaded reference
//! to pass the full 445-module characterization suite.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;

use mutamarket::db;
use mutamarket::db::reference::{load_reference, seed_reference};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};

#[tokio::test]
async fn postgres_roundtripped_reference_matches_the_legacy_snapshots() {
    let pool = db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");

    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");

    let roundtripped = load_reference(&pool).await.expect("load reference tables");

    common::assert_reference_matches_fixtures(&ReferenceData::from_tables(roundtripped));
}
