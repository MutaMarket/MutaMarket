//! Behavior tests for the search alerts: the save/list/delete routes
//! (normalization, the type requirement, the per-account cap, ownership
//! of deletes) and the `search-alerts` job (only listings that appear
//! after the alert was saved fire it, one notification per run, a
//! renewal by the same seller stays quiet while another seller's listing
//! fires again, the query's own filters decide the match). Saving needs
//! premium, and an alert pauses while its account has none.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use mutamarket::search_alerts::{self, RunStats, run_with_maturity};
use serde_json::json;
use std::path::Path;
use tower::ServiceExt;

/// Characters owned by this suite alone.
const OWNER_CHARACTER: i64 = 930_101;
const OTHER_CHARACTER: i64 = 930_102;

/// The contract ids this suite lists modules on.
const CONTRACT_BASE: i64 = 930_000_000;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String, serde_json::Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        request = request.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("valid request");

    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, location, json)
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

fn assert_validation(status: StatusCode, body: &serde_json::Value, field: &str, message: &str) {
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "body: {body}");
    assert_eq!(body["message"], json!("The given data was invalid."));
    assert_eq!(body["errors"][field], json!([message]), "field {field}");
}

const ALERT_KEYS: [&str; 6] = [
    "created_at",
    "id",
    "last_notified_at",
    "notified_count",
    "query",
    "type",
];

/// The contract issuer `common::attach_contract` seeds.
const ISSUER_CHARACTER: i64 = 90999999;

/// Lists a module on a contract the way the region import leaves it: the
/// contract linked as the module's latest, and the issuer's ownership
/// row pointing at it (a renewal moves the row, like the importer's
/// upsert, and keeps its creation time).
async fn list_on_contract(pool: &sqlx::PgPool, module_id: i64, contract_id: i64, price: f64) {
    list_on_contract_by(pool, ISSUER_CHARACTER, module_id, contract_id, price).await;
}

async fn list_on_contract_by(
    pool: &sqlx::PgPool,
    character_id: i64,
    module_id: i64,
    contract_id: i64,
    price: f64,
) {
    common::attach_contract(
        pool,
        module_id,
        contract_id,
        "item_exchange",
        price,
        1,
        0,
        0,
    )
    .await;
    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id, contract_id)
         values ($3, $1, $2)
         on conflict (character_id, module_id) do update
         set contract_id = excluded.contract_id, public_asset_id = null, updated_at = now()",
    )
    .bind(module_id)
    .bind(contract_id)
    .bind(character_id)
    .execute(pool)
    .await
    .expect("ownership row");
}

/// The job with the maturity margin removed: the rows this suite just
/// inserted count immediately.
async fn run_now(pool: &sqlx::PgPool, reference: &ReferenceData) -> RunStats {
    run_with_maturity(pool, reference, 0.0).await.expect("run")
}

/// Lists a module as a published asset (no contract), with the
/// publisher's ownership row like the asset import leaves it.
async fn list_as_asset(pool: &sqlx::PgPool, module_id: i64, type_id: i64) {
    common::publish_asset(pool, module_id, type_id).await;
    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id, public_asset_id)
         select pa.character_id, pa.module_id, pa.id from public_assets pa
         where pa.module_id = $1 and pa.character_id = $2
         on conflict (character_id, module_id) do update
         set public_asset_id = excluded.public_asset_id, contract_id = null, updated_at = now()",
    )
    .bind(module_id)
    .bind(common::PUBLIC_SELLER_CHARACTER_ID)
    .execute(pool)
    .await
    .expect("asset ownership row");
}

/// The suite's outbox rows for a user, oldest first: (kind, subject, payload).
async fn outbox(pool: &sqlx::PgPool, user_id: i64) -> Vec<(String, String, serde_json::Value)> {
    sqlx::query_as(
        "select kind, subject, payload from notification_outbox where user_id = $1 order by id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .expect("outbox rows")
}

#[tokio::test]
async fn search_alerts_routes_and_job() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables)
        .await
        .expect("seed");
    let reference = ReferenceData::from_tables(tables);

    // Three modules of one type and one of another.
    let fixtures = common::load_module_fixtures();
    let (fixture_a, fixture_b) = (&fixtures[0], &fixtures[1]);
    assert_ne!(fixture_a.type_id, fixture_b.type_id);
    let mut ids_a = Vec::new();
    for (fixture, count, ids) in [(fixture_a, 3, &mut ids_a), (fixture_b, 1, &mut Vec::new())] {
        for module in &fixture.modules[..count] {
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
            ids.push(module.module_id);
        }
    }
    let (a1, a2, a3) = (ids_a[0], ids_a[1], ids_a[2]);
    let b1 = fixture_b.modules[0].module_id;
    let type_a = fixture_a.type_id;

    // Clean listings and accounts of earlier runs and other suites.
    sqlx::query("delete from public_module_ownerships where module_id = any($1)")
        .bind(vec![a1, a2, a3, b1])
        .execute(&pool)
        .await
        .expect("clean ownerships");
    sqlx::query("delete from public_assets where module_id = any($1)")
        .bind(vec![a1, a2, a3, b1])
        .execute(&pool)
        .await
        .expect("clean public assets");
    sqlx::query("update modules set latest_contract_id = null where id = any($1)")
        .bind(vec![a1, a2, a3, b1])
        .execute(&pool)
        .await
        .expect("unlink contracts");
    sqlx::query("delete from contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_BASE)
        .execute(&pool)
        .await
        .expect("clean contracts");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![OWNER_CHARACTER, OTHER_CHARACTER])
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Alert Owner", "Alert Other"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let mut users = Vec::new();
    for (name, character_id) in [
        ("Alert Owner", OWNER_CHARACTER),
        ("Alert Other", OTHER_CHARACTER),
    ] {
        let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
            .bind(name)
            .fetch_one(&pool)
            .await
            .expect("user");
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(character_id)
            .bind(name)
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("character");
        let session = mutamarket::auth::session::create_session(&pool, user_id, Some(character_id))
            .await
            .expect("session");
        users.push((user_id, session));
    }
    let (owner_id, owner) = (users[0].0, users[0].1.clone());
    let (other_id, other) = (users[1].0, users[1].1.clone());
    // The owner holds premium from the start; the other account earns it
    // below.
    sqlx::query(
        "update characters set premium_paid_until = now() + interval '30 days' where id = $1",
    )
    .bind(OWNER_CHARACTER)
    .execute(&pool)
    .await
    .expect("owner premium");

    let app = mutamarket::server::test_router().await;

    // --- guests ---------------------------------------------------------
    let (status, location, _) = send(&app, "POST", "/search-alerts", None, Some(json!({}))).await;
    assert!(
        status.is_redirection(),
        "guest POST redirects, got {status}"
    );
    assert_eq!(location, "/login");
    let (status, location, _) = send(&app, "DELETE", "/search-alerts/1", None, None).await;
    assert!(
        status.is_redirection(),
        "guest DELETE redirects, got {status}"
    );
    assert_eq!(location, "/login");
    let (status, _, body) = send(&app, "GET", "/api/search-alerts", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], json!("Unauthenticated."));

    // --- premium ----------------------------------------------------------
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&other),
        Some(json!({ "query": format!("type/{type_a}") })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "body: {body}");
    assert_eq!(body["message"], json!("Premium required."));
    sqlx::query(
        "update characters set premium_paid_until = now() + interval '30 days' where id = $1",
    )
    .bind(OTHER_CHARACTER)
    .execute(&pool)
    .await
    .expect("other premium");

    // --- validation -------------------------------------------------------
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({})),
    )
    .await;
    assert_validation(status, &body, "query", "The query field is required.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": 5 })),
    )
    .await;
    assert_validation(status, &body, "query", "The query field must be a string.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": "goldbar/contracts-only" })),
    )
    .await;
    assert_validation(
        status,
        &body,
        "query",
        "Pick a module type before saving an alert.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": "type/no-such-abyssal-thing" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], json!("Please provide a valid type."));
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}/meta-level/abc") })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], json!("You provided an invalid meta level"));

    // --- save: normalized, idempotent -------------------------------------
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({
            "query": format!("goldbar/type/{type_a}/sort/price/desc/with-personal-modules/page/3")
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(sorted_keys(&body), ALERT_KEYS);
    assert_eq!(body["query"], json!(format!("type/{type_a}/goldbar")));
    assert_eq!(sorted_keys(&body["type"]), ["id", "name"]);
    assert_eq!(body["type"]["id"], json!(type_a));
    assert_eq!(body["notified_count"], json!(0));
    assert_eq!(body["last_notified_at"], serde_json::Value::Null);
    let goldbar_alert = body["id"].as_i64().expect("alert id");

    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}/goldbar") })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the same query answers the existing alert"
    );
    assert_eq!(body["id"], json!(goldbar_alert));

    // --- list: per account, newest first ----------------------------------
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let plain_alert = body["id"].as_i64().expect("alert id");
    let (status, _, body) = send(&app, "GET", "/api/search-alerts", Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK);
    let alerts = body.as_array().expect("alert array");
    assert_eq!(
        alerts
            .iter()
            .map(|alert| alert["id"].as_i64())
            .collect::<Vec<_>>(),
        vec![Some(plain_alert), Some(goldbar_alert)]
    );
    for alert in alerts {
        assert_eq!(sorted_keys(alert), ALERT_KEYS);
    }
    let (_, _, body) = send(&app, "GET", "/api/search-alerts", Some(&other), None).await;
    assert_eq!(body, json!([]));

    // --- delete: only your own -------------------------------------------
    let (status, _, body) = send(
        &app,
        "DELETE",
        &format!("/search-alerts/{goldbar_alert}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], json!("Not found."));
    let (status, _, _) = send(
        &app,
        "DELETE",
        &format!("/search-alerts/{goldbar_alert}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, _, body) = send(&app, "GET", "/api/search-alerts", Some(&owner), None).await;
    assert_eq!(body.as_array().map(Vec::len), Some(1));

    // --- the cap ------------------------------------------------------------
    for price in 1..search_alerts::MAX_ALERTS_PER_USER {
        let (status, _, body) = send(
            &app,
            "POST",
            "/search-alerts",
            Some(&owner),
            Some(json!({ "query": format!("type/{type_a}/contract-price/{price}") })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "alert {price}: {body}");
    }
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}/goldbar") })),
    )
    .await;
    assert_validation(
        status,
        &body,
        "query",
        "You can save at most 10 search alerts.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}") })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "an existing query still answers under the cap"
    );
    assert_eq!(body["id"], json!(plain_alert));
    sqlx::query("delete from search_alerts where user_id = $1 and id <> $2")
        .bind(owner_id)
        .bind(plain_alert)
        .execute(&pool)
        .await
        .expect("trim alerts");

    // --- the job ------------------------------------------------------------
    // A listing from before the alert was saved never fires it.
    sqlx::query("delete from search_alerts where user_id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("reset alerts");
    sqlx::query("delete from notification_outbox where user_id = any($1)")
        .bind(vec![owner_id, other_id])
        .execute(&pool)
        .await
        .expect("clean outbox");
    list_on_contract(&pool, a1, CONTRACT_BASE + 1, 5_000_000.0).await;

    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&owner),
        Some(json!({ "query": format!("type/{type_a}") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let owner_alert = body["id"].as_i64().expect("alert id");
    // The other account only wants contracts under 1M ISK.
    let (status, _, body) = send(
        &app,
        "POST",
        "/search-alerts",
        Some(&other),
        Some(json!({ "query": format!("type/{type_a}/contracts-only/contract-price/1000000") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    let other_alert = body["id"].as_i64().expect("alert id");

    let stats = run_now(&pool, &reference).await;
    assert_eq!(
        stats,
        RunStats {
            alerts: 2,
            notified: 0,
            matches: 0,
            paused: 0
        }
    );
    assert!(outbox(&pool, owner_id).await.is_empty());

    // Two listings appear in one run: a contract and a published asset.
    // The owner gets one notification with both, the other account's
    // contracts-only alert none (5M is over its price cap).
    list_on_contract(&pool, a2, CONTRACT_BASE + 2, 5_000_000.0).await;
    list_as_asset(&pool, a3, type_a).await;
    // The other type's listing matches nobody.
    list_on_contract(&pool, b1, CONTRACT_BASE + 3, 500_000.0).await;

    let stats = run_now(&pool, &reference).await;
    assert_eq!(
        stats,
        RunStats {
            alerts: 2,
            notified: 1,
            matches: 2,
            paused: 0
        }
    );
    let rows = outbox(&pool, owner_id).await;
    assert_eq!(rows.len(), 1);
    let (kind, subject, payload) = &rows[0];
    assert_eq!(kind, "search-alert");
    assert_eq!(subject, "2 new modules matching your search alert");
    assert_eq!(
        sorted_keys(payload),
        ["alert_id", "discord", "module_ids", "query"]
    );
    assert_eq!(payload["alert_id"], json!(owner_alert));
    assert_eq!(payload["query"], json!(format!("type/{type_a}")));
    let mut expected = vec![a2, a3];
    expected.sort_unstable_by(|x, y| y.cmp(x));
    assert_eq!(payload["module_ids"], json!(expected));
    assert_eq!(
        payload["discord"]["content"],
        json!("2 new modules match one of your search alerts!")
    );
    assert!(outbox(&pool, other_id).await.is_empty());

    let (_, _, body) = send(&app, "GET", "/api/search-alerts", Some(&owner), None).await;
    assert_eq!(body[0]["notified_count"], json!(2));
    assert!(body[0]["last_notified_at"].is_string());

    // Nothing new: no second notification.
    let stats = run_now(&pool, &reference).await;
    assert_eq!(stats.notified, 0);
    assert_eq!(outbox(&pool, owner_id).await.len(), 1);

    // The same seller renewing the contract, even cheaper, moves the
    // listing row in place: nobody is told again.
    list_on_contract(&pool, a2, CONTRACT_BASE + 4, 800_000.0).await;
    let stats = run_now(&pool, &reference).await;
    assert_eq!(stats.notified, 0);
    assert_eq!(outbox(&pool, owner_id).await.len(), 1);
    assert!(outbox(&pool, other_id).await.is_empty());

    // Another seller listing the module is a new listing row: both
    // alerts fire (800k is under the other account's price cap).
    list_on_contract_by(
        &pool,
        common::PUBLIC_SELLER_CHARACTER_ID,
        a2,
        CONTRACT_BASE + 6,
        800_000.0,
    )
    .await;
    let stats = run_now(&pool, &reference).await;
    assert_eq!(
        stats,
        RunStats {
            alerts: 2,
            notified: 2,
            matches: 2,
            paused: 0
        }
    );
    let rows = outbox(&pool, owner_id).await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].1, "New module matching your search alert");
    assert_eq!(rows[1].2["module_ids"], json!([a2]));
    let rows = outbox(&pool, other_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].2["alert_id"], json!(other_alert));
    assert_eq!(rows[0].2["module_ids"], json!([a2]));

    // A listing that vanished before the run (contract gone) is not
    // reported: the for-sale filter sees no live listing.
    list_on_contract(&pool, a1, CONTRACT_BASE + 5, 100_000.0).await;
    sqlx::query("delete from contracts where id = $1")
        .bind(CONTRACT_BASE + 5)
        .execute(&pool)
        .await
        .expect("expire contract");
    let stats = run_now(&pool, &reference).await;
    assert_eq!(stats.notified, 0);

    // The production margin: a row younger than the maturity stays in
    // the open window and is not reported yet. Meanwhile the other
    // account's premium lapsed: its alert pauses (100k would have been
    // under its price cap) and only the owner is told once the window
    // closes over the row.
    sqlx::query(
        "update characters set premium_paid_until = now() - interval '1 day' where id = $1",
    )
    .bind(OTHER_CHARACTER)
    .execute(&pool)
    .await
    .expect("other premium lapsed");
    list_on_contract(&pool, a1, CONTRACT_BASE + 7, 100_000.0).await;
    let stats = search_alerts::run(&pool, &reference).await.expect("run");
    assert_eq!(stats.notified, 0);
    let stats = run_now(&pool, &reference).await;
    assert_eq!(
        stats,
        RunStats {
            alerts: 2,
            notified: 1,
            matches: 1,
            paused: 1
        }
    );
    assert_eq!(outbox(&pool, owner_id).await.len(), 3);
    assert_eq!(outbox(&pool, other_id).await.len(), 1);
}
