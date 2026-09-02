//! Behavior tests for the `/ws` user event stream: guests are rejected at
//! the handshake, an authenticated socket receives the Echo-shaped
//! `AssetImportUpdated` envelope as an initial snapshot, and row changes
//! are pushed without the client asking.
//!
//! Needs the local database: `docker compose up -d postgres`.

use std::sync::Arc;
use std::time::Duration;

use mutamarket::auth::session::create_session;
use mutamarket::auth::sso::SsoClient;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::mutation::reference::ReferenceData;
use sqlx::PgPool;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

const WS_CHARACTER: i64 = 96_000_001;

/// Waiting for a pushed frame: comfortably above the one-second watch
/// interval of the server.
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

/// Serves the production router on an ephemeral port; websockets need a
/// real connection, not an oneshot.
async fn serve_app(pool: PgPool) -> String {
    let app = mutamarket::server::router(
        pool,
        EsiClient::new("http://127.0.0.1:9"),
        SsoClient::new(
            "http://127.0.0.1:9",
            "client",
            "secret",
            "http://test/eve/callback",
        ),
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator_stub(),
        Arc::new(ReferenceData::default()),
        None,
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind app");
    let address = listener.local_addr().expect("app address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve app");
    });

    format!("{address}")
}

async fn setup() -> PgPool {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    pool
}

/// A user owning one character, with a session bound to that character.
async fn seed_user(pool: &PgPool, character_id: i64) -> (i64, String) {
    sqlx::query("update characters set latest_asset_import_id = null where id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("unlink import");
    sqlx::query("delete from asset_imports where character_id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("clean imports");
    sqlx::query("delete from characters where id = $1")
        .bind(character_id)
        .execute(pool)
        .await
        .expect("clean character");

    let user_id: i64 =
        sqlx::query_scalar("insert into users (name) values ('WS Pilot') returning id")
            .fetch_one(pool)
            .await
            .expect("create user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'WS Pilot', $2)")
        .bind(character_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("create character");

    let token = create_session(pool, user_id, Some(character_id))
        .await
        .expect("create session");

    (user_id, token)
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn next_text_frame(socket: &mut Socket) -> serde_json::Value {
    use futures_util::StreamExt;

    loop {
        let frame = tokio::time::timeout(FRAME_TIMEOUT, socket.next())
            .await
            .expect("a frame before the timeout")
            .expect("stream open")
            .expect("frame readable");

        if let Message::Text(text) = frame {
            return serde_json::from_str(&text).expect("JSON envelope");
        }
    }
}

#[tokio::test]
async fn guests_are_rejected_at_the_handshake() {
    let pool = setup().await;
    let address = serve_app(pool).await;

    let error = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
        .await
        .expect_err("guest handshake must fail");

    match error {
        WsError::Http(response) => assert_eq!(response.status(), 401),
        other => panic!("expected an HTTP 401 rejection, got {other:?}"),
    }
}

#[tokio::test]
async fn asset_import_changes_are_pushed_to_the_user_channel() {
    use futures_util::SinkExt;

    let pool = setup().await;
    let (user_id, session) = seed_user(&pool, WS_CHARACTER).await;
    let address = serve_app(pool.clone()).await;

    // Snapshot state before connecting: a pending import.
    let import_id: i64 = sqlx::query_scalar(
        "insert into asset_imports (character_id, status, step) values ($1, 'pending', 'fetching_assets')
         returning id",
    )
    .bind(WS_CHARACTER)
    .fetch_one(&pool)
    .await
    .expect("seed import");

    let mut request = format!("ws://{address}/ws")
        .into_client_request()
        .expect("client request");
    request.headers_mut().insert(
        "Cookie",
        format!("mm_session={session}")
            .parse()
            .expect("cookie header"),
    );

    let (mut socket, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("authenticated handshake");

    // The initial snapshot arrives unprompted, in the Echo envelope shape
    // on the legacy Users.{id} channel.
    let envelope = next_text_frame(&mut socket).await;
    assert_eq!(
        envelope["channel"],
        serde_json::json!(format!("Users.{user_id}"))
    );
    assert_eq!(envelope["event"], serde_json::json!("AssetImportUpdated"));
    assert_eq!(envelope["data"]["status"], serde_json::json!("pending"));
    assert_eq!(
        envelope["data"]["step"],
        serde_json::json!("fetching_assets")
    );

    // The exact legacy asset_import prop key set (timestamps replaced by
    // the age, see AssetImportView).
    let mut keys: Vec<&str> = envelope["data"]
        .as_object()
        .expect("data object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "abyssal_modules_count",
            "abyssal_modules_failed_count",
            "abyssal_modules_imported_count",
            "assets_corporation_count",
            "assets_count",
            "character_id",
            "id",
            "status",
            "step",
            "updated_seconds_ago",
        ],
    );

    // A state transition is pushed without any client message.
    sqlx::query(
        "update asset_imports
         set status = 'processing', step = 'importing_abyssal_modules',
             abyssal_modules_count = 5, abyssal_modules_imported_count = 2, updated_at = now()
         where id = $1",
    )
    .bind(import_id)
    .execute(&pool)
    .await
    .expect("advance import");

    let envelope = next_text_frame(&mut socket).await;
    assert_eq!(envelope["data"]["status"], serde_json::json!("processing"));
    assert_eq!(
        envelope["data"]["step"],
        serde_json::json!("importing_abyssal_modules")
    );
    assert_eq!(
        envelope["data"]["abyssal_modules_count"],
        serde_json::json!(5)
    );
    assert_eq!(
        envelope["data"]["abyssal_modules_imported_count"],
        serde_json::json!(2)
    );

    socket.send(Message::Close(None)).await.expect("close");
}
