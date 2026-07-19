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


/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::EstimatorClient {
    mutamarket::estimator::EstimatorClient::new("http://127.0.0.1:9")
}

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
    let pool = db::test_pool()
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

    // The browser shows for-sale modules only; give ours a live contract.
    common::attach_contract(
        &pool,
        module.module_id,
        800_101,
        "item_exchange",
        150_000_000.0,
        1,
        0,
        0,
    )
    .await;

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

    // The attribute rows carry the unit-formatted values and display names.
    let card = mutamarket::modules::queries::module_detail(&pool, &reference, module.module_id)
        .await
        .expect("module detail query")
        .expect("module exists");
    let visual = card
        .mutated_attributes
        .iter()
        .find(|attribute| attribute.is_visual())
        .expect("a visual attribute");
    assert!(
        detail.contains(&visual.formatted_value()),
        "attribute rows carry {} for {}",
        visual.formatted_value(),
        visual.name,
    );
    assert!(
        detail.contains(&visual.display_name),
        "attribute rows carry the display name {}",
        visual.display_name,
    );
    assert!(
        detail.contains(&visual.formatted_difference()),
        "attribute rows carry the difference {}",
        visual.formatted_difference(),
    );

    // Display settings drive the attribute bar modes, like the legacy
    // BarTypeNormalized/BarAbsolute/AttributeScore components.
    let detail_url = format!("/modules/50mn-abyssal-microwarpdrive-{}", module.module_id);

    let (_, type_mode) = get_page(&app, &detail_url, Some("attribute_bar_mode=type")).await;
    assert!(
        type_mode.contains("bg-white/25"),
        "type mode renders the mutaplasmid range band",
    );

    let (_, absolute_mode) = get_page(
        &app,
        &detail_url,
        Some("attribute_bar_mode=absolute; show_attribute_scores=1"),
    )
    .await;
    assert!(
        absolute_mode.contains("attribute-absolute"),
        "absolute mode renders the left-origin fill",
    );
    assert!(
        absolute_mode.contains("text-red-500"),
        "scores show for these badly rolled attributes",
    );

    let (_, none_mode) = get_page(&app, &detail_url, Some("attribute_bar_mode=none")).await;
    assert!(!none_mode.contains("h-[3px]"), "none mode renders no bars");

    // PUT /display persists the settings as cookies and redirects back.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/display")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"display":"grid","attribute_bar_mode":"type","show_attribute_scores":true}"#,
                ))
                .expect("valid request"),
        )
        .await
        .expect("infallible");
    assert!(response.status().is_redirection());
    let cookies: Vec<_> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect();
    assert_eq!(cookies.len(), 3, "three display cookies set: {cookies:?}");
    assert!(cookies.iter().any(|cookie| cookie.starts_with("attribute_bar_mode=type")));

    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/display")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"attribute_bar_mode":"sideways"}"#))
                .expect("valid request"),
        )
        .await
        .expect("infallible");
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // An unknown module id is a real 404.
    let (status, missing) = get_page(&app, "/modules/hypnotic-web-999999999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(missing.contains("Module not found"));

    // A non-id query renders the browser, not a 404.
    let (status, browser) = get_page(&app, "/modules/damage-control", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(browser.contains("Abyssal Modules"));

    // Without a type the filter panel offers the category dialog trigger
    // (showing "All") but no sliders.
    let (_, no_type) = get_page(&app, "/", None).await;
    assert!(no_type.contains("Category"), "the panel has the category picker");
    assert!(no_type.contains("Meta group"), "the panel has the meta group filter");
    assert!(no_type.contains("Roll quality"), "the panel has the sort buttons");

    // With a type selected, each mutated attribute gets a range slider fed
    // by the aggregated roll bounds.
    let (status, with_type) =
        get_page(&app, "/modules/type/50mn-abyssal-microwarpdrive", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(with_type.contains("Clear type"), "the selected type can be cleared");
    let sliders = mutamarket::modules::queries::type_filter_attributes(&pool, fixture.type_id)
        .await
        .expect("filter attributes query");
    assert!(!sliders.is_empty(), "the fixture type has slider attributes");
    for slider in &sliders {
        assert!(
            slider.best != slider.worst,
            "slider bounds span a range: {slider:?}",
        );
    }

    // The display options bar sits above the grid.
    assert!(with_type.contains("Bars"), "the options bar renders");

    // The category trigger shows the selected type with the mutation words
    // stripped, like the legacy dialog trigger.
    assert!(
        with_type.contains("50MN  Microwarpdrive"),
        "the category trigger strips 'Abyssal' from the type name",
    );

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
    assert!(
        home_logged_in.contains("href=\"/personal/modules\"")
            && home_logged_in.contains("My modules"),
        "nav links the personal modules page for logged-in users",
    );
    let (_, home_guest) = get_page(&app, "/", None).await;
    assert!(!home_guest.contains("My modules"), "guests see no personal link");

    // The login page offers the EVE SSO entry; rel="external" keeps the
    // client-side router from capturing the OAuth redirect.
    let (status, login) = get_page(&app, "/login", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(login.contains("Log in with EVE Online"));
    assert!(login.contains("href=\"/eve\""));
    assert!(login.contains("rel=\"external\""));

    // Cleanup the session user to keep reruns deterministic.
    sqlx::query("delete from users where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("cleanup user");
}
