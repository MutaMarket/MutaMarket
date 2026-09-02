//! `GET /ws` — the user's private event stream.
//!
//! The legacy app broadcasts over Laravel Echo/Reverb (the Pusher
//! protocol) on the private channel `Users.{id}` (routes/channels.php).
//! This is the native replacement: a session-authenticated axum WebSocket
//! that pushes the same channel/event/data envelope Echo delivers, so
//! future events (messages, offers) reuse the shape:
//!
//! ```json
//! {"channel": "Users.1", "event": "AssetImportUpdated", "data": {...}}
//! ```
//!
//! First producer: asset import progress. The legacy UI *polls* the import
//! state over Inertia every two seconds (`AssetImportStatus.vue`); pushing
//! it over the socket is a deliberate upgrade. The event name
//! `AssetImportUpdated` has no legacy counterpart for the same reason. The
//! socket watches the database rather than an in-process bus so imports
//! run by other processes (scheduler in another instance, one-shot bins)
//! are seen too.

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use sqlx::{PgPool, Row};

use super::AppState;
use crate::auth::session::{self, Session};
use crate::view::personal::AssetImportView;

/// How often the socket checks the watched rows for changes. Half the
/// legacy client's two-second Inertia poll, so the pushed updates are at
/// least as fresh as the legacy UI ever was.
const WATCH_INTERVAL: Duration = Duration::from_millis(1000);

/// The private per-user channel name, like the legacy `Users.{id}` channel
/// in routes/channels.php.
fn user_channel(user_id: i64) -> String {
    format!("Users.{user_id}")
}

/// The upgrade handler. Guests get a plain 401: a WebSocket handshake
/// cannot follow the login redirect the page routes use, and the legacy
/// Echo auth endpoint rejects unauthenticated subscriptions the same way.
pub async fn websocket(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    // A cross-site page must not open a socket on the user's channel; the
    // Lax cookie already stops that in current browsers, the Origin check
    // makes it explicit.
    if super::support::is_cross_site(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(error) => {
            tracing::warn!("ws session lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    ws.on_upgrade(move |socket| user_stream(state, session, socket))
}

/// Pushes the user's events until the client disconnects.
async fn user_stream(state: AppState, session: Session, mut socket: WebSocket) {
    let channel = user_channel(session.user_id);
    let mut ticker = tokio::time::interval(WATCH_INTERVAL);
    let mut last_import: Option<AssetImportView> = None;
    let mut last_message: Option<i64> = None;
    let mut first_tick = true;

    loop {
        tokio::select! {
            message = socket.recv() => {
                match message {
                    // Pings are answered by axum automatically; any other
                    // client chatter is ignored, like Echo does for
                    // whisper-less channels.
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {}
                }
            }
            _ = ticker.tick() => {
                let import = match latest_asset_import(
                    &state.pool,
                    session.user_id,
                    session.active_character_id,
                ).await {
                    Ok(import) => import,
                    Err(error) => {
                        tracing::warn!("ws asset import watch failed: {error}");
                        continue;
                    }
                };

                // The initial snapshot always goes out (also when there is
                // no import yet); afterwards only real state changes are
                // pushed - updated_seconds_ago ticks every poll and must
                // not count as a change.
                let stable = |import: &Option<AssetImportView>| {
                    import.as_ref().map(|import| AssetImportView {
                        updated_seconds_ago: 0,
                        ..import.clone()
                    })
                };
                if first_tick || stable(&import) != stable(&last_import) {
                    let envelope = json!({
                        "channel": channel,
                        "event": "AssetImportUpdated",
                        "data": import,
                    });
                    let text = envelope.to_string();
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                    last_import = import;
                }

                // Second producer: offer messages. The legacy
                // MessageReceived/OfferReceived broadcasts collapse into
                // one event per new incoming message; the client
                // refetches whatever offer view it shows.
                match newest_incoming_message(&state.pool, session.user_id).await {
                    Ok(newest) => {
                        if !first_tick
                            && let Some((message_id, offer_id)) = newest
                            && Some(message_id) != last_message
                        {
                            let envelope = json!({
                                "channel": channel,
                                "event": "MessageReceived",
                                "data": { "message_id": message_id, "offer_id": offer_id },
                            });
                            if socket.send(Message::Text(envelope.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        last_message = newest.map(|(message_id, _)| message_id);
                    }
                    Err(error) => tracing::warn!("ws message watch failed: {error}"),
                }

                first_tick = false;
            }
        }
    }
}

/// The newest message addressed to one of the user's characters, as
/// (message id, offer id).
async fn newest_incoming_message(pool: &PgPool, user_id: i64) -> sqlx::Result<Option<(i64, i64)>> {
    sqlx::query_as(
        "select m.id, m.offer_id from messages m
         join characters c on c.id = m.receiver_id
         where c.user_id = $1
         order by m.id desc limit 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// The import shown to a user: the active character's latest import, like
/// the legacy `getActiveCharacter()->getLatestAssetImport()`; without an
/// active character in the session it falls back to the newest import
/// across the user's characters (the store action imports all of them).
pub async fn latest_asset_import(
    pool: &PgPool,
    user_id: i64,
    active_character_id: Option<i64>,
) -> sqlx::Result<Option<AssetImportView>> {
    let row = sqlx::query(
        "select ai.id, ai.character_id, ai.status, ai.step,
                ai.assets_count, ai.assets_corporation_count,
                ai.abyssal_modules_count, ai.abyssal_modules_imported_count,
                ai.abyssal_modules_failed_count,
                extract(epoch from now() - ai.updated_at)::bigint as updated_seconds_ago
         from asset_imports ai
         join characters c on c.id = ai.character_id
         where c.user_id = $1 and ($2::bigint is null or ai.character_id = $2)
         order by ai.created_at desc, ai.id desc
         limit 1",
    )
    .bind(user_id)
    .bind(active_character_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| AssetImportView {
        id: row.get("id"),
        character_id: row.get("character_id"),
        status: row.get("status"),
        step: row.get("step"),
        assets_count: row.get::<i32, _>("assets_count") as i64,
        assets_corporation_count: row.get::<i32, _>("assets_corporation_count") as i64,
        abyssal_modules_count: row.get::<i32, _>("abyssal_modules_count") as i64,
        abyssal_modules_imported_count: row.get::<i32, _>("abyssal_modules_imported_count") as i64,
        abyssal_modules_failed_count: row.get::<i32, _>("abyssal_modules_failed_count") as i64,
        updated_seconds_ago: row.get("updated_seconds_ago"),
    }))
}
