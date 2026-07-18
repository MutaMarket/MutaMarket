//! Behavior tests for the character name sync against a mock ESI names
//! endpoint, including the batch-rejection bisection for unresolvable ids.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::routing::post;
use axum::{Json, Router};
use mutamarket::characters::sync_character_names;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use serde_json::json;

/// A biomassed character: ESI rejects any names batch containing it.
const POISON_ID: i64 = 700_666;
const KNOWN_A: i64 = 700_001;
const KNOWN_B: i64 = 700_002;

fn mock_esi(requests: Arc<AtomicUsize>) -> Router {
    Router::new().route(
        "/latest/universe/names/",
        post(move |Json(ids): Json<Vec<i64>>| {
            let requests = requests.clone();
            async move {
                requests.fetch_add(1, Ordering::SeqCst);

                if ids.contains(&POISON_ID) {
                    return Err(axum::http::StatusCode::NOT_FOUND);
                }

                let names: Vec<serde_json::Value> = ids
                    .iter()
                    .map(|id| {
                        json!({
                            "id": id,
                            "name": format!("Pilot {id}"),
                            "category": "character",
                        })
                    })
                    .collect();
                Ok(Json(json!(names)))
            }
        }),
    )
}

#[tokio::test]
async fn name_sync_names_stubs_and_isolates_poison_ids() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    // Idempotency: reclaim the fixture ids from previous runs.
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![KNOWN_A, KNOWN_B, POISON_ID])
        .execute(&pool)
        .await
        .expect("cleanup");
    for id in [KNOWN_A, KNOWN_B, POISON_ID] {
        sqlx::query("insert into characters (id, name) values ($1, '')")
            .bind(id)
            .execute(&pool)
            .await
            .expect("stub character");
    }

    let requests = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
    let address = listener.local_addr().expect("mock address");
    let app = mock_esi(requests.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock ESI");
    });
    let esi = EsiClient::new(&format!("http://{address}"));

    // Other suites leave their own unnamed stubs in the shared test
    // database; the sync names all of them, ours included.
    let named = sync_character_names(&pool, &esi).await.expect("sync runs");
    assert!(named >= 2, "both resolvable stubs are named (got {named})");

    let rows: Vec<(i64, String, bool)> = sqlx::query_as(
        "select id, name, name_fetched_at is not null from characters
         where id = any($1) order by id",
    )
    .bind(vec![KNOWN_A, KNOWN_B, POISON_ID])
    .fetch_all(&pool)
    .await
    .expect("character rows");

    assert_eq!(
        rows,
        vec![
            (KNOWN_A, format!("Pilot {KNOWN_A}"), true),
            (KNOWN_B, format!("Pilot {KNOWN_B}"), true),
            // The poison id keeps its empty name but is stamped as fetched
            // so it is never retried, like the legacy command.
            (POISON_ID, String::new(), true),
        ],
    );

    assert!(
        requests.load(Ordering::SeqCst) >= 3,
        "the rejected batch is bisected into further requests",
    );

    // A second run has nothing left to fetch.
    let before = requests.load(Ordering::SeqCst);
    let named = sync_character_names(&pool, &esi).await.expect("second sync");
    assert_eq!(named, 0);
    assert_eq!(requests.load(Ordering::SeqCst), before, "no ids left, no requests made");
}
