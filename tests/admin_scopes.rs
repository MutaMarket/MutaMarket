//! Behavior tests for the admin-scope check (the legacy hourly
//! `app:check-admin-scopes`): missing-scope detection over the service
//! character's token union, the exact Discord alert payload against a
//! local mock webhook, the check-only path without a webhook, and the
//! legacy character-not-found failure.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use mutamarket::admin_scopes::{ScopeCheckError, check_admin_scopes};
use mutamarket::auth::scopes;
use mutamarket::db;
use serde_json::Value;
use sqlx::PgPool;

/// The service character under test (unique across suites).
const SERVICE: i64 = 91_300_001;
/// A character id no row exists for.
const UNKNOWN_CHARACTER: i64 = 91_300_099;

type Captured = Arc<Mutex<Vec<Value>>>;

/// Mock Discord webhook: `/webhook` records every posted body, `/broken`
/// always fails.
fn mock_webhook(captured: Captured) -> Router {
    async fn record(State(captured): State<Captured>, Json(body): Json<Value>) -> StatusCode {
        captured.lock().expect("captured lock").push(body);
        StatusCode::NO_CONTENT
    }
    Router::new()
        .route("/webhook", post(record))
        .route("/broken", post(async || StatusCode::INTERNAL_SERVER_ERROR))
        .with_state(captured)
}

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind mock webhook");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock webhook");
    });
    format!("http://{address}")
}

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    sqlx::query("insert into characters (id, name) values ($1, 'Scope Checked') on conflict (id) do nothing")
        .bind(SERVICE)
        .execute(&pool)
        .await
        .expect("seed service character");
    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(SERVICE)
        .execute(&pool)
        .await
        .expect("clean tokens");
    sqlx::query("delete from characters where id = $1")
        .bind(UNKNOWN_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean unknown character");
    pool
}

async fn insert_token(pool: &PgPool, token_scopes: &[&str]) {
    let token_scopes: Vec<String> = token_scopes.iter().map(|scope| (*scope).to_owned()).collect();
    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, character_owner_hash, scopes, expires_at)
         values ($1, 'access', 'refresh', 'owner-hash', $2, now() + interval '20 minutes')",
    )
    .bind(SERVICE)
    .bind(&token_scopes)
    .execute(pool)
    .await
    .expect("insert token");
}

fn sorted_keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value.as_object().expect("object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

#[tokio::test]
async fn admin_scope_check_detects_missing_scopes_and_alerts() {
    let pool = setup().await;
    let captured: Captured = Arc::new(Mutex::new(Vec::new()));
    let base = start_mock(mock_webhook(captured.clone())).await;
    let webhook = format!("{base}/webhook");

    // Full coverage split across two tokens: the union counts, no alert.
    insert_token(&pool, &scopes::ADMIN_LOGIN[..5]).await;
    insert_token(&pool, &scopes::ADMIN_LOGIN[5..]).await;
    let outcome = check_admin_scopes(&pool, SERVICE, Some(&webhook)).await.expect("full check");
    assert!(outcome.missing.is_empty());
    assert!(!outcome.alerted);
    assert_eq!(captured.lock().expect("captured lock").len(), 0);

    // Two scopes missing: alerted, in ADMIN_LOGIN order.
    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(SERVICE)
        .execute(&pool)
        .await
        .expect("reset tokens");
    let partial: Vec<&str> = scopes::ADMIN_LOGIN
        .into_iter()
        .filter(|scope| *scope != scopes::SEND_MAIL && *scope != scopes::READ_WALLET)
        .collect();
    insert_token(&pool, &partial).await;

    let outcome = check_admin_scopes(&pool, SERVICE, Some(&webhook)).await.expect("partial check");
    assert_eq!(outcome.missing, vec![scopes::SEND_MAIL, scopes::READ_WALLET]);
    assert!(outcome.alerted);

    // The legacy Http::post payload, exactly.
    let bodies = captured.lock().expect("captured lock").clone();
    assert_eq!(bodies.len(), 1);
    let body = &bodies[0];
    assert_eq!(sorted_keys(body), ["content", "embeds"]);
    assert_eq!(body["content"], "@everyone Admin character is missing ESI scopes!");
    let embeds = body["embeds"].as_array().expect("embeds");
    assert_eq!(embeds.len(), 1);
    let embed = &embeds[0];
    assert_eq!(sorted_keys(embed), ["color", "description", "fields", "timestamp", "title"]);
    assert_eq!(embed["title"], "Missing Admin ESI Scopes");
    assert_eq!(
        embed["description"],
        "The admin character is missing required ESI scopes.\n\n**Missing Scopes:**\n\
         • esi-mail.send_mail.v1\n• esi-wallet.read_character_wallet.v1"
    );
    // hexdec('EF4444'), the legacy red.
    assert_eq!(embed["color"], 15_680_580);
    let fields = embed["fields"].as_array().expect("fields");
    assert_eq!(fields.len(), 1);
    assert_eq!(sorted_keys(&fields[0]), ["name", "value"]);
    assert_eq!(fields[0]["name"], "Action Required");
    assert_eq!(fields[0]["value"], "[Grant Scopes](https://mutamarket.com/eve/admin)");
    let timestamp = embed["timestamp"].as_str().expect("timestamp");
    assert_eq!(timestamp.len(), 20, "ISO8601 UTC seconds: {timestamp}");
    assert!(timestamp.ends_with('Z') && timestamp.chars().nth(10) == Some('T'));

    // No webhook configured: the check still reports, nothing is posted
    // (the legacy nullable services.discord.alert_webhook).
    let outcome = check_admin_scopes(&pool, SERVICE, None).await.expect("check-only");
    assert_eq!(outcome.missing, vec![scopes::SEND_MAIL, scopes::READ_WALLET]);
    assert!(!outcome.alerted);
    assert_eq!(captured.lock().expect("captured lock").len(), 1);

    // A failing webhook surfaces as the webhook error.
    let broken = format!("{base}/broken");
    let error = check_admin_scopes(&pool, SERVICE, Some(&broken)).await.expect_err("broken hook");
    assert!(matches!(error, ScopeCheckError::Webhook(_)), "unexpected error: {error}");

    // The legacy "Admin character not found" failure.
    let error =
        check_admin_scopes(&pool, UNKNOWN_CHARACTER, Some(&webhook)).await.expect_err("no row");
    assert!(matches!(error, ScopeCheckError::CharacterNotFound));
    assert_eq!(error.to_string(), "Admin character not found");
}
