//! The notification delivery job routes to a linked user's Discord DM
//! channel instead of EVE mail, like the legacy notifications' `via()`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::Path;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use mutamarket::auth::linked::{DiscordClient, LinkedClients};
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::mutation::reference::ReferenceData;
use mutamarket::scheduler::{JobDeps, RunNowOutcome, Scheduler};

use crate::common::EnvGuard;

const USER_ID: i64 = 880_001;
const CHARACTER_ID: i64 = 95_000_881;
const CHANNEL_ID: i64 = 123_456_789;

type Captured = Arc<Mutex<Vec<(String, Value)>>>;

/// A Discord mock capturing `POST /channels/{id}/messages` (auth header +
/// body), answering the created message.
async fn start_mock_discord(captured: Captured) -> String {
    let app = Router::new().route(
        "/channels/{channel}/messages",
        post(
            move |Path(channel): Path<String>,
                  headers: axum::http::HeaderMap,
                  Json(body): Json<Value>| {
                let captured = captured.clone();
                async move {
                    let authorization = headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or_default()
                        .to_owned();
                    captured
                        .lock()
                        .expect("capture lock")
                        .push((format!("{channel}:{authorization}"), body));
                    Json(json!({ "id": "1" }))
                }
            },
        ),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock discord");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve mock discord");
    });
    format!("http://{address}")
}

async fn scheduler_with_discord(pool: &sqlx::PgPool, discord: DiscordClient) -> Arc<Scheduler> {
    Scheduler::disabled(JobDeps {
        pool: pool.clone(),
        activity: Arc::default(),
        reference: Arc::new(ReferenceData::default()),
        esi: EsiClient::new("http://127.0.0.1:9"),
        estimator: Estimator::new(),
        sso: SsoClient::new(
            "http://127.0.0.1:9",
            "client",
            "secret",
            "http://test/eve/callback",
        ),
        discord,
    })
}

async fn wait_delivered(pool: &sqlx::PgPool, id: i64) -> (String, Option<String>) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "select delivery, error from notification_outbox
             where id = $1 and delivered_at is not null",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .expect("outbox row");
        if let Some((Some(delivery), error)) = row {
            return (delivery, error);
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "notification {id} was not delivered in time"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn seed_recipient(pool: &sqlx::PgPool, discord_channel_id: Option<i64>) {
    sqlx::query("delete from notification_outbox where user_id = $1")
        .bind(USER_ID)
        .execute(pool)
        .await
        .expect("clean outbox");
    sqlx::query("delete from characters where id = $1")
        .bind(CHARACTER_ID)
        .execute(pool)
        .await
        .expect("clean character");
    sqlx::query("delete from users where id = $1")
        .bind(USER_ID)
        .execute(pool)
        .await
        .expect("clean user");
    sqlx::query("insert into users (id, name, discord_channel_id) values ($1, 'Tester', $2)")
        .bind(USER_ID)
        .bind(discord_channel_id)
        .execute(pool)
        .await
        .expect("seed user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Tester Char', $2)")
        .bind(CHARACTER_ID)
        .bind(USER_ID)
        .execute(pool)
        .await
        .expect("seed character");
}

#[tokio::test]
async fn linked_user_is_notified_on_discord_instead_of_mail() {
    let pool = db::test_pool().await.expect("Postgres not reachable");
    db::migrate(&pool).await.expect("migrations run");
    let _delivery = EnvGuard::capture("NOTIFY_DELIVERY");
    // SAFETY: the suite runs single-threaded (RUST_TEST_THREADS=1).
    unsafe { std::env::set_var("NOTIFY_DELIVERY", "esi") };

    let captured: Captured = Arc::default();
    let mock = start_mock_discord(captured.clone()).await;
    let discord = DiscordClient::new(
        &mock,
        "id",
        "secret",
        "http://test/discord/callback",
        "botto",
    );

    // --- linked user: delivered to Discord --------------------------------
    seed_recipient(&pool, Some(CHANNEL_ID)).await;
    let payload = json!({
        "offer_id": 7,
        "discord": mutamarket::notifications::offer_received_discord("Bob", "Abyssal Heat Sink", 42, 7),
    });
    let id = mutamarket::notifications::queue(
        &pool,
        USER_ID,
        "offer-received",
        "New offer",
        "body",
        payload,
    )
    .await
    .expect("queue notification");

    let scheduler = scheduler_with_discord(&pool, discord).await;
    assert!(matches!(
        scheduler.run_now("notification-delivery"),
        RunNowOutcome::Started
    ));
    let (delivery, error) = wait_delivered(&pool, id).await;
    assert_eq!(
        delivery, "discord",
        "linked user routes to Discord: {error:?}"
    );
    assert_eq!(error, None);

    let messages = captured.lock().expect("capture lock").clone();
    assert_eq!(messages.len(), 1, "one Discord message sent");
    let (target, body) = &messages[0];
    assert_eq!(
        target,
        &format!("{CHANNEL_ID}:Bot botto"),
        "bot posts to the DM channel"
    );
    assert_eq!(
        body["content"],
        "You have received a new offer from Bob for your Abyssal Heat Sink."
    );
    assert_eq!(body["embeds"][0]["title"], "New offer from Bob");

    // --- unlinked user: falls back to the mail transport ------------------
    seed_recipient(&pool, None).await;
    let id = mutamarket::notifications::queue(
        &pool,
        USER_ID,
        "offer-received",
        "New offer",
        "body",
        json!({ "offer_id": 8 }),
    )
    .await
    .expect("queue notification");
    assert!(matches!(
        scheduler.run_now("notification-delivery"),
        RunNowOutcome::Started
    ));
    let (delivery, _error) = wait_delivered(&pool, id).await;
    assert_eq!(delivery, "esi", "an unlinked user still routes to EVE mail");
    assert_eq!(
        captured.lock().expect("capture lock").len(),
        1,
        "no extra Discord message for the unlinked user"
    );

    let _ = LinkedClients::from_env();
}
