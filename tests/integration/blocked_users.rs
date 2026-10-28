//! Behavior tests for blocking users: the store endpoint with the legacy
//! FormRequest quirks (authorize before validation, the userless-character
//! 500), the leave-offers side effect in both directions, and the offer
//! creation gate.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use axum::Router;
use axum::http::StatusCode;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use serde_json::json;

/// Characters owned by this suite alone, so parallel suites never share
/// state.
const BLOCKER_CHARACTER: i64 = 920_301;
const BLOCKED_CHARACTER: i64 = 920_302;
/// A character row without a user account, for the ported 500 quirk.
const USERLESS_CHARACTER: i64 = 920_303;

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

#[tokio::test]
async fn blocking_users_leaves_their_offers() {
    let (pool, _db) = crate::common::db::fresh().await;

    let reference = crate::common::db::reference_data();

    // Two modules to make offers on.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    for seeded in &fixture.modules[..2] {
        process_module(
            &pool,
            &reference,
            &estimator_stub(),
            fixture.type_id,
            seeded.module_id,
            &DogmaItem {
                created_by: seeded.creator_id,
                source_type_id: seeded.source_type_id,
                mutator_type_id: seeded.mutaplasmid_id,
                dogma_attributes: common::fixture_dogma(seeded),
            },
        )
        .await
        .expect("process module");
    }
    let module = &fixture.modules[0];
    let second_module = &fixture.modules[1];

    // Two users plus a userless character; idempotent across runs.
    let characters = vec![BLOCKER_CHARACTER, BLOCKED_CHARACTER, USERLESS_CHARACTER];
    sqlx::query("delete from offers where sender_id = any($1) or receiver_id = any($1)")
        .bind(&characters)
        .execute(&pool)
        .await
        .expect("cleanup offers");
    sqlx::query("delete from characters where id = any($1)")
        .bind(&characters)
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Blocker User", "Blocked User"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let mut users = Vec::new();
    for (name, character_id) in [
        ("Blocker User", BLOCKER_CHARACTER),
        ("Blocked User", BLOCKED_CHARACTER),
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
    let (blocker_id, blocker) = (users[0].0, users[0].1.clone());
    let (blocked_id, blocked) = (users[1].0, users[1].1.clone());
    sqlx::query("insert into characters (id, name) values ($1, 'Userless Pilot')")
        .bind(USERLESS_CHARACTER)
        .execute(&pool)
        .await
        .expect("userless character");

    // One live offer in each direction.
    let offer_from_blocked = mutamarket::offers::create_offer(
        &pool,
        BLOCKED_CHARACTER,
        BLOCKER_CHARACTER,
        module.module_id,
        1_000_000.0,
        "want it?",
    )
    .await
    .expect("offer from the blocked user");
    let offer_from_blocker = mutamarket::offers::create_offer(
        &pool,
        BLOCKER_CHARACTER,
        BLOCKED_CHARACTER,
        module.module_id,
        2_000_000.0,
        "or mine?",
    )
    .await
    .expect("offer from the blocker");

    let app = mutamarket::server::test_router(pool.clone()).await;

    // Guests are redirected to login.
    let (status, location, _) = send(&app, "POST", "/blocked-users", None, None).await;
    assert!(
        status.is_redirection(),
        "guest POST redirects, got {status}"
    );
    assert_eq!(location, "/login");

    // Laravel validation with the default messages.
    let (status, _, body) = send(
        &app,
        "POST",
        "/blocked-users",
        Some(&blocker),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(errors["message"], json!("The given data was invalid."));
    assert_eq!(
        errors["errors"]["character_id"],
        json!(["The character id field is required."])
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/blocked-users",
        Some(&blocker),
        Some(json!({"character_id": 999_999_999})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        errors["errors"]["character_id"],
        json!(["The selected character id is invalid."])
    );

    // The ported 500: a character without a user account passes
    // validation and crashes the legacy action's `User` type hint.
    let (status, _, _) = send(
        &app,
        "POST",
        "/blocked-users",
        Some(&blocker),
        Some(json!({"character_id": USERLESS_CHARACTER})),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);

    // Blocking redirects to the offers page and records the block.
    let (status, location, _) = send(
        &app,
        "POST",
        "/blocked-users",
        Some(&blocker),
        Some(json!({"character_id": BLOCKED_CHARACTER})),
    )
    .await;
    assert!(status.is_redirection(), "block redirects, got {status}");
    assert_eq!(location, "/offers");
    let recorded: bool = sqlx::query_scalar(
        "select exists(select 1 from blocked_users where blocker_id = $1 and blocked_id = $2)",
    )
    .bind(blocker_id)
    .bind(blocked_id)
    .fetch_one(&pool)
    .await
    .expect("block row");
    assert!(recorded);

    // Both offers were left by their RECEIVING side only (the legacy
    // LeaveOffer is handed the receiver in both directions).
    let (left_by_sender, left_by_receiver): (bool, bool) = sqlx::query_as(
        "select left_by_sender_at is not null, left_by_receiver_at is not null
         from offers where id = $1",
    )
    .bind(offer_from_blocked)
    .fetch_one(&pool)
    .await
    .expect("offer from blocked");
    assert!(!left_by_sender, "the blocked sender keeps their thread");
    assert!(
        left_by_receiver,
        "the blocker's side left the incoming offer"
    );
    let (left_by_sender, left_by_receiver): (bool, bool) = sqlx::query_as(
        "select left_by_sender_at is not null, left_by_receiver_at is not null
         from offers where id = $1",
    )
    .bind(offer_from_blocker)
    .fetch_one(&pool)
    .await
    .expect("offer from blocker");
    assert!(!left_by_sender, "the blocker keeps their own sent thread");
    assert!(left_by_receiver, "the blocked side left the incoming offer");

    // Blocking again is rejected by the authorize() guard.
    let (status, _, body) = send(
        &app,
        "POST",
        "/blocked-users",
        Some(&blocker),
        Some(json!({"character_id": BLOCKED_CHARACTER})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(errors["message"], json!("This action is unauthorized."));

    // The block now gates offer creation with the exact legacy message.
    let (status, _, body) = send(
        &app,
        "POST",
        "/offers",
        Some(&blocked),
        Some(json!({
            "receiver_id": BLOCKER_CHARACTER,
            "module_id": module.module_id,
            "price": 3_000_000.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let errors: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        errors["message"],
        json!("You have been blocked by this user.")
    );

    // The unblocked direction is not gated: the blocker may still send
    // (only the receiver's block list counts, like OfferPolicy::create).
    let (status, _, _) = send(
        &app,
        "POST",
        "/offers",
        Some(&blocker),
        Some(json!({
            "receiver_id": BLOCKED_CHARACTER,
            "module_id": second_module.module_id,
            "price": 4_000_000.0,
        })),
    )
    .await;
    assert!(
        status.is_redirection(),
        "the blocker can still send offers to the blocked user, got {status}",
    );
}
