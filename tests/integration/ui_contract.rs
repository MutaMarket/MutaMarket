//! Behavior tests for `POST /ui/contract` (the legacy
//! `UIController::openContract`): validation, the OpenWindow scope gate
//! with its grant URL, and the ESI open-window call against a local mock
//! server, success and failure.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::ReferenceData;
use serde_json::json;
use sqlx::PgPool;

/// Characters owned by this suite alone, so parallel suites never share
/// state.
const SCOPED_CHARACTER: i64 = 920_401;
const UNSCOPED_CHARACTER: i64 = 920_402;
/// The seeded contract to open, and one whose open call the mock fails.
const CONTRACT: i64 = 920_401_777;
const FAILING_CONTRACT: i64 = 920_401_778;

const ACCESS_TOKEN: &str = "ui-contract-access";
/// The OpenWindow scope string, as stored on tokens.
const OPEN_WINDOW: &str = "esi-ui.open_window.v1";

fn app(pool: &PgPool, reference: ReferenceData, esi_url: &str) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new(esi_url),
        SsoClient::new(
            "http://127.0.0.1:9",
            "client",
            "secret",
            "http://test/eve/callback",
        ),
        mutamarket::auth::linked::LinkedClients::from_env(),
        mutamarket::estimator::Estimator::new(),
        std::sync::Arc::new(reference),
        None,
    )
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String, String) {
    let response = crate::common::http::request(method, path)
        .session(session)
        .json(body)
        .send(app)
        .await;
    (
        response.status,
        response.location().unwrap_or_default(),
        response.text(),
    )
}

/// Mock ESI: the open-window endpoint, recording each authorized call and
/// failing the designated contract with a 500.
fn mock_esi(calls: Arc<Mutex<Vec<i64>>>) -> Router {
    Router::new().route(
        "/latest/ui/openwindow/contract/",
        post(
            move |headers: HeaderMap,
                  axum::extract::Query(query): axum::extract::Query<HashMap<String, String>>| {
                let calls = calls.clone();
                async move {
                    if headers
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        != Some(&format!("Bearer {ACCESS_TOKEN}"))
                    {
                        return StatusCode::FORBIDDEN.into_response();
                    }
                    let contract_id: i64 = query
                        .get("contract_id")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or_default();
                    if contract_id == FAILING_CONTRACT {
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                    calls.lock().expect("lock").push(contract_id);
                    StatusCode::NO_CONTENT.into_response()
                }
            },
        ),
    )
}

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

#[tokio::test]
async fn opening_contracts_ingame() {
    let (pool, _db) = crate::common::db::fresh().await;

    let reference = crate::common::db::reference_data();

    // Contracts to open (region and issuer stubs like common::attach_contract).
    sqlx::query(
        "insert into regions (id, name) values (10000002, 'The Forge')
         on conflict (id) do nothing",
    )
    .execute(&pool)
    .await
    .expect("seed region");
    sqlx::query(
        "insert into characters (id, name) values (90999999, '') on conflict (id) do nothing",
    )
    .execute(&pool)
    .await
    .expect("seed issuer");
    for contract_id in [CONTRACT, FAILING_CONTRACT] {
        sqlx::query(
            "insert into contracts
             (id, region_id, issuer_id, type, unified_price, price, date_issued, date_expired)
             values ($1, 10000002, 90999999, 'item_exchange', 1000000, 1000000,
                     now(), now() + interval '7 days')
             on conflict (id) do nothing",
        )
        .bind(contract_id)
        .execute(&pool)
        .await
        .expect("seed contract");
    }

    // A scoped and an unscoped user; idempotent across runs.
    let characters = vec![SCOPED_CHARACTER, UNSCOPED_CHARACTER];
    sqlx::query("delete from esi_tokens where character_id = any($1)")
        .bind(&characters)
        .execute(&pool)
        .await
        .expect("cleanup tokens");
    sqlx::query("delete from characters where id = any($1)")
        .bind(&characters)
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Window Opener", "Window Wisher"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let mut sessions = Vec::new();
    for (name, character_id, scopes) in [
        ("Window Opener", SCOPED_CHARACTER, vec![OPEN_WINDOW]),
        ("Window Wisher", UNSCOPED_CHARACTER, vec![]),
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
        if !scopes.is_empty() {
            sqlx::query(
                "insert into esi_tokens
                 (character_id, access_token, refresh_token, token_type,
                  character_owner_hash, scopes, expires_at)
                 values ($1, $2, 'refresh', 'Bearer', 'owner', $3,
                         now() + interval '20 minutes')",
            )
            .bind(character_id)
            .bind(ACCESS_TOKEN)
            .bind(
                scopes
                    .iter()
                    .map(|scope| scope.to_string())
                    .collect::<Vec<_>>(),
            )
            .execute(&pool)
            .await
            .expect("seed token");
        }
        let session = create_session(&pool, user_id, Some(character_id))
            .await
            .expect("session");
        sessions.push(session);
    }
    let (scoped, unscoped) = (sessions[0].clone(), sessions[1].clone());

    let calls = Arc::new(Mutex::new(Vec::new()));
    let esi_url = start_mock(mock_esi(calls.clone())).await;
    let app = app(&pool, reference, &esi_url);

    // Guests are redirected to login.
    let (status, location, _) = send(&app, "POST", "/ui/contract", None, None).await;
    assert!(
        status.is_redirection(),
        "guest POST redirects, got {status}"
    );
    assert_eq!(location, "/login");

    // Laravel validation with the default messages.
    let (status, _, body) =
        send(&app, "POST", "/ui/contract", Some(&scoped), Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(errors["message"], json!("The given data was invalid."));
    assert_eq!(
        errors["errors"]["contract_id"],
        json!(["The contract id field is required."])
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/ui/contract",
        Some(&scoped),
        Some(json!({"contract_id": "abc"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        errors["errors"]["contract_id"],
        json!(["The contract id field must be an integer."]),
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/ui/contract",
        Some(&scoped),
        Some(json!({"contract_id": 999_999_999})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        errors["errors"]["contract_id"],
        json!(["The selected contract id is invalid."])
    );

    // Without the OpenWindow scope: the legacy notify (typo included)
    // pointing at the SSO grant URL.
    let (status, _, body) = send(
        &app,
        "POST",
        "/ui/contract",
        Some(&unscoped),
        Some(json!({"contract_id": CONTRACT})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        error["message"],
        json!("You need to grant the \"Open Window\" ESI scope to open th contract ingame!"),
    );
    assert_eq!(
        error["grant_scope_url"],
        json!("/eve?scopes=esi-ui.open_window.v1")
    );
    assert!(
        calls.lock().expect("lock").is_empty(),
        "no ESI call without the scope"
    );

    // With the scope: ESI receives the bearer-authorized open-window call
    // and the response heads back.
    let (status, _, body) = send(
        &app,
        "POST",
        "/ui/contract",
        Some(&scoped),
        Some(json!({"contract_id": CONTRACT})),
    )
    .await;
    assert!(
        status.is_redirection(),
        "success redirects back, got {status}: {body}"
    );
    assert_eq!(calls.lock().expect("lock").as_slice(), [CONTRACT]);

    // An ESI failure reports the exact legacy message.
    let (status, _, body) = send(
        &app,
        "POST",
        "/ui/contract",
        Some(&scoped),
        Some(json!({"contract_id": FAILING_CONTRACT})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let error: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        error["message"],
        json!("An error occurred while trying to open the contract in the EVE Online client."),
    );
    // The token survives a plain upstream failure (only 401/403 drop it).
    let token_alive: bool =
        sqlx::query_scalar("select exists(select 1 from esi_tokens where character_id = $1)")
            .bind(SCOPED_CHARACTER)
            .fetch_one(&pool)
            .await
            .expect("token lookup");
    assert!(token_alive);
}
