//! Postgres roundtrip test for the reference data: migrate, seed from the
//! fixture dumps, read everything back, and require the DB-loaded reference
//! to pass the full 445-module characterization suite.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;

use mutamarket::db;
use mutamarket::db::reference::{load_reference, seed_reference};
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

/// End-to-end check of the native SDE import: after `cargo run --bin
/// sde_import` has seeded live data, the DB content must still reproduce the
/// legacy characterization snapshots. Opt-in (`--ignored`) because it needs
/// that prior import and can legitimately drift when CCP changes the SDE.
#[tokio::test]
#[ignore = "requires a prior `cargo run --bin sde_import`"]
async fn live_imported_reference_matches_the_legacy_snapshots() {
    // Deliberately the development database: this checks the live import.
    let pool = db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");

    let live = load_reference(&pool).await.expect("load reference tables");

    common::assert_reference_matches_fixtures(&ReferenceData::from_tables(live));
}

#[tokio::test]
async fn postgres_roundtripped_reference_matches_the_legacy_snapshots() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");

    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");

    let roundtripped = load_reference(&pool).await.expect("load reference tables");

    common::assert_reference_matches_fixtures(&ReferenceData::from_tables(roundtripped));

    // Reseeding must be safe on a live database: the SDE updates regularly,
    // so a new import may never destroy ingested modules (a cascade from
    // truncated types once wiped them, plus contract items and market
    // histories).
    let reference = ReferenceData::from_tables(
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse"),
    );
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let module = &fixture.modules[0];

    process_module(
        &pool,
        &reference,
        &estimator_stub(),
        fixture.type_id,
        module.module_id,
        &DogmaItem {
            created_by: module.creator_id,
            source_type_id: module.source_type_id,
            mutator_type_id: module.mutaplasmid_id,
            dogma_attributes: common::fixture_dogma(module),
        },
    )
    .await
    .expect("process module");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("reseed reference tables");

    let survivors: i64 = sqlx::query_scalar("select count(*) from modules where id = $1")
        .bind(module.module_id)
        .fetch_one(&pool)
        .await
        .expect("count modules");
    assert_eq!(
        survivors, 1,
        "reseeding the reference data keeps ingested modules"
    );
    let attribute_rows: i64 =
        sqlx::query_scalar("select count(*) from mutated_attributes where module_id = $1")
            .bind(module.module_id)
            .fetch_one(&pool)
            .await
            .expect("count mutated attributes");
    assert!(
        attribute_rows > 0,
        "reseeding keeps the module's attributes"
    );
}
