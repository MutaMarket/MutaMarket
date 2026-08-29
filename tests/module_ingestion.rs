//! Persistence characterization for module ingestion, mirroring the legacy
//! "persists modules matching the fixture snapshot" test: the first module
//! of every fixture file is processed into Postgres and read back, and must
//! match the expected snapshot including `average_fraction`.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;

use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use sqlx::{PgPool, Row};

fn dogma_item(module: &common::ModuleFixture) -> DogmaItem {
    DogmaItem {
        created_by: module.creator_id,
        source_type_id: module.source_type_id,
        mutator_type_id: module.mutaplasmid_id,
        dogma_attributes: common::fixture_dogma(module),
    }
}

async fn check_persisted_module(
    pool: &PgPool,
    type_id: i64,
    module: &common::ModuleFixture,
    failures: &mut Vec<String>,
) {
    let context = format!("module {} (type {})", module.module_id, type_id);

    let row = sqlx::query(
        "select type_id, source_type_id, mutaplasmid_id, creator_id, average_fraction
         from modules where id = $1",
    )
    .bind(module.module_id)
    .fetch_optional(pool)
    .await
    .expect("module query");

    let Some(row) = row else {
        failures.push(format!("{context}: module row missing"));
        return;
    };

    if row.get::<i64, _>("type_id") != type_id
        || row.get::<Option<i64>, _>("source_type_id") != Some(module.source_type_id)
        || row.get::<Option<i64>, _>("mutaplasmid_id") != Some(module.mutaplasmid_id)
        || row.get::<Option<i64>, _>("creator_id") != Some(module.creator_id)
    {
        failures.push(format!("{context}: module identity columns diverge"));
    }

    match row.get::<Option<f64>, _>("average_fraction") {
        Some(average) if common::matches(module.expected.average_fraction, average) => {}
        other => failures.push(format!(
            "{context}: average_fraction expected {}, got {other:?}",
            module.expected.average_fraction,
        )),
    }

    let attribute_rows = sqlx::query(
        "select attribute_id, type_id, value, base_value, fraction, fraction_type,
                fraction_absolute, bar, is_virtual
         from mutated_attributes where module_id = $1",
    )
    .bind(module.module_id)
    .fetch_all(pool)
    .await
    .expect("mutated attributes query");

    if attribute_rows.len() != module.expected.attributes.len() {
        failures.push(format!(
            "{context}: expected {} attribute rows, got {}",
            module.expected.attributes.len(),
            attribute_rows.len(),
        ));
    }

    for expected in &module.expected.attributes {
        let attribute_context = format!("{context}, attribute {}", expected.attribute_id);

        let Some(row) = attribute_rows
            .iter()
            .find(|row| row.get::<i64, _>("attribute_id") == expected.attribute_id)
        else {
            failures.push(format!("{attribute_context}: row missing"));
            continue;
        };

        let floats = [
            ("value", expected.value, row.get::<f64, _>("value")),
            (
                "base_value",
                expected.base_value,
                row.get::<f64, _>("base_value"),
            ),
            ("fraction", expected.fraction, row.get::<f64, _>("fraction")),
            (
                "fraction_type",
                expected.fraction_type,
                row.get::<f64, _>("fraction_type"),
            ),
            (
                "fraction_absolute",
                expected.fraction_absolute,
                row.get::<f64, _>("fraction_absolute"),
            ),
        ];

        for (field, expected_value, actual_value) in floats {
            if !common::matches(expected_value, actual_value) {
                failures.push(format!(
                    "{attribute_context}, {field}: expected {expected_value}, got {actual_value}"
                ));
            }
        }

        if row.get::<i64, _>("type_id") != type_id {
            failures.push(format!("{attribute_context}: type_id diverges"));
        }
        if i64::from(row.get::<i16, _>("bar")) != expected.bar {
            failures.push(format!(
                "{attribute_context}: bar expected {}, got {}",
                expected.bar,
                row.get::<i16, _>("bar"),
            ));
        }
        if row.get::<bool, _>("is_virtual") != expected.is_virtual {
            failures.push(format!("{attribute_context}: is_virtual diverges"));
        }
    }
}

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

#[tokio::test]
async fn persists_modules_matching_the_legacy_fixture_snapshots() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    let fixtures = common::load_module_fixtures();
    let mut failures = Vec::new();

    // Like the legacy persistence test: the first module of every fixture
    // file exercises the full pipeline against the database.
    for fixture in &fixtures {
        let module = &fixture.modules[0];

        process_module(
            &pool,
            &reference,
            &estimator_stub(),
            fixture.type_id,
            module.module_id,
            &dogma_item(module),
        )
        .await
        .expect("process module");

        check_persisted_module(&pool, fixture.type_id, module, &mut failures).await;
    }

    // Reprocessing must upsert in place: same rows, same values.
    let first = &fixtures[0];
    let module = &first.modules[0];
    process_module(
        &pool,
        &reference,
        &estimator_stub(),
        first.type_id,
        module.module_id,
        &dogma_item(module),
    )
    .await
    .expect("reprocess module");
    check_persisted_module(&pool, first.type_id, module, &mut failures).await;

    let duplicate_check: i64 =
        sqlx::query_scalar("select count(*) from mutated_attributes where module_id = $1")
            .bind(module.module_id)
            .fetch_one(&pool)
            .await
            .expect("duplicate check");
    assert_eq!(
        duplicate_check as usize,
        module.expected.attributes.len(),
        "reprocessing must not duplicate attribute rows",
    );

    assert!(
        failures.is_empty(),
        "{} persistence checks diverge from the legacy snapshots (showing up to 40):\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );

    assert_eq!(
        fixtures.len(),
        89,
        "fixture file count changed unexpectedly"
    );
}
