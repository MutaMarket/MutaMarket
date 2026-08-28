//! Collection-location endpoints, ported from the legacy
//! `CollectionLocationController`: bulk add/sync/remove of a location's
//! modules. Success responses are the legacy Inertia-style 302 back to
//! the referer; failures mirror the legacy order: 404 for an unknown
//! collection, 403 for a non-owner, then the Laravel 422 shape for
//! invalid payloads.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use super::social::{back, require_session, validation_error};
use crate::collections;

/// Whether an assets row with this primary key exists, the legacy
/// `exists:assets,id` rule. Legacy quirk kept: the rule checks the whole
/// assets table, not just the caller's own rows.
async fn asset_exists(pool: &PgPool, asset_id: i64) -> sqlx::Result<bool> {
    sqlx::query_scalar("select exists (select 1 from assets where id = $1)")
        .bind(asset_id)
        .fetch_one(pool)
        .await
}

fn database_error(error: sqlx::Error) -> Response {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

#[derive(Deserialize, Default)]
struct CollectionLocationPayload {
    collection_id: Option<i64>,
    location_id: Option<i64>,
}

/// The shared front half of the /collection-locations endpoints, in the
/// legacy FormRequest order: session, findOrFail on collection_id (404),
/// the update policy (403), then the location_id rules (422).
async fn location_request(
    pool: &PgPool,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<(i64, collections::Collection, i64), Response> {
    let session = require_session(pool, headers).await?;
    let payload: CollectionLocationPayload = serde_json::from_slice(body).unwrap_or_default();

    // The legacy authorize() findOrFails the collection before any
    // validation runs, so a missing or unknown collection_id is a 404.
    let collection = match payload.collection_id {
        Some(collection_id) => match collections::collection_by_id(pool, collection_id).await {
            Ok(Some(collection)) => collection,
            Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
            Err(error) => return Err(database_error(error)),
        },
        None => return Err(StatusCode::NOT_FOUND.into_response()),
    };
    if !collection.owned_by(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }

    let Some(location_id) = payload.location_id else {
        return Err(validation_error(
            json!({"location_id": ["The location id field is required."]}),
        ));
    };
    match asset_exists(pool, location_id).await {
        Ok(true) => {}
        Ok(false) => {
            return Err(validation_error(
                json!({"location_id": ["The selected location id is invalid."]}),
            ));
        }
        Err(error) => return Err(database_error(error)),
    }

    Ok((session.user_id, collection, location_id))
}

/// `POST /collection-locations` — add every module inside the location.
pub async fn store(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::add_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(_) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `PUT /collection-locations` — replace the collection's modules with
/// the location's.
pub async fn put(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::sync_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(()) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}

/// `DELETE /collection-locations` — remove every module inside the
/// location from the collection.
pub async fn destroy(State(pool): State<PgPool>, headers: HeaderMap, body: Bytes) -> Response {
    let (user_id, collection, location_id) = match location_request(&pool, &headers, &body).await {
        Ok(parts) => parts,
        Err(response) => return response,
    };

    match collections::remove_location_modules(&pool, user_id, collection.id, location_id).await {
        Ok(_) => back(&headers).into_response(),
        Err(error) => database_error(error),
    }
}
