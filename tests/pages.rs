//! Behavior tests for the SSR pages: real data rendered into HTML through
//! the full router, including login state and the module detail page.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use tower::ServiceExt;

async fn get_page(app: &Router, path: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::builder().uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }

    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn pages_render_modules_and_login_state() {
    let pool = db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let module = &fixture.modules[0];

    process_module(
        &pool,
        &reference,
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

    let app = mutamarket::server::test_router().await;

    // The home page lists the ingested module and shows the guest state.
    let (status, home) = get_page(&app, "/", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(home.contains("Abyssal Modules"), "home renders the browser");
    assert!(
        home.contains(&format!("/modules/50mn-abyssal-microwarpdrive-{}", module.module_id)),
        "home links the ingested module",
    );
    assert!(home.contains("Log in"), "guests see the login link");

    // The module detail page renders the type, source and attribute data.
    let (status, detail) = get_page(
        &app,
        &format!("/modules/50mn-abyssal-microwarpdrive-{}", module.module_id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(detail.contains("50MN Abyssal Microwarpdrive"));
    assert!(detail.contains("Mutated from"), "shows the source module");
    assert!(detail.contains("Roll quality:"), "shows the average fraction");
    // The rolled value of the first expected attribute, formatted.
    let expected_value =
        mutamarket::modules::view::format_number(module.expected.attributes[0].value);
    assert!(
        detail.contains(&expected_value),
        "attribute table carries the rolled values",
    );

    // An unknown module id is a real 404.
    let (status, missing) = get_page(&app, "/modules/hypnotic-web-999999999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(missing.contains("Module not found"));

    // A non-id query renders the browser, not a 404.
    let (status, browser) = get_page(&app, "/modules/damage-control", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(browser.contains("Abyssal Modules"));

    // A logged-in user sees their name in the navigation.
    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
        .bind("Page Test Pilot")
        .fetch_one(&pool)
        .await
        .expect("create user");
    let session = mutamarket::auth::session::create_session(&pool, user_id, None)
        .await
        .expect("create session");

    let (status, home_logged_in) =
        get_page(&app, "/", Some(&format!("mm_session={session}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(home_logged_in.contains("Page Test Pilot"), "nav shows the user");
    assert!(home_logged_in.contains("Log out"), "nav offers logout");

    // The login page offers the EVE SSO entry.
    let (status, login) = get_page(&app, "/login", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(login.contains("Log in with EVE Online"));
    assert!(login.contains("href=\"/eve\""));

    // Cleanup the session user to keep reruns deterministic.
    sqlx::query("delete from users where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("cleanup user");
}
