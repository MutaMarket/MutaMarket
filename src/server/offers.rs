//! The offer routes, the legacy `OfferController` + `MessageController`.
//!
//! Divergences, documented once here: the legacy web routes answered
//! with redirects carrying flash toasts; the fetch-driven frontend gets
//! the same texts as JSON statuses instead (403 blocked, 409 duplicate,
//! 422 validation). Successful creation redirects to the new offer's
//! page rather than `back()` so the dialog lands the buyer in the
//! thread. Offers also carry an explicit `price` (see the migration).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::auth::session;
use crate::offers;
use crate::view::offers::{
    LatestMessageView, MessageView, OfferListView, OfferModuleSummary, OfferParticipant,
    OfferThreadView,
};

/// The legacy `offers.create.defaultMessage`, with the price the legacy
/// text expected users to type in by hand.
fn default_message(price: f64) -> String {
    format!(
        "Hey, I can offer you {} ISK for it. Let me know if you're interested!",
        crate::notifications::format_isk(price)
    )
}

async fn session_or_login(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<session::Session, Response> {
    match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "offer session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// The session's active character, or the user's first (the legacy
/// `getActiveCharacter()`).
async fn active_character(
    state: &AppState,
    session: &session::Session,
) -> sqlx::Result<Option<i64>> {
    match session.active_character_id {
        Some(id) => Ok(Some(id)),
        None => {
            sqlx::query_scalar("select id from characters where user_id = $1 order by id limit 1")
                .bind(session.user_id)
                .fetch_optional(&state.pool)
                .await
        }
    }
}

fn error_json(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({ "message": message }))).into_response()
}

fn validation_error(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(serde_json::json!({
            "message": "The given data was invalid.",
            "errors": { field: [message] },
        })),
    )
        .into_response()
}

/// `POST /offers` — the legacy `OfferController::store`.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let Ok(Some(sender)) = active_character(&state, &session).await else {
        return Redirect::to("/login").into_response();
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        receiver_id: Option<i64>,
        module_id: Option<i64>,
        price: Option<f64>,
        message: Option<String>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();

    let Some(receiver_id) = payload.receiver_id else {
        return validation_error("receiver_id", "The receiver id field is required.");
    };
    let Some(module_id) = payload.module_id else {
        return validation_error("module_id", "The module id field is required.");
    };
    let Some(price) = payload.price.filter(|price| price.is_finite()) else {
        return validation_error("price", "The price field is required.");
    };
    if price <= 0.0 {
        return validation_error("price", "The price field must be greater than 0.");
    }

    let receiver_exists: bool =
        match sqlx::query_scalar("select exists(select 1 from characters where id = $1)")
            .bind(receiver_id)
            .fetch_one(&state.pool)
            .await
        {
            Ok(exists) => exists,
            Err(error) => return db_error(error),
        };
    if !receiver_exists {
        return validation_error("receiver_id", "The selected receiver id is invalid.");
    }
    let module_exists: bool =
        match sqlx::query_scalar("select exists(select 1 from modules where id = $1)")
            .bind(module_id)
            .fetch_one(&state.pool)
            .await
        {
            Ok(exists) => exists,
            Err(error) => return db_error(error),
        };
    if !module_exists {
        return validation_error("module_id", "The selected module id is invalid.");
    }

    let message = match payload.message.as_deref().map(str::trim) {
        Some(message) if !message.is_empty() => message.to_owned(),
        _ => default_message(price),
    };

    let offer_id =
        match offers::create_offer(&state.pool, sender, receiver_id, module_id, price, &message)
            .await
        {
            Ok(id) => id,
            Err(offers::CreateOfferError::Blocked) => {
                return error_json(StatusCode::FORBIDDEN, "You have been blocked by this user.");
            }
            Err(offers::CreateOfferError::Duplicate) => {
                return error_json(
                    StatusCode::CONFLICT,
                    "You have already sent an offer for this module.",
                );
            }
            Err(offers::CreateOfferError::Db(error)) => return db_error(error),
        };

    // The legacy deferred OfferReceived notification, queued to the
    // outbox for the delivery job.
    if let Err(error) = queue_offer_received(&state, offer_id).await {
        tracing::warn!(%error, offer_id, "queueing the offer notification failed");
    }

    Redirect::to(&format!("/offers/{offer_id}")).into_response()
}

/// Queues the receiver's offer-received notification.
async fn queue_offer_received(state: &AppState, offer_id: i64) -> sqlx::Result<()> {
    let row: Option<(i64, String, String, i64, i64, String, f64)> = sqlx::query_as(
        "select rc.user_id, rc.name, sc.name, mo.type_id, mo.id, t.name, o.price
         from offers o
         join characters rc on rc.id = o.receiver_id
         join characters sc on sc.id = o.sender_id
         join modules mo on mo.id = o.module_id
         join types t on t.id = mo.type_id
         where o.id = $1 and rc.user_id is not null",
    )
    .bind(offer_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some((user_id, receiver_name, sender_name, type_id, module_id, type_name, price)) = row
    else {
        // Receiver characters without an account cannot be notified.
        return Ok(());
    };

    let (subject, body) = crate::notifications::offer_received_mail(
        &receiver_name,
        &sender_name,
        type_id,
        module_id,
        &type_name,
        offer_id,
        price,
    );
    crate::notifications::queue(
        &state.pool,
        user_id,
        "offer-received",
        &subject,
        &body,
        serde_json::json!({ "offer_id": offer_id }),
    )
    .await?;
    Ok(())
}

/// `POST /messages` — the legacy `MessageController::store`: appends to
/// the thread and redirects back.
pub async fn store_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        offer_id: Option<i64>,
        content: Option<String>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(offer_id) = payload.offer_id else {
        return validation_error("offer_id", "The offer id field is required.");
    };
    let Some(content) = payload.content.map(|content| content.trim().to_owned()) else {
        return validation_error("content", "The content field is required.");
    };
    if content.is_empty() {
        return validation_error("content", "The content field is required.");
    }

    let offer = match offers::offer(&state.pool, offer_id).await {
        Ok(Some(offer)) => offer,
        Ok(None) => return error_json(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => return db_error(error),
    };

    match offers::send_message(&state.pool, &offer, session.user_id, &content).await {
        Ok(Some(_)) => back(&headers).into_response(),
        Ok(None) => error_json(StatusCode::FORBIDDEN, "Forbidden."),
        Err(error) => db_error(error),
    }
}

/// `DELETE /offers/{offer}` — the legacy `OfferController::destroy`
/// (leave semantics), redirecting to the index like `to_route('offers')`.
pub async fn destroy(
    State(state): State<AppState>,
    axum::extract::Path(offer_id): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let offer = match offers::offer(&state.pool, offer_id).await {
        Ok(Some(offer)) => offer,
        Ok(None) => return error_json(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => return db_error(error),
    };

    match offers::leave_offer(&state.pool, &offer, session.user_id).await {
        Ok(true) => Redirect::to("/offers").into_response(),
        Ok(false) => error_json(StatusCode::FORBIDDEN, "Forbidden."),
        Err(error) => db_error(error),
    }
}

async fn require_api_session(
    pool: &sqlx::PgPool,
    headers: &HeaderMap,
) -> Result<session::Session, Response> {
    match session::session_from_headers(pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated.")),
        Err(error) => Err(super::api::database_error(error)),
    }
}

/// `GET /api/offers` — the index data of the legacy
/// `OfferController::index`.
pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let characters = match offers::user_character_ids(&state.pool, session.user_id).await {
        Ok(characters) => characters,
        Err(error) => return db_error(error),
    };
    let rows = match offers::offers_for_user(&state.pool, session.user_id).await {
        Ok(rows) => rows,
        Err(error) => return db_error(error),
    };

    let list: Vec<OfferListView> = rows
        .into_iter()
        .map(|row| {
            // The legacy is_read: mine, or already read by the receiver.
            let is_read = characters.contains(&row.latest_sender_id) || row.latest_read;
            OfferListView {
                id: row.id,
                sender: OfferParticipant { id: row.sender_id, name: row.sender_name },
                receiver: OfferParticipant { id: row.receiver_id, name: row.receiver_name },
                module: OfferModuleSummary {
                    id: row.module_id,
                    type_id: row.module_type_id,
                    type_name: row.module_type_name,
                },
                price: row.price,
                latest_message: LatestMessageView {
                    content: row.latest_content,
                    sender_id: row.latest_sender_id,
                    created_at: row.latest_created_at,
                },
                is_read,
                created_at: row.created_at,
            }
        })
        .collect();

    axum::Json(list).into_response()
}

/// `GET /api/offers/sent` — the signed-in user's active sent offers as
/// (module_id, offer id) pairs, backing the cards' "Go to offer" swap
/// (the legacy `withLatestOfferMadeByAuthenticatedUser`).
pub async fn sent(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    match offers::sent_offer_modules(&state.pool, session.user_id).await {
        Ok(rows) => axum::Json(
            rows.into_iter()
                .map(|(module_id, id)| serde_json::json!({ "module_id": module_id, "id": id }))
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => db_error(error),
    }
}

/// `GET /api/offers/{offer}` — the thread of the legacy
/// `OfferController::show`; viewing marks the viewer's side read.
pub async fn show(
    State(state): State<AppState>,
    axum::extract::Path(offer_id): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let offer = match offers::offer(&state.pool, offer_id).await {
        Ok(Some(offer)) => offer,
        Ok(None) => return error_json(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => return db_error(error),
    };
    let characters = match offers::user_character_ids(&state.pool, session.user_id).await {
        Ok(characters) => characters,
        Err(error) => return db_error(error),
    };
    let Some(own_character_id) = offer.own_character(&characters) else {
        return error_json(StatusCode::FORBIDDEN, "Forbidden.");
    };

    if let Err(error) = offers::mark_read(&state.pool, offer.id, session.user_id).await {
        tracing::warn!(%error, offer_id, "marking offer messages read failed");
    }

    let names: Vec<(i64, String)> =
        match sqlx::query_as("select id, name from characters where id = any($1)")
            .bind(vec![offer.sender_id, offer.receiver_id])
            .fetch_all(&state.pool)
            .await
        {
            Ok(names) => names,
            Err(error) => return db_error(error),
        };
    let name_of = |id: i64| {
        names
            .iter()
            .find(|(character, _)| *character == id)
            .map(|(_, name)| name.clone())
            .unwrap_or_default()
    };

    let module = match crate::modules::queries::details_for(
        &state.pool,
        &state.reference,
        vec![offer.module_id],
    )
    .await
    {
        Ok(mut details) => details.pop(),
        Err(error) => return db_error(error),
    };

    let messages = match offers::offer_messages(&state.pool, offer.id).await {
        Ok(messages) => messages,
        Err(error) => return db_error(error),
    };

    axum::Json(OfferThreadView {
        id: offer.id,
        sender: OfferParticipant { id: offer.sender_id, name: name_of(offer.sender_id) },
        receiver: OfferParticipant { id: offer.receiver_id, name: name_of(offer.receiver_id) },
        price: offer.price,
        own_character_id,
        left_by_sender: offer.left_by_sender,
        left_by_receiver: offer.left_by_receiver,
        module,
        messages: messages
            .into_iter()
            .map(|message| MessageView {
                mine: characters.contains(&message.sender_id),
                id: message.id,
                sender: OfferParticipant { id: message.sender_id, name: message.sender_name },
                content: message.content,
                created_at: message.created_at,
            })
            .collect(),
    })
    .into_response()
}

fn back(headers: &HeaderMap) -> Redirect {
    let target = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/offers");
    Redirect::to(target)
}

fn db_error(error: sqlx::Error) -> Response {
    tracing::warn!(%error, "offer database error");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
