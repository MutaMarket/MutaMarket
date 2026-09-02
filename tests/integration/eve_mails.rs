//! Behavior tests for the EVE mail ingestion against a mock ESI: the
//! legacy get-mails chain ported — headers stored with recipients, new
//! mail bodies fetched and their abyssal module links imported and
//! linked, mails marked read in-game, appraisal replies queued through
//! the notification outbox, and detail failures retried on the next
//! scan.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::esi::EsiClient;
use mutamarket::mails::{MAIL_REPLY_KIND, sync_eve_mails};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use sqlx::PgPool;

const SERVICE: i64 = 93_200_001;
const TOKENLESS: i64 = 93_200_003;
const PLAYER: i64 = 93_200_002;
const MAIL_NEW: i64 = 700_001;
const MAIL_READ: i64 = 700_002;
const MAIL_BROKEN: i64 = 700_003;
/// A non-abyssal type in a link: filtered out before any import.
const TRITANIUM_TYPE: i64 = 34;
/// A mailing-list recipient id: never becomes a character stub.
const MAILING_LIST: i64 = 145_678;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

struct MockState {
    detail_calls: Mutex<HashMap<i64, usize>>,
    read_puts: Mutex<Vec<i64>>,
    broken_healed: AtomicBool,
}

/// Mock ESI: headers, per-mail details (the broken one 500s until
/// healed), the read-flag PUT and the dynamic items of two fixture
/// modules.
fn mock_esi(
    state: Arc<MockState>,
    new_module: serde_json::Value,
    read_module: serde_json::Value,
) -> Router {
    let details_state = state.clone();
    let new_detail = new_module.clone();
    let read_detail = read_module.clone();

    Router::new()
        .route(
            "/latest/characters/{character_id}/mail/",
            get(|| async {
                Json(json!([
                    {
                        "mail_id": MAIL_NEW,
                        "from": PLAYER,
                        "subject": "Appraise these please",
                        "timestamp": "2026-08-28T09:15:00Z",
                        "is_read": false,
                        "recipients": [
                            { "recipient_id": SERVICE, "recipient_type": "character" },
                            { "recipient_id": MAILING_LIST, "recipient_type": "mailing_list" },
                        ],
                    },
                    {
                        "mail_id": MAIL_READ,
                        "from": PLAYER,
                        "subject": "Old mail",
                        "timestamp": "2026-08-27T08:00:00Z",
                        "is_read": true,
                        "recipients": [
                            { "recipient_id": SERVICE, "recipient_type": "character" },
                        ],
                    },
                    {
                        "mail_id": MAIL_BROKEN,
                        "from": PLAYER,
                        "subject": "Flaky mail",
                        "timestamp": "2026-08-28T10:00:00Z",
                        "is_read": false,
                        "recipients": [
                            { "recipient_id": SERVICE, "recipient_type": "character" },
                        ],
                    },
                ]))
            }),
        )
        .route(
            "/latest/characters/{character_id}/mail/{mail_id}/",
            get(move |AxumPath((_, mail_id)): AxumPath<(i64, i64)>| {
                let state = details_state.clone();
                let new_detail = new_detail.clone();
                let read_detail = read_detail.clone();
                async move {
                    *state
                        .detail_calls
                        .lock()
                        .expect("calls")
                        .entry(mail_id)
                        .or_insert(0) += 1;
                    match mail_id {
                        MAIL_NEW => Json(json!({
                            "from": PLAYER,
                            "subject": "Appraise these please",
                            "timestamp": "2026-08-28T09:15:00Z",
                            "read": false,
                            "body": format!(
                                "Hi, <a href=\"showinfo:{}//{}\">this one</a> and \
                                 <a href=\"showinfo:{TRITANIUM_TYPE}//42\">some trit</a>",
                                new_detail["type_id"].as_i64().expect("type"),
                                new_detail["item_id"].as_i64().expect("item"),
                            ),
                            "recipients": [
                                { "recipient_id": SERVICE, "recipient_type": "character" },
                            ],
                        }))
                        .into_response(),
                        MAIL_READ => Json(json!({
                            "from": PLAYER,
                            "subject": "Old mail",
                            "timestamp": "2026-08-27T08:00:00Z",
                            "read": true,
                            "body": format!(
                                "<a href=\"showinfo:{}//{}\">already seen</a>",
                                read_detail["type_id"].as_i64().expect("type"),
                                read_detail["item_id"].as_i64().expect("item"),
                            ),
                            "recipients": [
                                { "recipient_id": SERVICE, "recipient_type": "character" },
                            ],
                        }))
                        .into_response(),
                        MAIL_BROKEN if state.broken_healed.load(Ordering::SeqCst) => Json(json!({
                            "from": PLAYER,
                            "subject": "Flaky mail",
                            "timestamp": "2026-08-28T10:00:00Z",
                            "read": false,
                            "body": "no links here",
                            "recipients": [],
                        }))
                        .into_response(),
                        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
            })
            .put(move |AxumPath((_, mail_id)): AxumPath<(i64, i64)>| {
                let state = state.clone();
                async move {
                    state.read_puts.lock().expect("puts").push(mail_id);
                    StatusCode::NO_CONTENT
                }
            }),
        )
        .route(
            "/latest/dogma/dynamic/items/{type_id}/{item_id}/",
            get(move |AxumPath((type_id, item_id)): AxumPath<(i64, i64)>| {
                let new_module = new_module.clone();
                let read_module = read_module.clone();
                async move {
                    for module in [&new_module, &read_module] {
                        if module["type_id"] == json!(type_id)
                            && module["item_id"] == json!(item_id)
                        {
                            return Json(module["dogma"].clone()).into_response();
                        }
                    }
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
}

fn dogma_payload(fixture_type_id: i64, module: &common::ModuleFixture) -> serde_json::Value {
    json!({
        "type_id": fixture_type_id,
        "item_id": module.module_id,
        "dogma": {
            "created_by": module.creator_id,
            "mutator_type_id": module.mutaplasmid_id,
            "source_type_id": module.source_type_id,
            "dogma_attributes": module
                .input_attributes
                .iter()
                .map(|attribute| json!({
                    "attribute_id": attribute.attribute_id,
                    "value": attribute.value,
                }))
                .collect::<Vec<_>>(),
        },
    })
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

async fn seed_character(pool: &PgPool, character_id: i64, name: &str, scopes: &[&str]) {
    sqlx::query(
        "insert into characters (id, name) values ($1, $2)
         on conflict (id) do update set name = excluded.name",
    )
    .bind(character_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("seed character");

    sqlx::query("delete from esi_tokens where character_id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("clean tokens");
    if scopes.is_empty() {
        return;
    }
    sqlx::query(
        "insert into esi_tokens
         (character_id, access_token, refresh_token, token_type, character_owner_hash, scopes,
          expires_at)
         values ($1, 'mail-access', 'refresh', 'Bearer', 'owner', $2,
                 now() + interval '20 minutes')",
    )
    .bind(character_id)
    .bind(
        scopes
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect::<Vec<_>>(),
    )
    .execute(pool)
    .await
    .expect("seed token");
}

fn sso_stub(base: &str) -> SsoClient {
    SsoClient::new(base, "client", "secret", "http://test/eve/callback")
}

#[tokio::test]
async fn the_inbox_scan_ingests_links_replies_and_retries() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    // Idempotent across runs.
    sqlx::query("delete from eve_mails where id = any($1)")
        .bind(vec![MAIL_NEW, MAIL_READ, MAIL_BROKEN])
        .execute(&pool)
        .await
        .expect("clean mails");
    sqlx::query("delete from notification_outbox where kind = $1")
        .bind(MAIL_REPLY_KIND)
        .execute(&pool)
        .await
        .expect("clean outbox");
    seed_character(
        &pool,
        SERVICE,
        "MutaMate",
        &["esi-mail.read_mail.v1", "esi-mail.organize_mail.v1"],
    )
    .await;
    seed_character(&pool, PLAYER, "Mail Sender", &[]).await;
    seed_character(&pool, TOKENLESS, "No Token", &[]).await;

    let fixtures = common::load_module_fixtures();
    let new_fixture = fixtures
        .iter()
        .find(|f| f.type_id == 47736)
        .expect("fixture");
    let read_fixture = fixtures
        .iter()
        .find(|f| f.type_id == 47740)
        .expect("fixture");
    let new_module = &new_fixture.modules[0];
    let read_module = &read_fixture.modules[0];

    let state = Arc::new(MockState {
        detail_calls: Mutex::new(HashMap::new()),
        read_puts: Mutex::new(Vec::new()),
        broken_healed: AtomicBool::new(false),
    });
    let esi_url = start_mock(mock_esi(
        state.clone(),
        dogma_payload(new_fixture.type_id, new_module),
        dogma_payload(read_fixture.type_id, read_module),
    ))
    .await;
    let esi = EsiClient::new(&esi_url);
    let sso = sso_stub(&esi_url);
    let estimator = estimator_stub();

    // Without a mail-read token the sync reports itself skipped.
    let skipped = sync_eve_mails(
        &pool,
        &reference,
        &esi,
        &sso,
        &estimator,
        TOKENLESS,
        true,
        |_line| {},
    )
    .await
    .expect("tokenless sync");
    assert!(skipped.is_none(), "no token means a skip, not an error");

    let progress_lines = AtomicUsize::new(0);
    let stats = sync_eve_mails(
        &pool,
        &reference,
        &esi,
        &sso,
        &estimator,
        SERVICE,
        true,
        |_line| {
            progress_lines.fetch_add(1, Ordering::Relaxed);
        },
    )
    .await
    .expect("first scan")
    .expect("token present");
    assert_eq!(
        (
            stats.mails,
            stats.new,
            stats.modules,
            stats.replies,
            stats.failed
        ),
        (3, 2, 2, 1, 1),
        "two mails process (one already read in-game), the broken detail fails",
    );
    assert_eq!(
        progress_lines.load(Ordering::Relaxed),
        3,
        "one progress line per mail"
    );

    // The new mail: full detail stored, marked read locally and on ESI.
    let (subject, timestamp, is_read, sender, body): (String, String, bool, i64, Option<String>) =
        sqlx::query_as(
            "select subject, timestamp::text, is_read, character_id, body
             from eve_mails where id = $1",
        )
        .bind(MAIL_NEW)
        .fetch_one(&pool)
        .await
        .expect("new mail row");
    assert_eq!(subject, "Appraise these please");
    assert_eq!(timestamp, "2026-08-28 09:15:00+00");
    assert!(is_read, "processed unread mail is marked read");
    assert_eq!(sender, PLAYER);
    assert!(
        body.as_deref()
            .is_some_and(|body| body.contains("showinfo:")),
        "the detail body is stored",
    );
    assert_eq!(
        *state.read_puts.lock().expect("puts"),
        vec![MAIL_NEW],
        "one read PUT, new mail only"
    );

    // Recipients: the character recipient and the sender, no mailing list.
    let recipients: Vec<i64> = sqlx::query_scalar(
        "select character_id from eve_mail_recipients where eve_mail_id = $1 order by character_id",
    )
    .bind(MAIL_NEW)
    .fetch_all(&pool)
    .await
    .expect("recipients");
    assert_eq!(recipients, vec![SERVICE, PLAYER]);

    // Modules: the abyssal link imported and linked, the Tritanium link
    // filtered out; the read mail's link is linked too (the legacy
    // CreateMailAction runs before the is_read check).
    let linked: Vec<(i64, i64)> = sqlx::query_as(
        "select eve_mail_id, module_id from eve_mail_module
         where eve_mail_id = any($1) order by eve_mail_id",
    )
    .bind(vec![MAIL_NEW, MAIL_READ, MAIL_BROKEN])
    .fetch_all(&pool)
    .await
    .expect("module links");
    assert_eq!(
        linked,
        vec![
            (MAIL_NEW, new_module.module_id),
            (MAIL_READ, read_module.module_id)
        ],
    );
    let imported: i64 = sqlx::query_scalar("select count(*) from modules where id = $1")
        .bind(new_module.module_id)
        .fetch_one(&pool)
        .await
        .expect("module imported");
    assert_eq!(imported, 1);

    // The read-in-game mail keeps its read flag and gets no reply.
    let read_flag: bool = sqlx::query_scalar("select is_read from eve_mails where id = $1")
        .bind(MAIL_READ)
        .fetch_one(&pool)
        .await
        .expect("read mail flag");
    assert!(read_flag);

    // Exactly one appraisal reply, addressed to the sender directly.
    type OutboxRow = (Option<i64>, Option<i64>, String, String);
    let replies: Vec<OutboxRow> = sqlx::query_as(
        "select user_id, recipient_character_id, subject, body
         from notification_outbox where kind = $1 order by id",
    )
    .bind(MAIL_REPLY_KIND)
    .fetch_all(&pool)
    .await
    .expect("outbox rows");
    assert_eq!(replies.len(), 1);
    let (user_id, recipient, subject, body) = &replies[0];
    assert_eq!(
        *user_id, None,
        "replies are character-addressed, not user rows"
    );
    assert_eq!(*recipient, Some(PLAYER));
    assert_eq!(subject, "Modules processed");
    let type_name: String = sqlx::query_scalar("select name from types where id = $1")
        .bind(new_fixture.type_id)
        .fetch_one(&pool)
        .await
        .expect("type name");
    let expected_body = format!(
        "Hello Mail Sender,\n\n\
         We successfully processed your mail with the following modules:\n\n\
         <a href=\"showinfo:{type_id}//{module_id}\">{type_name}</a>\n\
         <a href=\"https://mutamarket.com/modules/{module_id}\">[View on MutaMarket]</a>\n\
         {value_line}\n\n\
         Sincerely,\nThe MutaMarket Team",
        type_id = new_fixture.type_id,
        module_id = new_module.module_id,
        value_line = "No estimated value available",
    );
    assert_eq!(body, &expected_body, "the reply mirrors the blade template");

    // The broken detail stays unprocessed and is retried on the next
    // scan; the already-processed mails are not refetched.
    let broken_body: Option<String> =
        sqlx::query_scalar("select body from eve_mails where id = $1")
            .bind(MAIL_BROKEN)
            .fetch_one(&pool)
            .await
            .expect("broken mail body");
    assert_eq!(broken_body, None);

    state.broken_healed.store(true, Ordering::SeqCst);
    // The second scan runs without ESI delivery: the healed mail is
    // still marked read locally, but no read PUT leaves the process.
    let stats = sync_eve_mails(
        &pool,
        &reference,
        &esi,
        &sso,
        &estimator,
        SERVICE,
        false,
        |_line| {},
    )
    .await
    .expect("second scan")
    .expect("token present");
    assert_eq!(
        (
            stats.mails,
            stats.new,
            stats.modules,
            stats.replies,
            stats.failed
        ),
        (3, 1, 0, 0, 0),
        "only the healed mail processes; a linkless mail queues no reply",
    );
    {
        let calls = state.detail_calls.lock().expect("calls");
        assert_eq!(calls[&MAIL_NEW], 1, "processed mails are not refetched");
        assert_eq!(calls[&MAIL_READ], 1);
        assert_eq!(calls[&MAIL_BROKEN], 2, "the failed detail was retried");
    }
    assert_eq!(
        *state.read_puts.lock().expect("puts"),
        vec![MAIL_NEW],
        "without esi delivery the healed mail gets no read PUT",
    );
    let replies: i64 =
        sqlx::query_scalar("select count(*) from notification_outbox where kind = $1")
            .bind(MAIL_REPLY_KIND)
            .fetch_one(&pool)
            .await
            .expect("reply count");
    assert_eq!(replies, 1, "no duplicate replies on rescan");
}
