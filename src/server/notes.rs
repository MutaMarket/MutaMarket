//! The notes routes: `POST /notes` (legacy `NoteController::store`) and
//! `POST /collection-notes` (`CollectionNoteController::store`).
//!
//! Divergence, as documented in `server::offers`: the legacy answered
//! `back()->notify(...)` flash toasts; the fetch-driven frontend gets the
//! bare referer redirect on success and JSON statuses with the legacy
//! texts on failure.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use super::AppState;
use super::support::{back, db_error, session_or_login, validation_error};
use crate::modules::notes::NoteEntry;

/// A Laravel `integer`-rule value: a JSON integer, or an integer string.
fn as_integer(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

/// Validates the `notes` array of both endpoints against the legacy
/// rules (`notes.*.module_id` required integer exists:modules,
/// `notes.*.content` nullable string), with Laravel's default messages.
async fn validate_notes(
    pool: &sqlx::PgPool,
    body: &serde_json::Value,
) -> Result<Vec<NoteEntry>, Response> {
    let notes = &body["notes"];
    if notes.is_null() || notes.as_array().is_some_and(Vec::is_empty) {
        return Err(validation_error("notes", "The notes field is required."));
    }
    let Some(items) = notes.as_array() else {
        return Err(validation_error(
            "notes",
            "The notes field must be an array.",
        ));
    };

    let mut entries = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        // Laravel's :attribute has underscores replaced by spaces, the
        // wildcard index kept: `notes.0.module id`.
        let module_id_field = format!("notes.{index}.module_id");
        let module_id_value = &item["module_id"];
        if module_id_value.is_null() {
            return Err(validation_error(
                &module_id_field,
                &format!("The notes.{index}.module id field is required."),
            ));
        }
        let Some(module_id) = as_integer(module_id_value) else {
            return Err(validation_error(
                &module_id_field,
                &format!("The notes.{index}.module id field must be an integer."),
            ));
        };

        let content = match &item["content"] {
            serde_json::Value::Null => None,
            serde_json::Value::String(content) => Some(content.clone()),
            _ => {
                return Err(validation_error(
                    &format!("notes.{index}.content"),
                    &format!("The notes.{index}.content field must be a string."),
                ));
            }
        };

        entries.push(NoteEntry { module_id, content });
    }

    // The exists:modules,id rule, batched.
    let ids: Vec<i64> = entries.iter().map(|entry| entry.module_id).collect();
    let known: Vec<i64> = sqlx::query_scalar("select id from modules where id = any($1)")
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(|error| db_error(error, "notes"))?;
    if let Some(index) = ids.iter().position(|id| !known.contains(id)) {
        return Err(validation_error(
            &format!("notes.{index}.module_id"),
            &format!("The selected notes.{index}.module id is invalid."),
        ));
    }

    Ok(entries)
}

/// `POST /notes` — bulk upsert of the user's module notes.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers, "notes").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();

    let entries = match validate_notes(&state.pool, &payload).await {
        Ok(entries) => entries,
        Err(response) => return response,
    };

    match crate::modules::notes::store_notes(&state.pool, session.user_id, &entries).await {
        Ok(()) => back(&headers).into_response(),
        Err(error) => db_error(error, "notes"),
    }
}

/// `POST /collection-notes` — bulk upsert of a collection's notes.
///
/// Deliberate divergence: the legacy `StoreCollectionNotesRequest` checked
/// `$user->can('create', [Note::class, $collection])`, which Laravel
/// resolved to `NotePolicy::create(User)` (the collection argument was
/// dropped), so any signed-in user could write or delete the notes of
/// any collection under the owner's name. The intended
/// `CollectionNotePolicy` rule applies here: only the owner. A missing
/// or unknown collection_id still 404s before validation, like the
/// legacy `findOrFail` in authorize().
pub async fn store_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers, "notes").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();

    let collection = match as_integer(&payload["collection_id"]) {
        Some(id) => match crate::collections::collection_by_id(&state.pool, id).await {
            Ok(collection) => collection,
            Err(error) => return db_error(error, "notes"),
        },
        None => None,
    };
    let Some(collection) = collection else {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "message": "Not found." })),
        )
            .into_response();
    };
    if !collection.owned_by(session.user_id) {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "message": "Forbidden." })),
        )
            .into_response();
    }
    let collection_id = collection.id;

    let entries = match validate_notes(&state.pool, &payload).await {
        Ok(entries) => entries,
        Err(response) => return response,
    };

    match crate::modules::notes::store_collection_notes(&state.pool, collection_id, &entries).await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => db_error(error, "notes"),
    }
}
