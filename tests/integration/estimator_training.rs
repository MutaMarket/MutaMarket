//! Behavior tests for the native estimator training pipeline against the
//! test database: dataset assembly (sold modules + anchor input types,
//! name-ordered non-derived features), the minimum-data gate, statistics
//! updates including the meta-group `data_statistics` remap, model
//! persistence and determinism, and the full-sweep estimate clearing.
//!
//! Needs the local database: `docker compose up -d postgres`.

use mutamarket::db;
use mutamarket::estimator::forest::Forest;
use mutamarket::estimator::training::{self, MINIMUM_DATA_COUNT, TrainOutcome};
use serde_json::json;
use sqlx::PgPool;

/// Synthetic ids owned by this suite only.
const TRAIN_TYPE: i64 = 990_200_001;
const GATED_TYPE: i64 = 990_200_002;
const SOURCE_TYPE: i64 = 990_200_011;
const ANCHOR_TYPE: i64 = 990_200_012;
const MUTAPLASMID: i64 = 990_200_021;
const ATTRIBUTE_ALPHA: i64 = 990_200_031;
const ATTRIBUTE_BETA: i64 = 990_200_032;
const ATTRIBUTE_DERIVED: i64 = 990_200_033;
const MODULE_BASE: i64 = 990_200_100;
const GATED_MODULE: i64 = 990_200_090;
const CONTRACT_BASE: i64 = 990_201_000;
const ISSUER: i64 = 990_200_050;
const REGION: i64 = 990_200_060;

/// Sold modules seeded for the trainable type; comfortably past the gate.
const SOLD_MODULES: i64 = 60;

/// Meta level attribute (633) — the anchor filter reads it.
const META_LEVEL_ATTRIBUTE_ID: i64 = 633;

/// Serializes the tests: they all reseed and train the same scenario
/// rows, so concurrent runs would race.
static SCENARIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    pool
}

async fn execute(pool: &PgPool, sql: &str, binds: &[i64]) {
    let mut query = sqlx::query(sql);
    for bind in binds {
        query = query.bind(bind);
    }
    query.execute(pool).await.expect("seed statement");
}

/// Seeds the whole training scenario. Idempotent: every statement upserts
/// or re-creates the rows this suite owns.
async fn seed_scenario(pool: &PgPool) {
    // Meta groups from the SDE seed can be assumed (Tech I = 1, Tech II
    // = 2), but the test database may start empty — upsert them.
    for (id, name) in [(1_i64, "Tech I"), (2_i64, "Tech II")] {
        sqlx::query(
            "insert into meta_groups (id, name) values ($1, $2) on conflict (id) do nothing",
        )
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .expect("seed meta group");
    }

    for (id, name, meta_group) in [
        (TRAIN_TYPE, "Estimator Training Output", None),
        (GATED_TYPE, "Estimator Training Gated", None),
        (SOURCE_TYPE, "Estimator Training Source", Some(1_i64)),
        (ANCHOR_TYPE, "Estimator Training Anchor", Some(1_i64)),
    ] {
        sqlx::query(
            "insert into types (id, name, published, meta_group_id) values ($1, $2, true, $3)
             on conflict (id) do update set name = excluded.name, published = true,
                 meta_group_id = excluded.meta_group_id",
        )
        .bind(id)
        .bind(name)
        .bind(meta_group)
        .execute(pool)
        .await
        .expect("seed type");
    }

    // Feature attributes: beta sorts before alpha by name to prove the
    // name ordering; the derived one must be excluded.
    for (id, name, derived) in [
        (ATTRIBUTE_ALPHA, "estZuluAttribute", false),
        (ATTRIBUTE_BETA, "estAlphaAttribute", false),
        (ATTRIBUTE_DERIVED, "estDerivedAttribute", true),
    ] {
        sqlx::query(
            "insert into attributes (id, name, derived) values ($1, $2, $3)
             on conflict (id) do update set name = excluded.name, derived = excluded.derived",
        )
        .bind(id)
        .bind(name)
        .bind(derived)
        .execute(pool)
        .await
        .expect("seed attribute");
    }

    execute(
        pool,
        "delete from estimator_attributes where type_id in ($1, $2)",
        &[TRAIN_TYPE, GATED_TYPE],
    )
    .await;
    for attribute in [ATTRIBUTE_ALPHA, ATTRIBUTE_BETA, ATTRIBUTE_DERIVED] {
        execute(
            pool,
            "insert into estimator_attributes (type_id, attribute_id) values ($1, $2)",
            &[TRAIN_TYPE, attribute],
        )
        .await;
    }

    // The statistics rows carry the legacy-seeded meta group keys the
    // remap must preserve.
    for type_id in [TRAIN_TYPE, GATED_TYPE] {
        sqlx::query(
            "insert into estimator_statistics (type_id, name, data_count, data_statistics)
             values ($1, 'estimator_training_test', 0, '{\"Tech I\": 0, \"Tech II\": 0}'::jsonb)
             on conflict (type_id) do update set
                 data_count = 0, r2 = null, mae = null, nmae = null, last_trained_at = null,
                 data_statistics = '{\"Tech I\": 0, \"Tech II\": 0}'::jsonb",
        )
        .bind(type_id)
        .execute(pool)
        .await
        .expect("seed statistic");
    }

    // The mutaplasmid links output type, source (input) types and the
    // mutated attributes.
    sqlx::query(
        "insert into mutaplasmids (id, name, output_type_id)
         values ($1, 'Estimator Training Mutaplasmid', $2)
         on conflict (id) do update set output_type_id = excluded.output_type_id",
    )
    .bind(MUTAPLASMID)
    .bind(TRAIN_TYPE)
    .execute(pool)
    .await
    .expect("seed mutaplasmid");
    execute(
        pool,
        "delete from mutaplasmid_input_types where mutaplasmid_id = $1",
        &[MUTAPLASMID],
    )
    .await;
    execute(
        pool,
        "insert into mutaplasmid_input_types (id, mutaplasmid_id, type_id) values ($1, $1, $2)",
        &[MUTAPLASMID, ANCHOR_TYPE],
    )
    .await;

    // Source and anchor type attribute values (the anchor also needs a
    // low meta level to qualify).
    for (row, (type_id, attribute_id, value)) in [
        (SOURCE_TYPE, ATTRIBUTE_ALPHA, 10.0_f64),
        (SOURCE_TYPE, ATTRIBUTE_BETA, 5.0),
        (ANCHOR_TYPE, ATTRIBUTE_ALPHA, 8.0),
        // No beta value on the anchor: its feature falls back to 0.
        (ANCHOR_TYPE, META_LEVEL_ATTRIBUTE_ID, 1.0),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query(
            "insert into type_attributes (id, type_id, attribute_id, value)
             values ($1, $2, $3, $4)
             on conflict (type_id, attribute_id) do update set value = excluded.value",
        )
        .bind(990_202_000_i64 + row as i64)
        .bind(type_id)
        .bind(attribute_id)
        .bind(value)
        .execute(pool)
        .await
        .expect("seed type attribute");
    }
    sqlx::query(
        "insert into attributes (id, name) values ($1, 'metaLevel') on conflict (id) do nothing",
    )
    .bind(META_LEVEL_ATTRIBUTE_ID)
    .execute(pool)
    .await
    .expect("seed meta level attribute");

    // Foreign-key scaffolding for the contracts and market history.
    execute(
        pool,
        "insert into characters (id) values ($1) on conflict (id) do nothing",
        &[ISSUER],
    )
    .await;
    sqlx::query(
        "insert into regions (id, name) values ($1, 'Estimator Training Region')
         on conflict (id) do nothing",
    )
    .bind(REGION)
    .execute(pool)
    .await
    .expect("seed region");

    // The anchor's market price.
    sqlx::query("delete from market_histories where type_id = $1")
        .bind(ANCHOR_TYPE)
        .execute(pool)
        .await
        .expect("clean market history");
    sqlx::query(
        "insert into market_histories (type_id, region_id, date, average, highest, lowest)
         values ($1, $2, current_date, 3000000, 3200000, 2800000)",
    )
    .bind(ANCHOR_TYPE)
    .bind(REGION)
    .execute(pool)
    .await
    .expect("seed market history");

    // Sold modules: alpha mutated per module and driving the price
    // linearly, beta left to the source-type fallback.
    execute(
        pool,
        "delete from training_modules where module_id between $1 and $2",
        &[MODULE_BASE, MODULE_BASE + SOLD_MODULES],
    )
    .await;
    execute(
        pool,
        "delete from historic_contracts where id between $1 and $2",
        &[CONTRACT_BASE, CONTRACT_BASE + SOLD_MODULES],
    )
    .await;
    for index in 0..SOLD_MODULES {
        let module_id = MODULE_BASE + index;
        let contract_id = CONTRACT_BASE + index;
        // A permutation of 10..70 so every consecutive CV fold spans
        // the full value range (a monotonic sequence would make each
        // fold pure extrapolation, which trees cannot do).
        let alpha = 10.0 + ((index * 37) % SOLD_MODULES) as f64;
        let price = 1_000_000.0 * alpha;

        sqlx::query(
            "insert into modules (id, type_id, source_type_id, estimated_value)
             values ($1, $2, $3, 123.0)
             on conflict (id) do update set type_id = excluded.type_id,
                 source_type_id = excluded.source_type_id, estimated_value = 123.0",
        )
        .bind(module_id)
        .bind(TRAIN_TYPE)
        .bind(SOURCE_TYPE)
        .execute(pool)
        .await
        .expect("seed module");
        sqlx::query(
            "insert into mutated_attributes
             (module_id, attribute_id, type_id, value, base_value, fraction, fraction_type,
              fraction_absolute, bar, is_virtual)
             values ($1, $2, $3, $4, 0, 0, 0, 0, 0, false)
             on conflict (module_id, attribute_id) do update set value = excluded.value",
        )
        .bind(module_id)
        .bind(ATTRIBUTE_ALPHA)
        .bind(TRAIN_TYPE)
        .bind(alpha)
        .execute(pool)
        .await
        .expect("seed mutated attribute");
        sqlx::query(
            "insert into historic_contracts (id, status, region_id, issuer_id, type, unified_price)
             values ($1, 'finished', $2, $3, 'item_exchange', $4)
             on conflict (id) do update set unified_price = excluded.unified_price",
        )
        .bind(contract_id)
        .bind(REGION)
        .bind(ISSUER)
        .bind(price)
        .execute(pool)
        .await
        .expect("seed historic contract");
        sqlx::query(
            "insert into training_modules (module_id, historic_contract_id) values ($1, $2)",
        )
        .bind(module_id)
        .bind(contract_id)
        .execute(pool)
        .await
        .expect("seed training module");
    }

    // A module of the gated (undertrained) type with a stale estimate the
    // sweep must clear.
    sqlx::query(
        "insert into modules (id, type_id, estimated_value) values ($1, $2, 999.0)
         on conflict (id) do update set type_id = excluded.type_id, estimated_value = 999.0",
    )
    .bind(GATED_MODULE)
    .bind(GATED_TYPE)
    .execute(pool)
    .await
    .expect("seed gated module");
}

async fn statistic_row(pool: &PgPool, type_id: i64) -> serde_json::Value {
    sqlx::query_scalar(
        "select to_jsonb(es) - 'id' - 'created_at' - 'updated_at' from estimator_statistics es
         where es.type_id = $1",
    )
    .bind(type_id)
    .fetch_one(pool)
    .await
    .expect("statistic row")
}

#[tokio::test]
async fn training_fits_stores_and_reports_like_legacy() {
    let _guard = SCENARIO_LOCK.lock().await;
    let pool = setup().await;
    seed_scenario(&pool).await;

    let outcome = training::train_type(&pool, TRAIN_TYPE)
        .await
        .expect("train");
    let TrainOutcome::Trained {
        data_count,
        rows,
        metrics,
    } = outcome
    else {
        panic!("expected a trained outcome, got {outcome:?}");
    };

    // 60 sold modules count as data; the anchor type adds one more row.
    assert_eq!(data_count, SOLD_MODULES);
    assert_eq!(rows, SOLD_MODULES as usize + 1);

    // The price is a clean linear function of the mutated attribute, so
    // cross-validation must explain nearly everything.
    assert!(metrics.r2 > 0.9, "r2 too low: {}", metrics.r2);
    assert!(metrics.nmae < 15.0, "nmae too high: {}", metrics.nmae);

    // The statistics row mirrors the legacy update: metrics, data_count,
    // last_trained_at, and data_statistics remapped onto its existing
    // meta-group keys (all sold modules have a Tech I source type).
    let row = statistic_row(&pool, TRAIN_TYPE).await;
    assert_eq!(row["data_count"], json!(SOLD_MODULES));
    assert_eq!(row["r2"], json!(metrics.r2));
    assert_eq!(row["mae"], json!(metrics.mae));
    assert_eq!(row["nmae"], json!(metrics.nmae));
    assert!(row["last_trained_at"].is_string());
    assert_eq!(
        row["data_statistics"],
        json!({ "Tech I": SOLD_MODULES, "Tech II": 0 }),
    );

    // The stored model: non-derived features in attribute-name order, and
    // a forest whose predictions track the seeded linear price.
    let (feature_names, model): (serde_json::Value, Vec<u8>) =
        sqlx::query_as("select feature_names, model from estimator_models where type_id = $1")
            .bind(TRAIN_TYPE)
            .fetch_one(&pool)
            .await
            .expect("model row");
    assert_eq!(
        feature_names,
        json!(["estAlphaAttribute", "estZuluAttribute"])
    );

    let forest = Forest::from_bytes(&model).expect("model deserializes");
    // Feature order: [estAlphaAttribute (beta id, source fallback 5.0),
    // estZuluAttribute (alpha id, mutated)].
    let prediction = forest.predict(&[5.0, 40.0]);
    let expected = 1_000_000.0 * 40.0;
    assert!(
        (prediction - expected).abs() / expected < 0.05,
        "prediction {prediction} too far from {expected}",
    );
}

#[tokio::test]
async fn training_is_deterministic() {
    let _guard = SCENARIO_LOCK.lock().await;
    let pool = setup().await;
    seed_scenario(&pool).await;

    training::train_type(&pool, TRAIN_TYPE)
        .await
        .expect("first train");
    let first: Vec<u8> =
        sqlx::query_scalar("select model from estimator_models where type_id = $1")
            .bind(TRAIN_TYPE)
            .fetch_one(&pool)
            .await
            .expect("first model");

    training::train_type(&pool, TRAIN_TYPE)
        .await
        .expect("second train");
    let second: Vec<u8> =
        sqlx::query_scalar("select model from estimator_models where type_id = $1")
            .bind(TRAIN_TYPE)
            .fetch_one(&pool)
            .await
            .expect("second model");

    assert_eq!(
        first, second,
        "same data and seed must produce identical model bytes"
    );
}

#[tokio::test]
async fn undertrained_types_get_nulled_metrics_and_no_model() {
    let _guard = SCENARIO_LOCK.lock().await;
    let pool = setup().await;
    seed_scenario(&pool).await;

    let outcome = training::train_type(&pool, GATED_TYPE)
        .await
        .expect("train");
    let TrainOutcome::NotEnoughData { data_count } = outcome else {
        panic!("expected the gate, got {outcome:?}");
    };
    assert!(data_count < MINIMUM_DATA_COUNT);
    assert_eq!(data_count, 0);

    // Metrics nulled, the stamp and count still advance, data_statistics
    // untouched — the legacy else-branch of updateEstimatorData.
    let row = statistic_row(&pool, GATED_TYPE).await;
    assert_eq!(row["r2"], json!(null));
    assert_eq!(row["mae"], json!(null));
    assert_eq!(row["nmae"], json!(null));
    assert_eq!(row["data_count"], json!(0));
    assert!(row["last_trained_at"].is_string());
    assert_eq!(row["data_statistics"], json!({ "Tech I": 0, "Tech II": 0 }));

    let model_rows: i64 =
        sqlx::query_scalar("select count(*) from estimator_models where type_id = $1")
            .bind(GATED_TYPE)
            .fetch_one(&pool)
            .await
            .expect("model count");
    assert_eq!(model_rows, 0);
}

#[tokio::test]
async fn the_full_sweep_trains_output_types_and_clears_untrained_estimates() {
    let _guard = SCENARIO_LOCK.lock().await;
    let pool = setup().await;
    seed_scenario(&pool).await;

    let mut progress = Vec::new();
    let run = training::train_all(&pool, |line| progress.push(line))
        .await
        .expect("sweep");

    // The suite's output type trains; every other mutaplasmid output type
    // in the test database lacks training modules and is skipped.
    assert!(run.trained >= 1);
    assert!(
        progress
            .iter()
            .any(|line| line.contains("Estimator Training Output"))
    );

    // The gated type's stale estimate is cleared by the final sweep, the
    // trained type's modules keep theirs.
    let gated: (Option<f64>, Option<String>) = sqlx::query_as(
        "select estimated_value, estimated_value_updated_at::text from modules where id = $1",
    )
    .bind(GATED_MODULE)
    .fetch_one(&pool)
    .await
    .expect("gated module");
    assert_eq!(gated.0, None);
    assert!(gated.1.is_some());

    let trained_estimate: Option<f64> =
        sqlx::query_scalar("select estimated_value from modules where id = $1")
            .bind(MODULE_BASE)
            .fetch_one(&pool)
            .await
            .expect("trained module");
    assert_eq!(trained_estimate, Some(123.0));
}
