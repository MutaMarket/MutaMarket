//! Behavior tests for `GET /api/nav-state`: the navigation payload the
//! frontend layout loads — the session user plus the account's characters
//! with active and asset-scope flags.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use sqlx::PgPool;
use tower::ServiceExt;

/// Test-owned characters, cleaned and recreated per run.
const CHARACTER_ONE: i64 = 97_200_001;
const CHARACTER_TWO: i64 = 97_200_002;

/// The read-assets scope string, as stored on tokens.
const READ_ASSETS: &str = "esi-assets.read_assets.v1";

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

async fn get_json(
    app: &Router,
    path: &str,
    session: Option<&str>,
) -> (StatusCode, String, serde_json::Value) {
    let mut builder = Request::builder().uri(path);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }

    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("request"))
        .await
        .expect("infallible");

    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();

    (
        status,
        content_type,
        serde_json::from_slice(&body).expect("JSON body"),
    )
}

/// A fresh user owning both test characters; only the first one holds the
/// Read Assets scope.
async fn seed_account(pool: &PgPool) -> i64 {
    for character_id in [CHARACTER_ONE, CHARACTER_TWO] {
        sqlx::query("update characters set latest_asset_import_id = null where id = $1")
            .bind(character_id)
            .execute(pool)
            .await
            .expect("unlink import");
        for table in ["asset_imports", "assets", "esi_tokens"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "delete from {table} where character_id = $1"
            )))
            .bind(character_id)
            .execute(pool)
            .await
            .expect("clean character state");
        }
        sqlx::query("delete from characters where id = $1")
            .bind(character_id)
            .execute(pool)
            .await
            .expect("clean character");
    }

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Nav Pilot') returning id")
            .fetch_one(pool)
            .await
            .expect("create user");
    sqlx::query(
        "insert into characters (id, name, user_id, corporation_id)
         values ($1, 'Nav Pilot', $2, 1000001), ($3, 'Nav Alt', $2, null)",
    )
    .bind(CHARACTER_ONE)
    .bind(user_id)
    .bind(CHARACTER_TWO)
    .execute(pool)
    .await
    .expect("create characters");

    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, 'access', 'refresh', 'Bearer', 'owner', $2, now() + interval '20 minutes')",
    )
    .bind(CHARACTER_ONE)
    .bind(vec![READ_ASSETS.to_owned()])
    .execute(pool)
    .await
    .expect("seed token");

    user_id
}

#[tokio::test]
async fn nav_state_carries_the_user_and_characters() {
    let (pool, _db) = crate::common::db::fresh().await;
    let app = mutamarket::server::test_router(pool.clone()).await;

    // Guests get a JSON null, like the legacy null auth.user shared prop.
    let (status, content_type, body) = get_json(&app, "/api/nav-state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        content_type.starts_with("application/json"),
        "{content_type}"
    );
    assert_eq!(body, serde_json::Value::Null);

    let user_id = seed_account(&pool).await;

    // A session acting as the second character.
    let session = create_session(&pool, user_id, Some(CHARACTER_TWO))
        .await
        .expect("create session");
    let (status, _, body) = get_json(&app, "/api/nav-state", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        [
            "characters",
            "raffle",
            "scope_catalogue",
            "unread_offers",
            "user"
        ],
    );
    // No drawn prize for this account: the legacy RaffleData null.
    assert!(body["raffle"].is_null());
    // No offer threads either: the legacy unread_offers count is zero.
    assert_eq!(body["unread_offers"], 0);
    assert_eq!(
        sorted_keys(&body["user"]),
        [
            "accent_color",
            "active_character_id",
            "ad_free",
            "has_premium",
            "is_admin",
            "is_patreon_member",
            "name",
            "premium_ad_free",
        ],
    );
    assert_eq!(body["user"]["has_premium"], false);
    assert_eq!(body["user"]["ad_free"], false);
    assert_eq!(body["user"]["premium_ad_free"], false);
    assert_eq!(body["user"]["is_patreon_member"], false);
    // A non-premium account without a free pick carries no accent color.
    assert!(body["user"]["accent_color"].is_null());
    assert_eq!(body["user"]["name"], "Nav Pilot");
    assert_eq!(body["user"]["active_character_id"], CHARACTER_TWO);
    assert_eq!(body["user"]["is_admin"], false);

    // A Patreon backer (the legacy patreon.is_premium) is flagged as such.
    sqlx::query("update users set is_patreon_member = true where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("flag patreon member");
    let (_, _, backer) = get_json(&app, "/api/nav-state", Some(&session)).await;
    assert_eq!(backer["user"]["is_patreon_member"], true);
    assert_eq!(
        backer["user"]["ad_free"], false,
        "Patreon is not ad-free premium"
    );

    // The ad-free plan alone is not enough, the premium must be live;
    // then any one character makes the whole account ad-free.
    sqlx::query("update users set premium_ad_free = true where id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("ad-free plan");
    let (_, _, planned) = get_json(&app, "/api/nav-state", Some(&session)).await;
    assert_eq!(planned["user"]["ad_free"], false);
    assert_eq!(planned["user"]["premium_ad_free"], true);
    sqlx::query(
        "update characters set premium_paid_until = now() + interval '3 days' where id = $1",
    )
    .bind(CHARACTER_TWO)
    .execute(&pool)
    .await
    .expect("live premium");
    let (_, _, ad_free) = get_json(&app, "/api/nav-state", Some(&session)).await;
    assert_eq!(ad_free["user"]["ad_free"], true);
    assert_eq!(ad_free["user"]["has_premium"], true);
    // The character carries its end date for the sidebar status.
    let premium_character = &ad_free["characters"][1];
    assert_eq!(premium_character["id"], CHARACTER_TWO);
    let paid_until = premium_character["premium_paid_until"]
        .as_str()
        .expect("live premium has an end date");
    assert_eq!(paid_until.len(), "2026-01-01T00:00:00Z".len());
    assert!(paid_until.ends_with('Z'));
    assert_eq!(
        ad_free["characters"][0]["premium_paid_until"],
        serde_json::Value::Null
    );

    let characters = body["characters"].as_array().expect("characters array");
    assert_eq!(
        characters.len(),
        2,
        "both account characters, ordered by id"
    );
    for character in characters {
        assert_eq!(
            sorted_keys(character),
            [
                "active",
                "corporation_id",
                "granted_scopes",
                "has_asset_token",
                "id",
                "name",
                "premium_paid_until",
                "scope_warnings_muted",
            ],
        );
    }
    assert_eq!(characters[0]["id"], CHARACTER_ONE);
    assert_eq!(characters[0]["name"], "Nav Pilot");
    assert_eq!(characters[0]["corporation_id"], 1000001);
    assert_eq!(characters[0]["has_asset_token"], true);
    assert_eq!(characters[0]["active"], false);
    assert_eq!(characters[1]["id"], CHARACTER_TWO);
    assert_eq!(characters[1]["corporation_id"], serde_json::Value::Null);
    assert_eq!(characters[1]["has_asset_token"], false);
    assert_eq!(characters[1]["active"], true);

    // The granted scopes are the union over the character's tokens, and
    // a character without one grants nothing.
    let granted = characters[0]["granted_scopes"]
        .as_array()
        .expect("granted scopes");
    assert!(
        granted
            .iter()
            .any(|scope| scope == "esi-assets.read_assets.v1"),
        "the seeded asset token shows up",
    );
    assert_eq!(characters[1]["granted_scopes"], serde_json::json!([]));
    assert_eq!(characters[0]["scope_warnings_muted"], false);
    // Neither has premium: no end date.
    assert_eq!(characters[0]["premium_paid_until"], serde_json::Value::Null);

    // The scope vocabulary the menu and settings summary render.
    let catalogue = body["scope_catalogue"].as_array().expect("scope catalogue");
    assert!(!catalogue.is_empty());
    for scope in catalogue {
        assert_eq!(
            sorted_keys(scope),
            ["description", "id", "label", "optional"],
        );
    }

    // Without a chosen character the active flag falls back to the first
    // one, like the legacy getActiveCharacter.
    let session = create_session(&pool, user_id, None)
        .await
        .expect("create session");
    let (status, _, body) = get_json(&app, "/api/nav-state", Some(&session)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["active_character_id"], serde_json::Value::Null);
    assert_eq!(body["characters"][0]["active"], true);
    assert_eq!(body["characters"][1]["active"], false);
}
