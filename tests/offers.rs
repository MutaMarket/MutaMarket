//! Behavior tests for the offers system (the legacy `OfferController`,
//! `MessageController`, `LeaveOffer` and the notification pipeline):
//! creation with the price divergence, the duplicate and block guards
//! with their exact legacy texts, the thread round trip, leave
//! semantics, and the outbox.
//!
//! Needs the local database: `docker compose up -d postgres`.

mod common;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const BUYER_CHARACTER: i64 = 990_500_001;
const SELLER_CHARACTER: i64 = 990_500_002;
const BLOCKER_CHARACTER: i64 = 990_500_003;

async fn setup() -> (PgPool, ReferenceData) {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables).await.expect("seed");
    (pool, ReferenceData::from_tables(tables))
}

fn app(pool: &PgPool, reference: ReferenceData) -> Router {
    mutamarket::server::router(
        pool.clone(),
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new("http://127.0.0.1:9", "client", "secret", "http://test/eve/callback"),
        mutamarket::auth::linked::LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(reference),
        None,
    )
}

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value, String) {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("request");
    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null), location)
}

async fn seed_character(pool: &PgPool, id: i64, name: &str, user_name: &str) -> (i64, String) {
    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
        .bind(user_name)
        .fetch_one(pool)
        .await
        .expect("user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("character");
    let session = create_session(pool, user_id, Some(id)).await.expect("session");
    (user_id, session)
}

#[tokio::test]
async fn offers_round_trip_like_the_legacy_controllers() {
    let (pool, reference) = setup().await;

    // A module to offer on, from the fixtures.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[2];
    let module = &fixture.modules[0];
    process_module(
        &pool,
        &reference,
        &Estimator::new(),
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

    // Idempotent slate for the three test identities.
    for character in [BUYER_CHARACTER, SELLER_CHARACTER, BLOCKER_CHARACTER] {
        sqlx::query(
            "delete from users where id in (select user_id from characters where id = $1)",
        )
        .bind(character)
        .execute(&pool)
        .await
        .expect("clean user");
        sqlx::query("delete from characters where id = $1")
            .bind(character)
            .execute(&pool)
            .await
            .expect("clean character");
    }
    sqlx::query("delete from offers where module_id = $1")
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("clean offers");

    let (buyer_user, buyer) = seed_character(&pool, BUYER_CHARACTER, "Offer Buyer", "Buyer").await;
    let (seller_user, seller) =
        seed_character(&pool, SELLER_CHARACTER, "Offer Seller", "Seller").await;
    sqlx::query("delete from notification_outbox where user_id in ($1, $2)")
        .bind(buyer_user)
        .bind(seller_user)
        .execute(&pool)
        .await
        .expect("clean outbox");

    let app = app(&pool, reference);

    // Guests: the actions redirect to login, the api answers 401.
    let (status, _, location) =
        send(&app, Method::POST, "/offers", None, Some(json!({}))).await;
    assert!(status.is_redirection());
    assert_eq!(location, "/login");
    let (status, body, _) = send(&app, Method::GET, "/api/offers", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], json!("Unauthenticated."));

    // Validation: the price divergence is required and positive.
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({ "receiver_id": SELLER_CHARACTER, "module_id": module.module_id })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["price"][0], json!("The price field is required."));
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({
            "receiver_id": SELLER_CHARACTER,
            "module_id": module.module_id,
            "price": -5,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["errors"]["price"][0], json!("The price field must be greater than 0."));

    // Creation lands the buyer in the new thread.
    let (status, _, location) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({
            "receiver_id": SELLER_CHARACTER,
            "module_id": module.module_id,
            "price": 1_500_000_000.0,
            "message": "Would you take 1.5b?",
        })),
    )
    .await;
    assert!(status.is_redirection(), "offer creation redirects: {status}");
    let offer_id: i64 =
        location.strip_prefix("/offers/").expect("offer path").parse().expect("offer id");

    // The receiver's notification sits in the outbox, undelivered.
    let (kind, subject, body_text, delivered): (String, String, String, bool) =
        sqlx::query_as(
            "select kind, subject, body, delivered_at is not null
             from notification_outbox where user_id = $1",
        )
        .bind(seller_user)
        .fetch_one(&pool)
        .await
        .expect("outbox row");
    assert_eq!(kind, "offer-received");
    assert_eq!(subject, "New Offer Received");
    assert!(body_text.contains("1,500,000,000 ISK"), "price in the mail body: {body_text}");
    assert!(body_text.contains("Offer Buyer"));
    assert!(!delivered);

    // A second offer for the same module is refused with the legacy text.
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({
            "receiver_id": SELLER_CHARACTER,
            "module_id": module.module_id,
            "price": 2_000_000_000.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["message"], json!("You have already sent an offer for this module."));

    // The buyer's index shows the thread with the exact key set.
    let (status, body, _) = send(&app, Method::GET, "/api/offers", Some(&buyer), None).await;
    assert_eq!(status, StatusCode::OK);
    let list = body.as_array().expect("offers list");
    assert_eq!(list.len(), 1);
    let mut keys: Vec<&str> =
        list[0].as_object().expect("offer").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["created_at", "id", "is_read", "latest_message", "module", "price", "receiver", "sender"],
    );
    assert_eq!(list[0]["id"], json!(offer_id));
    assert_eq!(list[0]["price"], json!(1_500_000_000.0));
    assert_eq!(list[0]["sender"]["name"], json!("Offer Buyer"));
    assert_eq!(list[0]["is_read"], json!(true), "own messages count as read");
    assert_eq!(list[0]["latest_message"]["content"], json!("Would you take 1.5b?"));

    // With the seller's public asset live, the module is flagged in the
    // buyer's sent set (the card's "Go to offer" swap) and the module
    // payload carries the owner.
    let asset_id: i64 = sqlx::query_scalar(
        "insert into assets (character_id, item_id, type_id, name, location_id, location_flag,
                             location_type, quantity, is_abyssal)
         values ($1, $2, $3, '', 60003760, 'Hangar', 'station', 1, true)
         returning id",
    )
    .bind(SELLER_CHARACTER)
    .bind(module.module_id)
    .bind(fixture.type_id)
    .fetch_one(&pool)
    .await
    .expect("seller asset");
    sqlx::query(
        "insert into public_assets (character_id, asset_id, module_id) values ($1, $2, $3)",
    )
    .bind(SELLER_CHARACTER)
    .bind(asset_id)
    .bind(module.module_id)
    .execute(&pool)
    .await
    .expect("public asset");
    let (status, body, _) = send(&app, Method::GET, "/api/offers/sent", Some(&buyer), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([{ "id": offer_id, "module_id": module.module_id }]));
    let (_, detail, _) = send(
        &app,
        Method::GET,
        &format!("/api/module-page/{}", module.module_id),
        None,
        None,
    )
    .await;
    assert_eq!(
        detail["module"]["public_asset"]["owner"],
        json!({ "id": SELLER_CHARACTER, "name": "Offer Seller" }),
    );
    assert!(detail["module"]["public_asset"]["price"].is_null(), "price column unported");

    // The seller sees it unread until the show marks it read.
    let (_, body, _) = send(&app, Method::GET, "/api/offers", Some(&seller), None).await;
    assert_eq!(body[0]["is_read"], json!(false));
    let (status, thread, _) =
        send(&app, Method::GET, &format!("/api/offers/{offer_id}"), Some(&seller), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(thread["own_character_id"], json!(SELLER_CHARACTER));
    assert_eq!(thread["messages"].as_array().expect("messages").len(), 1);
    assert_eq!(thread["messages"][0]["mine"], json!(false));
    assert_eq!(thread["module"]["id"], json!(module.module_id));
    let (_, body, _) = send(&app, Method::GET, "/api/offers", Some(&seller), None).await;
    assert_eq!(body[0]["is_read"], json!(true), "viewing marked the thread read");

    // The seller replies; a third account may not touch the thread.
    let (status, _, _) = send(
        &app,
        Method::POST,
        "/messages",
        Some(&seller),
        Some(json!({ "offer_id": offer_id, "content": "Make it 1.8 and deal." })),
    )
    .await;
    assert!(status.is_redirection(), "message send redirects back: {status}");
    let (_, thread, _) =
        send(&app, Method::GET, &format!("/api/offers/{offer_id}"), Some(&buyer), None).await;
    assert_eq!(thread["messages"].as_array().expect("messages").len(), 2);
    assert_eq!(thread["messages"][1]["mine"], json!(false));

    let (_, stranger) =
        seed_character(&pool, BLOCKER_CHARACTER, "Offer Stranger", "Stranger").await;
    let (status, body, _) = send(
        &app,
        Method::GET,
        &format!("/api/offers/{offer_id}"),
        Some(&stranger),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("Forbidden."));

    // Leaving: the buyer's side is stamped; once the seller leaves too
    // the offer is gone for both.
    let (status, _, location) = send(
        &app,
        Method::DELETE,
        &format!("/offers/{offer_id}"),
        Some(&buyer),
        None,
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(location, "/offers");
    let (_, body, _) = send(&app, Method::GET, "/api/offers", Some(&buyer), None).await;
    assert_eq!(body.as_array().expect("offers").len(), 0, "left threads disappear");
    let (_, body, _) = send(&app, Method::GET, "/api/offers/sent", Some(&buyer), None).await;
    assert_eq!(body, json!([]), "left offers stop flagging the module");
    let (_, body, _) = send(&app, Method::GET, "/api/offers", Some(&seller), None).await;
    assert_eq!(body.as_array().expect("offers").len(), 1, "the other side still sees it");
    let (status, _, _) = send(
        &app,
        Method::DELETE,
        &format!("/offers/{offer_id}"),
        Some(&seller),
        None,
    )
    .await;
    assert!(status.is_redirection());
    let deleted: bool =
        sqlx::query_scalar("select deleted_at is not null from offers where id = $1")
            .bind(offer_id)
            .fetch_one(&pool)
            .await
            .expect("offer row");
    assert!(deleted, "both sides left, the offer is soft-deleted");

    // Blocks: the stranger blocks the buyer; the buyer's offer to the
    // stranger is refused with the legacy text.
    let stranger_user: i64 =
        sqlx::query_scalar("select user_id from characters where id = $1")
            .bind(BLOCKER_CHARACTER)
            .fetch_one(&pool)
            .await
            .expect("stranger user");
    sqlx::query("insert into blocked_users (blocker_id, blocked_id) values ($1, $2)")
        .bind(stranger_user)
        .bind(buyer_user)
        .execute(&pool)
        .await
        .expect("block");
    let (status, body, _) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({
            "receiver_id": BLOCKER_CHARACTER,
            "module_id": module.module_id,
            "price": 100.0,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"], json!("You have been blocked by this user."));
}

#[tokio::test]
async fn unread_messages_notify_after_the_legacy_delay() {
    let (pool, reference) = setup().await;

    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[3];
    let module = &fixture.modules[0];
    process_module(
        &pool,
        &reference,
        &Estimator::new(),
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

    const NOTIFY_BUYER: i64 = 990_500_011;
    const NOTIFY_SELLER: i64 = 990_500_012;
    for character in [NOTIFY_BUYER, NOTIFY_SELLER] {
        sqlx::query(
            "delete from users where id in (select user_id from characters where id = $1)",
        )
        .bind(character)
        .execute(&pool)
        .await
        .expect("clean user");
        sqlx::query("delete from characters where id = $1")
            .bind(character)
            .execute(&pool)
            .await
            .expect("clean character");
    }

    let (_, buyer) = seed_character(&pool, NOTIFY_BUYER, "Notify Buyer", "NotifyBuyer").await;
    let (seller_user, _) =
        seed_character(&pool, NOTIFY_SELLER, "Notify Seller", "NotifySeller").await;
    sqlx::query("delete from notification_outbox where user_id = $1")
        .bind(seller_user)
        .execute(&pool)
        .await
        .expect("clean outbox");

    let app = app(&pool, reference);
    let (status, _, location) = send(
        &app,
        Method::POST,
        "/offers",
        Some(&buyer),
        Some(json!({
            "receiver_id": NOTIFY_SELLER,
            "module_id": module.module_id,
            "price": 750_000_000.0,
        })),
    )
    .await;
    assert!(status.is_redirection(), "{status}");
    let offer_id: i64 =
        location.strip_prefix("/offers/").expect("offer path").parse().expect("offer id");

    // The offer-received row is queued; the message scan must not fire
    // for the first message (it is stamped notified on creation).
    let scanned = mutamarket::notifications::queue_unread_message_notifications(&pool)
        .await
        .expect("scan");
    assert_eq!(scanned, 0, "the creation message counts as notified");

    // A follow-up message, unread and older than the legacy delay.
    sqlx::query(
        "insert into messages (offer_id, sender_id, receiver_id, content, created_at)
         values ($1, $2, $3, 'Still interested?', now() - interval '11 minutes')",
    )
    .bind(offer_id)
    .bind(NOTIFY_BUYER)
    .bind(NOTIFY_SELLER)
    .execute(&pool)
    .await
    .expect("late message");

    let scanned = mutamarket::notifications::queue_unread_message_notifications(&pool)
        .await
        .expect("scan");
    assert_eq!(scanned, 1);
    // Idempotent: the messages are stamped, a second scan is silent.
    let scanned = mutamarket::notifications::queue_unread_message_notifications(&pool)
        .await
        .expect("rescan");
    assert_eq!(scanned, 0);

    let (subject, body_text): (String, String) = sqlx::query_as(
        "select subject, body from notification_outbox
         where user_id = $1 and kind = 'messages-received'",
    )
    .bind(seller_user)
    .fetch_one(&pool)
    .await
    .expect("messages-received row");
    assert_eq!(subject, "New Messages Received");
    assert!(body_text.contains(&format!("/offers/{offer_id}")), "{body_text}");
    assert!(body_text.contains("Hello Notify Seller"), "{body_text}");

    // The simulated delivery drain stamps the rows without mailing.
    let pending =
        mutamarket::notifications::pending(&pool, 50).await.expect("pending rows");
    assert!(pending.iter().any(|row| row.user_id == seller_user));
    for row in &pending {
        assert!(row.recipient_character_id.is_some(), "fallback recipient resolves");
        mutamarket::notifications::mark_delivered(&pool, row.id, "simulated", None)
            .await
            .expect("mark delivered");
    }
    let left = mutamarket::notifications::pending(&pool, 50).await.expect("pending rows");
    assert!(left.iter().all(|row| row.user_id != seller_user), "drained");
}
