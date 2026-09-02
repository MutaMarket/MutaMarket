//! Behavior tests for ESI access-token acquisition: lazy refresh through a
//! mock SSO token endpoint, persistence of rotated tokens, scope matching,
//! and the legacy delete-on-rejection behavior for revoked refresh tokens.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::auth::tokens::{TokenError, characters_with_scope, valid_access_token};
use mutamarket::db;
use serde_json::json;
use sqlx::PgPool;

const SCOPE: &str = "esi-assets.read_assets.v1";
const OTHER_SCOPE: &str = "esi-contracts.read_character_contracts.v1";
const CLIENT_ID: &str = "client-id";
const CLIENT_SECRET: &str = "client-secret";

/// What the mock SSO token endpoint saw and how it should answer.
#[derive(Clone)]
struct MockSso {
    /// Form bodies of every refresh request received.
    requests: Arc<Mutex<Vec<(String, String)>>>,
    calls: Arc<AtomicUsize>,
    /// Status codes to serve before succeeding (drained front to back).
    failures: Arc<Mutex<Vec<StatusCode>>>,
}

impl MockSso {
    fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            calls: Arc::new(AtomicUsize::new(0)),
            failures: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

async fn token_endpoint(
    State(mock): State<MockSso>,
    headers: HeaderMap,
    body: String,
) -> axum::response::Response {
    let call = mock.calls.fetch_add(1, Ordering::SeqCst);

    let authorization = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    mock.requests
        .lock()
        .expect("requests lock")
        .push((authorization, body));

    let failure = {
        let mut failures = mock.failures.lock().expect("failures lock");
        (!failures.is_empty()).then(|| failures.remove(0))
    };
    if let Some(status) = failure {
        return (status, "upstream error").into_response();
    }

    Json(json!({
        "access_token": format!("refreshed-access-{call}"),
        "refresh_token": format!("rotated-refresh-{call}"),
        "token_type": "Bearer",
        "expires_in": 1199,
    }))
    .into_response()
}

async fn start_mock_sso(mock: MockSso) -> String {
    let app = Router::new()
        .route("/v2/oauth/token", post(token_endpoint))
        .with_state(mock);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock SSO");
    let address = listener.local_addr().expect("mock SSO address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock SSO");
    });

    format!("http://{address}")
}

/// Seeds the character and a token row; `expires_in_minutes` may be
/// negative for an already-expired token. Returns the token row id.
async fn seed_token(
    pool: &PgPool,
    character_id: i64,
    access_token: &str,
    refresh_token: &str,
    scopes: &[&str],
    expires_in_minutes: i32,
) -> i64 {
    sqlx::query(
        "insert into characters (id, name) values ($1, 'Token Pilot') on conflict (id) do nothing",
    )
    .bind(character_id)
    .execute(pool)
    .await
    .expect("seed character");

    sqlx::query_scalar(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, $2, $3, 'Bearer', 'owner-hash', $4, now() + make_interval(mins => $5))
         returning id",
    )
    .bind(character_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(
        scopes
            .iter()
            .map(|scope| scope.to_string())
            .collect::<Vec<_>>(),
    )
    .bind(expires_in_minutes)
    .fetch_one(pool)
    .await
    .expect("seed token")
}

/// Each test gets its own character so the parallel test threads never
/// race on each other's token rows.
async fn setup(character_id: i64) -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Idempotent across runs and suites: drop this character's tokens.
    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(character_id)
        .execute(&pool)
        .await
        .expect("clean tokens");

    pool
}

fn sso_client(base_url: &str) -> SsoClient {
    SsoClient::new(
        base_url,
        CLIENT_ID,
        CLIENT_SECRET,
        "http://test/eve/callback",
    )
}

#[tokio::test]
async fn valid_tokens_are_returned_without_a_refresh() {
    const CHARACTER: i64 = 92_000_001;
    let pool = setup(CHARACTER).await;
    let mock = MockSso::new();
    let sso = sso_client(&start_mock_sso(mock.clone()).await);

    // Well past the five-minute expiry buffer.
    seed_token(
        &pool,
        CHARACTER,
        "live-access",
        "live-refresh",
        &[SCOPE, "publicData"],
        20,
    )
    .await;

    let token = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect("token lookup")
        .expect("token present");
    assert_eq!(token.access_token, "live-access");
    assert_eq!(
        mock.calls.load(Ordering::SeqCst),
        0,
        "no refresh for a live token"
    );

    // Characters holding the scope are listed for the sync fan-outs
    // (other suites may seed more; ours must be among them).
    assert!(
        characters_with_scope(&pool, SCOPE)
            .await
            .expect("scope query")
            .contains(&CHARACTER),
    );
    assert!(
        !characters_with_scope(&pool, OTHER_SCOPE)
            .await
            .expect("scope query")
            .contains(&CHARACTER),
    );
}

#[tokio::test]
async fn tokens_inside_the_expiry_buffer_are_refreshed_and_persisted() {
    const CHARACTER: i64 = 92_000_002;
    let pool = setup(CHARACTER).await;
    let mock = MockSso::new();
    let sso = sso_client(&start_mock_sso(mock.clone()).await);

    // Expires in two minutes: inside the legacy five-minute buffer.
    let token_id = seed_token(&pool, CHARACTER, "stale-access", "old-refresh", &[SCOPE], 2).await;

    let token = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect("token refresh")
        .expect("token present");
    assert_eq!(token.access_token, "refreshed-access-0");
    assert_eq!(token.token_id, token_id);

    // The grant went out with basic auth and the legacy form fields.
    let requests = mock.requests.lock().expect("requests lock").clone();
    assert_eq!(requests.len(), 1);
    let (authorization, body) = &requests[0];
    let expected_basic = format!(
        "Basic {}",
        base64_encode(format!("{CLIENT_ID}:{CLIENT_SECRET}").as_bytes()),
    );
    assert_eq!(authorization, &expected_basic);
    assert_eq!(body, "grant_type=refresh_token&refresh_token=old-refresh");

    // Both tokens rotated in place, expiry pushed out.
    let (access, refresh, live): (String, String, bool) = sqlx::query_as(
        "select access_token, refresh_token, expires_at > now() + interval '15 minutes'
         from esi_tokens where id = $1",
    )
    .bind(token_id)
    .fetch_one(&pool)
    .await
    .expect("token row");
    assert_eq!(access, "refreshed-access-0");
    assert_eq!(refresh, "rotated-refresh-0");
    assert!(live, "expires_at moved into the future");

    // The next call rides the refreshed token without another grant.
    let again = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect("token lookup")
        .expect("token present");
    assert_eq!(again.access_token, "refreshed-access-0");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn rejected_refreshes_delete_the_token() {
    const CHARACTER: i64 = 92_000_003;
    let pool = setup(CHARACTER).await;
    let mock = MockSso::new();
    // A revoked refresh token: the SSO answers 400 invalid_grant.
    mock.failures
        .lock()
        .expect("failures lock")
        .push(StatusCode::BAD_REQUEST);
    let sso = sso_client(&start_mock_sso(mock.clone()).await);

    let token_id = seed_token(
        &pool,
        CHARACTER,
        "stale-access",
        "revoked-refresh",
        &[SCOPE],
        -1,
    )
    .await;

    let error = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect_err("refresh must fail");
    assert!(
        matches!(&error, TokenError::RefreshRejected { status, .. } if *status == StatusCode::BAD_REQUEST),
        "unexpected error: {error}",
    );

    // The row is hard-deleted, like the legacy connector: the character
    // has no token until the next SSO login.
    let row: Option<i64> = sqlx::query_scalar("select id from esi_tokens where id = $1")
        .bind(token_id)
        .fetch_optional(&pool)
        .await
        .expect("token lookup");
    assert!(row.is_none(), "rejected refresh deletes the token row");

    assert!(
        valid_access_token(&pool, &sso, CHARACTER, SCOPE)
            .await
            .expect("token lookup")
            .is_none(),
        "no token left with the scope",
    );
    // Only the failure attempt reached the SSO; the follow-up found no row.
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn server_errors_are_retried_before_succeeding() {
    const CHARACTER: i64 = 92_000_004;
    let pool = setup(CHARACTER).await;
    let mock = MockSso::new();
    // One 502, then success: the legacy connector retries 5xx.
    mock.failures
        .lock()
        .expect("failures lock")
        .push(StatusCode::BAD_GATEWAY);
    let sso = sso_client(&start_mock_sso(mock.clone()).await);

    seed_token(
        &pool,
        CHARACTER,
        "stale-access",
        "old-refresh",
        &[SCOPE],
        -1,
    )
    .await;

    let token = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect("refresh retried")
        .expect("token present");
    assert_eq!(token.access_token, "refreshed-access-1");
    assert_eq!(
        mock.calls.load(Ordering::SeqCst),
        2,
        "one retry after the 502"
    );
}

#[tokio::test]
async fn the_newest_token_with_the_scope_wins() {
    const CHARACTER: i64 = 92_000_005;
    let pool = setup(CHARACTER).await;
    let mock = MockSso::new();
    let sso = sso_client(&start_mock_sso(mock.clone()).await);

    // An older token with the scope, a newer one without it, and the
    // newest one with it again — the legacy latest() picks the last.
    seed_token(&pool, CHARACTER, "old-access", "old-refresh", &[SCOPE], 30).await;
    seed_token(
        &pool,
        CHARACTER,
        "unrelated-access",
        "unrelated-refresh",
        &[OTHER_SCOPE],
        30,
    )
    .await;
    let newest = seed_token(
        &pool,
        CHARACTER,
        "new-access",
        "new-refresh",
        &[SCOPE, OTHER_SCOPE],
        30,
    )
    .await;
    // created_at ties within the same test run: break it explicitly.
    sqlx::query("update esi_tokens set created_at = now() + interval '1 second' where id = $1")
        .bind(newest)
        .execute(&pool)
        .await
        .expect("bump created_at");

    let token = valid_access_token(&pool, &sso, CHARACTER, SCOPE)
        .await
        .expect("token lookup")
        .expect("token present");
    assert_eq!(token.access_token, "new-access");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
}

/// Minimal standard base64 for the basic-auth assertion.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}
