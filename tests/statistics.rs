//! Behavior tests for the unified statistics endpoints: the market
//! overview, the top-creators leaderboard (legacy
//! `StatisticsController`) and the personal creation stats (legacy
//! `StatsController`).
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// Both tests share one seeding pass: re-seeding would wipe the other
/// test's user mid-flight and invalidate its session.
static SEEDED: OnceCell<(String, String)> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static (String, String) {
    SEEDED.get_or_init(|| seed(pool)).await
}

/// Fixture abyssal types the seeded modules are instances of.
const WEBIFIER_TYPE_ID: i64 = 47702;
const MWD_50MN_TYPE_ID: i64 = 47408;
/// Fixture source/mutaplasmid pair used for the money-spent pricing.
const KHANID_WEBIFIER_TYPE_ID: i64 = 28514;
const WEBIFIER_MUTAPLASMID_ID: i64 = 47699;
/// A fixture type deliberately left without market history.
const UNPRICED_SOURCE_TYPE_ID: i64 = 526;

/// Seeded ids, far outside the ESI ranges other suites use.
const ALICE_ID: i64 = 990_001_001;
const BOB_ID: i64 = 990_001_002;
const OUTSIDER_ID: i64 = 990_001_003;
const MODULE_ID_BASE: i64 = 990_002_000;

const FORGE_REGION_ID: i64 = 10_000_002;
const KHANID_AVERAGE: f64 = 10_000_000.0;
const MUTAPLASMID_AVERAGE: f64 = 3_000_000.0;

async fn seed(pool: &PgPool) -> (String, String) {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables).await.expect("seed reference tables");

    // Idempotent: wipe this suite's users, characters and modules.
    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![ALICE_ID, BOB_ID, OUTSIDER_ID])
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Stats Owner", "Stats Outsider"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Stats Owner') returning id")
            .fetch_one(pool)
            .await
            .expect("create owner");
    let outsider_user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Stats Outsider') returning id")
            .fetch_one(pool)
            .await
            .expect("create outsider user");

    for (id, name, user_id) in [
        (ALICE_ID, "Statfix Alice", Some(owner_id)),
        (BOB_ID, "Statfix Bob", Some(owner_id)),
        (OUTSIDER_ID, "Statfix Outsider", Some(outsider_user_id)),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("create character");
    }

    // Alice creates three webifiers, Bob one webifier and two MWDs, the
    // outsider one webifier. Values feed the personal totals as
    // (id offset, type, creator, estimated value, source type).
    type ModuleSeed = (i64, i64, i64, Option<f64>, Option<i64>);
    let modules: [ModuleSeed; 6] = [
        (0, WEBIFIER_TYPE_ID, ALICE_ID, Some(100_000_000.0), Some(KHANID_WEBIFIER_TYPE_ID)),
        (1, WEBIFIER_TYPE_ID, ALICE_ID, Some(50_000_000.0), Some(KHANID_WEBIFIER_TYPE_ID)),
        // Unpriced source: counts everywhere except money spent.
        (2, WEBIFIER_TYPE_ID, ALICE_ID, Some(25_000_000.0), Some(UNPRICED_SOURCE_TYPE_ID)),
        (3, WEBIFIER_TYPE_ID, BOB_ID, None, Some(KHANID_WEBIFIER_TYPE_ID)),
        (4, MWD_50MN_TYPE_ID, BOB_ID, None, None),
        (5, MWD_50MN_TYPE_ID, BOB_ID, None, None),
    ];
    for (offset, type_id, creator_id, value, source_type_id) in modules {
        sqlx::query(
            "insert into modules (id, type_id, creator_id, estimated_value,
                                  source_type_id, mutaplasmid_id)
             values ($1, $2, $3, $4, $5, $6)",
        )
        .bind(MODULE_ID_BASE + offset)
        .bind(type_id)
        .bind(creator_id)
        .bind(value)
        .bind(source_type_id)
        .bind(source_type_id.map(|_| WEBIFIER_MUTAPLASMID_ID))
        .execute(pool)
        .await
        .expect("create module");
    }
    sqlx::query(
        "insert into modules (id, type_id, creator_id) values ($1, $2, $3)",
    )
    .bind(MODULE_ID_BASE + 6)
    .bind(WEBIFIER_TYPE_ID)
    .bind(OUTSIDER_ID)
    .execute(pool)
    .await
    .expect("create outsider module");

    // Latest-day market pricing for the spent total; two days verify the
    // newest one wins.
    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region");
    for (type_id, date, average) in [
        (KHANID_WEBIFIER_TYPE_ID, "2026-08-01", 99_000_000.0),
        (KHANID_WEBIFIER_TYPE_ID, "2026-08-20", KHANID_AVERAGE),
        (WEBIFIER_MUTAPLASMID_ID, "2026-08-20", MUTAPLASMID_AVERAGE),
    ] {
        sqlx::query(
            "insert into market_histories (type_id, region_id, date, average, highest, lowest)
             values ($1, $2, $3::date, $4, $4, $4)
             on conflict (type_id, region_id, date) do update set average = excluded.average",
        )
        .bind(type_id)
        .bind(FORGE_REGION_ID)
        .bind(date)
        .bind(average)
        .execute(pool)
        .await
        .expect("history");
    }
    sqlx::query("delete from market_histories where type_id = $1")
        .bind(UNPRICED_SOURCE_TYPE_ID)
        .execute(pool)
        .await
        .expect("unpriced source stays unpriced");

    // The endpoints read the materialized views; pick up the seed.
    mutamarket::modules::stats::refresh_statistics_views(pool)
        .await
        .expect("statistics views refresh");

    let owner_session =
        create_session(pool, owner_id, Some(ALICE_ID)).await.expect("owner session");
    let outsider_session =
        create_session(pool, outsider_user_id, Some(OUTSIDER_ID)).await.expect("outsider session");
    (owner_session, outsider_session)
}

async fn get_json(
    app: &axum::Router,
    uri: &str,
    session: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().uri(uri);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
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
async fn overview_and_leaderboard_serve_the_statistics_page() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // The overview: archive stats plus the value/creator aggregates.
    let (status, body) = get_json(&app, "/api/statistics/overview", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec![
            "average_value",
            "characters_count",
            "creators_count",
            "refreshed_at",
            "stats",
            "total_value",
        ],
    );
    assert!(body["stats"]["total_count"].as_i64().expect("count") >= 7);
    assert!(body["creators_count"].as_i64().expect("creators") >= 3);
    assert!(body["total_value"].as_f64().expect("value") >= 175_000_000.0);

    // The leaderboard, name-scoped to this suite's characters: Alice and
    // Bob tie at three creations; the outsider trails with one.
    let (status, body) =
        get_json(&app, "/api/statistics/top?name=Statfix", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), vec!["data", "meta"]);
    assert_eq!(
        sorted_keys(&body["meta"]),
        vec!["current_page", "per_page", "total"],
    );
    assert_eq!(body["meta"]["total"].as_i64(), Some(3));
    assert_eq!(body["meta"]["current_page"].as_i64(), Some(1));
    assert_eq!(body["meta"]["per_page"].as_i64(), Some(15));

    let rows = body["data"].as_array().expect("rows");
    assert_eq!(rows.len(), 3);
    for row in rows {
        assert_eq!(
            sorted_keys(row),
            vec!["id", "modules_created_count", "name", "rank_number"],
        );
    }
    let by_name = |name: &str| {
        rows.iter()
            .find(|row| row["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("{name} listed"))
    };
    assert_eq!(by_name("Statfix Alice")["modules_created_count"].as_i64(), Some(3));
    assert_eq!(by_name("Statfix Bob")["modules_created_count"].as_i64(), Some(3));
    assert_eq!(by_name("Statfix Outsider")["modules_created_count"].as_i64(), Some(1));
    // Ties share a rank (rank(), not row_number()); the outsider ranks
    // strictly below both.
    assert_eq!(
        by_name("Statfix Alice")["rank_number"].as_i64(),
        by_name("Statfix Bob")["rank_number"].as_i64(),
    );
    assert!(
        by_name("Statfix Outsider")["rank_number"].as_i64()
            > by_name("Statfix Alice")["rank_number"].as_i64(),
    );

    // The default order is rank ascending; name sorting flips to the
    // requested direction.
    let (_, sorted) = get_json(
        &app,
        "/api/statistics/top?name=Statfix&sort_field=name&sort_direction=desc",
        None,
    )
    .await;
    let names: Vec<&str> = sorted["data"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["Statfix Outsider", "Statfix Bob", "Statfix Alice"]);

    // The type segment scopes the counts like the legacy search: only
    // Bob created 50MN MWDs.
    let (status, body) = get_json(
        &app,
        &format!("/api/statistics/top/type/{MWD_50MN_TYPE_ID}?name=Statfix"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["data"].as_array().expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"].as_str(), Some("Statfix Bob"));
    assert_eq!(rows[0]["modules_created_count"].as_i64(), Some(2));
    assert_eq!(rows[0]["rank_number"].as_i64(), Some(1));
}

#[tokio::test]
async fn personal_stats_total_the_users_creations() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner_session, _) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests get the legacy JSON 401.
    let (status, body) = get_json(&app, "/api/personal/stats", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));

    let (status, body) = get_json(&app, "/api/personal/stats", Some(owner_session)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        sorted_keys(&body),
        vec!["stats", "total_modules", "total_spent", "total_value"],
    );

    // Both owned characters count; the outsider's module does not.
    assert_eq!(body["total_modules"].as_i64(), Some(6));
    assert_eq!(body["total_value"].as_f64(), Some(175_000_000.0));
    // Money spent prices the three fully-priced webifiers at the latest
    // market day (10M + 3M each); the unpriced-source and history-less
    // modules drop out like the legacy inner joins.
    assert_eq!(body["total_spent"].as_f64(), Some(3.0 * (KHANID_AVERAGE + MUTAPLASMID_AVERAGE)));

    let stats = body["stats"].as_array().expect("stats rows");
    for row in stats {
        assert_eq!(sorted_keys(row), vec!["count", "creator", "type"]);
        assert_eq!(sorted_keys(&row["type"]), vec!["id", "name"]);
        assert_eq!(sorted_keys(&row["creator"]), vec!["id", "name"]);
    }
    // Grouped per (type, creator), ordered by count descending.
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0]["creator"]["name"].as_str(), Some("Statfix Alice"));
    assert_eq!(stats[0]["type"]["id"].as_i64(), Some(WEBIFIER_TYPE_ID));
    assert_eq!(stats[0]["count"].as_i64(), Some(3));
    let counts: Vec<i64> =
        stats.iter().map(|row| row["count"].as_i64().expect("count")).collect();
    assert_eq!(counts, vec![3, 2, 1]);
}
