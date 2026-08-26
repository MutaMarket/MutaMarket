//! `POST /modules` — the appraise flow, the legacy
//! `ModuleController::store`: resolve an in-game item link (or an
//! explicit type/item pair), fetch and ingest the module like
//! `GetModuleJob::dispatchSync`, and send the client to its show page.
//!
//! Divergence from legacy: the page posts through fetch(), so failures
//! answer 422 JSON with the legacy notification text instead of a
//! redirect-with-flash (flash notifications are not ported).

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::modules::ingest;
use crate::modules::link::ModuleLink;

/// The legacy failure notification body.
const FAILED_MESSAGE: &str =
    "We were unable to add the module to the database. Please check your input and try again.";

#[derive(serde::Deserialize, Default)]
struct Payload {
    message: Option<String>,
    type_id: Option<i64>,
    item_id: Option<i64>,
}

pub async fn store(State(state): State<AppState>, body: Bytes) -> Response {
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();

    // The legacy rules: a message, or an explicit type_id + item_id.
    let (type_id, item_id) = match payload {
        Payload { message: Some(message), .. } if !message.is_empty() => {
            match ModuleLink::first_from(&message) {
                Some(link) => (link.type_id, link.item_id),
                None => return failure(),
            }
        }
        Payload { type_id: Some(type_id), item_id: Some(item_id), .. } => (type_id, item_id),
        _ => {
            return super::api::error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "The message field is required when item id is not present.",
            );
        }
    };

    match ingest::import_module(
        &state.pool,
        &state.reference,
        &state.esi,
        &state.estimator,
        type_id,
        item_id,
    )
    .await
    {
        // The legacy redirects to modules.show by bare item id; the show
        // route resolves any slug by its trailing identifier.
        Ok(()) => Redirect::to(&format!("/modules/{item_id}")).into_response(),
        Err(error) => {
            tracing::info!("appraise for item {item_id} failed: {error}");
            failure()
        }
    }
}

fn failure() -> Response {
    super::api::error(StatusCode::UNPROCESSABLE_ENTITY, FAILED_MESSAGE)
}
