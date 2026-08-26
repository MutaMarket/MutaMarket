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

/// Whether the request carries an admin session; guests, plain users
/// and database hiccups all read `false` (the legacy `$request->user()
/// ?->is_admin` null chain).
async fn requester_is_admin(state: &AppState, headers: &axum::http::HeaderMap) -> bool {
    let Ok(Some(session)) =
        crate::auth::session::session_from_headers(&state.pool, headers).await
    else {
        return false;
    };
    sqlx::query_scalar("select is_admin from users where id = $1")
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// The module's finished contracts, newest first: the contract-history
/// tab's rows (legacy `$module->historicContracts()->with('issuer')`).
/// `ignore_for_training` rides along for admins only, like the legacy
/// resource.
async fn module_historic_contracts(
    pool: &PgPool,
    item_id: i64,
    for_admin: bool,
) -> sqlx::Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        "select distinct on (hc.id)
                hc.id, hc.type, hc.unified_price as price, hc.asking_for_items,
                hc.plex_count, hc.non_abyssal_modules_count,
                hc.abyssal_modules_count, hc.status, hc.ignore_for_training,
                hc.date_issued::text as date_issued,
                hc.date_expired::text as date_expired,
                ic.id as issuer_id, ic.name as issuer_name,
                ic.description as issuer_description,
                ic.corporation_id as issuer_corporation_id,
                (ic.premium_paid_until is not null and ic.premium_paid_until > now())
                    as issuer_has_premium
         from historic_contract_items hci
         join historic_contracts hc on hc.id = hci.historic_contract_id
         left join characters ic on ic.id = hc.issuer_id
         where hci.item_id = $1
         order by hc.id desc",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let issuer = row.get::<Option<i64>, _>("issuer_id").map(|issuer_id| {
                let name: String =
                    row.get::<Option<String>, _>("issuer_name").unwrap_or_default();
                json!({
                    "id": issuer_id,
                    "slug": crate::modules::view::module_slug(&name, issuer_id),
                    "name": name,
                    "description": row.get::<Option<String>, _>("issuer_description"),
                    "has_premium": row
                        .get::<Option<bool>, _>("issuer_has_premium")
                        .unwrap_or(false),
                    "corporation_id": row.get::<Option<i64>, _>("issuer_corporation_id"),
                })
            });

            let mut contract = json!({
                "id": row.get::<i64, _>("id"),
                "type": row.get::<String, _>("type"),
                "price": row.get::<Option<f64>, _>("price"),
                "asking_for_items": row.get::<bool, _>("asking_for_items"),
                "plex_count": row.get::<i32, _>("plex_count"),
                "non_abyssal_modules_count": row.get::<i32, _>("non_abyssal_modules_count"),
                "abyssal_modules_count": row.get::<i32, _>("abyssal_modules_count"),
                "issuer": issuer,
                "status": row.get::<String, _>("status"),
                "date_issued": row.get::<Option<String>, _>("date_issued"),
                "date_expired": row.get::<Option<String>, _>("date_expired"),
            });
            if for_admin {
                contract["ignore_for_training"] =
                    json!(row.get::<bool, _>("ignore_for_training"));
            }
            contract
        })
        .collect())
}

/// The similar-sold tab shows the nearest sold rolls, the legacy
/// `Module::similarModules` default limit.
const SIMILAR_MODULES_LIMIT: i64 = 8;

/// `GET /api/module-page/{module}/similar` — the similar-sold tab data,
/// the legacy deferred `similar_modules` prop: premium accounts get the
/// nearest sold modules of the same type by attribute distance, everyone
/// else an empty list (the frontend shows the blurred teaser).
pub async fn module_similar(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(query): Path<String>,
) -> Response {
    let Some(item_id) = module_id_from_slug(&query) else {
        return error(
            StatusCode::NOT_FOUND,
            "No module with this item id is known to MutaMarket.",
        );
    };

    let type_id: Option<i64> = match sqlx::query_scalar("select type_id from modules where id = $1")
        .bind(item_id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(type_id) => type_id,
        Err(db_error) => return database_error(db_error),
    };
    let Some(type_id) = type_id else {
        return error(
            StatusCode::NOT_FOUND,
            "No module with this item id is known to MutaMarket.",
        );
    };

    let has_premium = match crate::auth::session::session_from_headers(&state.pool, &headers).await
    {
        Ok(Some(session)) => {
            let premium: Result<Option<bool>, _> = sqlx::query_scalar(
                "select exists (select 1 from characters
                                where user_id = $1 and premium_paid_until > now())",
            )
            .bind(session.user_id)
            .fetch_optional(&state.pool)
            .await;
            match premium {
                Ok(premium) => premium.unwrap_or(false),
                Err(db_error) => return database_error(db_error),
            }
        }
        Ok(None) => false,
        Err(db_error) => return database_error(db_error),
    };
    if !has_premium {
        return Json(json!({ "similar_modules": [] })).into_response();
    }

    /// (module_id, historic_contract_id, sold_for, sold_at).
    type NeighborRow = (i64, i64, Option<f64>, Option<String>);

    // Euclidean distance over the non-virtual, non-derived roll fractions
    // (legacy addSelect distance). Rolls without comparable attributes
    // (null distance) sort last under Postgres, a documented divergence
    // from MySQL's nulls-first ascending order.
    let neighbors: Result<Vec<NeighborRow>, _> = sqlx::query_as(
        "select m.id, tm.historic_contract_id, hc.unified_price as sold_for,
                hc.date_issued::text as sold_at
         from modules m
         join training_modules tm on tm.module_id = m.id
         join historic_contracts hc on hc.id = tm.historic_contract_id
         where m.type_id = $2 and m.id <> $1
         order by (select sum(power(ma.fraction_absolute - src.fraction_absolute, 2))
                   from mutated_attributes ma
                   join mutated_attributes src
                     on src.attribute_id = ma.attribute_id and src.module_id = $1
                   join attributes a on a.id = ma.attribute_id
                   where ma.module_id = m.id and not ma.is_virtual and not a.derived)
         limit $3",
    )
    .bind(item_id)
    .bind(type_id)
    .bind(SIMILAR_MODULES_LIMIT)
    .fetch_all(&state.pool)
    .await;
    let neighbors = match neighbors {
        Ok(neighbors) => neighbors,
        Err(db_error) => return database_error(db_error),
    };

    let mut similar = Vec::with_capacity(neighbors.len());
    for (module_id, contract_id, sold_for, sold_at) in neighbors {
        let module = match queries::module_detail(&state.pool, &state.reference, module_id).await
        {
            Ok(Some(module)) => module,
            Ok(None) => continue,
            Err(db_error) => return database_error(db_error),
        };
        let mut entry = serde_json::to_value(&module).expect("module serializes");
        entry["training_module"] = json!({
            "contract_id": contract_id,
            "sold_for": sold_for,
            "sold_at": sold_at,
        });
        similar.push(entry);
    }

    Json(json!({ "similar_modules": similar })).into_response()
}

use crate::modules::META_LEVEL_ATTRIBUTE_ID;

/// The source-type table's meta group display order, the legacy
/// META_GROUP_SORT_ORDER (Deadspace before Officer).
fn meta_group_rank(meta_group_id: Option<i64>) -> i64 {
    match meta_group_id {
        Some(1) => 1,
        Some(2) => 2,
        Some(3) => 3,
        Some(4) => 4,
        Some(6) => 5,
        Some(5) => 6,
        Some(other) => other,
        None => i64::MAX,
    }
}

/// The published input types of the module's mutaplasmid with their base
/// values for the module's mutated attributes, meta level and latest
/// market average — the source-type comparison table's data, computed
/// from the reference tables instead of the legacy client-bundled
/// statics (`specs/module-show.md` §4).
async fn source_type_comparisons(
    pool: &PgPool,
    module: &ModuleDetail,
) -> sqlx::Result<Vec<serde_json::Value>> {
    let Some(mutaplasmid) = &module.mutaplasmid else {
        return Ok(Vec::new());
    };

    let types: Vec<(i64, String, Option<i64>, Option<f64>)> = sqlx::query_as(
        "select t.id, t.name, t.meta_group_id, ml.value as meta_level
         from mutaplasmid_input_types mit
         join types t on t.id = mit.type_id and t.published
         left join type_attributes ml
             on ml.type_id = t.id and ml.attribute_id = $2
         where mit.mutaplasmid_id = $1",
    )
    .bind(mutaplasmid.id)
    .bind(META_LEVEL_ATTRIBUTE_ID)
    .fetch_all(pool)
    .await?;

    let type_ids: Vec<i64> = types.iter().map(|(id, ..)| *id).collect();
    let attribute_ids: Vec<i64> =
        module.mutated_attributes.iter().map(|attribute| attribute.attribute_id).collect();

    let values: Vec<(i64, i64, Option<f64>)> = sqlx::query_as(
        "select type_id, attribute_id, value from type_attributes
         where type_id = any($1) and attribute_id = any($2)",
    )
    .bind(&type_ids)
    .bind(&attribute_ids)
    .fetch_all(pool)
    .await?;
    let value_of: std::collections::HashMap<(i64, i64), Option<f64>> = values
        .into_iter()
        .map(|(type_id, attribute_id, value)| ((type_id, attribute_id), value))
        .collect();

    let prices: Vec<(i64, f64)> = sqlx::query_as(
        "select distinct on (type_id) type_id, average from market_histories
         where type_id = any($1) order by type_id, date desc",
    )
    .bind(&type_ids)
    .fetch_all(pool)
    .await?;
    let price_of: std::collections::HashMap<i64, f64> = prices.into_iter().collect();

    let mut comparisons: Vec<(i64, i64, String, serde_json::Value)> = types
        .into_iter()
        .map(|(id, name, meta_group_id, meta_level)| {
            let attributes: Vec<serde_json::Value> = attribute_ids
                .iter()
                .map(|attribute_id| {
                    json!({
                        "id": attribute_id,
                        // The legacy comparison falls back to 0 for
                        // attributes the input type does not carry.
                        "value": value_of
                            .get(&(id, *attribute_id))
                            .copied()
                            .flatten()
                            .unwrap_or(0.0),
                    })
                })
                .collect();

            let meta_level = meta_level.map(|level| level as i64);
            let item = json!({
                "type": {
                    "id": id,
                    "name": &name,
                    "meta_group_id": meta_group_id,
                    "meta_level": meta_level,
                },
                "attributes": attributes,
                "average_price": price_of.get(&id),
            });

            (meta_group_rank(meta_group_id), meta_level.unwrap_or(0), name, item)
        })
        .collect();

    // The legacy default order: meta group rank, meta level, name.
    comparisons.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

    Ok(comparisons.into_iter().map(|(.., item)| item).collect())
}

/// `GET /api/module-page/{module}` — the show page payload: the module
/// plus its type's estimator statistic sheet (`null` when the type has
/// no trained statistic row), per `specs/module-show.md` §1.
pub async fn module_page(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(query): Path<String>,
) -> Response {
    let Some(item_id) = module_id_from_slug(&query) else {
        return error(
            StatusCode::NOT_FOUND,
            "No module with this item id is known to MutaMarket.",
        );
    };

    let module = match queries::module_detail(&state.pool, &state.reference, item_id).await {
        Ok(Some(module)) => module,
        Ok(None) => {
            return error(
                StatusCode::NOT_FOUND,
                "No module with this item id is known to MutaMarket.",
            );
        }
        Err(db_error) => return database_error(db_error),
    };

    let statistic = sqlx::query(
        "select r2, mae, nmae, data_count, data_statistics,
                last_trained_at::text as last_trained_at
         from estimator_statistics where type_id = $1",
    )
    .bind(module.r#type.id)
    .fetch_optional(&state.pool)
    .await;
    let statistic = match statistic {
        Ok(row) => row.map(|row| {
            json!({
                "r2": row.get::<Option<f64>, _>("r2"),
                "mae": row.get::<Option<f64>, _>("mae"),
                "nmae": row.get::<Option<f64>, _>("nmae"),
                "data_count": row.get::<i64, _>("data_count"),
                "data_statistics": row.get::<Option<serde_json::Value>, _>("data_statistics"),
                "last_trained_at": row.get::<Option<String>, _>("last_trained_at"),
            })
        }),
        Err(db_error) => return database_error(db_error),
    };

    let comparisons = match source_type_comparisons(&state.pool, &module).await {
        Ok(comparisons) => comparisons,
        Err(db_error) => return database_error(db_error),
    };

    let for_admin = requester_is_admin(&state, &headers).await;
    let historic = match module_historic_contracts(&state.pool, item_id, for_admin).await {
        Ok(historic) => historic,
        Err(db_error) => return database_error(db_error),
    };

    // The type's roll extremes, feeding the search-menu variance bounds
    // (legacy page prop `abyssal_type_statistics`, trimmed to the fields
    // the ModuleFinder uses).
    /// (attribute_id, best, worst, high_is_good, is_virtual).
    type StatisticRow = (i64, f64, f64, bool, bool);
    let statistics: Result<Vec<StatisticRow>, _> = sqlx::query_as(
        "select attribute_id, best, worst, high_is_good, is_virtual
         from abyssal_type_statistics where type_id = $1 order by attribute_id",
    )
    .bind(module.r#type.id)
    .fetch_all(&state.pool)
    .await;
    let type_statistics = match statistics {
        Ok(rows) => rows
            .into_iter()
            .map(|(attribute_id, best, worst, high_is_good, is_virtual)| {
                json!({
                    "attribute_id": attribute_id,
                    "best": best,
                    "worst": worst,
                    "high_is_good": high_is_good,
                    "is_virtual": is_virtual,
                })
            })
            .collect::<Vec<_>>(),
        Err(db_error) => return database_error(db_error),
    };

    Json(json!({
        "module": module,
        "estimator_statistic": statistic,
        "source_type_comparisons": comparisons,
        "historic_contracts": historic,
        "abyssal_type_statistics": type_statistics,
    }))
    .into_response()
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
/// header, the legacy `getAllModulesStats`. `unlisted=true` (the
/// all-modules page) counts the bar totals across the whole archive
/// instead of only for-sale modules.
pub async fn module_stats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<CardsParams>,
) -> Response {
    let unlisted = params.unlisted.unwrap_or(false);
    match crate::modules::stats::all_modules_stats(&state.pool, unlisted).await {
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

    let attribute_ids: Vec<i64> =
        attributes.iter().map(|attribute| attribute.attribute_id).collect();
    let source_types =
        queries::type_filter_source_types(&state.pool, type_filter.id, &attribute_ids)
            .await
            .map_err(SearchError::Db)?;

    Ok(Some(FilterPanelData {
        type_id: type_filter.id,
        type_name: type_filter.name,
        attributes,
        source_types,
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

