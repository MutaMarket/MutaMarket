//! Behavior tests for ad-free premium (a rewrite addition): the account's
//! plan switch at `POST /premium/plan` converting every linked
//! character's remaining time at its ISK value, the premium page listing
//! the plan with each character's time, and gifts between accounts on
//! different plans.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::http::{Method, StatusCode};
use mutamarket::auth::session::create_session;
use serde_json::json;
use sqlx::PgPool;

/// Holds 30 days of plain premium.
const PILOT: i64 = 990_031_001;
/// The same account's second character, 9 days of premium.
const PILOT_ALT: i64 = 990_031_002;
/// Someone else's character, no premium.
const OUTSIDER: i64 = 990_031_003;

async fn seed(pool: &PgPool) -> String {
    let user: i64 =
        sqlx::query_scalar("insert into users (name) values ('Plan Pilot') returning id")
            .fetch_one(pool)
            .await
            .expect("user");
    let other: i64 =
        sqlx::query_scalar("insert into users (name) values ('Plan Outsider') returning id")
            .fetch_one(pool)
            .await
            .expect("other user");
    for (id, name, user_id, premium) in [
        (PILOT, "Plan Prime", user, Some("30 days")),
        (PILOT_ALT, "Plan Alt", user, Some("9 days")),
        (OUTSIDER, "Plan Outsider", other, None),
    ] {
        sqlx::query(
            "insert into characters (id, name, user_id, premium_paid_until)
             values ($1, $2, $3, case when $4::text is null then null
                                      else now() + $4::interval end)",
        )
        .bind(id)
        .bind(name)
        .bind(user_id)
        .bind(premium)
        .execute(pool)
        .await
        .expect("character");
    }
    create_session(pool, user, Some(PILOT))
        .await
        .expect("session")
}

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let response = crate::common::http::request(method, uri)
        .session(session)
        .json(body)
        .send(app)
        .await;
    (response.status, response.json())
}

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// Days of premium left, to the hour, so the conversions read exactly.
async fn days_left(pool: &PgPool, character_id: i64) -> Option<f64> {
    sqlx::query_scalar(
        "select round((extract(epoch from premium_paid_until - now()) / 86400)::numeric, 1)::float8
         from characters where id = $1",
    )
    .bind(character_id)
    .fetch_one(pool)
    .await
    .expect("days")
}

fn plan(ad_free: bool) -> serde_json::Value {
    json!({ "ad_free": ad_free })
}

async fn set_account_plan(pool: &PgPool, character_id: i64, ad_free: bool) {
    sqlx::query(
        "update users set premium_ad_free = $2
         where id = (select user_id from characters where id = $1)",
    )
    .bind(character_id)
    .bind(ad_free)
    .execute(pool)
    .await
    .expect("account plan");
}

#[tokio::test]
async fn switching_converts_every_linked_characters_time() {
    let (pool, _db) = crate::common::db::fresh().await;
    let session = seed(&pool).await;
    let app = mutamarket::server::test_router(pool.clone()).await;

    // The premium page shows the account's plan with every character;
    // only live premium carries an end date.
    let (status, page) = send(&app, Method::GET, "/api/premium/page", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&page["plan"]), ["ad_free", "characters"]);
    assert_eq!(page["plan"]["ad_free"], json!(false));
    let characters = page["plan"]["characters"].as_array().expect("characters");
    assert_eq!(characters.len(), 2);
    for character in characters {
        assert_eq!(sorted_keys(character), ["id", "name", "premium_paid_until"]);
        assert!(character["premium_paid_until"].is_string());
    }
    assert_eq!(characters[0]["id"], json!(PILOT));
    assert_eq!(characters[1]["id"], json!(PILOT_ALT));

    // Guests bounce to the login page, bad bodies are refused with the
    // exact sentence.
    let (status, _) = send(&app, Method::POST, "/premium/plan", None, Some(plan(true))).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let (status, response) = send(
        &app,
        Method::POST,
        "/premium/plan",
        Some(&session),
        Some(json!({ "character_id": PILOT })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response["message"],
        json!("Send whether the account should be ad-free.")
    );

    // Going ad-free converts both characters: 30 days make 10, 9 make 3.
    // Someone else's account is left alone.
    let (status, switched) = send(
        &app,
        Method::POST,
        "/premium/plan",
        Some(&session),
        Some(plan(true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&switched), ["ad_free", "characters"]);
    assert_eq!(switched["ad_free"], json!(true));
    assert_eq!(switched["characters"].as_array().map(Vec::len), Some(2));
    assert_eq!(switched["characters"][0]["name"], json!("Plan Prime"));
    assert_eq!(days_left(&pool, PILOT).await, Some(10.0));
    assert_eq!(days_left(&pool, PILOT_ALT).await, Some(3.0));
    assert_eq!(days_left(&pool, OUTSIDER).await, None);
    let (_, nav) = send(&app, Method::GET, "/api/nav-state", Some(&session), None).await;
    assert_eq!(nav["user"]["ad_free"], json!(true));
    assert_eq!(nav["user"]["premium_ad_free"], json!(true));

    // Asking for the plan it is already on changes nothing.
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/plan",
        Some(&session),
        Some(plan(true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(days_left(&pool, PILOT).await, Some(10.0));
    assert_eq!(days_left(&pool, PILOT_ALT).await, Some(3.0));

    // Switching back triples it again: no time is gained or lost.
    let (status, back) = send(
        &app,
        Method::POST,
        "/premium/plan",
        Some(&session),
        Some(plan(false)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(back["ad_free"], json!(false));
    assert_eq!(days_left(&pool, PILOT).await, Some(30.0));
    assert_eq!(days_left(&pool, PILOT_ALT).await, Some(9.0));
    let (_, nav) = send(&app, Method::GET, "/api/nav-state", Some(&session), None).await;
    assert_eq!(nav["user"]["ad_free"], json!(false));
    assert_eq!(nav["user"]["premium_ad_free"], json!(false));
}

#[tokio::test]
async fn without_live_premium_only_the_plan_flips() {
    let (pool, _db) = crate::common::db::fresh().await;
    seed(&pool).await;
    let outsider_user: i64 = sqlx::query_scalar("select user_id from characters where id = $1")
        .bind(OUTSIDER)
        .fetch_one(&pool)
        .await
        .expect("outsider user");
    let session = create_session(&pool, outsider_user, Some(OUTSIDER))
        .await
        .expect("session");
    let app = mutamarket::server::test_router(pool.clone()).await;

    let (status, switched) = send(
        &app,
        Method::POST,
        "/premium/plan",
        Some(&session),
        Some(plan(true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(switched["ad_free"], json!(true));
    assert!(switched["characters"][0]["premium_paid_until"].is_null());
    assert_eq!(days_left(&pool, OUTSIDER).await, None);
    let (_, nav) = send(&app, Method::GET, "/api/nav-state", Some(&session), None).await;
    assert_eq!(nav["user"]["premium_ad_free"], json!(true));
    assert_eq!(
        nav["user"]["ad_free"],
        json!(false),
        "an ad-free plan without premium hides nothing"
    );
}

#[tokio::test]
async fn gifts_between_plans_convert_at_their_value() {
    let (pool, _db) = crate::common::db::fresh().await;
    let session = seed(&pool).await;
    let app = mutamarket::server::test_router(pool.clone()).await;
    let gift = |from: i64, to: &str, days: i64| json!({ "from_character_id": from, "to_character_name": to, "days": days });

    // Plain days into an ad-free account: 6 days make 2.
    set_account_plan(&pool, OUTSIDER, true).await;
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(PILOT, "Plan Outsider", 6)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(days_left(&pool, PILOT).await, Some(24.0));
    assert_eq!(days_left(&pool, OUTSIDER).await, Some(2.0));
    let body: String = sqlx::query_scalar(
        "select o.body from notification_outbox o
         join users u on u.id = o.user_id
         where u.name = 'Plan Outsider' and o.kind = 'premium-gift'",
    )
    .fetch_one(&pool)
    .await
    .expect("gift notice");
    assert!(
        body.contains(
            "Plan Prime just sent you 6 days of MutaMarket premium. Your ad-free plan costs 3x \
             as much, so they make 2 days of it. Your premium now runs until"
        ),
        "body: {body}"
    );

    // Ad-free days into a plain account: 1 day makes 3.
    set_account_plan(&pool, OUTSIDER, false).await;
    set_account_plan(&pool, PILOT, true).await;
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(PILOT, "Plan Outsider", 1)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(days_left(&pool, PILOT).await, Some(23.0));
    assert_eq!(days_left(&pool, OUTSIDER).await, Some(5.0));

    // Between linked characters the plan is the same: a day stays a day.
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(PILOT, "Plan Alt", 1)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(days_left(&pool, PILOT).await, Some(22.0));
    assert_eq!(days_left(&pool, PILOT_ALT).await, Some(10.0));
}
