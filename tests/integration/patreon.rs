//! Behavior tests for the Patreon subscriber sync against a mock
//! campaign API (the legacy `GetPatreonSubscribers` +
//! `UpdateSubscribedPatreonMembers`): tier filtering, flag grants and
//! revocations, and the null-patreon_id quirk.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::routing::get;
use axum::{Json, Router};
use mutamarket::db;
use mutamarket::patreon::{PatreonCampaignClient, sync_patreon_subscribers};
use serde_json::json;
use sqlx::PgPool;

/// The premium tier every test asserts around.
const PREMIUM_TIER: i64 = 42;

/// Patreon user ids of the seeded accounts.
const SUBSCRIBED: i64 = 971_001;
const WRONG_TIER: i64 = 971_002;
const NO_TIERS: i64 = 971_003;
const DEPARTED: i64 = 971_004;

/// Mock Patreon: one campaign, three members with different tier sets.
/// The member listing is only served under the *string* id from the
/// campaign-details response, pinning the legacy two-step lookup.
fn mock_patreon() -> Router {
    Router::new()
        .route(
            "/campaigns",
            get(|| async { Json(json!({ "data": [{ "id": "555", "type": "campaign" }] })) }),
        )
        .route(
            "/campaigns/{id}",
            get(|| async {
                Json(json!({ "data": { "id": "555", "type": "campaign", "attributes": {} } }))
            }),
        )
        .route(
            "/campaigns/{id}/members",
            get(|| async {
                Json(json!({ "data": [
                    { "id": "m-subscribed", "type": "member" },
                    { "id": "m-wrong-tier", "type": "member" },
                    { "id": "m-no-tiers", "type": "member" },
                ] }))
            }),
        )
        .route(
            "/members/{id}",
            get(
                |axum::extract::Path(id): axum::extract::Path<String>| async move {
                    let (tiers, user_id) = match id.as_str() {
                        "m-subscribed" => (json!([{ "id": "42", "type": "tier" }]), SUBSCRIBED),
                        "m-wrong-tier" => (json!([{ "id": "7", "type": "tier" }]), WRONG_TIER),
                        _ => (json!([]), NO_TIERS),
                    };
                    Json(json!({ "data": {
                    "id": id,
                    "type": "member",
                    "relationships": {
                        "currently_entitled_tiers": { "data": tiers },
                        "user": { "data": { "id": user_id.to_string(), "type": "user" } },
                    },
                } }))
                },
            ),
        )
}

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve mock Patreon");
    });
    format!("http://{address}")
}

async fn seed_user(pool: &PgPool, name: &str, patreon_id: Option<i64>, member: bool) -> i64 {
    sqlx::query("delete from users where name = $1")
        .bind(name)
        .execute(pool)
        .await
        .expect("clean user");
    sqlx::query_scalar(
        "insert into users (name, patreon_id, is_patreon_member) values ($1, $2, $3) returning id",
    )
    .bind(name)
    .bind(patreon_id)
    .bind(member)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[tokio::test]
async fn premium_tiers_grant_the_flag_and_everyone_else_loses_it() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let subscribed = seed_user(&pool, "Patreon Subscribed", Some(SUBSCRIBED), false).await;
    let wrong_tier = seed_user(&pool, "Patreon Wrong Tier", Some(WRONG_TIER), true).await;
    let no_tiers = seed_user(&pool, "Patreon No Tiers", Some(NO_TIERS), true).await;
    let departed = seed_user(&pool, "Patreon Departed", Some(DEPARTED), true).await;
    // The legacy whereNotIn quirk: a flagged user without a linked
    // patreon_id is never unflagged.
    let unlinked = seed_user(&pool, "Patreon Unlinked", None, true).await;

    let base = start_mock(mock_patreon()).await;
    let client = PatreonCampaignClient::new(&base, "creator-token");

    let stats = sync_patreon_subscribers(&pool, &client, &[PREMIUM_TIER])
        .await
        .expect("sync");
    assert_eq!(
        (stats.campaigns, stats.members, stats.premium_members),
        (1, 3, 1)
    );

    let flags: Vec<(i64, bool)> =
        sqlx::query_as("select id, is_patreon_member from users where id = any($1) order by id")
            .bind(vec![subscribed, wrong_tier, no_tiers, departed, unlinked])
            .fetch_all(&pool)
            .await
            .expect("flags");
    let expect: Vec<(i64, bool)> = vec![
        (subscribed, true),
        (wrong_tier, false),
        (no_tiers, false),
        (departed, false),
        (unlinked, true),
    ];
    assert_eq!(flags, expect);

    // A re-run is a no-op.
    let stats = sync_patreon_subscribers(&pool, &client, &[PREMIUM_TIER])
        .await
        .expect("re-run");
    assert_eq!(stats.premium_members, 1);
    let still: bool = sqlx::query_scalar("select is_patreon_member from users where id = $1")
        .bind(subscribed)
        .fetch_one(&pool)
        .await
        .expect("flag");
    assert!(still);

    // An empty PATREON_PREMIUM_TIERS parses to no tiers: nobody
    // qualifies and everyone flagged loses the flag — even the unlinked
    // user, because an empty NOT IN matched every row in legacy just as
    // an empty `<> all` does here. (Sequential on purpose: the flag
    // updates are global, so a parallel test would race them.)
    let stats = sync_patreon_subscribers(&pool, &client, &[])
        .await
        .expect("tierless sync");
    assert_eq!((stats.members, stats.premium_members), (3, 0));
    let flags: Vec<(i64, bool)> =
        sqlx::query_as("select id, is_patreon_member from users where id = any($1) order by id")
            .bind(vec![subscribed, unlinked])
            .fetch_all(&pool)
            .await
            .expect("flags after tierless sync");
    let expect: Vec<(i64, bool)> = vec![(subscribed, false), (unlinked, false)];
    assert_eq!(flags, expect);
}
