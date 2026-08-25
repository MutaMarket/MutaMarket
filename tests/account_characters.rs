//! Behavior tests for multi-character accounts: the add-to-account SSO
//! flow, switching the acting character, and the removal guards — the
//! legacy EveController/UserCharacterController semantics.


use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Method, Request, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::ReferenceData;
use serde_json::json;
use tower::ServiceExt;

/// Same HS256 test key the sso suite uses.
const JWT_SECRET: &[u8] = b"0123456789abcdef0123456789abcdef";
const JWT_SECRET_BASE64URL: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY";
const JWT_KEY_ID: &str = "test-key";

const PILOT_ONE: i64 = 810_001;
const PILOT_TWO: i64 = 810_002;

/// The identity the mock SSO currently issues: (character id, name).
type Identity = Arc<Mutex<(i64, String)>>;

fn signed_access_token(character_id: i64, name: &str) -> String {
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs()
        + 1200;

    let claims = json!({
        "sub": format!("CHARACTER:EVE:{character_id}"),
        "name": name,
        "owner": format!("owner-{character_id}"),
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
    .expect("token signs")
}

async fn start_mock_sso(identity: Identity) -> String {
    let token_identity = identity.clone();
    let affiliation_identity = identity.clone();

    let app = Router::new()
        .route(
            "/v2/oauth/token",
            post(move || {
                let (character_id, name) = token_identity.lock().expect("identity").clone();
                async move {
                    Json(json!({
                        "access_token": signed_access_token(character_id, &name),
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
                        "kty": "oct", "kid": JWT_KEY_ID, "alg": "HS256",
                        "k": JWT_SECRET_BASE64URL,
                    }]
                }))
            }),
        )
        .route(
            "/latest/characters/affiliation/",
            post(move || {
                let (character_id, _) = affiliation_identity.lock().expect("identity").clone();
                async move {
                    Json(json!([
                        { "character_id": character_id, "corporation_id": 1_000_001 }
                    ]))
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind mock SSO");
    let address = listener.local_addr().expect("mock SSO address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock SSO");
    });

    format!("http://{address}")
}

fn estimator_stub() -> mutamarket::estimator::EstimatorClient {
    mutamarket::estimator::EstimatorClient::new("http://127.0.0.1:9")
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

/// Completes the SSO round trip, optionally with an existing session and
/// the add-to-account flag, returning the callback response.
async fn oauth_round_trip(
    app: &Router,
    add_to_account: bool,
    session: Option<&str>,
) -> axum::response::Response {
    let login_uri = if add_to_account { "/eve?add_to_account=true" } else { "/eve" };
    let login =
        send(app, Request::builder().uri(login_uri).body(Body::empty()).expect("request")).await;
    let state = location(&login).split("state=").nth(1).expect("state").to_owned();
    let state_cookie = cookie_from(&login, "mm_oauth_state").expect("state cookie");

    let mut cookies = format!("mm_oauth_state={state_cookie}");
    if add_to_account {
        let marker = cookie_from(&login, "mm_add_account").expect("add-to-account marker cookie");
        cookies.push_str(&format!("; mm_add_account={marker}"));
    }
    if let Some(session) = session {
        cookies.push_str(&format!("; mm_session={session}"));
    }

    send(
        app,
        Request::builder()
            .uri(format!("/eve/callback?code=mock-code&state={state}"))
            .header(header::COOKIE, cookies)
            .body(Body::empty())
            .expect("request"),
    )
    .await
}

#[tokio::test]
async fn accounts_add_switch_and_remove_characters() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Idempotency across runs.
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![PILOT_ONE, PILOT_TWO])
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Pilot One", "Pilot Two"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let identity: Identity = Arc::new(Mutex::new((PILOT_ONE, "Pilot One".to_owned())));
    let mock_url = start_mock_sso(identity.clone()).await;

    let app = mutamarket::server::router(
        pool.clone(),
        EsiClient::new(&mock_url),
        SsoClient::new(&mock_url, "client-id", "client-secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator_stub(),
        std::sync::Arc::new(ReferenceData::default()),
    );

    // Log in as Pilot One; a fresh account owns the character.
    let callback = oauth_round_trip(&app, false, None).await;
    let session = cookie_from(&callback, "mm_session").expect("session cookie");
    let user_id: Option<i64> =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(PILOT_ONE)
            .fetch_one(&pool)
            .await
            .expect("character row");
    let user_id = user_id.expect("character linked");

    // Add Pilot Two to the same account: no new session is minted, the
    // character joins the existing user and becomes the acting character.
    *identity.lock().expect("identity") = (PILOT_TWO, "Pilot Two".to_owned());
    let callback = oauth_round_trip(&app, true, Some(&session)).await;
    assert!(callback.status().is_redirection());
    assert_eq!(location(&callback), "/");
    assert!(
        cookie_from(&callback, "mm_session").is_none(),
        "adding a character keeps the existing session",
    );
    let second_user: Option<i64> =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(PILOT_TWO)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert_eq!(second_user, Some(user_id), "the character joins the account");
    let active: Option<i64> =
        sqlx::query_scalar("select active_character_id from sessions where token = $1")
            .bind(&session)
            .fetch_one(&pool)
            .await
            .expect("session row");
    assert_eq!(active, Some(PILOT_TWO), "the new character becomes active");

    // Switching: guests are redirected, foreign characters are forbidden,
    // owned ones become the session's acting character.
    let response = send(
        &app,
        Request::builder()
            .method(Method::PUT)
            .uri(format!("/auth/character/{PILOT_ONE}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(location(&response), "/login");

    let response = send(
        &app,
        Request::builder()
            .method(Method::PUT)
            .uri(format!("/auth/character/pilot-one-{PILOT_ONE}"))
            .header(header::COOKIE, format!("mm_session={session}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(response.status().is_redirection(), "switch redirects back");
    let active: Option<i64> =
        sqlx::query_scalar("select active_character_id from sessions where token = $1")
            .bind(&session)
            .fetch_one(&pool)
            .await
            .expect("session row");
    assert_eq!(active, Some(PILOT_ONE));

    let foreign = send(
        &app,
        Request::builder()
            .method(Method::PUT)
            .uri("/auth/character/999999998")
            .header(header::COOKIE, format!("mm_session={session}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert_eq!(foreign.status(), axum::http::StatusCode::FORBIDDEN);

    // Removing the active character falls back to the remaining one;
    // removing the last character is refused (still linked).
    let response = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri(format!("/auth/character/{PILOT_ONE}"))
            .header(header::COOKIE, format!("mm_session={session}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(response.status().is_redirection());
    let unlinked: Option<i64> =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(PILOT_ONE)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert_eq!(unlinked, None, "the character unlinks from the account");
    let active: Option<i64> =
        sqlx::query_scalar("select active_character_id from sessions where token = $1")
            .bind(&session)
            .fetch_one(&pool)
            .await
            .expect("session row");
    assert_eq!(active, Some(PILOT_TWO), "the active falls back to the remaining character");

    let response = send(
        &app,
        Request::builder()
            .method(Method::DELETE)
            .uri(format!("/auth/character/{PILOT_TWO}"))
            .header(header::COOKIE, format!("mm_session={session}"))
            .body(Body::empty())
            .expect("request"),
    )
    .await;
    assert!(response.status().is_redirection());
    let still_linked: Option<i64> =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(PILOT_TWO)
            .fetch_one(&pool)
            .await
            .expect("character row");
    assert_eq!(still_linked, Some(user_id), "the last character cannot be removed");
}
