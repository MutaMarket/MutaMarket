//! The asset-location pages (legacy `LocationController` and
//! `LocationCollectionController`): the tree of stations, structures
//! and containers holding the account's abyssal modules, the
//! per-location module browser, and the create-a-collection-from-a-
//! location action.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};

use super::AppState;
use super::support::require_api_session;
use crate::auth::session;
use crate::modules::search::{Scope, SearchError};
use crate::modules::view::slugify;

/// The browse pages' card page size (matches the module browser).
const LOCATION_PAGE_SIZE: i64 = 48;

/// The `holding` membership: every asset item of the user that is an
/// abyssal module or an ancestor container/ship of one (the legacy
/// `whereHas('descendantsAndSelf', is_abyssal)`).
const HOLDING_CTE: &str = "
    with recursive holding as (
        select a.item_id, a.location_id from assets a
        join characters ch on ch.id = a.character_id
        where ch.user_id = $1 and a.is_abyssal
        union
        select p.item_id, p.location_id from assets p
        join characters ch on ch.id = p.character_id
        join holding h on h.location_id = p.item_id
        where ch.user_id = $1
    )";

/// `GET /api/locations` — the tree page data: containers and ships
/// holding abyssals, the root stations/structures, and the per-location
/// module counts (legacy `LocationController::index`).
pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    match index_payload(&state.pool, session.user_id).await {
        Ok(payload) => axum::Json(payload).into_response(),
        Err(error) => super::api::database_error(error),
    }
}

async fn index_payload(pool: &PgPool, user_id: i64) -> sqlx::Result<serde_json::Value> {
    // The container/ship rows: the user's non-abyssal assets holding
    // abyssals somewhere below (legacy LocationResource collection).
    let locations = sqlx::query(&format!(
        "{HOLDING_CTE}
         select a.item_id, a.type_id, t.name as type_name, a.name,
                a.location_id, a.location_type, a.location_flag, a.index,
                a.character_id, a.corporation_id
         from assets a
         join characters ch on ch.id = a.character_id
         join types t on t.id = a.type_id
         where ch.user_id = $1 and not a.is_abyssal
           and a.item_id in (select item_id from holding)",
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    // The tree roots: locations of holding assets that are not
    // themselves user assets (stations and structures).
    let root_ids: Vec<i64> = sqlx::query_scalar(&format!(
        "{HOLDING_CTE}
         select distinct a.location_id from assets a
         join characters ch on ch.id = a.character_id
         where ch.user_id = $1
           and a.item_id in (select item_id from holding)
           and a.location_id is not null
           and not exists (
               select 1 from assets p
               join characters pch on pch.id = p.character_id
               where pch.user_id = $1 and p.item_id = a.location_id)",
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let stations: Vec<(i64, String, Option<i64>)> =
        sqlx::query_as("select id, name, type_id from stations where id = any($1)")
            .bind(&root_ids)
            .fetch_all(pool)
            .await?;
    // Unresolved structures keep a null name until the resolver sweep
    // or an asset sync names them; they still root the tree.
    let structures: Vec<(i64, Option<String>, Option<i64>)> =
        sqlx::query_as("select id, name, type_id from structures where id = any($1)")
            .bind(&root_ids)
            .fetch_all(pool)
            .await?;
    // Roots known to neither table (e.g. structures of legacy-imported
    // assets that no sync has touched yet) appear as placeholders
    // instead of swallowing their whole subtree.
    let known: std::collections::HashSet<i64> = stations
        .iter()
        .map(|(id, ..)| *id)
        .chain(structures.iter().map(|(id, ..)| *id))
        .collect();
    let placeholders: Vec<(i64, Option<String>, Option<i64>)> = root_ids
        .iter()
        .filter(|id| !known.contains(id))
        .map(|id| (*id, None, None))
        .collect();

    let counts: Vec<(i64, i64)> = sqlx::query_as(
        "select a.location_id, count(*) from assets a
         join characters ch on ch.id = a.character_id
         where ch.user_id = $1 and a.is_abyssal and a.location_id is not null
         group by a.location_id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let location_rows: Vec<serde_json::Value> = locations
        .iter()
        .map(|row| {
            let name: Option<String> = row.get("name");
            let type_name: String = row.get("type_name");
            // The legacy slug: the asset name, or the type name for
            // unnamed containers.
            let name_slug = match name.as_deref().map(slugify).filter(|slug| !slug.is_empty()) {
                Some(slug) => slug,
                None => slugify(&type_name),
            };
            json!({
                "id": row.get::<i64, _>("item_id"),
                "type": { "id": row.get::<i64, _>("type_id"), "name": type_name },
                "name": name,
                "location": {
                    "id": row.get::<Option<i64>, _>("location_id"),
                    "type": row.get::<String, _>("location_type"),
                    "flag": row.get::<String, _>("location_flag"),
                    "index": row.get::<i64, _>("index"),
                },
                "character_id": row.get::<i64, _>("character_id"),
                "corporation_id": row.get::<Option<i64>, _>("corporation_id"),
                "slug": format!("{name_slug}-{}", row.get::<i64, _>("item_id")),
            })
        })
        .collect();

    let station_rows: Vec<serde_json::Value> = stations
        .iter()
        .map(|(id, name, type_id)| {
            json!({
                "id": id,
                "type_id": type_id,
                "name": name,
                "slug": format!("{}-{id}", slugify(name)),
            })
        })
        .collect();
    let structure_rows: Vec<serde_json::Value> = structures
        .iter()
        .chain(placeholders.iter())
        .map(|(id, name, type_id)| {
            let slug = match name.as_deref().map(slugify).filter(|slug| !slug.is_empty()) {
                Some(slug) => format!("{slug}-{id}"),
                None => format!("unknown-structure-{id}"),
            };
            json!({
                "id": id,
                "type_id": type_id,
                "name": name,
                "slug": slug,
            })
        })
        .collect();

    Ok(json!({
        "locations": location_rows,
        "stations": station_rows,
        "structures": structure_rows,
        "location_modules_count": counts
            .into_iter()
            .map(|(location_id, count)| (location_id.to_string(), json!(count)))
            .collect::<serde_json::Map<String, serde_json::Value>>(),
    }))
}

pub async fn show_root(
    State(state): State<AppState>,
    axum::extract::Path(location): axum::extract::Path<String>,
    headers: HeaderMap,
) -> Response {
    show_response(&state, &headers, &location, "").await
}

/// `GET /api/locations/{location}/{query?}` — the per-location module
/// browser (legacy `LocationController::show`).
pub async fn show(
    State(state): State<AppState>,
    axum::extract::Path((location, query)): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    show_response(&state, &headers, &location, &query).await
}

/// The legacy route-segment parsing: the trailing dash part is the id.
fn location_id_from_slug(slug: &str) -> i64 {
    slug.rsplit('-')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

async fn show_response(
    state: &AppState,
    headers: &HeaderMap,
    location_slug: &str,
    query: &str,
) -> Response {
    let session = match require_api_session(&state.pool, headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let location_id = location_id_from_slug(location_slug);

    let location = match resolve_location(&state.pool, session.user_id, location_id).await {
        Ok(Some(location)) => location,
        Ok(None) => {
            return super::api::error(StatusCode::NOT_FOUND, "This location does not exist.");
        }
        Err(error) => return super::api::database_error(error),
    };

    let search = match crate::modules::search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(SearchError::TypeNotFound) => {
            return super::api::error(StatusCode::NOT_FOUND, "Please provide a valid type.");
        }
        Err(SearchError::Invalid(message)) => {
            return super::api::error(StatusCode::BAD_REQUEST, &message);
        }
        Err(SearchError::Db(error)) => return super::api::database_error(error),
    };

    let ids = match crate::modules::search::scoped_module_ids(
        &state.pool,
        &search,
        Scope::InLocation {
            location_id,
            user_id: session.user_id,
        },
        LOCATION_PAGE_SIZE,
    )
    .await
    {
        Ok(ids) => ids,
        Err(error) => return super::api::database_error(error),
    };
    let mut modules =
        match crate::modules::queries::details_for(&state.pool, &state.reference, ids).await {
            Ok(modules) => modules,
            Err(error) => return super::api::database_error(error),
        };
    // The legacy LocationController loads withDefaultRelations, so the
    // user's notes ride along.
    if let Err(error) =
        crate::modules::queries::attach_user_notes(&state.pool, session.user_id, &mut modules).await
    {
        return super::api::database_error(error);
    }

    let available_types: Vec<i64> = match sqlx::query_scalar(&format!(
        "{}
         select distinct m.type_id from modules m
         where m.id in (select item_id from under_location where is_abyssal)",
        under_location_cte(),
    ))
    .bind(session.user_id)
    .bind(location_id)
    .fetch_all(&state.pool)
    .await
    {
        Ok(types) => types,
        Err(error) => return super::api::database_error(error),
    };

    let stats = match location_stats(&state.pool, session.user_id, location_id).await {
        Ok(stats) => stats,
        Err(error) => return super::api::database_error(error),
    };

    axum::Json(json!({
        "location": location,
        "modules": modules,
        "available_types": available_types,
        "stats": stats,
    }))
    .into_response()
}

/// The membership of one location: every user asset directly at it or
/// nested below (the legacy `inLocation` ancestorsAndSelf walk, taken
/// downward).
fn under_location_cte() -> &'static str {
    "with recursive under_location as (
        select a.item_id, a.is_abyssal from assets a
        join characters ch on ch.id = a.character_id
        where ch.user_id = $1 and a.location_id = $2
        union
        select a.item_id, a.is_abyssal from assets a
        join characters ch on ch.id = a.character_id
        join under_location u on a.location_id = u.item_id
        where ch.user_id = $1
    )"
}

/// The legacy resolution order: station, structure, the user's own
/// asset (container/ship), then a bare type id.
async fn resolve_location(
    pool: &PgPool,
    user_id: i64,
    location_id: i64,
) -> sqlx::Result<Option<serde_json::Value>> {
    let station: Option<(i64, String, Option<i64>)> =
        sqlx::query_as("select id, name, type_id from stations where id = $1")
            .bind(location_id)
            .fetch_optional(pool)
            .await?;
    let structure: Option<(i64, Option<String>, Option<i64>)> = match station {
        Some(_) => None,
        None => {
            sqlx::query_as("select id, name, type_id from structures where id = $1")
                .bind(location_id)
                .fetch_optional(pool)
                .await?
        }
    };
    let resolved = station
        .map(|(id, name, type_id)| (id, Some(name), type_id))
        .or(structure);
    if let Some((id, name, type_id)) = resolved {
        let type_name: Option<String> = sqlx::query_scalar("select name from types where id = $1")
            .bind(type_id)
            .fetch_optional(pool)
            .await?;
        let slug = match name.as_deref().map(slugify).filter(|slug| !slug.is_empty()) {
            Some(slug) => format!("{slug}-{id}"),
            None => format!("unknown-structure-{id}"),
        };
        return Ok(Some(json!({
            "id": id,
            "type": { "id": type_id, "name": type_name },
            "name": name,
            "character_id": serde_json::Value::Null,
            "corporation_id": serde_json::Value::Null,
            "slug": slug,
        })));
    }

    let asset = sqlx::query(
        "select a.item_id, a.name, a.type_id, t.name as type_name,
                a.location_id, a.location_type, a.location_flag, a.index,
                a.character_id, a.corporation_id
         from assets a
         join characters ch on ch.id = a.character_id
         join types t on t.id = a.type_id
         where a.item_id = $1 and ch.user_id = $2",
    )
    .bind(location_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if let Some(row) = asset {
        let parent_location_id: Option<i64> = row.get("location_id");
        // The breadcrumb parent: the containing asset's type, or the
        // hosting station/structure.
        let parent: Option<(String, Option<i64>)> = match parent_location_id {
            Some(parent_id) => {
                let container: Option<(String,)> = sqlx::query_as(
                    "select t.name from assets p
                     join characters ch on ch.id = p.character_id
                     join types t on t.id = p.type_id
                     where p.item_id = $1 and ch.user_id = $2",
                )
                .bind(parent_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                match container {
                    Some((name,)) => Some((name, None)),
                    None => sqlx::query_as(
                        "select name, id from stations where id = $1
                         union all
                         select coalesce(name, ''), id from structures where id = $1
                         limit 1",
                    )
                    .bind(parent_id)
                    .fetch_optional(pool)
                    .await?
                    .map(|(name, id): (String, i64)| (name, Some(id))),
                }
            }
            None => None,
        };
        let name: Option<String> = row.get("name");
        let type_name: String = row.get("type_name");
        let name_slug = match name.as_deref().map(slugify).filter(|slug| !slug.is_empty()) {
            Some(slug) => slug,
            None => slugify(&type_name),
        };
        return Ok(Some(json!({
            "id": row.get::<i64, _>("item_id"),
            "type": { "id": row.get::<i64, _>("type_id"), "name": type_name },
            "name": name,
            "location": parent_location_id.map(|parent_id| {
                let parent_name = parent.as_ref().map(|(name, _)| name.clone()).unwrap_or_default();
                json!({
                    "id": parent_id,
                    "type": { "name": parent_name },
                    "flag": row.get::<String, _>("location_flag"),
                    "index": row.get::<i64, _>("index"),
                    "slug": format!("{}-{parent_id}", slugify(&parent_name)),
                })
            }),
            "character_id": row.get::<i64, _>("character_id"),
            "corporation_id": row.get::<Option<i64>, _>("corporation_id"),
            "slug": format!("{name_slug}-{}", row.get::<i64, _>("item_id")),
        })));
    }

    // A location the user's assets reference but no table knows yet
    // (an unresolved structure): still browsable as a placeholder.
    let referenced: bool = sqlx::query_scalar(
        "select exists (select 1 from assets a
             join characters ch on ch.id = a.character_id
             where ch.user_id = $2 and a.location_id = $1)",
    )
    .bind(location_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    if referenced {
        return Ok(Some(json!({
            "id": location_id,
            "type": { "id": serde_json::Value::Null, "name": "Structure" },
            "name": serde_json::Value::Null,
            "character_id": serde_json::Value::Null,
            "corporation_id": serde_json::Value::Null,
            "slug": format!("unknown-structure-{location_id}"),
        })));
    }

    // The legacy last resort: a bare type id renders as its own type.
    let type_row: Option<(i64, String)> =
        sqlx::query_as("select id, name from types where id = $1")
            .bind(location_id)
            .fetch_optional(pool)
            .await?;
    Ok(type_row.map(|(id, name)| {
        json!({
            "id": id,
            "type": { "id": id, "name": name },
            "name": name,
            "character_id": serde_json::Value::Null,
            "corporation_id": serde_json::Value::Null,
            "slug": format!("{}-{id}", slugify(&name)),
        })
    }))
}

/// The legacy `getLocationModulesStats`: totals over the modules inside
/// the location.
async fn location_stats(
    pool: &PgPool,
    user_id: i64,
    location_id: i64,
) -> sqlx::Result<serde_json::Value> {
    let row = sqlx::query(&format!(
        "{}
         select count(*) as total_count,
                coalesce(sum(m.estimated_value), 0) as total_value,
                coalesce(avg(m.estimated_value), 0) as average_value,
                count(*) filter (where exists (select 1 from mutated_attributes b
                    where b.module_id = m.id and b.bar = 1)) as goldbars_count,
                count(*) filter (where exists (select 1 from mutated_attributes b
                    where b.module_id = m.id and b.bar = -1)) as brownbars_count,
                count(*) filter (where exists (select 1 from mutated_attributes b
                    where b.module_id = m.id and b.bar = 2)) as diamondbars_count
         from modules m
         where m.id in (select item_id from under_location where is_abyssal)",
        under_location_cte(),
    ))
    .bind(user_id)
    .bind(location_id)
    .fetch_one(pool)
    .await?;
    Ok(json!({
        "total_count": row.get::<i64, _>("total_count"),
        "total_value": row.get::<f64, _>("total_value"),
        "average_value": row.get::<f64, _>("average_value"),
        "goldbars_count": row.get::<i64, _>("goldbars_count"),
        "brownbars_count": row.get::<i64, _>("brownbars_count"),
        "diamondbars_count": row.get::<i64, _>("diamondbars_count"),
    }))
}

#[derive(Debug, Deserialize, Default)]
pub struct LocationCollectionPayload {
    pub location_id: Option<i64>,
}

/// `POST /location-collections` — the legacy
/// `LocationCollectionController::store`: a private collection named
/// after the location, filled with its modules, then the legacy
/// redirect to the new collection.
pub async fn store_collection(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };
    let payload: LocationCollectionPayload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(location_id) = payload.location_id else {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The location id field is required.",
        );
    };

    let location = match resolve_location(&state.pool, session.user_id, location_id).await {
        Ok(Some(location)) => location,
        Ok(None) => {
            return super::api::error(StatusCode::NOT_FOUND, "This location does not exist.");
        }
        Err(error) => return super::api::database_error(error),
    };
    // The legacy name pick: the location's name, or its type's name.
    let name = location["name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .or_else(|| location["type"]["name"].as_str())
        .unwrap_or("Unknown Location")
        .to_owned();

    let character_id: Option<i64> = match session.active_character_id {
        Some(character_id) => Some(character_id),
        None => match sqlx::query_scalar(
            "select id from characters where user_id = $1 order by id limit 1",
        )
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
        {
            Ok(character_id) => character_id,
            Err(error) => return super::api::database_error(error),
        },
    };
    let Some(character_id) = character_id else {
        return Redirect::to("/locations").into_response();
    };

    let collection = match crate::collections::create_collection(
        &state.pool,
        character_id,
        &name,
        None,
        "private",
    )
    .await
    {
        Ok(collection) => collection,
        Err(error) => return super::api::database_error(error),
    };

    let inserted: Result<u64, sqlx::Error> = sqlx::query(&format!(
        "{}
         insert into collection_modules (collection_id, module_id)
         select $3, m.id from modules m
         where m.id in (select item_id from under_location where is_abyssal)
         on conflict do nothing",
        under_location_cte(),
    ))
    .bind(session.user_id)
    .bind(location_id)
    .bind(collection.id)
    .execute(&state.pool)
    .await
    .map(|result| result.rows_affected());
    if let Err(error) = inserted {
        return super::api::database_error(error);
    }

    Redirect::to(&format!("/collections/{}", collection.slug())).into_response()
}
