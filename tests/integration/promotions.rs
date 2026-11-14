//! Behavior tests for promoted modules: a donation with a
//! `promote <item id>` reason buys days at the top of the for-sale
//! browser's default order, everything it cannot buy falls back to
//! premium, and a chosen sort, the public API and the other listings
//! keep their own order.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use axum::http::StatusCode;
use mutamarket::donations::create_donation;
use mutamarket::esi::EsiWalletJournalEntry;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::premium::{DEFAULT_MONTHLY_COST, DEFAULT_YEARLY_COST, PremiumCosts};
use mutamarket::promotions::DAILY_COST;
use serde_json::json;
use sqlx::PgPool;

/// 50MN Abyssal Microwarpdrive, the fixture type every test lists.
const MWD_TYPE_ID: i64 = 47408;

/// The service character donations go to (MutaMate's stand-in).
const SERVICE: i64 = 91_200_001;
/// A donor with a user account (gets the confirmation mail).
const DONOR: i64 = 91_200_002;

const COSTS: PremiumCosts = PremiumCosts {
    monthly: DEFAULT_MONTHLY_COST,
    yearly: DEFAULT_YEARLY_COST,
};

/// Seconds in a day, for reading a promotion's end as days from now.
const DAY_SECONDS: f64 = 86_400.0;

/// An incoming donation as ESI's wallet journal reports it.
fn donation(journal_id: i64, amount: f64, reason: &str) -> EsiWalletJournalEntry {
    serde_json::from_value(json!({
        "id": journal_id,
        "ref_type": "player_donation",
        "amount": amount,
        "date": "2026-11-14T12:00:00Z",
        "first_party_id": DONOR,
        "second_party_id": SERVICE,
        "reason": reason,
    }))
    .expect("journal entry")
}

/// Four for-sale MWDs (three contracts and one public listing) plus an
/// unlisted fifth, and the donor's account. Returns the module ids in
/// that order, oldest (lowest id) first, so the contracts get dearer
/// with the id: 100M, 200M and 300M.
async fn setup(pool: &PgPool) -> [i64; 5] {
    let reference = common::db::reference_data();
    let fixtures = common::load_module_fixtures();
    let mwd = fixtures
        .iter()
        .find(|fixture| fixture.type_id == MWD_TYPE_ID)
        .expect("MWD fixture");
    let mut ids = [0; 5];
    for (slot, module) in ids.iter_mut().zip(&mwd.modules) {
        *slot = module.module_id;
    }
    ids.sort_unstable();
    for module in &mwd.modules[..ids.len()] {
        process_module(
            pool,
            &reference,
            &mutamarket::estimator::Estimator::new(),
            MWD_TYPE_ID,
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
    }
    for (index, module_id) in ids[..3].iter().enumerate() {
        let contract_id = 810_001 + index as i64;
        let price = 100_000_000.0 * (index as f64 + 1.0);
        common::attach_contract(
            pool,
            *module_id,
            contract_id,
            "item_exchange",
            price,
            1,
            0,
            0,
        )
        .await;
    }
    common::publish_asset(pool, ids[3], MWD_TYPE_ID).await;

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Promotion Tester') returning id")
            .fetch_one(pool)
            .await
            .expect("user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Keen Seller', $2)")
        .bind(DONOR)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed donor");
    ids
}

/// The promotion's end in days from now, `None` when the module has none.
async fn promoted_days(pool: &PgPool, module_id: i64) -> Option<f64> {
    sqlx::query_scalar(
        "select (extract(epoch from promoted_until - now()) / $2)::double precision
         from modules where id = $1",
    )
    .bind(module_id)
    .bind(DAY_SECONDS)
    .fetch_one(pool)
    .await
    .expect("module row")
}

/// The donor's premium balance: what was credited in total and what is
/// held toward the next month.
async fn premium_balance(pool: &PgPool) -> (f64, f64) {
    sqlx::query_as("select premium_paid_total, premium_payment_rest from characters where id = $1")
        .bind(DONOR)
        .fetch_one(pool)
        .await
        .expect("donor row")
}

async fn mail_bodies(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "select o.body from notification_outbox o join users u on u.id = o.user_id
         where u.name = 'Promotion Tester' order by o.id",
    )
    .fetch_all(pool)
    .await
    .expect("outbox")
}

async fn card_ids(app: &axum::Router, path: &str) -> (Vec<i64>, serde_json::Value) {
    let response = common::http::request("GET", path).send(app).await;
    assert_eq!(response.status, StatusCode::OK, "{path}");
    let body = response.json();
    let ids = body
        .as_array()
        .expect("bare card array")
        .iter()
        .map(|card| card["id"].as_i64().expect("card id"))
        .collect();
    (ids, body)
}

fn sorted_desc(ids: &[i64]) -> Vec<i64> {
    let mut ids = ids.to_vec();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    ids
}

/// Within an allowed drift of the seconds a test takes.
fn close_to(days: Option<f64>, expected: f64) -> bool {
    days.is_some_and(|days| (days - expected).abs() < 0.01)
}

#[tokio::test]
async fn a_promote_reason_buys_days_and_the_rest_is_premium() {
    let (pool, _db) = common::db::fresh().await;
    let ids = setup(&pool).await;
    let module_id = ids[0];

    // Three days and 10M over.
    let amount = DAILY_COST * 3.0 + 10_000_000.0;
    let entry = donation(9_810_001, amount, &format!("promote {module_id}"));
    assert!(create_donation(&pool, &entry, COSTS).await.expect("create"));

    assert!(
        close_to(promoted_days(&pool, module_id).await, 3.0),
        "three days from now"
    );
    let record: Vec<(i64, i32)> = sqlx::query_as(
        "select p.module_id, p.days from module_promotions p
         join donations d on d.id = p.donation_id where d.journal_id = 9810001",
    )
    .fetch_all(&pool)
    .await
    .expect("promotion rows");
    assert_eq!(record, [(module_id, 3)]);

    // Only the 10M above the three days reached premium.
    assert_eq!(premium_balance(&pool).await, (10_000_000.0, 10_000_000.0));

    let bodies = mail_bodies(&pool).await;
    assert_eq!(bodies.len(), 1);
    assert!(
        bodies[0].contains(&format!(
            "Your donation promotes module {module_id} for 3 days, until "
        )),
        "body: {}",
        bodies[0]
    );
    assert!(
        bodies[0].contains("The 10,000,000 ISK above its price went to premium."),
        "body: {}",
        bodies[0]
    );

    // The same journal entry again changes nothing.
    assert!(!create_donation(&pool, &entry, COSTS).await.expect("rerun"));
    assert!(close_to(promoted_days(&pool, module_id).await, 3.0));

    // A second donation extends the running promotion, and an exact
    // amount leaves premium alone.
    let entry = donation(
        9_810_002,
        DAILY_COST * 2.0,
        &format!("Promote  {module_id} "),
    );
    assert!(create_donation(&pool, &entry, COSTS).await.expect("extend"));
    assert!(
        close_to(promoted_days(&pool, module_id).await, 5.0),
        "extended to five days"
    );
    assert_eq!(premium_balance(&pool).await, (10_000_000.0, 10_000_000.0));
    let bodies = mail_bodies(&pool).await;
    assert_eq!(bodies.len(), 2);
    assert!(
        !bodies[1].contains("premium"),
        "an exact amount has no premium part: {}",
        bodies[1]
    );

    // The item id alone works as the reason too, and so does the
    // module's link.
    let entry = donation(9_810_005, DAILY_COST, &module_id.to_string());
    assert!(
        create_donation(&pool, &entry, COSTS)
            .await
            .expect("bare id")
    );
    assert!(
        close_to(promoted_days(&pool, module_id).await, 6.0),
        "extended to six days"
    );

    let link = format!("https://mutamarket.com/modules/50mn-abyssal-microwarpdrive-{module_id}");
    let entry = donation(9_810_004, DAILY_COST, &link);
    assert!(create_donation(&pool, &entry, COSTS).await.expect("link"));
    assert!(
        close_to(promoted_days(&pool, module_id).await, 7.0),
        "extended to seven days"
    );

    // A lapsed promotion restarts from now instead of from its old end.
    sqlx::query("update modules set promoted_until = now() - interval '10 days' where id = $1")
        .bind(module_id)
        .execute(&pool)
        .await
        .expect("lapse");
    let entry = donation(9_810_003, DAILY_COST, &format!("promote {module_id}"));
    assert!(
        create_donation(&pool, &entry, COSTS)
            .await
            .expect("restart")
    );
    assert!(
        close_to(promoted_days(&pool, module_id).await, 1.0),
        "one day from now"
    );
}

#[tokio::test]
async fn a_code_that_buys_nothing_is_an_ordinary_donation() {
    let (pool, _db) = common::db::fresh().await;
    let ids = setup(&pool).await;
    let (listed, unlisted) = (ids[0], ids[4]);

    // Less than a day, a module nobody sells, an unknown module and a
    // reason that is no code: all of it goes to premium.
    let below_a_day = DAILY_COST - 1.0;
    let cases = [
        (9_820_001, below_a_day, format!("promote {listed}")),
        (9_820_002, DAILY_COST, format!("promote {unlisted}")),
        (9_820_003, DAILY_COST, "promote 1".to_owned()),
        (9_820_004, DAILY_COST, format!("please promote {listed}")),
        (9_820_006, DAILY_COST, "1".to_owned()),
        (9_820_007, DAILY_COST, "thanks for the site".to_owned()),
    ];
    let mut total = 0.0;
    for (journal_id, amount, reason) in cases {
        let entry = donation(journal_id, amount, &reason);
        assert!(create_donation(&pool, &entry, COSTS).await.expect("create"));
        total += amount;
        assert_eq!(
            premium_balance(&pool).await.0,
            total,
            "reason {reason:?} credits premium in full"
        );
    }

    assert_eq!(promoted_days(&pool, listed).await, None);
    assert_eq!(promoted_days(&pool, unlisted).await, None);
    let promotions: i64 = sqlx::query_scalar("select count(*) from module_promotions")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(promotions, 0);
    for body in mail_bodies(&pool).await {
        assert!(!body.contains("promotes module"), "body: {body}");
    }

    // A module sold through a public listing instead of a contract can
    // be promoted too.
    let public = ids[3];
    let entry = donation(9_820_005, DAILY_COST, &format!("promote {public}"));
    assert!(create_donation(&pool, &entry, COSTS).await.expect("create"));
    assert!(close_to(promoted_days(&pool, public).await, 1.0));
}

#[tokio::test]
async fn promoted_modules_lead_the_default_order_only() {
    let (pool, _db) = common::db::fresh().await;
    let ids = setup(&pool).await;
    let for_sale = sorted_desc(&ids[..4]);
    let app = mutamarket::server::test_router(pool.clone()).await;
    let cards_path = format!("/api/module-cards/type/{MWD_TYPE_ID}");

    // Nothing promoted: newest first, no card carries the key.
    let (order, body) = card_ids(&app, &cards_path).await;
    assert_eq!(order, for_sale);
    for card in body.as_array().expect("cards") {
        common::assert_default_module_keys(card, false, &[]);
    }

    // Promote the two oldest listings and let a third promotion lapse.
    let (oldest, second_oldest, lapsed) = (for_sale[3], for_sale[2], for_sale[1]);
    sqlx::query("update modules set promoted_until = now() + interval '2 days' where id = any($1)")
        .bind(vec![oldest, second_oldest])
        .execute(&pool)
        .await
        .expect("promote");
    sqlx::query("update modules set promoted_until = now() - interval '1 minute' where id = $1")
        .bind(lapsed)
        .execute(&pool)
        .await
        .expect("lapse");

    // The promoted pair leads, newest first, then the rest; only the
    // pair carries `promoted_until`.
    let expected = vec![second_oldest, oldest, for_sale[0], lapsed];
    let (order, body) = card_ids(&app, &cards_path).await;
    assert_eq!(order, expected);
    for card in body.as_array().expect("cards") {
        let promoted = [oldest, second_oldest].contains(&card["id"].as_i64().expect("id"));
        let extra: &[&str] = if promoted { &["promoted_until"] } else { &[] };
        common::assert_default_module_keys(card, false, extra);
        if promoted {
            let until = card["promoted_until"].as_str().expect("end date");
            assert!(
                until.len() == 20 && until.ends_with('Z'),
                "an ISO UTC timestamp, got {until}"
            );
        }
    }

    // The lead is made of matches only: under this price bound the
    // 100M contract is the one promoted module left, ahead of nothing.
    let path = format!("{cards_path}/contract-price/100000000/contracts-only");
    let (order, _) = card_ids(&app, &path).await;
    assert_eq!(order, [oldest]);
    // With all three contracts matching, the promoted pair leads the
    // dearest one, which is newer than both.
    let path = format!("{cards_path}/contract-price/300000000/contracts-only");
    let (order, _) = card_ids(&app, &path).await;
    assert_eq!(order, [second_oldest, oldest, lapsed]);

    // Any chosen sort is the visitor's order: no lead, no key.
    for (sort, expected) in [
        ("sort/price/asc", vec![ids[0], ids[1], ids[2]]),
        ("sort/price/desc", vec![ids[2], ids[1], ids[0]]),
    ] {
        let path = format!("{cards_path}/contracts-only/{sort}");
        let (order, body) = card_ids(&app, &path).await;
        assert_eq!(order, expected, "{sort}");
        for card in body.as_array().expect("cards") {
            common::assert_default_module_keys(card, false, &[]);
        }
    }

    // The all-modules page and the public API keep the plain order.
    let (order, body) = card_ids(&app, &format!("{cards_path}?unlisted=true")).await;
    assert_eq!(order, sorted_desc(&ids));
    for card in body.as_array().expect("cards") {
        common::assert_default_module_keys(card, false, &[]);
    }
    let response = common::http::request("GET", &format!("/api/modules/type/{MWD_TYPE_ID}"))
        .send(&app)
        .await;
    assert_eq!(response.status, StatusCode::OK);
    let api_order: Vec<i64> = response.json()["data"]
        .as_array()
        .expect("data")
        .iter()
        .map(|module| module["id"].as_i64().expect("id"))
        .collect();
    assert_eq!(api_order, for_sale);
    assert!(
        response.json()["data"]
            .as_array()
            .expect("data")
            .iter()
            .all(|module| module.get("promoted_until").is_none()),
        "the public API resource has no promotion key"
    );
}

#[tokio::test]
async fn promoted_and_ordinary_modules_page_as_one_list() {
    let (pool, _db) = common::db::fresh().await;
    let ids = setup(&pool).await;
    let for_sale = sorted_desc(&ids[..4]);
    let (oldest, second_oldest) = (for_sale[3], for_sale[2]);
    sqlx::query("update modules set promoted_until = now() + interval '2 days' where id = any($1)")
        .bind(vec![oldest, second_oldest])
        .execute(&pool)
        .await
        .expect("promote");

    let search = |page: i64| {
        let pool = pool.clone();
        async move {
            let reference = common::db::reference_data();
            let query = format!("type/{MWD_TYPE_ID}/page/{page}");
            let search = mutamarket::modules::search::parse(&pool, &reference, &query)
                .await
                .expect("parse");
            // Pages of three, so the first one mixes both halves.
            mutamarket::modules::search::module_ids(
                &pool,
                &search,
                mutamarket::modules::search::Visibility::ForSale,
                None,
                3,
            )
            .await
            .expect("page")
        }
    };
    assert_eq!(search(1).await, [second_oldest, oldest, for_sale[0]]);
    assert_eq!(search(2).await, [for_sale[1]]);
    assert_eq!(search(3).await, Vec::<i64>::new());

    // With every listing promoted, the pages are the promoted list alone.
    sqlx::query("update modules set promoted_until = now() + interval '2 days' where id = any($1)")
        .bind(&for_sale)
        .execute(&pool)
        .await
        .expect("promote all");
    assert_eq!(search(1).await, for_sale[..3]);
    assert_eq!(search(2).await, for_sale[3..]);
}

#[tokio::test]
async fn the_module_page_carries_a_running_promotion() {
    let (pool, _db) = common::db::fresh().await;
    let ids = setup(&pool).await;
    let app = mutamarket::server::test_router(pool.clone()).await;
    let path = format!("/api/module-page/{}", ids[0]);

    let body = common::http::request("GET", &path).send(&app).await.json();
    common::assert_default_module_keys(&body["module"], false, &[]);

    sqlx::query("update modules set promoted_until = '2099-01-02 03:04:05+00' where id = $1")
        .bind(ids[0])
        .execute(&pool)
        .await
        .expect("promote");
    let body = common::http::request("GET", &path).send(&app).await.json();
    common::assert_default_module_keys(&body["module"], false, &["promoted_until"]);
    assert_eq!(body["module"]["promoted_until"], "2099-01-02T03:04:05Z");
}
