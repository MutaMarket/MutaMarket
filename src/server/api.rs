//! The public JSON API, ported from the legacy `Api\ModuleController` and
//! statistics controllers. Contract- and estimator-dependent behavior
//! (price filters, sale listings, estimated values) arrives with those
//! milestones. Data loading is shared with the Leptos pages via
//! `modules::queries`.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};

use super::AppState;
use crate::modules::ingest::import_module;
use crate::modules::link::ModuleLink;
use crate::modules::queries;
use crate::modules::view::module_id_from_slug;

/// Modules per index page, like the legacy cursor pagination.
const MODULES_PAGE_SIZE: i64 = 100;

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

fn database_error(error: sqlx::Error) -> Response {
    eprintln!("api database error: {error}");
    self::error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

/// `GET /api/modules` — the legacy index requires a type option in the query
/// path, so the bare route always rejects.
pub async fn modules_index_root() -> Response {
    error(StatusCode::NOT_FOUND, "Please provide a valid type.")
}

/// `GET /api/modules/{query}`: a slug ending in digits is a module lookup,
/// anything else is the type-scoped module index with filter segments.
pub async fn modules_show_or_index(
    State(state): State<AppState>,
    Path(query): Path<String>,
) -> Response {
    match module_id_from_slug(&query) {
        Some(item_id) => show_module(&state, item_id).await,
        None => module_index(&state, &query).await,
    }
}

async fn show_module(state: &AppState, item_id: i64) -> Response {
    match queries::module_detail(&state.pool, &state.reference, item_id).await {
        Ok(Some(detail)) => Json(json!({ "data": detail })).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "No module with this item id is known to MutaMarket.",
        ),
        Err(error) => database_error(error),
    }
}

async fn module_index(state: &AppState, query: &str) -> Response {
    let Some(type_option) = type_option(query) else {
        return error(StatusCode::NOT_FOUND, "Please provide a valid type.");
    };

    let (type_id, _) = match queries::find_type(&state.pool, &type_option).await {
        Ok(Some(found)) => found,
        Ok(None) => return error(StatusCode::NOT_FOUND, "Please provide a valid type."),
        Err(error) => return database_error(error),
    };

    match queries::modules_of_type(&state.pool, &state.reference, type_id, MODULES_PAGE_SIZE).await
    {
        Ok(modules) => Json(json!({ "data": modules })).into_response(),
        Err(error) => database_error(error),
    }
}

/// `GET /api/estimator-statistics`
pub async fn estimator_statistics(State(pool): State<PgPool>) -> Response {
    let rows = sqlx::query(
        "select id, type_id, name, data_count, r2, mae, last_trained_at::text as last_trained_at,
                data_statistics
         from estimator_statistics
         order by id",
    )
    .fetch_all(&pool)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return database_error(error),
    };

    let statistics: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "type_id": row.get::<i64, _>("type_id"),
                "name": row.get::<String, _>("name"),
                "data_count": row.get::<i64, _>("data_count"),
                "r2": row.get::<Option<f64>, _>("r2"),
                "mae": row.get::<Option<f64>, _>("mae"),
                "last_trained_at": row.get::<Option<String>, _>("last_trained_at"),
                "data_statistics": row.get::<Option<serde_json::Value>, _>("data_statistics"),
            })
        })
        .collect();

    Json(statistics).into_response()
}

#[derive(Deserialize, Default)]
struct StoreModulePayload {
    message: Option<String>,
    type_id: Option<i64>,
    item_id: Option<i64>,
}

/// `POST /api/modules` — import a module from EVE by item link message or
/// explicit type and item id, fetching its rolled attributes from ESI.
/// Mirrors the legacy controller: an already-known module is returned
/// without a refetch.
pub async fn store_module(State(state): State<AppState>, body: Bytes) -> Response {
    let payload: StoreModulePayload = serde_json::from_slice(&body).unwrap_or_default();

    if let Some(validation_error) = validate_store_payload(&payload) {
        return validation_error;
    }

    // A message takes precedence and must contain an item link; explicit
    // ids are used as given.
    let (type_id, item_id) = match &payload.message {
        Some(message) => match ModuleLink::first_from(message) {
            Some(link) => (Some(link.type_id), Some(link.item_id)),
            None => (None, None),
        },
        None => (payload.type_id, payload.item_id),
    };

    let (Some(type_id), Some(item_id)) = (type_id, item_id) else {
        return error(StatusCode::BAD_REQUEST, "Failed to add module!");
    };

    if let Err(import_error) =
        import_module(&state.pool, &state.reference, &state.esi, type_id, item_id).await
    {
        eprintln!("module import failed for {type_id}/{item_id}: {import_error}");
        return error(StatusCode::BAD_REQUEST, "Failed to add module!");
    }

    show_module(&state, item_id).await
}

/// The legacy `required_without` validation rules, with Laravel's response
/// shape: a 422 carrying the first error as `message` plus per-field
/// `errors`.
fn validate_store_payload(payload: &StoreModulePayload) -> Option<Response> {
    let mut errors = serde_json::Map::new();

    if payload.message.is_none() && payload.item_id.is_none() {
        errors.insert(
            "message".to_owned(),
            json!(["The message field is required when item id is not present."]),
        );
        errors.insert(
            "item_id".to_owned(),
            json!(["The item id field is required when message is not present."]),
        );
    }

    if payload.message.is_none() && payload.type_id.is_none() {
        errors.insert(
            "type_id".to_owned(),
            json!(["The type id field is required when message is not present."]),
        );
    }

    if errors.is_empty() {
        return None;
    }

    let first_message = errors
        .values()
        .next()
        .and_then(|messages| messages[0].as_str())
        .unwrap_or("The given data was invalid.")
        .to_owned();

    Some(
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "message": first_message, "errors": errors })),
        )
            .into_response(),
    )
}

/// The `type/{id-or-slug}` option from a filter query path.
fn type_option(query: &str) -> Option<String> {
    let mut segments = query.split('/').filter(|segment| !segment.is_empty());

    while let Some(segment) = segments.next() {
        if segment == "type" {
            return segments.next().map(str::to_owned);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::type_option;

    #[test]
    fn type_options_parse_from_filter_queries() {
        assert_eq!(type_option("type/47408"), Some("47408".to_owned()));
        assert_eq!(
            type_option("sort/price/asc/type/abyssal-ballistic-control-system"),
            Some("abyssal-ballistic-control-system".to_owned()),
        );
        assert_eq!(type_option("sort/price/asc"), None);
        assert_eq!(type_option(""), None);
    }
}
