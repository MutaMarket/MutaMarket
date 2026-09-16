//! Behavior tests for the relist-based failure inference: which archived
//! contracts a later same-seller listing proves failed, which statuses
//! it refuses to overwrite, and that a second run is a no-op.
//!
//! The pass sweeps the whole contract history, so a run here also
//! rewrites relisted contracts other suites seeded. Those suites clean
//! and re-seed the rows they assert about, and the suites never
//! interleave (`RUST_TEST_THREADS=1`), so a flip lands between their
//! runs, never inside one.
//!
//! Needs the local database: `docker compose up -d postgres`.

use mutamarket::contracts::relists::infer_failed_relists;
use mutamarket::db;
use sqlx::PgPool;

const REGION: i64 = 10_000_002;
/// The relisting seller, and the second seller of the buy-back chain.
const SELLER: i64 = 990_009_901;
const OTHER_SELLER: i64 = 990_009_902;
/// A type of this suite's own, so the fixtures never touch a reference
/// row other suites read.
const TYPE_ID: i64 = 990_009_100;

const MODULE_BASE: i64 = 990_009_000;
const CONTRACT_BASE: i64 = 990_009_500;

/// Module 0: listed unknown, then relisted by the same seller.
const RELISTED: i64 = CONTRACT_BASE;
const RELISTED_AGAIN: i64 = CONTRACT_BASE + 1;
/// Module 1: listed unknown, then listed by someone else, which proves
/// nothing (they may well have bought it).
const SOLD_ON: i64 = CONTRACT_BASE + 2;
const SOLD_ON_NEXT: i64 = CONTRACT_BASE + 3;
/// Module 2: ESI saw the acceptance, the seller listed it again anyway.
const BOUGHT_BACK: i64 = CONTRACT_BASE + 4;
const BOUGHT_BACK_AGAIN: i64 = CONTRACT_BASE + 5;
/// Module 3: seller -> other seller -> seller. Only the middle contract
/// is followed by its own seller's listing.
const CHAIN_FIRST: i64 = CONTRACT_BASE + 6;
const CHAIN_MIDDLE: i64 = CONTRACT_BASE + 7;
const CHAIN_LAST: i64 = CONTRACT_BASE + 8;
/// Module 4: the relist is still running, so it sits in `contracts`.
const RELISTED_LIVE: i64 = CONTRACT_BASE + 9;
const LIVE_CONTRACT: i64 = CONTRACT_BASE + 10;

/// The fixtures carry a second, non-abyssal item: that is the case the
/// ESI probe is never asked about, so it is where the `unknown` backlog
/// comes from, and it keeps these contracts out of the moderator review
/// pool other suites assert on. The rule itself ignores item counts.
///
/// (contract, module offset, issuer, status, days ago).
const ARCHIVED: [(i64, i64, i64, &str, i32); 9] = [
    (RELISTED, 0, SELLER, "unknown", 30),
    (RELISTED_AGAIN, 0, SELLER, "unknown", 20),
    (SOLD_ON, 1, SELLER, "unknown", 30),
    (SOLD_ON_NEXT, 1, OTHER_SELLER, "unknown", 20),
    (BOUGHT_BACK, 2, SELLER, "completed", 30),
    (BOUGHT_BACK_AGAIN, 2, SELLER, "unknown", 20),
    (CHAIN_FIRST, 3, SELLER, "unknown", 30),
    (CHAIN_MIDDLE, 3, OTHER_SELLER, "unknown", 20),
    (CHAIN_LAST, 3, SELLER, "unknown", 10),
];

async fn seed(pool: &PgPool) {
    sqlx::query("delete from historic_contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_BASE)
        .execute(pool)
        .await
        .expect("clean historic contracts");
    sqlx::query("delete from contracts where id >= $1 and id < $1 + 100")
        .bind(CONTRACT_BASE)
        .execute(pool)
        .await
        .expect("clean contracts");
    sqlx::query("delete from modules where id >= $1 and id < $1 + 100")
        .bind(MODULE_BASE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![SELLER, OTHER_SELLER])
        .execute(pool)
        .await
        .expect("clean sellers");

    sqlx::query("insert into regions (id, name) values ($1, 'The Forge') on conflict do nothing")
        .bind(REGION)
        .execute(pool)
        .await
        .expect("seed region");
    sqlx::query(
        "insert into types (id, name, published) values ($1, 'Relist Fixture Module', false)
         on conflict (id) do nothing",
    )
    .bind(TYPE_ID)
    .execute(pool)
    .await
    .expect("seed type");
    for (id, name) in [(SELLER, "Relist Seller"), (OTHER_SELLER, "Other Seller")] {
        sqlx::query("insert into characters (id, name) values ($1, $2)")
            .bind(id)
            .bind(name)
            .execute(pool)
            .await
            .expect("seed character");
    }
    for offset in 0..5 {
        sqlx::query("insert into modules (id, type_id) values ($1, $2)")
            .bind(MODULE_BASE + offset)
            .bind(TYPE_ID)
            .execute(pool)
            .await
            .expect("seed module");
    }

    for (contract_id, module_offset, issuer, status, days_ago) in ARCHIVED {
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, date_issued, unified_price,
                  abyssal_modules_count, non_abyssal_modules_count)
             values ($1, $2, $3, $4, 'item_exchange',
                     now() - make_interval(days => $5), 5000000, 1, 1)",
        )
        .bind(contract_id)
        .bind(status)
        .bind(REGION)
        .bind(issuer)
        .bind(days_ago)
        .execute(pool)
        .await
        .expect("seed historic contract");
        sqlx::query(
            "insert into historic_contract_items
                 (historic_contract_id, record_id, type_id, item_id)
             values ($1, 1, $2, $3)",
        )
        .bind(contract_id)
        .bind(TYPE_ID)
        .bind(MODULE_BASE + module_offset)
        .execute(pool)
        .await
        .expect("seed historic contract item");
    }

    // Module 4: the archived listing and the running relist that
    // supersedes it.
    sqlx::query(
        "insert into historic_contracts
             (id, status, region_id, issuer_id, type, date_issued, unified_price,
              abyssal_modules_count, non_abyssal_modules_count)
         values ($1, 'unknown', $2, $3, 'item_exchange',
                 now() - interval '30 days', 5000000, 1, 1)",
    )
    .bind(RELISTED_LIVE)
    .bind(REGION)
    .bind(SELLER)
    .execute(pool)
    .await
    .expect("seed historic contract");
    sqlx::query(
        "insert into historic_contract_items (historic_contract_id, record_id, type_id, item_id)
         values ($1, 1, $2, $3)",
    )
    .bind(RELISTED_LIVE)
    .bind(TYPE_ID)
    .bind(MODULE_BASE + 4)
    .execute(pool)
    .await
    .expect("seed historic contract item");
    sqlx::query(
        "insert into contracts
             (id, region_id, issuer_id, type, date_issued, unified_price, abyssal_modules_count)
         values ($1, $2, $3, 'item_exchange', now() - interval '2 days', 4000000, 1)",
    )
    .bind(LIVE_CONTRACT)
    .bind(REGION)
    .bind(SELLER)
    .execute(pool)
    .await
    .expect("seed live contract");
    sqlx::query(
        "insert into contract_items (contract_id, record_id, type_id, item_id)
         values ($1, 1, $2, $3)",
    )
    .bind(LIVE_CONTRACT)
    .bind(TYPE_ID)
    .bind(MODULE_BASE + 4)
    .execute(pool)
    .await
    .expect("seed live contract item");
}

/// (status, relisted_by_contract_id) of the suite's contracts, in id
/// order.
async fn outcomes(pool: &PgPool) -> Vec<(i64, String, Option<i64>)> {
    sqlx::query_as(
        "select id, status, relisted_by_contract_id from historic_contracts
         where id >= $1 and id < $1 + 100 order by id",
    )
    .bind(CONTRACT_BASE)
    .fetch_all(pool)
    .await
    .expect("read outcomes")
}

#[tokio::test]
async fn a_relist_by_the_same_seller_proves_the_earlier_contract_failed() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    // The sweep is global and the test database outlives a run, so
    // relists other suites left behind are absorbed first; what the
    // asserted run reports is then this suite's fixtures alone.
    infer_failed_relists(&pool).await.expect("inference runs");
    seed(&pool).await;

    let stats = infer_failed_relists(&pool).await.expect("inference runs");

    assert_eq!(
        outcomes(&pool).await,
        [
            // Relisted by its own seller: proven failed, pointing at the
            // listing that proves it.
            (RELISTED, "failed".to_owned(), Some(RELISTED_AGAIN)),
            // The last listing of a module has nothing after it.
            (RELISTED_AGAIN, "unknown".to_owned(), None),
            // A different seller listed it next: they may have bought it.
            (SOLD_ON, "unknown".to_owned(), None),
            (SOLD_ON_NEXT, "unknown".to_owned(), None),
            // ESI watched this one get accepted; the relist is evidence
            // of a buy-back, not a reason to overrule the probe.
            (BOUGHT_BACK, "completed".to_owned(), Some(BOUGHT_BACK_AGAIN)),
            (BOUGHT_BACK_AGAIN, "unknown".to_owned(), None),
            // seller -> other seller -> seller: only the middle listing
            // is followed by its own seller's.
            (CHAIN_FIRST, "unknown".to_owned(), None),
            (CHAIN_MIDDLE, "unknown".to_owned(), None),
            (CHAIN_LAST, "unknown".to_owned(), None),
            // The relist is still running, so the proof sits in
            // `contracts` rather than the archive.
            (RELISTED_LIVE, "failed".to_owned(), Some(LIVE_CONTRACT)),
        ],
    );
    assert_eq!(
        (stats.resolved, stats.conflicting, stats.confirmed),
        (2, 1, 0),
        "two unknowns resolved, one accepted contract flagged"
    );

    // Idempotent: the same data proves nothing new.
    let again = infer_failed_relists(&pool).await.expect("inference runs");
    assert_eq!(
        (again.resolved, again.conflicting, again.confirmed),
        (0, 0, 0)
    );
}
