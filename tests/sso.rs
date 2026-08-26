//! Behavior tests for the EVE SSO login flow, with the SSO token/verify
//! endpoints and the ESI affiliation endpoint replaced by local mocks.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::ReferenceData;
use serde_json::json;
use tower::ServiceExt;


/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

const CHARACTER_ID: i64 = 90_000_001;
const CHARACTER_NAME: &str = "Test Pilot";

/// Symmetric test key served via the mock JWKS and used to sign the mock
/// access tokens; the verify code treats it like EVE's RSA keys.
const JWT_SECRET: &[u8] = b"0123456789abcdef0123456789abcdef";
const JWT_SECRET_BASE64URL: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";
const JWT_KEY_ID: &str = "test-key";

/// A signed EVE-shaped access token for the current owner hash.
fn signed_access_token(owner_hash: &str) -> String {
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 1200;

    let claims = json!({
        "sub": format!("CHARACTER:EVE:{CHARACTER_ID}"),
        "name": CHARACTER_NAME,
        "owner": owner_hash,
        "scp": ["publicData", "esi-assets.read_assets.v1"],
        "iss": "login.eveonline.com",
        "exp": expires_at,
    });

    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some(JWT_KEY_ID.to_owned());

    jsonwebtoken::encode(
        &header,
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET),
    )
    .expect("sign token")
}

/// A mock for both the SSO (token + JWKS) and ESI (affiliation) endpoints;
/// the owner hash is mutable so tests can simulate a character transfer.
async fn start_mock_sso(owner_hash: Arc<Mutex<String>>) -> String {
    let token_hash = owner_hash.clone();

    let app = Router::new()
        .route(
            "/v2/oauth/token",
            post(move || {
                let hash = token_hash.lock().expect("owner hash lock").clone();
                async move {
                    Json(json!({
                        "access_token": signed_access_token(&hash),
                        "refresh_token": "mock-refresh-token",
                        "token_type": "Bearer",
                        "expires_in": 1199,
                    }))
                    .into_response()
                }
            }),
        )
        .route(
            "/oauth/jwks",
            get(|| async {
                Json(json!({
                    "keys": [{
                        "kty": "oct",
                        "kid": JWT_KEY_ID,
                        "alg": "HS256",
                        "k": JWT_SECRET_BASE64URL,
                    }]
                }))
            }),
        )
        .route(
            "/latest/characters/affiliation/",
            post(|| async {
                Json(json!([
                    { "character_id": CHARACTER_ID, "corporation_id": 1_000_001 }
                ]))
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock SSO");
    let address = listener.local_addr().expect("mock SSO address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock SSO");
    });

    format!("http://{address}")
}

fn cookie_from(response: &axum::response::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|cookie| cookie.starts_with(&format!("{name}=")) && !cookie.contains("Max-Age=0"))
        .and_then(|cookie| cookie.split(';').next())
        .and_then(|pair| pair.split_once('='))
        .map(|(_, value)| value.to_owned())
}

fn location(response: &axum::response::Response) -> String {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

async fn send(app: &Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.expect("infallible")
}

/// Runs the full login round trip and returns the session cookie.
async fn log_in(app: &Router) -> String {
    let login = send(app, Request::builder().uri("/eve").body(Body::empty()).expect("request")).await;
    assert!(login.status().is_redirection());

    let authorize_url = location(&login);
    let state = authorize_url
        .split("state=")
        .nth(1)
        .expect("state parameter")
        .to_owned();
    let state_cookie = cookie_from(&login, "mm_oauth_state").expect("state cookie");

    let callback = send(
        app,
        Request::builder()
            .uri(format!("/eve/callback?code=mock-code&state={state}"))
            .header(header::COOKIE, format!("mm_oauth_state={state_cookie}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;

    assert!(callback.status().is_redirection());
    assert_eq!(location(&callback), "/");

    cookie_from(&callback, "mm_session").expect("session cookie")
}

#[tokio::test]
async fn sso_login_creates_accounts_and_sessions() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Isolate from previous runs of this test.
    sqlx::query("delete from characters where id = $1")
        .bind(CHARACTER_ID)
        .execute(&pool)
        .await
        .expect("clean character");

    let owner_hash = Arc::new(Mutex::new("owner-hash-1".to_owned()));
    let mock_url = start_mock_sso(owner_hash.clone()).await;

    let app = mutamarket::server::router(
        pool.clone(),
        EsiClient::new(&mock_url),
        SsoClient::new(&mock_url, "client-id", "client-secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator_stub(),
        Arc::new(ReferenceData::default()),
        None,
    );

    // The login redirect points at the SSO with our client id and a state.
    let login = send(&app, Request::builder().uri("/eve").body(Body::empty()).expect("request")).await;
    let authorize_url = location(&login);
    assert!(authorize_url.starts_with(&format!("{mock_url}/v2/oauth/authorize/")));
    assert!(authorize_url.contains("client_id=client-id"));
    assert!(authorize_url.contains("scope=esi-structures.read_character.v1%20"));

    // A callback with a wrong state is rejected without a session.
    let bad_state = send(
        &app,
        Request::builder()
            .uri("/eve/callback?code=mock-code&state=wrong")
            .header(header::COOKIE, "mm_oauth_state=other")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(location(&bad_state), "/");
    assert!(cookie_from(&bad_state, "mm_session").is_none());

    // First login creates the account, character link and token.
    let session = log_in(&app).await;

    let (user_id, stored_hash, name): (Option<i64>, Option<String>, String) = sqlx::query_as(
        "select user_id, character_owner_hash, name from characters where id = $1",
    )
    .bind(CHARACTER_ID)
    .fetch_one(&pool)
    .await
    .expect("character row");
    let first_user_id = user_id.expect("character linked to a user");
    assert_eq!(stored_hash.as_deref(), Some("owner-hash-1"));
    assert_eq!(name, CHARACTER_NAME);

    let scopes: Vec<String> = sqlx::query_scalar(
        "select unnest(scopes) from esi_tokens where character_id = $1 order by 1",
    )
    .bind(CHARACTER_ID)
    .fetch_all(&pool)
    .await
    .expect("token scopes");
    assert_eq!(scopes, vec!["esi-assets.read_assets.v1", "publicData"]);

    let session_user: i64 = sqlx::query_scalar("select user_id from sessions where token = $1")
        .bind(&session)
        .fetch_one(&pool)
        .await
        .expect("session row");
    assert_eq!(session_user, first_user_id);

    // A second login with the same owner hash reuses the account.
    let _again = log_in(&app).await;
    let (user_id_again,): (Option<i64>,) =
        sqlx::query_as("select user_id from characters where id = $1")
            .bind(CHARACTER_ID)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert_eq!(user_id_again, Some(first_user_id));

    // A changed owner hash means the character was sold: new account, and
    // the old account (now characterless) is cleaned up.
    *owner_hash.lock().expect("owner hash lock") = "owner-hash-2".to_owned();
    let _transferred = log_in(&app).await;
    let (user_id_after_transfer, hash_after_transfer): (Option<i64>, Option<String>) =
        sqlx::query_as("select user_id, character_owner_hash from characters where id = $1")
            .bind(CHARACTER_ID)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert_ne!(user_id_after_transfer, Some(first_user_id));
    assert_eq!(hash_after_transfer.as_deref(), Some("owner-hash-2"));

    let old_user_still_there: Option<i64> = sqlx::query_scalar("select id from users where id = $1")
        .bind(first_user_id)
        .fetch_optional(&pool)
        .await
        .expect("old user lookup");
    assert!(
        old_user_still_there.is_none(),
        "characterless account must be deleted after a transfer",
    );

    // Logout destroys the session; logging out as a guest redirects to login.
    let logout = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/logout")
            .header(header::COOKIE, format!("mm_session={session}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(location(&logout), "/");

    let session_still_there: Option<i64> =
        sqlx::query_scalar("select user_id from sessions where token = $1")
            .bind(&session)
            .fetch_optional(&pool)
            .await
            .expect("session lookup");
    assert!(session_still_there.is_none());

    let guest_logout = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/logout")
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(location(&guest_logout), "/login");
}
