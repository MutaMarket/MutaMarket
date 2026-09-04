//! Behavior tests for `POST /premium/gift` (a rewrite addition): whole
//! days of one character's premium move to any known character, with
//! the audit row and the recipient's notice committed alongside, and the
//! premium page listing the account's giftable characters.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const DONOR: i64 = 990_021_001;
const DONOR_ALT: i64 = 990_021_002;
const RIVAL: i64 = 990_021_003;
/// A character nobody has claimed, so its notice goes to it directly.
const STRANGER: i64 = 990_021_004;

async fn seed(pool: &PgPool) -> (String, i64, i64) {
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![DONOR, DONOR_ALT, RIVAL, STRANGER])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Gift Donor", "Gift Rival"])
        .execute(pool)
        .await
        .expect("clean users");
    let donor_user: i64 =
        sqlx::query_scalar("insert into users (name) values ('Gift Donor') returning id")
            .fetch_one(pool)
            .await
            .expect("donor user");
    let rival_user: i64 =
        sqlx::query_scalar("insert into users (name) values ('Gift Rival') returning id")
            .fetch_one(pool)
            .await
            .expect("rival user");
    for (id, name, user_id, premium) in [
        (DONOR, "Gifter Prime", Some(donor_user), Some("30 days")),
        (DONOR_ALT, "Gifter Alt", Some(donor_user), None),
        (RIVAL, "Gift Rival", Some(rival_user), None),
        (STRANGER, "Gift Stranger", None, None),
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
    let session = create_session(pool, donor_user, Some(DONOR))
        .await
        .expect("session");
    (session, donor_user, rival_user)
}

async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let body = body.map_or_else(Body::empty, |value| Body::from(value.to_string()));
    let response = app
        .clone()
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
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

async fn remaining_days(pool: &PgPool, character_id: i64) -> Option<i64> {
    sqlx::query_scalar(
        "select floor(extract(epoch from premium_paid_until - now()) / 86400)::bigint
         from characters where id = $1",
    )
    .bind(character_id)
    .fetch_one(pool)
    .await
    .expect("days")
}

#[tokio::test]
async fn premium_days_move_between_characters_with_a_notice() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (session, _donor_user, rival_user) = seed(&pool).await;
    let app = mutamarket::server::test_router().await;
    let gift = |from: i64, to: &str, days: i64| json!({ "from_character_id": from, "to_character_name": to, "days": days });

    // The premium page lists the account's giftable characters: only the
    // one holding premium, with its whole days left.
    let (status, page) = send(&app, Method::GET, "/api/premium/page", Some(&session), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&page), ["giftable", "sample_modules"]);
    let giftable = page["giftable"].as_array().expect("giftable");
    assert_eq!(giftable.len(), 1);
    assert_eq!(
        sorted_keys(&giftable[0]),
        ["id", "name", "premium_paid_until", "remaining_days"]
    );
    assert_eq!(giftable[0]["id"], json!(DONOR));
    assert_eq!(giftable[0]["remaining_days"], json!(29));
    let (_, guest_page) = send(&app, Method::GET, "/api/premium/page", None, None).await;
    assert!(guest_page["giftable"].is_null());

    // Guests bounce to the login page.
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        None,
        Some(gift(DONOR, "Gift Rival", 5)),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);

    // Every refusal carries its exact sentence and moves nothing.
    for (body, status, message) in [
        (
            gift(RIVAL, "Gift Stranger", 5),
            StatusCode::FORBIDDEN,
            "That character is not on your account.",
        ),
        (
            gift(DONOR_ALT, "Gift Rival", 5),
            StatusCode::UNPROCESSABLE_ENTITY,
            "That character does not have that many premium days left.",
        ),
        (
            gift(DONOR, "Gift Rival", 30),
            StatusCode::UNPROCESSABLE_ENTITY,
            "That character does not have that many premium days left.",
        ),
        (
            gift(DONOR, "Nobody Here", 5),
            StatusCode::UNPROCESSABLE_ENTITY,
            "No character by that name is known here.",
        ),
        (
            gift(DONOR, "gifter prime", 5),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Pick a different character to receive the days.",
        ),
        (
            gift(DONOR, "Gift Rival", 0),
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose between 1 and 3660 days.",
        ),
    ] {
        let (got, response) = send(
            &app,
            Method::POST,
            "/premium/gift",
            Some(&session),
            Some(body),
        )
        .await;
        assert_eq!(got, status, "{message}");
        assert_eq!(response["message"].as_str(), Some(message));
    }
    assert_eq!(remaining_days(&pool, DONOR).await, Some(29));
    assert_eq!(remaining_days(&pool, RIVAL).await, None);

    // A gift to a claimed character: the donor loses the days, the
    // recipient's premium starts now, the audit row and the account's
    // notice land in the same commit.
    let (status, outcome) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(DONOR, "gift rival", 5)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&outcome),
        [
            "days",
            "from",
            "to_character_id",
            "to_character_name",
            "to_premium_paid_until"
        ]
    );
    assert_eq!(outcome["days"], json!(5));
    assert_eq!(outcome["to_character_id"], json!(RIVAL));
    assert_eq!(outcome["to_character_name"], json!("Gift Rival"));
    assert_eq!(outcome["from"]["remaining_days"], json!(24));
    assert_eq!(remaining_days(&pool, DONOR).await, Some(24));
    assert_eq!(remaining_days(&pool, RIVAL).await, Some(4));
    let audit: Vec<(i64, i64, i32)> = sqlx::query_as(
        "select from_character_id, to_character_id, days from premium_gifts
         where from_character_id = $1 order by id",
    )
    .bind(DONOR)
    .fetch_all(&pool)
    .await
    .expect("audit rows");
    assert_eq!(audit, vec![(DONOR, RIVAL, 5)]);
    let notice: (String, String, serde_json::Value) = sqlx::query_as(
        "select kind, subject, payload from notification_outbox
         where user_id = $1 order by id desc limit 1",
    )
    .bind(rival_user)
    .fetch_one(&pool)
    .await
    .expect("notice");
    assert_eq!(notice.0, "premium-gift");
    assert_eq!(notice.1, "You received premium time");
    assert_eq!(
        notice.2,
        json!({ "from_character_id": DONOR, "to_character_id": RIVAL, "days": 5 })
    );

    // A second gift extends the recipient from its current expiry, and an
    // unclaimed character is notified directly.
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(DONOR, "Gift Rival", 3)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(remaining_days(&pool, RIVAL).await, Some(7));
    let (status, _) = send(
        &app,
        Method::POST,
        "/premium/gift",
        Some(&session),
        Some(gift(DONOR, "Gift Stranger", 1)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(remaining_days(&pool, DONOR).await, Some(20));
    let direct: i64 = sqlx::query_scalar(
        "select count(*) from notification_outbox
         where recipient_character_id = $1 and kind = 'premium-gift'",
    )
    .bind(STRANGER)
    .fetch_one(&pool)
    .await
    .expect("direct notice");
    assert_eq!(direct, 1);
}
