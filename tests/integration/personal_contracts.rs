//! Behavior tests for the personal contracts page (legacy
//! `ContractController`): the three merged sources with their exact
//! per-model `ContractResource` key sets, the raw-ESI status folding,
//! the last-month default window, the admin-only training flag, and the
//! synchronous refresh action's redirects.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::path::Path;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::auth::session::create_session;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use sqlx::PgPool;
use tokio::sync::OnceCell;
use tower::ServiceExt;

/// Fixture abyssal type of the seeded modules.
const WEBIFIER_TYPE_ID: i64 = 47702;
const FORGE_REGION_ID: i64 = 10_000_002;

const MODULE_ID_BASE: i64 = 990_006_000;
const CONTRACT_ID_BASE: i64 = 990_006_500;
const CHARACTER_ID_BASE: i64 = 990_006_900;

/// The owner's characters, a stranger who accepts one contract, and an
/// admin with a contract of their own.
const CHAR_A: i64 = CHARACTER_ID_BASE + 1;
const CHAR_B: i64 = CHARACTER_ID_BASE + 2;
const OTHER: i64 = CHARACTER_ID_BASE + 3;
const ADMIN_CHAR: i64 = CHARACTER_ID_BASE + 4;

/// One contract id per source.
const PUBLIC_CONTRACT: i64 = CONTRACT_ID_BASE;
const HISTORIC_CONTRACT: i64 = CONTRACT_ID_BASE + 1;
const OLD_HISTORIC_CONTRACT: i64 = CONTRACT_ID_BASE + 2;
const ACCEPTED_PERSONAL: i64 = CONTRACT_ID_BASE + 3;
const ASSIGNED_PERSONAL: i64 = CONTRACT_ID_BASE + 4;
const EMPTY_PERSONAL: i64 = CONTRACT_ID_BASE + 5;
const ADMIN_HISTORIC: i64 = CONTRACT_ID_BASE + 6;
const ALLIANCE_ACCEPTED: i64 = CONTRACT_ID_BASE + 7;
const ACCEPTOR_ALLIANCE: i64 = CHARACTER_ID_BASE + 20;
const CORPORATION_ACCEPTED: i64 = CONTRACT_ID_BASE + 8;
const ACCEPTOR_CORPORATION: i64 = CHARACTER_ID_BASE + 21;

/// (owner session, admin session).
static SEEDED: OnceCell<(String, String)> = OnceCell::const_new();

async fn seed_once(pool: &PgPool) -> &'static (String, String) {
    SEEDED.get_or_init(|| seed(pool)).await
}

async fn seed(pool: &PgPool) -> (String, String) {
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(pool, &tables)
        .await
        .expect("seed reference tables");

    // Items cascade with their contracts.
    sqlx::query("delete from contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean contracts");
    sqlx::query("delete from historic_contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean historic contracts");
    sqlx::query("delete from character_contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_ID_BASE)
        .execute(pool)
        .await
        .expect("clean character contracts");
    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_ID_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from characters where id >= $1 and id < $1 + 100")
        .bind(CHARACTER_ID_BASE)
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Contracts Owner", "Contracts Admin"])
        .execute(pool)
        .await
        .expect("clean users");

    let owner_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Contracts Owner') returning id")
            .fetch_one(pool)
            .await
            .expect("create owner");
    let admin_id: i64 = sqlx::query_scalar(
        "insert into users (name, is_admin) values ('Contracts Admin', true) returning id",
    )
    .fetch_one(pool)
    .await
    .expect("create admin");
    for (id, name, user) in [
        (CHAR_A, "Contract Alice", Some(owner_id)),
        (CHAR_B, "Contract Bob", Some(owner_id)),
        (OTHER, "Contract Stranger", None),
        (ADMIN_CHAR, "Contract Admin", Some(admin_id)),
    ] {
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(user)
            .execute(pool)
            .await
            .expect("create character");
    }

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(FORGE_REGION_ID)
        .execute(pool)
        .await
        .expect("region");

    for offset in 0..2 {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(MODULE_ID_BASE + offset)
            .bind(WEBIFIER_TYPE_ID)
            .execute(pool)
            .await
            .expect("create module");
    }

    // An outstanding public contract issued by Alice, five days old.
    sqlx::query(
        "insert into contracts
             (id, region_id, issuer_id, type, date_issued, date_expired, price, unified_price,
              abyssal_modules_count, plex_count)
         values ($1, $2, $3, 'item_exchange', now() - interval '5 days',
                 now() + interval '9 days', 500000000, 500000000, 1, 0)",
    )
    .bind(PUBLIC_CONTRACT)
    .bind(FORGE_REGION_ID)
    .bind(CHAR_A)
    .execute(pool)
    .await
    .expect("create public contract");
    sqlx::query(
        "insert into contract_items (contract_id, record_id, type_id, item_id)
         values ($1, 1, $2, $3)",
    )
    .bind(PUBLIC_CONTRACT)
    .bind(WEBIFIER_TYPE_ID)
    .bind(MODULE_ID_BASE)
    .execute(pool)
    .await
    .expect("create public contract item");

    // A completed archived contract four days old, and one sixty days old
    // (outside the default month window).
    for (id, days_ago) in [(HISTORIC_CONTRACT, 4), (OLD_HISTORIC_CONTRACT, 60)] {
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, date_issued, price, unified_price,
                  abyssal_modules_count)
             values ($1, 'completed', $2, $3, 'item_exchange',
                     now() - make_interval(days => $4), 250000000, 250000000, 1)",
        )
        .bind(id)
        .bind(FORGE_REGION_ID)
        .bind(CHAR_A)
        .bind(days_ago)
        .execute(pool)
        .await
        .expect("create historic contract");
    }
    sqlx::query(
        "insert into historic_contract_items (historic_contract_id, record_id, type_id, item_id)
         values ($1, 1, $2, $3)
         on conflict (historic_contract_id, record_id) do nothing",
    )
    .bind(HISTORIC_CONTRACT)
    .bind(WEBIFIER_TYPE_ID)
    .bind(MODULE_ID_BASE + 1)
    .execute(pool)
    .await
    .expect("create historic contract item");

    // The admin's own archived contract, for the admin-only key.
    sqlx::query(
        "insert into historic_contracts
             (id, status, region_id, issuer_id, type, date_issued, price, unified_price,
              abyssal_modules_count)
         values ($1, 'completed', $2, $3, 'item_exchange', now() - interval '2 days',
                 100000000, 100000000, 1)",
    )
    .bind(ADMIN_HISTORIC)
    .bind(FORGE_REGION_ID)
    .bind(ADMIN_CHAR)
    .execute(pool)
    .await
    .expect("create admin historic contract");

    // A private personal contract Alice issued, accepted by the stranger:
    // the raw ESI status folds to completed.
    sqlx::query(
        "insert into character_contracts
             (id, issuer_id, type, availability, status, date_issued, date_expired,
              date_accepted, price, unified_price, acceptor_id, acceptor_type,
              abyssal_modules_count)
         values ($1, $2, 'item_exchange', 'personal', 'finished_issuer',
                 now() - interval '3 days', now() + interval '11 days',
                 now() - interval '2 days', 100000000, 100000000, $3, 'character', 1)",
    )
    .bind(ACCEPTED_PERSONAL)
    .bind(CHAR_A)
    .bind(OTHER)
    .execute(pool)
    .await
    .expect("create accepted personal contract");
    sqlx::query(
        "insert into character_contract_items (character_contract_id, type_id, record_id)
         values ($1, $2, 1) on conflict do nothing",
    )
    .bind(ACCEPTED_PERSONAL)
    .bind(WEBIFIER_TYPE_ID)
    .execute(pool)
    .await
    .expect("create personal contract item");

    // A personal contract Alice issued and an alliance accepted: the
    // legacy morphTo serializes the AllianceResource.
    sqlx::query(
        "insert into alliances (id, name) values ($1, 'Accepting Alliance')
         on conflict (id) do update set name = excluded.name",
    )
    .bind(ACCEPTOR_ALLIANCE)
    .execute(pool)
    .await
    .expect("create acceptor alliance");
    sqlx::query(
        "insert into character_contracts
             (id, issuer_id, type, availability, status, date_issued, date_expired,
              date_accepted, price, unified_price, acceptor_id, acceptor_type,
              abyssal_modules_count)
         values ($1, $2, 'item_exchange', 'personal', 'finished_contractor',
                 now() - interval '3 days', now() + interval '11 days',
                 now() - interval '2 days', 100000000, 100000000, $3, 'alliance', 1)",
    )
    .bind(ALLIANCE_ACCEPTED)
    .bind(CHAR_A)
    .bind(ACCEPTOR_ALLIANCE)
    .execute(pool)
    .await
    .expect("create alliance-accepted contract");

    // A personal contract Alice issued and a corporation accepted: the
    // legacy morphTo serializes the CorporationResource.
    sqlx::query(
        "insert into corporations (id, name) values ($1, 'Accepting Corp')
         on conflict (id) do update set name = excluded.name",
    )
    .bind(ACCEPTOR_CORPORATION)
    .execute(pool)
    .await
    .expect("create acceptor corporation");
    sqlx::query(
        "insert into character_contracts
             (id, issuer_id, type, availability, status, date_issued, date_expired,
              date_accepted, price, unified_price, acceptor_id, acceptor_type,
              abyssal_modules_count)
         values ($1, $2, 'item_exchange', 'personal', 'finished_contractor',
                 now() - interval '3 days', now() + interval '11 days',
                 now() - interval '2 days', 100000000, 100000000, $3, 'corporation', 1)",
    )
    .bind(CORPORATION_ACCEPTED)
    .bind(CHAR_A)
    .bind(ACCEPTOR_CORPORATION)
    .execute(pool)
    .await
    .expect("create corporation-accepted contract");

    // A public personal contract assigned to Bob by the stranger, still
    // outstanding, with no acceptor yet.
    sqlx::query(
        "insert into character_contracts
             (id, issuer_id, assignee_id, type, availability, status, date_issued,
              price, unified_price, abyssal_modules_count, non_abyssal_modules_count,
              plex_count)
         values ($1, $2, $3, 'item_exchange', 'public', 'outstanding',
                 now() - interval '1 day', 50000000, 50000000, 2, 1, 2)",
    )
    .bind(ASSIGNED_PERSONAL)
    .bind(OTHER)
    .bind(CHAR_B)
    .execute(pool)
    .await
    .expect("create assigned personal contract");

    // No abyssal modules: excluded, like the legacy count filter.
    sqlx::query(
        "insert into character_contracts
             (id, issuer_id, type, availability, status, date_issued, abyssal_modules_count)
         values ($1, $2, 'courier', 'public', 'outstanding', now() - interval '1 day', 0)",
    )
    .bind(EMPTY_PERSONAL)
    .bind(CHAR_A)
    .execute(pool)
    .await
    .expect("create empty personal contract");

    let owner = create_session(pool, owner_id, Some(CHAR_A))
        .await
        .expect("session");
    let admin = create_session(pool, admin_id, Some(ADMIN_CHAR))
        .await
        .expect("session");
    (owner, admin)
}

async fn request(
    app: &axum::Router,
    method: Method,
    uri: &str,
    session: Option<&str>,
    referer: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(session) = session {
        builder = builder.header(header::COOKIE, format!("mm_session={session}"));
    }
    if let Some(referer) = referer {
        builder = builder.header(header::REFERER, referer);
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible")
}

async fn get_json(
    app: &axum::Router,
    uri: &str,
    session: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let response = request(app, Method::GET, uri, session, None).await;
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
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn contract(body: &serde_json::Value, id: i64) -> &serde_json::Value {
    body["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .find(|contract| contract["id"].as_i64() == Some(id))
        .unwrap_or_else(|| panic!("contract {id} present"))
}

const CHARACTER_KEYS: [&str; 6] = [
    "corporation_id",
    "description",
    "has_premium",
    "id",
    "name",
    "slug",
];

#[tokio::test]
async fn page_merges_the_three_sources_with_exact_key_sets() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner, _) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests get the API 401 (the page route redirects in the frontend).
    let (status, body) = get_json(&app, "/api/personal/contracts", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"].as_str(), Some("Unauthenticated."));

    let (status, body) = get_json(&app, "/api/personal/contracts", Some(owner)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(sorted_keys(&body), ["contracts", "date_end", "date_start"]);

    // The spread order: outstanding, historic, character contracts.
    let ids: Vec<i64> = body["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .filter_map(|contract| contract["id"].as_i64())
        .collect();
    assert_eq!(
        ids,
        vec![
            PUBLIC_CONTRACT,
            HISTORIC_CONTRACT,
            ACCEPTED_PERSONAL,
            ASSIGNED_PERSONAL,
            ALLIANCE_ACCEPTED,
            CORPORATION_ACCEPTED,
        ],
    );

    // The public contract: no status/availability/acceptor columns, so
    // none of their keys — the legacy whenHas key set.
    let public = contract(&body, PUBLIC_CONTRACT);
    assert_eq!(
        sorted_keys(public),
        [
            "abyssal_modules_count",
            "asking_for_items",
            "date_expired",
            "date_issued",
            "id",
            "issuer",
            "modules",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "type",
        ],
    );
    assert_eq!(sorted_keys(&public["issuer"]), CHARACTER_KEYS);
    assert_eq!(public["issuer"]["name"].as_str(), Some("Contract Alice"));
    assert_eq!(public["price"].as_f64(), Some(500_000_000.0));
    let modules = public["modules"].as_array().expect("modules loaded");
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0]["id"].as_i64(), Some(MODULE_ID_BASE));
    // The page is auth-only, so the cards carry the viewer's `note`
    // (the legacy withDefaultRelations loadout ending in withUserNote).
    assert_eq!(
        sorted_keys(&modules[0]),
        [
            "average_fraction",
            "contract",
            "creator",
            "estimated_value",
            "estimated_value_updated_at",
            "id",
            "mutaplasmid",
            "mutated_attributes",
            "note",
            "public_asset",
            "slug",
            "source_type",
            "type",
        ],
    );
    assert!(
        modules[0]["note"].is_null(),
        "no note recorded for the viewer"
    );

    // The archived contract adds the status key, nothing else (the
    // training flag stays admin-only).
    let historic = contract(&body, HISTORIC_CONTRACT);
    assert_eq!(
        sorted_keys(historic),
        [
            "abyssal_modules_count",
            "asking_for_items",
            "date_expired",
            "date_issued",
            "id",
            "issuer",
            "modules",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "status",
            "type",
        ],
    );
    assert_eq!(historic["status"].as_str(), Some("completed"));
    assert_eq!(
        historic["modules"].as_array().expect("modules")[0]["id"].as_i64(),
        Some(MODULE_ID_BASE + 1),
    );

    // The personal contract carries the character-contract-only keys:
    // types instead of module cards, the availability-derived privacy
    // flag, the acceptor and its dates, the folded raw status.
    let accepted = contract(&body, ACCEPTED_PERSONAL);
    assert_eq!(
        sorted_keys(accepted),
        [
            "abyssal_modules_count",
            "acceptor",
            "acceptor_type",
            "asking_for_items",
            "date_accepted",
            "date_expired",
            "date_issued",
            "id",
            "is_private",
            "issuer",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "status",
            "type",
            "types",
        ],
    );
    assert_eq!(
        accepted["status"].as_str(),
        Some("completed"),
        "finished_issuer folds"
    );
    assert_eq!(accepted["is_private"].as_bool(), Some(true));
    assert_eq!(accepted["acceptor_type"].as_str(), Some("character"));
    assert_eq!(sorted_keys(&accepted["acceptor"]), CHARACTER_KEYS);
    assert_eq!(accepted["acceptor"]["id"].as_i64(), Some(OTHER));
    assert!(accepted["date_accepted"].is_string());
    let types = accepted["types"].as_array().expect("types loaded");
    assert_eq!(types.len(), 1);
    assert_eq!(sorted_keys(&types[0]), ["id", "name"]);
    assert_eq!(types[0]["id"].as_i64(), Some(WEBIFIER_TYPE_ID));

    // An alliance acceptor serializes as the legacy AllianceResource.
    let alliance_accepted = contract(&body, ALLIANCE_ACCEPTED);
    assert_eq!(
        alliance_accepted["acceptor_type"].as_str(),
        Some("alliance")
    );
    assert_eq!(sorted_keys(&alliance_accepted["acceptor"]), ["id", "name"]);
    assert_eq!(
        alliance_accepted["acceptor"]["id"].as_i64(),
        Some(ACCEPTOR_ALLIANCE)
    );
    assert_eq!(
        alliance_accepted["acceptor"]["name"].as_str(),
        Some("Accepting Alliance")
    );

    // A corporation acceptor serializes as the legacy CorporationResource.
    let corporation_accepted = contract(&body, CORPORATION_ACCEPTED);
    assert_eq!(
        corporation_accepted["acceptor_type"].as_str(),
        Some("corporation")
    );
    assert_eq!(
        sorted_keys(&corporation_accepted["acceptor"]),
        ["id", "name"]
    );
    assert_eq!(
        corporation_accepted["acceptor"]["id"].as_i64(),
        Some(ACCEPTOR_CORPORATION)
    );
    assert_eq!(
        corporation_accepted["acceptor"]["name"].as_str(),
        Some("Accepting Corp")
    );

    // The assigned contract reaches the page through the assignee column;
    // no acceptor yet stays a null acceptor, public stays non-private.
    let assigned = contract(&body, ASSIGNED_PERSONAL);
    assert_eq!(assigned["status"].as_str(), Some("outstanding"));
    assert_eq!(assigned["is_private"].as_bool(), Some(false));
    assert!(assigned["acceptor"].is_null());
    assert_eq!(assigned["plex_count"].as_i64(), Some(2));
    assert_eq!(assigned["non_abyssal_modules_count"].as_i64(), Some(1));
    assert_eq!(
        assigned["types"]
            .as_array()
            .expect("types key loads empty")
            .len(),
        0
    );
}

#[tokio::test]
async fn date_window_filters_and_echoes() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner, _) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // The default month window hides the sixty-day-old contract.
    let (_, body) = get_json(&app, "/api/personal/contracts", Some(owner)).await;
    assert!(
        !body["contracts"]
            .as_array()
            .expect("contracts")
            .iter()
            .any(|contract| contract["id"].as_i64() == Some(OLD_HISTORIC_CONTRACT)),
        "the old contract stays outside the default window",
    );

    // An explicit range brings it back and is echoed to the client.
    let (status, body) = get_json(
        &app,
        "/api/personal/contracts?date_start=2020-01-01&date_end=2100-01-01",
        Some(owner),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["date_start"]
            .as_str()
            .expect("echoed")
            .starts_with("2020-01-01")
    );
    assert!(
        body["date_end"]
            .as_str()
            .expect("echoed")
            .starts_with("2100-01-01")
    );
    assert!(
        body["contracts"]
            .as_array()
            .expect("contracts")
            .iter()
            .any(|contract| contract["id"].as_i64() == Some(OLD_HISTORIC_CONTRACT)),
        "the explicit range includes the old contract",
    );
}

#[tokio::test]
async fn training_flag_is_admin_only() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner, admin) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // The owner is no admin: the flag key stays absent.
    let (_, body) = get_json(&app, "/api/personal/contracts", Some(owner)).await;
    assert!(
        contract(&body, HISTORIC_CONTRACT)
            .get("ignore_for_training")
            .is_none()
    );

    // The admin's own archived contract carries the flag, like the
    // legacy $request->user()->is_admin condition.
    let (_, body) = get_json(&app, "/api/personal/contracts", Some(admin)).await;
    assert_eq!(
        contract(&body, ADMIN_HISTORIC)["ignore_for_training"].as_bool(),
        Some(false),
    );
}

#[tokio::test]
async fn refresh_redirects_back() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let (owner, _) = seed_once(&pool).await;
    let app = mutamarket::server::test_router().await;

    // Guests are redirected to the login page.
    let response = request(&app, Method::POST, "/personal/contracts", None, None).await;
    assert!(response.status().is_redirection());
    assert_eq!(response.headers()[header::LOCATION], "/login");

    // The refresh runs synchronously (characters without a contracts
    // token are skipped like the legacy job) and lands back on the
    // referring page.
    let response = request(
        &app,
        Method::POST,
        "/personal/contracts",
        Some(owner),
        Some("/personal/contracts?date_start=2026-01-01"),
    )
    .await;
    assert!(response.status().is_redirection());
    assert_eq!(
        response.headers()[header::LOCATION],
        "/personal/contracts?date_start=2026-01-01",
    );

    // Without a referer the contracts page is the fallback.
    let response = request(&app, Method::POST, "/personal/contracts", Some(owner), None).await;
    assert!(response.status().is_redirection());
    assert_eq!(response.headers()[header::LOCATION], "/personal/contracts");
}
