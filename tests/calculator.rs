//! Behavior tests for `GET /api/calculator[/{query}]`, the legacy
//! `CalculatorController`: seed the fixture reference, drive the real
//! router and verify the ProbabilityResource shape, the legacy cost
//! model over the latest market history, the exact interval math, and
//! the corrected (non-independent) derived-attribute probability.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::probability::{RequestedBound, combination_probability};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use mutamarket::sde::statistics::compute_abyssal_statistics;
use sqlx::PgPool;
use tower::ServiceExt;

/// The fixture abyssal output type the suite queries.
const WEBIFIER_ABYSSAL_TYPE_ID: i64 = 47702;
/// One published fixture combination of that type with recorded
/// statistics: Decayed Stasis Webifier Mutaplasmid on the Khanid Navy
/// Stasis Webifier.
const WEBIFIER_MUTAPLASMID_ID: i64 = 47699;
const KHANID_WEBIFIER_TYPE_ID: i64 = 28514;
/// The combination's recorded cpu statistic (attribute 50): best 15.2,
/// worst 20, so the midpoint bound 17.6 is achievable half the time in
/// either roll direction.
const CPU_ATTRIBUTE_ID: i64 = 50;
const CPU_MIDPOINT: f64 = 17.6;

/// The derived-math fixture: Small Abyssal Armor Repairer, whose
/// armorRepairPerTime (repair amount / duration) is a ratio of two
/// rolled attributes.
const REPAIRER_ABYSSAL_TYPE_ID: i64 = 47769;
const REPAIRER_MUTAPLASMID_ID: i64 = 47766;
const GORGET_REPAIRER_TYPE_ID: i64 = 23795;
const REPAIR_PER_TIME_ATTRIBUTE_ID: i64 = 5_000_002;
/// Inside the combination's achievable [0.01421, 0.01775] band.
const REPAIR_PER_TIME_BOUND: f64 = 0.016;

/// Everything the suite seeds market histories for lives in The Forge.
const FORGE_REGION_ID: i64 = 10_000_002;
const MUTAPLASMID_STALE_AVERAGE: f64 = 2_000_000.0;
const MUTAPLASMID_LATEST_AVERAGE: f64 = 3_000_000.0;
const KHANID_AVERAGE: f64 = 10_000_000.0;

async fn seed(pool: &PgPool) -> ReferenceTables {
    let mut tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    tables.abyssal_statistics = compute_abyssal_statistics(&tables);
    seed_reference(pool, &tables).await.expect("seed reference tables");

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region seeded");

    // Idempotent across runs: only the two seeded types carry a history,
    // every other combination of the queried outputs stays priceless.
    sqlx::query(
        "delete from market_histories where type_id in (
             select mit.type_id from mutaplasmid_input_types mit
             join mutaplasmids mp on mp.id = mit.mutaplasmid_id
             where mp.output_type_id = any($1))
         or type_id in (select id from mutaplasmids where output_type_id = any($1))",
    )
    .bind(vec![WEBIFIER_ABYSSAL_TYPE_ID, REPAIRER_ABYSSAL_TYPE_ID])
    .execute(pool)
    .await
    .expect("stale histories cleared");

    for (type_id, date, average) in [
        (WEBIFIER_MUTAPLASMID_ID, "2026-08-01", MUTAPLASMID_STALE_AVERAGE),
        (WEBIFIER_MUTAPLASMID_ID, "2026-08-20", MUTAPLASMID_LATEST_AVERAGE),
        (KHANID_WEBIFIER_TYPE_ID, "2026-08-10", KHANID_AVERAGE),
    ] {
        sqlx::query(
            "insert into market_histories
                 (type_id, region_id, date, average, highest, lowest)
             values ($1, $2, $3::date, $4, $4, $4)
             on conflict (type_id, region_id, date)
                 do update set average = excluded.average",
        )
        .bind(type_id)
        .bind(FORGE_REGION_ID)
        .bind(date)
        .bind(average)
        .execute(pool)
        .await
        .expect("history seeded");
    }

    tables
}

async fn get_json(app: &axum::Router, uri: &str) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK, "{uri}");
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(content_type.starts_with("application/json"), "{uri}: {content_type}");
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    serde_json::from_slice(&bytes).expect("valid JSON")
}

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn find_row(
    rows: &[serde_json::Value],
    mutaplasmid_id: i64,
    type_id: i64,
) -> &serde_json::Value {
    rows.iter()
        .find(|row| {
            row["mutaplasmid"]["id"].as_i64() == Some(mutaplasmid_id)
                && row["type"]["id"].as_i64() == Some(type_id)
        })
        .expect("combination row present")
}

#[tokio::test]
async fn calculator_matches_the_legacy_contract() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables = seed(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Without a type the page renders its select-a-category state: the
    // legacy controller passed a null probability prop.
    assert_eq!(get_json(&app, "/api/calculator").await, serde_json::Value::Null);
    assert_eq!(
        get_json(&app, "/api/calculator/meta-group/faction").await,
        serde_json::Value::Null
    );

    // With a type: one ProbabilityResource per published combination.
    let body =
        get_json(&app, &format!("/api/calculator/type/{WEBIFIER_ABYSSAL_TYPE_ID}")).await;
    let rows = body.as_array().expect("a bare JSON array");

    let published: std::collections::HashSet<i64> = tables
        .types
        .iter()
        .filter(|row| row.published)
        .map(|row| row.id)
        .collect();
    let mutaplasmids: std::collections::HashSet<i64> = tables
        .mutaplasmids
        .iter()
        .filter(|row| row.output_type_id == WEBIFIER_ABYSSAL_TYPE_ID)
        .map(|row| row.id)
        .collect();
    let expected_combos = tables
        .input_types
        .iter()
        .filter(|row| {
            mutaplasmids.contains(&row.mutaplasmid_id) && published.contains(&row.type_id)
        })
        .count();
    assert_eq!(rows.len(), expected_combos);
    assert!(!rows.is_empty(), "no combinations seeded");

    for row in rows {
        assert_eq!(
            sorted_keys(row),
            vec!["cost", "cost_mutaplasmid", "cost_type", "mutaplasmid", "probability", "type"],
        );
        assert_eq!(sorted_keys(&row["mutaplasmid"]), vec!["id", "name"]);
        assert_eq!(sorted_keys(&row["type"]), vec!["id", "name"]);
        // No bounds requested: every roll qualifies.
        assert_eq!(row["probability"].as_f64(), Some(1.0));
    }

    // The legacy cost model over the latest recorded market day.
    let priced = find_row(rows, WEBIFIER_MUTAPLASMID_ID, KHANID_WEBIFIER_TYPE_ID);
    assert_eq!(priced["cost_mutaplasmid"].as_f64(), Some(MUTAPLASMID_LATEST_AVERAGE));
    assert_eq!(priced["cost_type"].as_f64(), Some(KHANID_AVERAGE));
    assert_eq!(priced["cost"].as_f64(), Some(MUTAPLASMID_LATEST_AVERAGE + KHANID_AVERAGE));

    // An unpriced source type: null coalescing like the legacy resource
    // (mutaplasmid cost falls back to 0, the total stays unknown).
    let unpriced = rows
        .iter()
        .find(|row| {
            row["mutaplasmid"]["id"].as_i64() != Some(WEBIFIER_MUTAPLASMID_ID)
                && row["type"]["id"].as_i64() != Some(KHANID_WEBIFIER_TYPE_ID)
        })
        .expect("an unpriced combination");
    assert_eq!(unpriced["cost_mutaplasmid"].as_f64(), Some(0.0));
    assert!(unpriced["cost_type"].is_null());
    assert!(unpriced["cost"].is_null());

    // Meta filters gate the source types like the legacy query.
    let faction = get_json(
        &app,
        &format!("/api/calculator/type/{WEBIFIER_ABYSSAL_TYPE_ID}/meta-group/faction"),
    )
    .await;
    let faction_rows = faction.as_array().expect("a bare JSON array");
    assert!(!faction_rows.is_empty());
    assert!(faction_rows.len() < rows.len());
    assert!(
        faction_rows
            .iter()
            .any(|row| row["type"]["id"].as_i64() == Some(KHANID_WEBIFIER_TYPE_ID)),
        "the faction webifier survives the faction filter",
    );
}

#[tokio::test]
async fn attribute_bounds_drive_the_probability() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables = seed(&pool).await;
    let app = mutamarket::server::test_router().await;

    // The exact legacy interval arithmetic: a bound at the midpoint of
    // the recorded [15.2, 20] cpu band is achievable half the time,
    // whichever direction the single value resolves to.
    let body = get_json(
        &app,
        &format!(
            "/api/calculator/type/{WEBIFIER_ABYSSAL_TYPE_ID}/attributes/{CPU_ATTRIBUTE_ID}/{CPU_MIDPOINT}"
        ),
    )
    .await;
    let rows = body.as_array().expect("a bare JSON array");
    let row = find_row(rows, WEBIFIER_MUTAPLASMID_ID, KHANID_WEBIFIER_TYPE_ID);
    let probability = row["probability"].as_f64().expect("a number");
    assert!((probability - 0.5).abs() < 1e-9, "got {probability}");
    // The cost scales inversely with the probability.
    let cost = row["cost"].as_f64().expect("a number");
    let expected = (MUTAPLASMID_LATEST_AVERAGE + KHANID_AVERAGE) / probability;
    assert!((cost - expected).abs() < 1e-6, "got {cost}");

    // A derived-attribute bound takes the corrected joint math: the
    // endpoint agrees with the library evaluated on the same fixtures,
    // and diverges from the legacy independence assumption.
    let body = get_json(
        &app,
        &format!(
            "/api/calculator/type/{REPAIRER_ABYSSAL_TYPE_ID}/attributes/{REPAIR_PER_TIME_ATTRIBUTE_ID}/{REPAIR_PER_TIME_BOUND}"
        ),
    )
    .await;
    let rows = body.as_array().expect("a bare JSON array");
    let row = find_row(rows, REPAIRER_MUTAPLASMID_ID, GORGET_REPAIRER_TYPE_ID);
    let probability = row["probability"].as_f64().expect("a number");

    let reference = ReferenceData::from_tables(tables);
    let expected = combination_probability(
        &reference,
        GORGET_REPAIRER_TYPE_ID,
        REPAIRER_MUTAPLASMID_ID,
        &[RequestedBound {
            attribute_id: REPAIR_PER_TIME_ATTRIBUTE_ID,
            min: Some(REPAIR_PER_TIME_BOUND),
            max: None,
        }],
    )
    .expect("combination evaluates");
    assert!((probability - expected).abs() < 1e-12, "got {probability}, expected {expected}");
    assert!(probability > 0.0 && probability < 1.0);

    // The recorded band is [0.01421, 0.01775]; treating the ratio as an
    // independent uniform would report (0.01775-0.016)/0.00354 ≈ 0.494.
    let independent = (0.01775 - REPAIR_PER_TIME_BOUND) / (0.01775 - 0.01421);
    assert!(
        (probability - independent).abs() > 0.01,
        "joint probability {probability} should diverge from the naive {independent}",
    );
}
