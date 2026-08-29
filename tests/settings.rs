//! Behavior tests for the settings page: `GET /api/settings`,
//! `PUT /settings` (the notify-character pick with its legacy
//! validation and cross-account steal) and the linked-account
//! visibility toggles.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// Both tests share one seeding pass: re-seeding would wipe the other
/// test's user mid-flight and invalidate its session.
static SEEDED: OnceCell<(String, String, String, i64, i64, i64)> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static (String, String, String, i64, i64, i64) {
    SEEDED.get_or_init(|| seed(pool)).await
}

const OWNER_CHARACTERS: [i64; 2] = [990_003_001, 990_003_002];
const RIVAL_CHARACTER: i64 = 990_003_003;

async fn seed(pool: &PgPool) -> (String, String, String, i64, i64, i64) {
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![
            OWNER_CHARACTERS[0],
            OWNER_CHARACTERS[1],
            RIVAL_CHARACTER,
        ])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Settings Owner", "Settings Rival", "Settings Toggler"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 = sqlx::query_scalar(
        "insert into users (name, discord_name, discord_avatar) values
         ('Settings Owner', 'owner#1234', 'https://cdn.example/avatar.png') returning id",
    )
    .fetch_one(pool)
    .await
    .expect("create owner");
    let rival_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Settings Rival') returning id")
            .fetch_one(pool)
            .await
            .expect("create rival");
    // The visibility test gets its own user so its toggling never races
    // the page-data assertions.
    let toggler_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Settings Toggler') returning id")
            .fetch_one(pool)
            .await
            .expect("create toggler");

    for (id, name, user_id) in [
        (OWNER_CHARACTERS[0], "Settex Prime", owner_id),
        (OWNER_CHARACTERS[1], "Settex Alt", owner_id),
        (RIVAL_CHARACTER, "Settex Rival", rival_id),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("create character");
    }

    let owner = create_session(pool, owner_id, Some(OWNER_CHARACTERS[0]))
        .await
        .expect("owner session");
    let rival = create_session(pool, rival_id, Some(RIVAL_CHARACTER))
        .await
        .expect("rival session");
    let toggler = create_session(pool, toggler_id, None)
        .await
        .expect("toggler session");
    (owner, rival, toggler, owner_id, rival_id, toggler_id)
}

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
) -> (StatusCode, Option<String>, serde_json::Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
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

#[tokio::test]
async fn settings_page_data_and_notify_pick() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner, rival, _, owner_id, rival_id, _) = seed_once(&pool).await.clone();
    let app = mutamarket::server::test_router().await;

    // Guests get the JSON 401 on the API and the login redirect on the
    // page PUT.
    let (status, _, body) = request(&app, Method::GET, "/api/settings", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));
    let (status, location, _) =
        request(&app, Method::PUT, "/settings?character_to_notify=1", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));

    // The page data: characters, no explicit pick yet, one linked
    // account card, the unported raffle list stays empty.
    let (status, _, body) = request(&app, Method::GET, "/api/settings", Some(&owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec![
            "character_to_notify",
            "characters",
            "discord",
            "patreon",
            "raffle_wins",
            "twitch"
        ],
    );
    assert_eq!(body["characters"].as_array().expect("characters").len(), 2);
    assert_eq!(sorted_keys(&body["characters"][0]), vec!["id", "name"]);
    assert!(body["character_to_notify"].is_null());
    assert_eq!(
        sorted_keys(&body["discord"]),
        vec!["avatar", "is_public", "name"]
    );
    assert_eq!(body["discord"]["name"].as_str(), Some("owner#1234"));
    assert_eq!(body["discord"]["is_public"].as_bool(), Some(false));
    assert!(body["twitch"].is_null());
    assert!(body["patreon"].is_null());
    assert_eq!(
        body["raffle_wins"].as_array().expect("raffle wins").len(),
        0
    );

    // Picking someone else's character (or garbage) is the legacy
    // validation error.
    for invalid in [
        format!("character_to_notify={RIVAL_CHARACTER}"),
        "nonsense=1".to_owned(),
    ] {
        let (status, _, body) = request(
            &app,
            Method::PUT,
            &format!("/settings?{invalid}"),
            Some(&owner),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{invalid}");
        assert_eq!(
            body["message"].as_str(),
            Some("The selected character to notify is invalid."),
        );
    }

    // A valid pick lands in notify_characters.
    let (status, _, _) = request(
        &app,
        Method::PUT,
        &format!("/settings?character_to_notify={}", OWNER_CHARACTERS[1]),
        Some(&owner),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let picked: Option<i64> =
        sqlx::query_scalar("select character_id from notify_characters where user_id = $1")
            .bind(owner_id)
            .fetch_optional(&pool)
            .await
            .expect("notify row");
    assert_eq!(picked, Some(OWNER_CHARACTERS[1]));
    let (_, _, body) = request(&app, Method::GET, "/api/settings", Some(&owner)).await;
    assert_eq!(
        body["character_to_notify"]["id"].as_i64(),
        Some(OWNER_CHARACTERS[1])
    );

    // The legacy steal: another account claiming the same character
    // would first need to own it; simulate the legacy delete by giving
    // the rival a stale row on the owner's character.
    sqlx::query(
        "insert into notify_characters (user_id, character_id) values ($1, $2)
         on conflict (user_id) do update set character_id = excluded.character_id",
    )
    .bind(rival_id)
    .bind(OWNER_CHARACTERS[1])
    .execute(&pool)
    .await
    .expect("stale rival row");
    let (status, _, _) = request(
        &app,
        Method::PUT,
        &format!("/settings?character_to_notify={}", OWNER_CHARACTERS[1]),
        Some(&owner),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rival_row: Option<i64> =
        sqlx::query_scalar("select character_id from notify_characters where user_id = $1")
            .bind(rival_id)
            .fetch_optional(&pool)
            .await
            .expect("rival row");
    assert_eq!(
        rival_row, None,
        "the pick steals the character from other accounts"
    );

    let _ = rival;
}

#[tokio::test]
async fn visibility_toggles_flip_and_redirect() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (_, _, owner, _, _, owner_id) = seed_once(&pool).await.clone();
    let app = mutamarket::server::test_router().await;

    // Guests bounce to the login page.
    let (status, location, _) = request(&app, Method::PUT, "/discord?is_public=1", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));

    for (path, column) in [
        ("/discord", "discord_is_public"),
        ("/twitch", "twitch_is_public"),
        ("/patreon", "patreon_is_public"),
    ] {
        let (status, location, _) = request(
            &app,
            Method::PUT,
            &format!("{path}?is_public=1"),
            Some(&owner),
        )
        .await;
        assert_eq!(status, StatusCode::SEE_OTHER, "{path}");
        // The legacy controllers land back on the settings page.
        assert_eq!(location.as_deref(), Some("/settings"), "{path}");
        let flag: bool = sqlx::query_scalar(&format!("select {column} from users where id = $1"))
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("flag");
        assert!(flag, "{column} set");

        let (status, _, _) = request(
            &app,
            Method::PUT,
            &format!("{path}?is_public=0"),
            Some(&owner),
        )
        .await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        let flag: bool = sqlx::query_scalar(&format!("select {column} from users where id = $1"))
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("flag");
        assert!(!flag, "{column} cleared");
    }

    // The Laravel required|boolean rule rejects anything else.
    let (status, _, body) =
        request(&app, Method::PUT, "/discord?is_public=maybe", Some(&owner)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["message"].as_str(),
        Some("The is public field is required.")
    );
}
