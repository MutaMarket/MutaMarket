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
use crate::auth::session;
use crate::modules::notes::NoteEntry;

/// Attaches the signed-in user's notes to module payloads when the
/// request carries a session — the `auth()->check()` gate of the legacy
/// `withUserNote`. Guests are a no-op, leaving the `note` key absent.
pub async fn attach_notes_if_authed(
    state: &AppState,
    headers: &HeaderMap,
    modules: &mut [crate::modules::view::ModuleDetail],
) -> sqlx::Result<()> {
    if let Some(session) = session::session_from_headers(&state.pool, headers).await? {
        crate::modules::queries::attach_user_notes(&state.pool, session.user_id, modules).await?;
    }
    Ok(())
}

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
        return Err(validation_error("notes", "The notes field must be an array."));
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
/// Authorization mirrors the legacy quirk: `StoreCollectionNotesRequest`
/// checks `$user->can('create', [Note::class, $collection])`, which
/// resolves to `NotePolicy::create(User)` — the extra collection argument
/// is silently ignored and the policy always returns true. So ANY
/// signed-in user may write notes on any collection (they land under the
/// owner's user id). Its `findOrFail` in authorize() also means a missing
/// or unknown collection_id 404s before validation ever runs.
pub async fn store_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers, "notes").await {
        Ok(session) => session,
        Err(response) => return response,
    };
    // The session's user is authenticated but otherwise unused: see the
    // policy quirk above.
    let _ = session;

    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();

    let collection_id = as_integer(&payload["collection_id"]);
    let collection_exists: bool = match sqlx::query_scalar(
        "select exists(select 1 from collections where id = $1)",
    )
    .bind(collection_id)
    .fetch_one(&state.pool)
    .await
    {
        Ok(exists) => exists,
        Err(error) => return db_error(error, "notes"),
    };
    if !collection_exists {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "message": "Not found." })),
        )
            .into_response();
    }
    let collection_id = collection_id.expect("an existing collection has an id");

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
