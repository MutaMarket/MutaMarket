//! Behavior tests for donation ingestion against a mock ESI (the legacy
//! `GetWalletJournalJob` + `CreateDonationAction`): filtering to incoming
//! player donations, first-seen premium crediting with the confirmation
//! mail, character stubs for unknown donors, and idempotent re-runs.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::donations::{DonationSyncError, sync_wallet_donations};
use mutamarket::esi::EsiClient;
use serde_json::json;
use sqlx::PgPool;

/// The service character whose wallet is read (MutaMate's stand-in).
const SERVICE: i64 = 91_100_001;
/// A second service character for the token-less test, so the parallel
/// tests never touch each other's tokens.
const TOKENLESS_SERVICE: i64 = 91_100_004;
/// A donor with a user account (gets the confirmation mail).
const DONOR: i64 = 91_100_002;
/// A donor without any character row yet (gets a stub, no mail).
const STRANGER: i64 = 91_100_003;

const DONOR_JOURNAL_ID: i64 = 9_800_001;
const STRANGER_JOURNAL_ID: i64 = 9_800_004;
const JOURNAL_IDS: [i64; 4] = [DONOR_JOURNAL_ID, 9_800_002, 9_800_003, STRANGER_JOURNAL_ID];

/// Mock ESI: two wallet-journal pages with donations mixed among
/// non-donation and outgoing entries.
fn mock_esi() -> Router {
    #[derive(serde::Deserialize)]
    struct PageParam {
        page: u32,
    }

    Router::new().route(
        "/latest/characters/{character_id}/wallet/journal/",
        get(|Query(param): Query<PageParam>| async move {
            let body = if param.page == 1 {
                json!([
                    {
                        "id": DONOR_JOURNAL_ID,
                        "ref_type": "player_donation",
                        "amount": 150_000_000.0,
                        "date": "2026-08-01T12:00:00Z",
                        "first_party_id": DONOR,
                        "second_party_id": SERVICE,
                    },
                    {
                        "id": 9_800_002,
                        "ref_type": "market_transaction",
                        "amount": 500_000_000.0,
                        "date": "2026-08-01T13:00:00Z",
                        "first_party_id": DONOR,
                        "second_party_id": SERVICE,
                    },
                    {
                        "id": 9_800_003,
                        "ref_type": "player_donation",
                        "amount": -25_000_000.0,
                        "date": "2026-08-01T14:00:00Z",
                        "first_party_id": SERVICE,
                        "second_party_id": DONOR,
                    },
                ])
            } else {
                json!([
                    {
                        "id": STRANGER_JOURNAL_ID,
                        "ref_type": "player_donation",
                        "amount": 40_000_000.0,
                        "date": "2026-08-02T09:30:00Z",
                        "first_party_id": STRANGER,
                        "second_party_id": SERVICE,
                    },
                ])
            };
            ([("x-pages", "2")], Json(body))
        }),
    )
}

async fn start_mock(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ESI");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve mock ESI");
    });
    format!("http://{address}")
}

fn sso_stub(base: &str) -> SsoClient {
    SsoClient::new(base, "client", "secret", "http://test/eve/callback")
}

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Idempotent slate: the donations under test, the stranger stub, the
    // donor's account and premium state, the service token.
    sqlx::query("delete from donations where journal_id = any($1)")
        .bind(&JOURNAL_IDS[..])
        .execute(&pool)
        .await
        .expect("clean donations");
    sqlx::query("delete from characters where id = $1")
        .bind(STRANGER)
        .execute(&pool)
        .await
        .expect("clean stranger");
    sqlx::query("delete from users where name = 'Donation Tester'")
        .execute(&pool)
        .await
        .expect("clean user");

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Donation Tester') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    sqlx::query(
        "insert into characters (id, name, user_id) values ($1, 'Generous Donor', $2)
         on conflict (id) do update
         set name = excluded.name, user_id = excluded.user_id,
             premium_paid_until = null, premium_paid_total = 0, premium_payment_rest = 0",
    )
    .bind(DONOR)
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("seed donor");
    sqlx::query("delete from notification_outbox where user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("clean outbox");

    sqlx::query(
        "insert into characters (id, name) values ($1, 'MutaMate') on conflict (id) do nothing",
    )
    .bind(SERVICE)
    .execute(&pool)
    .await
    .expect("seed service character");
    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(SERVICE)
        .execute(&pool)
        .await
        .expect("clean tokens");
    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, 'wallet-access', 'refresh', 'Bearer', 'owner', $2,
                 now() + interval '20 minutes')",
    )
    .bind(SERVICE)
    .bind(vec![mutamarket::auth::scopes::READ_WALLET.to_owned()])
    .execute(&pool)
    .await
    .expect("seed token");

    pool
}

#[tokio::test]
async fn donations_are_recorded_once_and_credit_premium() {
    let pool = setup().await;
    let esi_url = start_mock(mock_esi()).await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    let stats = sync_wallet_donations(&pool, &esi, &sso, SERVICE)
        .await
        .expect("sync");
    assert_eq!((stats.entries, stats.donations, stats.created), (4, 2, 2));

    // The donor's donation row, confirmed.
    let (amount, confirmed): (f64, bool) =
        sqlx::query_as("select amount, confirmation_sent from donations where journal_id = $1")
            .bind(DONOR_JOURNAL_ID)
            .fetch_one(&pool)
            .await
            .expect("donor donation");
    assert_eq!(amount, 150_000_000.0);
    assert!(confirmed);

    // 150M buys one month (window-checked; the exact PHP month math is
    // pinned in the premium unit tests) and holds 50M.
    let (until_in_days, rest, total): (Option<f64>, f64, f64) = sqlx::query_as(
        "select (extract(epoch from premium_paid_until - now()) / 86400)::double precision,
                premium_payment_rest, premium_paid_total
         from characters where id = $1",
    )
    .bind(DONOR)
    .fetch_one(&pool)
    .await
    .expect("donor premium");
    let until_in_days = until_in_days.expect("premium granted");
    assert!(
        (27.0..=32.0).contains(&until_in_days),
        "one month from now, got {until_in_days} days",
    );
    assert_eq!(rest, 50_000_000.0);
    assert_eq!(total, 150_000_000.0);

    // The confirmation mail is queued for the donor's user.
    let (kind, subject, body): (String, String, String) = sqlx::query_as(
        "select o.kind, o.subject, o.body from notification_outbox o
         join users u on u.id = o.user_id
         where u.name = 'Donation Tester'",
    )
    .fetch_one(&pool)
    .await
    .expect("outbox row");
    assert_eq!(kind, "donation-received");
    assert_eq!(subject, "Donation Received - Thank You!");
    assert!(body.starts_with("Hello Generous Donor,"), "body: {body}");
    assert!(
        body.contains("your donation of 150,000,000 ISK"),
        "body: {body}"
    );
    assert!(
        body.contains("extended your premium status by 1 month until"),
        "body: {body}"
    );
    assert!(body.contains("holding 50,000,000 ISK"), "body: {body}");

    // The unknown donor got a stub character, a saved-up balance below a
    // month, and no mail (no user).
    let (name, until, rest): (String, Option<String>, f64) = sqlx::query_as(
        "select name, premium_paid_until::text, premium_payment_rest
         from characters where id = $1",
    )
    .bind(STRANGER)
    .fetch_one(&pool)
    .await
    .expect("stranger stub");
    assert_eq!(name, "", "stub rows have the default empty name");
    assert_eq!(until, None);
    assert_eq!(rest, 40_000_000.0);

    // Filtered entries never became donations.
    let filtered: i64 =
        sqlx::query_scalar("select count(*) from donations where journal_id in (9800002, 9800003)")
            .fetch_one(&pool)
            .await
            .expect("filtered count");
    assert_eq!(filtered, 0, "non-donations and outgoing ISK are skipped");

    // A re-run sees everything already recorded: no new rows, no second
    // credit, no second mail.
    let stats = sync_wallet_donations(&pool, &esi, &sso, SERVICE)
        .await
        .expect("re-run");
    assert_eq!((stats.donations, stats.created), (2, 0));
    let (rows, total, mails): (i64, f64, i64) = sqlx::query_as(
        "select (select count(*) from donations where journal_id = any($1)),
                (select premium_paid_total from characters where id = $2),
                (select count(*) from notification_outbox o
                 join users u on u.id = o.user_id where u.name = 'Donation Tester')",
    )
    .bind(&JOURNAL_IDS[..])
    .bind(DONOR)
    .fetch_one(&pool)
    .await
    .expect("re-run state");
    assert_eq!(rows, 2);
    assert_eq!(total, 150_000_000.0);
    assert_eq!(mails, 1);
}

/// The expiry sweep's cast: lapsed with a user, still live, and lapsed
/// without a user.
const EXPIRED_OWNED: i64 = 91_100_005;
const STILL_LIVE: i64 = 91_100_006;
const EXPIRED_ORPHAN: i64 = 91_100_007;

#[tokio::test]
async fn expired_premium_is_cleared_and_announced_only_for_owned_characters() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Leftover lapsed characters from other suites would inflate the
    // sweep counts; neutralize them so the assertions stay exact.
    sqlx::query("update characters set premium_paid_until = null where premium_paid_until < now()")
        .execute(&pool)
        .await
        .expect("neutralize stale premium");
    sqlx::query("delete from users where name = 'Expiry Tester'")
        .execute(&pool)
        .await
        .expect("clean user");
    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('Expiry Tester') returning id")
            .fetch_one(&pool)
            .await
            .expect("user");
    sqlx::query("delete from notification_outbox where user_id = $1")
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("clean outbox");

    for (id, name, owner, until) in [
        (
            EXPIRED_OWNED,
            "Lapsed Member",
            Some(user_id),
            "now() - interval '1 hour'",
        ),
        (
            STILL_LIVE,
            "Live Member",
            Some(user_id),
            "now() + interval '1 day'",
        ),
        (
            EXPIRED_ORPHAN,
            "Ownerless",
            None,
            "now() - interval '1 hour'",
        ),
    ] {
        sqlx::query(&format!(
            "insert into characters (id, name, user_id, premium_paid_until)
             values ($1, $2, $3, {until})
             on conflict (id) do update
             set name = excluded.name, user_id = excluded.user_id,
                 premium_paid_until = excluded.premium_paid_until",
        ))
        .bind(id)
        .bind(name)
        .bind(owner)
        .execute(&pool)
        .await
        .expect("seed character");
    }

    let costs = mutamarket::premium::PremiumCosts {
        monthly: mutamarket::premium::DEFAULT_MONTHLY_COST,
        yearly: mutamarket::premium::DEFAULT_YEARLY_COST,
    };
    let expired = mutamarket::premium::remove_expired_premium(&pool, costs)
        .await
        .expect("expiry sweep");
    assert_eq!(expired, 1);

    let states: Vec<(i64, bool)> = sqlx::query_as(
        "select id, premium_paid_until is null from characters
         where id = any($1) order by id",
    )
    .bind(vec![EXPIRED_OWNED, STILL_LIVE, EXPIRED_ORPHAN])
    .fetch_all(&pool)
    .await
    .expect("premium states");
    assert_eq!(
        states,
        vec![
            (EXPIRED_OWNED, true),
            (STILL_LIVE, false),
            // The legacy whereHas('user') quirk: nobody clears an
            // ownerless character's lapsed premium.
            (EXPIRED_ORPHAN, false),
        ],
    );

    let (kind, subject, body): (String, String, String) =
        sqlx::query_as("select kind, subject, body from notification_outbox where user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("expiry notice");
    assert_eq!(kind, "premium-expired");
    assert_eq!(subject, "Your premium subscription has expired");
    assert_eq!(
        body,
        "Hello Lapsed Member,\n\n\
         We just wanted to let you know that your premium account has expired, but don't \
         worry! You can still use all the features of the site, but you will not show up as \
         a premium member anymore.\n\n\
         If you want to renew your premium account, you can do so by sending 100,000,000 ISK \
         per month or 1,000,000,000 ISK for a full year (save 2 months!) to this \
         character.\n\n\
         Thank you for supporting us!\n\
         The MutaMarket team",
    );

    // A second sweep finds nothing new and sends nothing new.
    let expired = mutamarket::premium::remove_expired_premium(&pool, costs)
        .await
        .expect("second sweep");
    assert_eq!(expired, 0);
    let notices: i64 =
        sqlx::query_scalar("select count(*) from notification_outbox where user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("notice count");
    assert_eq!(notices, 1);
}

#[tokio::test]
async fn a_missing_wallet_token_fails_the_run() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    sqlx::query(
        "insert into characters (id, name) values ($1, 'Tokenless') on conflict (id) do nothing",
    )
    .bind(TOKENLESS_SERVICE)
    .execute(&pool)
    .await
    .expect("seed character");
    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(TOKENLESS_SERVICE)
        .execute(&pool)
        .await
        .expect("drop tokens");

    let esi_url = start_mock(mock_esi()).await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);

    let error = sync_wallet_donations(&pool, &esi, &sso, TOKENLESS_SERVICE)
        .await
        .expect_err("no token means no run");
    assert!(matches!(error, DonationSyncError::NoToken));
}
