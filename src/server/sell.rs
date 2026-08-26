//! `/api/sell/*` — the sell page, the legacy `SellController::index`:
//! the active character's published modules under the full filter
//! grammar, the header stats, and the container list of the
//! select-modules dialog (publishing itself goes through the ported
//! `/public-assets` endpoints).

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

use super::AppState;
use crate::auth::session;
use crate::view::personal::{PersonalModuleEntry, SellLocation, SellPageData};

/// Modules per sell page, the legacy `simplePaginate(40)`.
const SELL_PAGE_SIZE: i64 = 40;

/// The session's active character, or the user's first.
async fn active_character(
    pool: &sqlx::PgPool,
    session: &session::Session,
) -> sqlx::Result<Option<i64>> {
    match session.active_character_id {
        Some(id) => Ok(Some(id)),
        None => {
            sqlx::query_scalar("select id from characters where user_id = $1 order by id limit 1")
                .bind(session.user_id)
                .fetch_optional(pool)
                .await
        }
    }
}

async fn require_character(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<i64, Response> {
    let session = match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(super::api::error(
            axum::http::StatusCode::UNAUTHORIZED,
            "Unauthenticated.",
        )),
        Err(error) => Err(super::api::database_error(error)),
    }?;

    match active_character(&state.pool, &session).await {
        Ok(Some(character_id)) => Ok(character_id),
        Ok(None) => Err(super::api::error(
            axum::http::StatusCode::UNAUTHORIZED,
            "Unauthenticated.",
        )),
        Err(error) => Err(super::api::database_error(error)),
    }
}

/// `GET /api/sell/page` — the header stats of the published set.
pub async fn page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let character_id = match require_character(&state, &headers).await {
        Ok(character_id) => character_id,
        Err(response) => return response,
    };

    let stats: Result<(i64, f64), _> = sqlx::query_as(
        "select count(*), coalesce(sum(m.estimated_value), 0)
         from modules m
         where m.id in (select pa.module_id from public_assets pa
                        where pa.module_id is not null and pa.character_id = $1)",
    )
    .bind(character_id)
    .fetch_one(&state.pool)
    .await;

    match stats {
        Ok((published_count, estimated_value_total)) => axum::Json(SellPageData {
            character_id,
            published_count,
            estimated_value_total,
        })
        .into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `GET /api/sell/modules?q=` — the published modules under the filter
/// grammar; a bad query degrades to the unfiltered set like the
/// personal page.
pub async fn modules(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<super::social::PageQueryParams>,
) -> Response {
    let character_id = match require_character(&state, &headers).await {
        Ok(character_id) => character_id,
        Err(response) => return response,
    };

    let query = params.q.as_deref().unwrap_or("");
    let search = crate::modules::search::parse(&state.pool, &state.reference, query).await;
    let ids: Result<Vec<i64>, sqlx::Error> = match search {
        Ok(search) => {
            crate::modules::search::scoped_module_ids(
                &state.pool,
                &search,
                crate::modules::search::Scope::PublishedBy(character_id),
                SELL_PAGE_SIZE,
            )
            .await
        }
        Err(crate::modules::search::SearchError::Db(error)) => Err(error),
        Err(_) => {
            sqlx::query_scalar(
                "select m.id from modules m
                 where m.id in (select pa.module_id from public_assets pa
                                where pa.module_id is not null and pa.character_id = $1)
                 order by m.id desc
                 limit $2",
            )
            .bind(character_id)
            .bind(SELL_PAGE_SIZE)
            .fetch_all(&state.pool)
            .await
        }
    };
    let ids = match ids {
        Ok(ids) => ids,
        Err(error) => return super::api::database_error(error),
    };

    let details =
        match crate::modules::queries::details_for(&state.pool, &state.reference, ids.clone())
            .await
        {
            Ok(details) => details,
            Err(error) => return super::api::database_error(error),
        };
    let mut locations = match crate::assets::module_locations(
        &state.pool,
        // module_locations scopes by user; resolve the character's user.
        match sqlx::query_scalar::<_, Option<i64>>(
            "select user_id from characters where id = $1",
        )
        .bind(character_id)
        .fetch_one(&state.pool)
        .await
        {
            Ok(Some(user_id)) => user_id,
            Ok(None) => return axum::Json(Vec::<PersonalModuleEntry>::new()).into_response(),
            Err(error) => return super::api::database_error(error),
        },
        &ids,
    )
    .await
    {
        Ok(locations) => locations,
        Err(error) => return super::api::database_error(error),
    };

    let entries: Vec<PersonalModuleEntry> = details
        .into_iter()
        .map(|module| {
            let location = locations.remove(&module.id);
            PersonalModuleEntry { module, location }
        })
        .collect();

    axum::Json(entries).into_response()
}

/// `GET /api/sell/locations` — the active character's containers with
/// abyssal descendants and their published state, the legacy
/// `Character::locations()` for the select-modules dialog.
pub async fn locations(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let character_id = match require_character(&state, &headers).await {
        Ok(character_id) => character_id,
        Err(response) => return response,
    };

    /// (asset_id, type_id, name, type_name, location_flag,
    /// abyssal_count, public_asset_id, station_name).
    type LocationRow = (i64, i64, String, String, String, i64, Option<i64>, Option<String>);
    let rows: Result<Vec<LocationRow>, _> = sqlx::query_as(
        "with recursive tree as (
             select a.id as root_asset, a.item_id as node
             from assets a
             where a.character_id = $1 and not a.is_abyssal and a.corporation_id is null
             union all
             select t.root_asset, child.item_id
             from tree t
             join assets child on child.location_id = t.node and child.character_id = $1
         ),
         -- Climb each container's parent chain to its topmost location
         -- (the hosting station or structure).
         up as (
             select a.id as root_asset, a.location_id, 0 as depth
             from assets a
             where a.character_id = $1 and not a.is_abyssal and a.corporation_id is null
             union all
             select up.root_asset, parent.location_id, up.depth + 1
             from up
             join assets parent on parent.item_id = up.location_id and parent.character_id = $1
         ),
         tops as (
             select distinct on (root_asset) root_asset, location_id
             from up order by root_asset, depth desc
         )
         select r.id, r.type_id, coalesce(nullif(r.name, ''), t2.name, '') as name,
                coalesce(t2.name, '') as type_name,
                r.location_flag,
                count(distinct ab.id) as abyssal_count,
                (select pa.id from public_assets pa
                 where pa.character_id = r.character_id and pa.asset_id = r.id
                 limit 1) as public_asset_id,
                coalesce(st.name, str.name) as station_name
         from assets r
         join tree on tree.root_asset = r.id
         join assets ab on ab.item_id = tree.node
              and ab.character_id = $1 and ab.is_abyssal
         left join types t2 on t2.id = r.type_id
         left join tops on tops.root_asset = r.id
         left join stations st on st.id = tops.location_id
         left join structures str on str.id = tops.location_id
         where r.character_id = $1
         group by r.id, t2.name, st.name, str.name
         order by abyssal_count desc, r.id",
    )
    .bind(character_id)
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => axum::Json(
            rows.into_iter()
                .map(
                    |(
                        asset_id,
                        type_id,
                        name,
                        type_name,
                        location_flag,
                        abyssal_count,
                        public_asset_id,
                        station_name,
                    )| {
                        SellLocation {
                            asset_id,
                            type_id,
                            name,
                            type_name,
                            location_flag,
                            abyssal_count,
                            public_asset_id,
                            station_name,
                        }
                    },
                )
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => super::api::database_error(error),
    }
}
