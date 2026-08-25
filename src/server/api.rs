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
use crate::modules::search::{SearchError, Visibility};
use crate::modules::view::{FilterPanelData, ModuleDetail, SearchFailure, module_id_from_slug};

/// Modules per index page, like the legacy cursor pagination.
const MODULES_PAGE_SIZE: i64 = 100;

/// Modules shown on the browser page, the legacy home page size.
const BROWSER_PAGE_SIZE: i64 = 30;

pub(super) fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

pub(super) fn database_error(error: sqlx::Error) -> Response {
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
#[derive(serde::Deserialize, Default)]
pub struct IndexParams {
    cursor: Option<String>,
}

pub async fn modules_show_or_index(
    State(state): State<AppState>,
    Path(query): Path<String>,
    axum::extract::Query(params): axum::extract::Query<IndexParams>,
) -> Response {
    match module_id_from_slug(&query) {
        Some(item_id) => show_module(&state, item_id).await,
        None => module_index(&state, &query, params.cursor.as_deref()).await,
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

/// The opaque pagination cursor: legacy encodes a keyset pointer, we
/// encode the offset — same contract (clients treat cursors as opaque and
/// follow `links.next`), documented divergence.
fn decode_cursor(cursor: Option<&str>) -> i64 {
    use base64::Engine;

    cursor
        .and_then(|cursor| base64::engine::general_purpose::STANDARD.decode(cursor).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value["offset"].as_i64())
        .unwrap_or(0)
        .max(0)
}

fn encode_cursor(offset: i64) -> String {
    use base64::Engine;

    base64::engine::general_purpose::STANDARD
        .encode(json!({ "offset": offset, "_pointsToNextItems": true }).to_string())
}

async fn module_index(state: &AppState, query: &str, cursor: Option<&str>) -> Response {
    let search = match crate::modules::search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(SearchError::TypeNotFound) => {
            return error(StatusCode::NOT_FOUND, "Please provide a valid type.");
        }
        Err(SearchError::Invalid(message)) => return error(StatusCode::BAD_REQUEST, &message),
        Err(SearchError::Db(db_error)) => return database_error(db_error),
    };

    // The legacy index requires a type option in the query path.
    if search.type_filter.is_none() {
        return error(StatusCode::NOT_FOUND, "Please provide a valid type.");
    }

    let offset = decode_cursor(cursor);
    // One extra row detects whether a next page exists.
    let mut ids = match crate::modules::search::module_ids_page(
        &state.pool,
        &search,
        Visibility::ForSale,
        MODULES_PAGE_SIZE + 1,
        offset,
    )
    .await
    {
        Ok(ids) => ids,
        Err(db_error) => return database_error(db_error),
    };
    let has_next = ids.len() as i64 > MODULES_PAGE_SIZE;
    ids.truncate(MODULES_PAGE_SIZE as usize);

    let path = format!("/api/modules/{query}");
    let next_cursor = has_next.then(|| encode_cursor(offset + MODULES_PAGE_SIZE));
    let prev_cursor = (offset > 0).then(|| encode_cursor((offset - MODULES_PAGE_SIZE).max(0)));

    match queries::details_for(&state.pool, &state.reference, ids).await {
        Ok(modules) => Json(json!({
            "data": modules,
            "links": {
                "first": serde_json::Value::Null,
                "last": serde_json::Value::Null,
                "prev": prev_cursor.as_ref().map(|cursor| format!("{path}?cursor={cursor}")),
                "next": next_cursor.as_ref().map(|cursor| format!("{path}?cursor={cursor}")),
            },
            "meta": {
                "path": path,
                "per_page": MODULES_PAGE_SIZE,
                "next_cursor": next_cursor,
                "prev_cursor": prev_cursor,
            },
        }))
        .into_response(),
        Err(db_error) => database_error(db_error),
    }
}

/// `GET /api/estimator-statistics` — the raw model serialization of every
/// row (`EstimatorStatistic::all()`), so every column is a key, including
/// nmae and the timestamps.
pub async fn estimator_statistics(State(pool): State<PgPool>) -> Response {
    let rows = sqlx::query(
        "select id, type_id, name, data_count, r2, mae, nmae,
                last_trained_at::text as last_trained_at,
                data_statistics,
                created_at::text as created_at,
                updated_at::text as updated_at
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
                "nmae": row.get::<Option<f64>, _>("nmae"),
                "last_trained_at": row.get::<Option<String>, _>("last_trained_at"),
                "data_statistics": row.get::<Option<serde_json::Value>, _>("data_statistics"),
                "created_at": row.get::<Option<String>, _>("created_at"),
                "updated_at": row.get::<Option<String>, _>("updated_at"),
            })
        })
        .collect();

    Json(statistics).into_response()
}

/// `GET /api/abyssal-type-statistics` — the per-abyssal-type roll extremes
/// with their attribute (and unit) and type (and meta group) loaded, exactly
/// like the legacy controller's eager loadout. The legacy response is the
/// bare resource array (no `data` wrapper), ordered by id; `meta_level` is
/// absent because Laravel's `whenHas` checks model attributes, never the
/// loaded relation.
pub async fn abyssal_type_statistics(State(pool): State<PgPool>) -> Response {
    let rows = sqlx::query(
        "select s.id, s.type_id, s.attribute_id, s.best, s.worst,
                s.high_is_good, s.is_virtual,
                a.name as attribute_name, a.display_name as attribute_display_name,
                a.high_is_good as attribute_high_is_good, a.derived as attribute_derived,
                u.id as unit_id, u.name as unit_name, u.display_name as unit_display_name,
                t.name as type_name, t.published as type_published,
                t.meta_group_id, mg.name as meta_group_name
         from abyssal_type_statistics s
         join attributes a on a.id = s.attribute_id
         left join units u on u.id = a.unit_id
         join types t on t.id = s.type_id
         left join meta_groups mg on mg.id = t.meta_group_id
         order by s.id",
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
            let unit = row.get::<Option<i64>, _>("unit_id").map(|unit_id| {
                json!({
                    "id": unit_id,
                    "name": row.get::<String, _>("unit_name"),
                    "display_name": row.get::<String, _>("unit_display_name"),
                })
            });

            json!({
                "id": row.get::<i64, _>("id"),
                "type_id": row.get::<i64, _>("type_id"),
                "attribute_id": row.get::<i64, _>("attribute_id"),
                "high_is_good": row.get::<bool, _>("high_is_good"),
                "is_virtual": row.get::<bool, _>("is_virtual"),
                "best": row.get::<f64, _>("best"),
                "worst": row.get::<f64, _>("worst"),
                "is_derived": row.get::<bool, _>("attribute_derived"),
                "attribute": {
                    "id": row.get::<i64, _>("attribute_id"),
                    "name": row.get::<String, _>("attribute_name"),
                    "display_name": row.get::<String, _>("attribute_display_name"),
                    "high_is_good": row.get::<bool, _>("attribute_high_is_good"),
                    "is_derived": row.get::<bool, _>("attribute_derived"),
                    "unit": unit,
                },
                "type": {
                    "id": row.get::<i64, _>("type_id"),
                    "name": row.get::<String, _>("type_name"),
                    "meta_group": row.get::<Option<String>, _>("meta_group_name"),
                    "meta_group_id": row.get::<Option<i64>, _>("meta_group_id"),
                    "published": row.get::<bool, _>("type_published"),
                },
            })
        })
        .collect();

    Json(statistics).into_response()
}

/// The modules matching a filter query path, with full card data. The
/// browser shows the for-sale set like the legacy home; `unlisted=true`
/// (the all-modules page) includes modules not currently for sale.
#[derive(Deserialize, Default)]
pub struct CardsParams {
    unlisted: Option<bool>,
}

/// `GET /api/module-cards` — the unfiltered browser card set.
pub async fn module_cards_root(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<CardsParams>,
) -> Response {
    cards_response(&state, "", params.unlisted.unwrap_or(false)).await
}

/// `GET /api/module-cards/{query}` — the card set for a filter query path.
pub async fn module_cards(
    State(state): State<AppState>,
    Path(query): Path<String>,
    axum::extract::Query(params): axum::extract::Query<CardsParams>,
) -> Response {
    cards_response(&state, &query, params.unlisted.unwrap_or(false)).await
}

async fn cards_response(state: &AppState, query: &str, include_unlisted: bool) -> Response {
    match search_module_cards(state, query, include_unlisted).await {
        Ok(Ok(modules)) => Json(modules).into_response(),
        Ok(Err(failure)) => error(
            if failure.not_found {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            },
            &failure.message,
        ),
        Err(db_error) => database_error(db_error),
    }
}

/// The browser card query shared with the Leptos server function: the
/// matching modules, or the user-facing failure with its legacy message.
pub async fn search_module_cards(
    state: &AppState,
    query: &str,
    include_unlisted: bool,
) -> sqlx::Result<Result<Vec<ModuleDetail>, SearchFailure>> {
    let search = match crate::modules::search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(SearchError::TypeNotFound) => {
            return Ok(Err(SearchFailure {
                message: "Please provide a valid type.".to_owned(),
                not_found: true,
            }));
        }
        Err(SearchError::Invalid(message)) => {
            return Ok(Err(SearchFailure { message, not_found: false }));
        }
        Err(SearchError::Db(error)) => return Err(error),
    };

    let visibility = if include_unlisted { Visibility::All } else { Visibility::ForSale };
    let ids =
        crate::modules::search::module_ids(&state.pool, &search, visibility, BROWSER_PAGE_SIZE)
            .await?;

    queries::details_for(&state.pool, &state.reference, ids).await.map(Ok)
}

/// `GET /api/module-stats` — market-wide statistics for the browser
/// header, the legacy `getAllModulesStats`.
pub async fn module_stats(State(state): State<AppState>) -> Response {
    match crate::modules::stats::all_modules_stats(&state.pool).await {
        Ok(stats) => Json(stats).into_response(),
        Err(db_error) => database_error(db_error),
    }
}

/// `GET /api/filter-panel/{type}` — the slider bounds for each mutated
/// attribute of a type, resolved like the search's type segment.
pub async fn filter_panel(
    State(state): State<AppState>,
    Path(type_slug): Path<String>,
) -> Response {
    match filter_panel_data(&state, &type_slug).await {
        Ok(Some(panel)) => Json(panel).into_response(),
        Ok(None) => error(StatusCode::NOT_FOUND, "Please provide a valid type."),
        Err(SearchError::Db(db_error)) => database_error(db_error),
        Err(SearchError::TypeNotFound) => error(StatusCode::NOT_FOUND, "Please provide a valid type."),
        Err(SearchError::Invalid(message)) => error(StatusCode::BAD_REQUEST, &message),
    }
}

/// The filter panel data shared with the Leptos server function; `None`
/// marks an unknown type.
pub async fn filter_panel_data(
    state: &AppState,
    type_slug: &str,
) -> Result<Option<FilterPanelData>, SearchError> {
    let type_filter = match crate::modules::search::resolve_type(&state.pool, type_slug).await {
        Ok(type_filter) => type_filter,
        Err(SearchError::TypeNotFound) => return Ok(None),
        Err(error) => return Err(error),
    };

    let attributes = queries::type_filter_attributes(&state.pool, type_filter.id)
        .await
        .map_err(SearchError::Db)?;

    Ok(Some(FilterPanelData {
        type_id: type_filter.id,
        type_name: type_filter.name,
        attributes,
    }))
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

    if let Err(import_error) = import_module(
        &state.pool,
        &state.reference,
        &state.esi,
        &state.estimator,
        type_id,
        item_id,
    )
    .await
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

