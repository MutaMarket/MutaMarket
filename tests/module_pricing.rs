//! Behavior tests for module pricing: the bulk upsert/delete semantics
//! (zero and negative prices delete), the Laravel-shaped validation, and
//! the public_asset price subselect on module payloads.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use std::path::Path;
use tower::ServiceExt;

/// Characters owned by this suite alone, so parallel suites never share
/// state.
const SELLER_CHARACTER: i64 = 920_201;
const STRANGER_CHARACTER: i64 = 920_202;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String, String) {
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
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, location, String::from_utf8_lossy(&bytes).into_owned())
}

/// Asserts a Laravel 422 with exactly one error message on one field.
fn assert_validation(status: StatusCode, body: &str, field: &str, message: &str) {
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "expected 422, body: {body}");
    let errors: serde_json::Value = serde_json::from_str(body).expect("json");
    assert_eq!(errors["message"], json!("The given data was invalid."));
    assert_eq!(errors["errors"][field], json!([message]), "field {field}");
}

async fn stored_price(pool: &sqlx::PgPool, user_id: i64, module_id: i64) -> Option<f64> {
    sqlx::query_scalar(
        "select price from module_pricing where user_id = $1 and module_id = $2",
    )
    .bind(user_id)
    .bind(module_id)
    .fetch_optional(pool)
    .await
    .expect("pricing lookup")
}

#[tokio::test]
async fn module_pricing_round_trip() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables).await.expect("seed");
    let reference = ReferenceData::from_tables(tables);

    // Two modules to price.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let mut module_ids = Vec::new();
    for module in &fixture.modules[..2] {
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
        module_ids.push(module.module_id);
    }
    let (module_a, module_b) = (module_ids[0], module_ids[1]);

    // A seller (owns the public asset) and a stranger; idempotent.
    sqlx::query("delete from public_assets where character_id = any($1)")
        .bind(vec![SELLER_CHARACTER, STRANGER_CHARACTER])
        .execute(&pool)
        .await
        .expect("cleanup public assets");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![SELLER_CHARACTER, STRANGER_CHARACTER])
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Pricing Seller", "Pricing Stranger"])
        .execute(&pool)
        .await
        .expect("cleanup users");
    // Other suites publish assets for these fixture modules too; the
    // payload assertions below rely on ours being the only one.
    sqlx::query("delete from public_assets where module_id = any($1)")
        .bind(&module_ids)
        .execute(&pool)
        .await
        .expect("cleanup module assets");

    let mut users = Vec::new();
    for (name, character_id) in
        [("Pricing Seller", SELLER_CHARACTER), ("Pricing Stranger", STRANGER_CHARACTER)]
    {
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
        let session =
            mutamarket::auth::session::create_session(&pool, user_id, Some(character_id))
                .await
                .expect("session");
        users.push((user_id, session));
    }
    let (seller_id, seller) = (users[0].0, users[0].1.clone());
    let (_stranger_id, stranger) = (users[1].0, users[1].1.clone());

    let app = mutamarket::server::test_router().await;

    // Guests are redirected to login.
    let (status, location, _) = send(
        &app,
        "POST",
        "/module-pricing",
        None,
        Some(json!({"module_pricing": []})),
    )
    .await;
    assert!(status.is_redirection(), "guest POST redirects, got {status}");
    assert_eq!(location, "/login");

    // Laravel-shaped validation, exact default messages.
    let (status, _, body) =
        send(&app, "POST", "/module-pricing", Some(&seller), Some(json!({}))).await;
    assert_validation(status, &body, "module_pricing", "The module pricing field is required.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": []})),
    )
    .await;
    assert_validation(status, &body, "module_pricing", "The module pricing field is required.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [{"price": 5}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "module_pricing.0.module_id",
        "The module pricing.0.module id field is required.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [{"module_id": 999_999_999, "price": 5}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "module_pricing.0.module_id",
        "The selected module pricing.0.module id is invalid.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [{"module_id": module_a}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "module_pricing.0.price",
        "The module pricing.0.price field is required.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [{"module_id": module_a, "price": "abc"}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "module_pricing.0.price",
        "The module pricing.0.price field must be a number.",
    );

    // Bulk store and upsert.
    let (status, _, _) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [
            {"module_id": module_a, "price": 1_500_000_000.0},
            {"module_id": module_b, "price": 250.5},
        ]})),
    )
    .await;
    assert!(status.is_redirection(), "store redirects, got {status}");
    assert_eq!(stored_price(&pool, seller_id, module_a).await, Some(1_500_000_000.0));
    assert_eq!(stored_price(&pool, seller_id, module_b).await, Some(250.5));

    let (status, _, _) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&seller),
        Some(json!({"module_pricing": [{"module_id": module_a, "price": 2_000_000.0}]})),
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(stored_price(&pool, seller_id, module_a).await, Some(2_000_000.0));
    let count: i64 = sqlx::query_scalar(
        "select count(*) from module_pricing where user_id = $1 and module_id = $2",
    )
    .bind(seller_id)
    .bind(module_a)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 1, "upsert must not duplicate");

    // The public_asset payload carries the owner's price. Publish the
    // asset for the seller.
    let asset_id: i64 = sqlx::query_scalar(
        "insert into assets (character_id, item_id, type_id, name, location_id, location_flag,
                             location_type, quantity, is_abyssal)
         values ($1, $2, $3, '', 60003760, 'Hangar', 'station', 1, true)
         returning id",
    )
    .bind(SELLER_CHARACTER)
    .bind(module_a)
    .bind(fixture.type_id)
    .fetch_one(&pool)
    .await
    .expect("seller asset");
    sqlx::query(
        "insert into public_assets (character_id, asset_id, module_id) values ($1, $2, $3)",
    )
    .bind(SELLER_CHARACTER)
    .bind(asset_id)
    .bind(module_a)
    .execute(&pool)
    .await
    .expect("publish asset");

    let (status, _, body) =
        send(&app, "GET", &format!("/api/module-page/{module_a}"), None, None).await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["module"]["public_asset"]["price"], json!(2_000_000.0));
    assert_eq!(
        page["module"]["public_asset"]["owner"]["id"],
        json!(SELLER_CHARACTER),
    );

    // A stranger's pricing does not leak onto someone else's asset: the
    // subselect joins module_pricing through the asset owner's user.
    let (status, _, _) = send(
        &app,
        "POST",
        "/module-pricing",
        Some(&stranger),
        Some(json!({"module_pricing": [{"module_id": module_a, "price": 1.0}]})),
    )
    .await;
    assert!(status.is_redirection());
    let (_, _, body) = send(&app, "GET", &format!("/api/module-page/{module_a}"), None, None).await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["module"]["public_asset"]["price"], json!(2_000_000.0));

    // Zero and negative prices delete the pricing (the legacy
    // noPriceSet), and an unpriced asset emits 0 through the legacy
    // `(float) null` cast.
    for gone in [json!(0), json!(-5)] {
        let (status, _, _) = send(
            &app,
            "POST",
            "/module-pricing",
            Some(&seller),
            Some(json!({"module_pricing": [
                {"module_id": module_a, "price": 42.0},
            ]})),
        )
        .await;
        assert!(status.is_redirection());
        let (status, _, _) = send(
            &app,
            "POST",
            "/module-pricing",
            Some(&seller),
            Some(json!({"module_pricing": [{"module_id": module_a, "price": gone}]})),
        )
        .await;
        assert!(status.is_redirection());
        assert_eq!(
            stored_price(&pool, seller_id, module_a).await,
            None,
            "price {gone} must delete the pricing",
        );
    }
    let (_, _, body) = send(&app, "GET", &format!("/api/module-page/{module_a}"), None, None).await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(page["module"]["public_asset"]["price"], json!(0.0));
}
