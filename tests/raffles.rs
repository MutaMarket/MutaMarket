//! Behavior tests for the raffle system: the winner's claim/decline
//! pair (`PUT|DELETE /raffle/{raffle_item}`), the admin management
//! endpoints (`GET /api/admin/raffles`, `POST /raffles`) and the hourly
//! draw job.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use mutamarket::raffles::{STATUS_ACTIVE, STATUS_CLAIMED, STATUS_PAID_OUT, STATUS_PENDING};
use sqlx::PgPool;
use tower::ServiceExt;

/// Test-owned characters, cleaned and recreated per run.
const WINNER_CHARACTER: i64 = 990_014_001;

/// A reference type the created items attach to; seeded idempotently.
const TEST_TYPE: i64 = 990_014_900;

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

struct Account {
    user_id: i64,
    session: String,
}

async fn seed_user(pool: &PgPool, name: &str, is_admin: bool) -> Account {
    sqlx::query("delete from users where name = $1")
        .bind(name)
        .execute(pool)
        .await
        .expect("clean user");
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name, is_admin) values ($1, $2) returning id")
            .bind(name)
            .bind(is_admin)
            .fetch_one(pool)
            .await
            .expect("create user");
    let session = create_session(pool, user_id, None).await.expect("session");
    Account { user_id, session }
}

async fn seed_type(pool: &PgPool) {
    sqlx::query(
        "insert into types (id, name, published) values ($1, 'Raffle Test Vedmak', true)
         on conflict (id) do nothing",
    )
    .bind(TEST_TYPE)
    .execute(pool)
    .await
    .expect("seed type");
}

async fn clean_items(pool: &PgPool) {
    sqlx::query("delete from raffle_items where code like 'RAFTEST-%'")
        .execute(pool)
        .await
        .expect("clean raffle items");
}

async fn seed_item(pool: &PgPool, code: &str, status: i32, winner_id: Option<i64>) -> i64 {
    sqlx::query_scalar(
        "insert into raffle_items (name, code, status, winner_id, expires_at)
         values ('Test Prize', $1, $2, $3,
                 case when $3 is null then null else now() + interval '30 minutes' end)
         returning id",
    )
    .bind(code)
    .bind(status)
    .bind(winner_id)
    .fetch_one(pool)
    .await
    .expect("seed raffle item")
}

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
    referer: Option<&str>,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    if let Some(referer) = referer {
        builder = builder.header(header::REFERER, referer);
    }
    let body = match body {
        Some(json) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).expect("valid request"))
        .await
        .expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        location,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn claim_and_decline_follow_the_legacy_guards() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let app = mutamarket::server::test_router().await;

    clean_items(&pool).await;
    let winner = seed_user(&pool, "Raffle Winner", false).await;
    let rival = seed_user(&pool, "Raffle Rival", false).await;

    // Guests bounce to the login page like every authed page route.
    for method in [Method::PUT, Method::DELETE] {
        let (status, location, _) = request(&app, method, "/raffle/1", None, None, None).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(location.as_deref(), Some("/login"));
    }

    // A missing item is the route-model-binding 404.
    let (status, _, _) = request(
        &app,
        Method::PUT,
        "/raffle/0",
        Some(&winner.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let item = seed_item(&pool, "RAFTEST-CLAIM", STATUS_ACTIVE, Some(winner.user_id)).await;

    // Someone else's prize and a non-active item are the legacy bare 403s.
    let (status, _, _) = request(
        &app,
        Method::PUT,
        &format!("/raffle/{item}"),
        Some(&rival.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let pending = seed_item(
        &pool,
        "RAFTEST-PENDING",
        STATUS_PENDING,
        Some(winner.user_id),
    )
    .await;
    for method in [Method::PUT, Method::DELETE] {
        let (status, _, _) = request(
            &app,
            method,
            &format!("/raffle/{pending}"),
            Some(&winner.session),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    // Claiming lands on the settings page where the code shows.
    let (status, location, _) = request(
        &app,
        Method::PUT,
        &format!("/raffle/{item}"),
        Some(&winner.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/settings"));
    let (status, winner_id): (i32, Option<i64>) =
        sqlx::query_as("select status, winner_id from raffle_items where id = $1")
            .bind(item)
            .fetch_one(&pool)
            .await
            .expect("claimed row");
    assert_eq!(status, STATUS_CLAIMED);
    assert_eq!(winner_id, Some(winner.user_id), "a claim keeps the winner");

    // A claimed item cannot be declined afterwards.
    let (status, _, _) = request(
        &app,
        Method::DELETE,
        &format!("/raffle/{item}"),
        Some(&winner.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Declining returns the prize to the pool and goes back().
    let declined = seed_item(
        &pool,
        "RAFTEST-DECLINE",
        STATUS_ACTIVE,
        Some(winner.user_id),
    )
    .await;
    let (status, location, _) = request(
        &app,
        Method::DELETE,
        &format!("/raffle/{declined}"),
        Some(&winner.session),
        None,
        Some("/all-modules"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/all-modules"));
    type ItemRow = (i32, Option<i64>, Option<String>);
    let (status, winner_id, expires_at): ItemRow = sqlx::query_as(
        "select status, winner_id, expires_at::text from raffle_items where id = $1",
    )
    .bind(declined)
    .fetch_one(&pool)
    .await
    .expect("declined row");
    assert_eq!(status, STATUS_PENDING);
    assert_eq!(winner_id, None);
    assert_eq!(expires_at, None);
}

async fn admin_store_validates_and_creates_one_item_per_code() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let app = mutamarket::server::test_router().await;

    seed_type(&pool).await;
    sqlx::query("delete from raffle_items where name = 'Raffle Store Prize'")
        .execute(&pool)
        .await
        .expect("clean stored items");
    let admin = seed_user(&pool, "Raffle Store Admin", true).await;
    let pleb = seed_user(&pool, "Raffle Store Pleb", false).await;

    // Guests bounce to the login page (the auth middleware runs before
    // the admin gate); non-admins get the AdminMiddleware text.
    let (status, location, _) = request(&app, Method::POST, "/raffles", None, None, None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));
    let (status, _, body) = request(
        &app,
        Method::POST,
        "/raffles",
        Some(&pleb.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"].as_str(), Some("Unauthorized access."));

    // The legacy validation messages, first failure per request.
    let cases = [
        (serde_json::json!({}), "name", "The name field is required."),
        (
            serde_json::json!({ "name": "x".repeat(256) }),
            "name",
            "The name field must not be greater than 255 characters.",
        ),
        (
            serde_json::json!({ "name": "Prize", "description": "x".repeat(256) }),
            "description",
            "The description field must not be greater than 255 characters.",
        ),
        (
            serde_json::json!({ "name": "Prize", "type_id": 1, "codes": "RAFTEST-X" }),
            "type_id",
            "The selected type id is invalid.",
        ),
        (
            serde_json::json!({ "name": "Prize" }),
            "codes",
            "The codes field is required.",
        ),
    ];
    for (payload, field, message) in cases {
        let (status, _, body) = request(
            &app,
            Method::POST,
            "/raffles",
            Some(&admin.session),
            Some(payload),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{field}");
        assert_eq!(
            body["message"].as_str(),
            Some("The given data was invalid."),
            "{field}"
        );
        assert_eq!(body["errors"][field][0].as_str(), Some(message));
    }

    // One pending item per code line: trimmed, blank lines dropped, and
    // the PHP array_filter quirk also drops a literal "0" code. An
    // empty description is stored as null, and the attached type brings
    // its icon URL.
    let (status, location, _) = request(
        &app,
        Method::POST,
        "/raffles",
        Some(&admin.session),
        Some(serde_json::json!({
            "name": "Raffle Store Prize",
            "description": "",
            "type_id": TEST_TYPE,
            "codes": "RAFTEST-STORE-A\n  RAFTEST-STORE-B  \n\n0\n",
        })),
        Some("/admin/raffles"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(
        location.as_deref(),
        Some("/admin/raffles"),
        "the legacy back()"
    );

    type StoredRow = (String, Option<String>, Option<i64>, Option<String>, i32);
    let rows: Vec<StoredRow> = sqlx::query_as(
        "select code, description, type_id, icon_url, status from raffle_items
         where name = 'Raffle Store Prize' order by code",
    )
    .fetch_all(&pool)
    .await
    .expect("stored rows");
    assert_eq!(
        rows,
        vec![
            (
                "RAFTEST-STORE-A".to_owned(),
                None,
                Some(TEST_TYPE),
                Some(format!("https://images.evetech.net/types/{TEST_TYPE}/icon")),
                STATUS_PENDING,
            ),
            (
                "RAFTEST-STORE-B".to_owned(),
                None,
                Some(TEST_TYPE),
                Some(format!("https://images.evetech.net/types/{TEST_TYPE}/icon")),
                STATUS_PENDING,
            ),
        ],
    );
}

async fn admin_index_lists_items_in_the_legacy_order() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let app = mutamarket::server::test_router().await;

    seed_type(&pool).await;
    sqlx::query("delete from raffle_items where code like 'RAFIDX-%'")
        .execute(&pool)
        .await
        .expect("clean index items");
    let admin = seed_user(&pool, "Raffle Index Admin", true).await;
    let pleb = seed_user(&pool, "Raffle Index Pleb", false).await;
    let winner = seed_user(&pool, "Raffle Index Winner", false).await;

    // The winner column prefers the notify character's name and id.
    sqlx::query("delete from characters where id = $1")
        .bind(WINNER_CHARACTER)
        .execute(&pool)
        .await
        .expect("clean character");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Raffle Notify Char', $2)")
        .bind(WINNER_CHARACTER)
        .bind(winner.user_id)
        .execute(&pool)
        .await
        .expect("create character");
    sqlx::query(
        "insert into notify_characters (user_id, character_id) values ($1, $2)
         on conflict (user_id) do update set character_id = excluded.character_id",
    )
    .bind(winner.user_id)
    .bind(WINNER_CHARACTER)
    .execute(&pool)
    .await
    .expect("notify pick");

    let (status, _, body) =
        request(&app, Method::GET, "/api/admin/raffles", None, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));
    let (status, _, body) = request(
        &app,
        Method::GET,
        "/api/admin/raffles",
        Some(&pleb.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["message"].as_str(), Some("Forbidden."));

    // Old claimed, newer paid out, pending, active: the page ranks
    // active, pending, then the finished ones by recency.
    let claimed = seed_item(
        &pool,
        "RAFIDX-CLAIMED",
        STATUS_CLAIMED,
        Some(winner.user_id),
    )
    .await;
    let paid_out = seed_item(&pool, "RAFIDX-PAIDOUT", STATUS_PAID_OUT, None).await;
    let pending = seed_item(&pool, "RAFIDX-PENDING", STATUS_PENDING, None).await;
    let active = seed_item(&pool, "RAFIDX-ACTIVE", STATUS_ACTIVE, Some(winner.user_id)).await;
    sqlx::query("update raffle_items set updated_at = now() - interval '1 hour' where id = $1")
        .bind(claimed)
        .execute(&pool)
        .await
        .expect("age the claim");
    sqlx::query("update raffle_items set type_id = $1, name = 'Typed Prize' where id = $2")
        .bind(TEST_TYPE)
        .bind(active)
        .execute(&pool)
        .await
        .expect("attach type");

    let (status, _, body) = request(
        &app,
        Method::GET,
        "/api/admin/raffles",
        Some(&admin.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec!["raffle_items", "type_search", "types"]
    );
    assert_eq!(body["type_search"].as_str(), Some(""));
    assert_eq!(body["types"].as_array().expect("types").len(), 0);

    let items: Vec<&serde_json::Value> = body["raffle_items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|item| {
            item["code"]
                .as_str()
                .unwrap_or_default()
                .starts_with("RAFIDX-")
        })
        .collect();
    let ids: Vec<i64> = items
        .iter()
        .map(|item| item["id"].as_i64().expect("id"))
        .collect();
    assert_eq!(ids, vec![active, pending, paid_out, claimed]);

    for item in &items {
        assert_eq!(
            sorted_keys(item),
            vec![
                "code",
                "created_at",
                "description",
                "expires_at",
                "id",
                "name",
                "status",
                "type",
                "winner",
            ],
        );
    }
    let active_item = items[0];
    assert_eq!(active_item["status"].as_i64(), Some(STATUS_ACTIVE as i64));
    assert_eq!(sorted_keys(&active_item["type"]), vec!["id", "name"]);
    assert_eq!(
        active_item["type"]["name"].as_str(),
        Some("Raffle Test Vedmak")
    );
    assert_eq!(
        sorted_keys(&active_item["winner"]),
        vec!["character_id", "id", "name"]
    );
    assert_eq!(
        active_item["winner"]["name"].as_str(),
        Some("Raffle Notify Char")
    );
    assert_eq!(
        active_item["winner"]["character_id"].as_i64(),
        Some(WINNER_CHARACTER)
    );
    assert!(
        active_item["expires_at"]
            .as_str()
            .expect("expires_at")
            .contains("+00:00"),
        "the legacy toIso8601String offset format",
    );
    let pending_item = items[1];
    assert!(pending_item["type"].is_null());
    assert!(pending_item["winner"].is_null());
    assert!(pending_item["expires_at"].is_null());

    // Without a notify pick the winner column falls back to the account
    // name.
    sqlx::query("delete from notify_characters where user_id = $1")
        .bind(winner.user_id)
        .execute(&pool)
        .await
        .expect("drop notify pick");
    let (_, _, body) = request(
        &app,
        Method::GET,
        "/api/admin/raffles",
        Some(&admin.session),
        None,
        None,
    )
    .await;
    let fallback = body["raffle_items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["id"].as_i64() == Some(active))
        .expect("active item")
        .clone();
    assert_eq!(
        fallback["winner"]["name"].as_str(),
        Some("Raffle Index Winner")
    );
    assert!(fallback["winner"]["character_id"].is_null());

    // The create form's type search: trimmed, echoed back, limited.
    let (status, _, body) = request(
        &app,
        Method::GET,
        "/api/admin/raffles?type_search=%20raffle%20test%20",
        Some(&admin.session),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["type_search"].as_str(), Some("raffle test"));
    let types = body["types"].as_array().expect("types");
    assert_eq!(types.len(), 1, "the ilike match on the seeded type");
    assert_eq!(types[0]["id"].as_i64(), Some(TEST_TYPE));
    assert_eq!(sorted_keys(&types[0]), vec!["id", "name"]);
}

/// The hourly draw, the legacy `DrawRaffleWinnerCommand`.
async fn the_draw_picks_active_users_and_yields_to_a_daily_winner() {
    let pool = db::test_pool().await.expect("test pool");
    db::migrate(&pool).await.expect("migrations run");
    // The daily-winner check reads the whole table, so this starts from
    // an empty pool rather than only its own prefix.
    sqlx::query("delete from raffle_items")
        .execute(&pool)
        .await
        .expect("clean raffle pool");

    let active = seed_user(&pool, "Raffle Active", false).await;
    let admin = seed_user(&pool, "Raffle Admin", true).await;
    let idle = seed_user(&pool, "Raffle Idle", false).await;

    // Every other account in the test database has been active through
    // its own session requests; the draw picks at random among the
    // eligible, so only these three carry activity here.
    sqlx::query("update users set last_active_at = null")
        .execute(&pool)
        .await
        .expect("clear activity");

    // Only the non-admin, recently active user is eligible: admins are
    // excluded and the idle user's activity predates the window.
    sqlx::query("update users set last_active_at = now() - interval '1 hour' where id = any($1)")
        .bind(vec![active.user_id, admin.user_id])
        .execute(&pool)
        .await
        .expect("recent activity");
    sqlx::query("update users set last_active_at = now() - interval '30 days' where id = $1")
        .bind(idle.user_id)
        .execute(&pool)
        .await
        .expect("stale activity");

    let prize = seed_item(
        &pool,
        "RAFTEST-DRAW-1",
        mutamarket::raffles::STATUS_PENDING,
        None,
    )
    .await;

    let stats = mutamarket::raffles::draw_winners(&pool)
        .await
        .expect("draw");
    assert_eq!((stats.drawn, stats.reset), (1, 0));

    let (status, winner): (i32, Option<i64>) =
        sqlx::query_as("select status, winner_id from raffle_items where id = $1")
            .bind(prize)
            .fetch_one(&pool)
            .await
            .expect("drawn prize");
    assert_eq!(status, mutamarket::raffles::STATUS_ACTIVE);
    assert_eq!(
        winner,
        Some(active.user_id),
        "admins and idle users never win"
    );

    // A prize claimed today short-circuits the next run: the day has its
    // winner, so the still-active prize returns to the pool undrawn.
    seed_item(
        &pool,
        "RAFTEST-DRAW-2",
        mutamarket::raffles::STATUS_CLAIMED,
        Some(active.user_id),
    )
    .await;

    let stats = mutamarket::raffles::draw_winners(&pool)
        .await
        .expect("second draw");
    assert_eq!((stats.drawn, stats.reset), (0, 1));

    let (status, winner): (i32, Option<i64>) =
        sqlx::query_as("select status, winner_id from raffle_items where id = $1")
            .bind(prize)
            .fetch_one(&pool)
            .await
            .expect("reset prize");
    assert_eq!(status, mutamarket::raffles::STATUS_PENDING);
    assert_eq!(winner, None);
}

/// One runtime, one sequence: the draw's "has there been a winner
/// today" check reads the whole table, so a sibling test claiming a
/// prize in parallel would decide this one's outcome.
#[tokio::test]
async fn raffle_behavior() {
    claim_and_decline_follow_the_legacy_guards().await;
    admin_store_validates_and_creates_one_item_per_code().await;
    admin_index_lists_items_in_the_legacy_order().await;
    the_draw_picks_active_users_and_yields_to_a_daily_winner().await;
}
