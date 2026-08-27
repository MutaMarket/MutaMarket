//! `/api/statistics/*` and `/api/personal/stats` — the unified
//! statistics page. A deliberate redesign: the legacy split the top
//! creator leaderboard (`StatisticsController`, `/statistics`) from the
//! personal creation stats (`StatsController`, `/personal/stats`); the
//! rewrite serves both, plus the market overview, to one page. The
//! individual numbers still port the legacy queries.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use super::AppState;
use crate::auth::session;

/// A personal stats row before shaping: (type id, type name,
/// creator id, creator name, count).
type PersonalRow = (i64, String, i64, String, i64);

/// The legacy leaderboard page size (Laravel's `paginate()` default).
const TOP_PAGE_SIZE: i64 = 15;

/// `GET /api/statistics/overview` — market-wide totals for the
/// statistics page header: the archive stats bar numbers (legacy
/// `getAllModulesStats`) plus the value and creator aggregates.
pub async fn overview(State(state): State<AppState>) -> Response {
    let stats = match crate::modules::stats::all_modules_stats(&state.pool, true).await {
        Ok(stats) => stats,
        Err(error) => return super::api::database_error(error),
    };

    let aggregates: Result<(Option<f64>, Option<f64>, i64, i64), _> = sqlx::query_as(
        "select sum(estimated_value), avg(estimated_value),
                count(distinct creator_id) filter (where creator_id is not null),
                (select count(*) from characters)
         from modules",
    )
    .fetch_one(&state.pool)
    .await;
    let (total_value, average_value, creators_count, characters_count) = match aggregates {
        Ok(aggregates) => aggregates,
        Err(error) => return super::api::database_error(error),
    };

    axum::Json(json!({
        "stats": stats,
        "total_value": total_value.unwrap_or(0.0),
        "average_value": average_value.unwrap_or(0.0),
        "creators_count": creators_count,
        "characters_count": characters_count,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct TopParams {
    pub name: Option<String>,
    pub sort_field: Option<String>,
    pub sort_direction: Option<String>,
}

pub async fn top_root(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<TopParams>,
) -> Response {
    top_response(&state, "", params).await
}

/// `GET /api/statistics/top/{query?}` — the legacy
/// `StatisticsController::index` leaderboard: creators ranked by
/// modules created, optionally scoped to one abyssal type through the
/// search query, name-filtered and paginated.
pub async fn top(
    State(state): State<AppState>,
    axum::extract::Path(query): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<TopParams>,
) -> Response {
    top_response(&state, &query, params).await
}

async fn top_response(state: &AppState, query: &str, params: TopParams) -> Response {
    let search = match crate::modules::search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(crate::modules::search::SearchError::Db(error)) => {
            return super::api::database_error(error);
        }
        Err(_) => {
            return super::api::error(StatusCode::NOT_FOUND, "Please provide a valid type.");
        }
    };
    let type_id = search.type_filter.as_ref().map(|type_filter| type_filter.id);

    // The legacy prefix match was MySQL `like`, case-insensitive by
    // collation; `ilike` keeps that behavior on Postgres.
    let name_pattern = params
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| format!("{}%", name.replace('%', "\\%").replace('_', "\\_")));

    let descending = params.sort_direction.as_deref() == Some("desc");
    let order = match params.sort_field.as_deref() {
        Some("name") if descending => "c.name desc",
        Some("name") => "c.name asc",
        Some("rank_number") if descending => "r.rank_number desc",
        // The legacy default: rank ascending.
        _ => "r.rank_number asc",
    };

    let page = search.page.max(1);
    let sql = format!(
        "with ranked as (
             select creator_id as id, count(*) as modules_created_count,
                    rank() over (order by count(*) desc) as rank_number
             from modules
             where creator_id is not null
               and ($1::bigint is null or type_id = $1)
             group by creator_id
         )
         select c.id, c.name, r.modules_created_count, r.rank_number,
                count(*) over () as total
         from ranked r
         join characters c on c.id = r.id
         where ($2::text is null or c.name ilike $2)
         order by {order}, r.rank_number asc, c.id asc
         limit $3 offset $4",
    );
    let rows = sqlx::query(&sql)
        .bind(type_id)
        .bind(&name_pattern)
        .bind(TOP_PAGE_SIZE)
        .bind((page - 1) * TOP_PAGE_SIZE)
        .fetch_all(&state.pool)
        .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return super::api::database_error(error),
    };

    let total = rows.first().map_or(0i64, |row| row.get("total"));
    let data: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "name": row.get::<String, _>("name"),
                "modules_created_count": row.get::<i64, _>("modules_created_count"),
                "rank_number": row.get::<i64, _>("rank_number"),
            })
        })
        .collect();

    axum::Json(json!({
        "data": data,
        "meta": {
            "current_page": page,
            "per_page": TOP_PAGE_SIZE,
            "total": total,
        },
    }))
    .into_response()
}

/// `GET /api/personal/stats` — the legacy `StatsController::index`: the
/// signed-in user's creation counts per (type, creator) and the three
/// headline totals. Money spent prices every module at the latest
/// recorded market day for its source and mutaplasmid (the legacy
/// unordered `marketHistory` hasOne over a single-row-per-type table),
/// and like the legacy inner joins it skips modules missing either
/// price.
pub async fn personal(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated."),
        Err(error) => return super::api::database_error(error),
    };
    let characters: Vec<i64> =
        match sqlx::query_scalar("select id from characters where user_id = $1 order by id")
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await
        {
            Ok(characters) => characters,
            Err(error) => return super::api::database_error(error),
        };

    let stats_rows: Result<Vec<PersonalRow>, _> = sqlx::query_as(
        "select m.type_id, t.name, m.creator_id, c.name, count(*)
         from modules m
         join types t on t.id = m.type_id
         join characters c on c.id = m.creator_id
         where m.creator_id = any($1)
         group by m.type_id, t.name, m.creator_id, c.name
         order by count(*) desc, t.name asc, c.name asc",
    )
    .bind(&characters)
    .fetch_all(&state.pool)
    .await;
    let stats_rows = match stats_rows {
        Ok(rows) => rows,
        Err(error) => return super::api::database_error(error),
    };

    let totals: Result<(i64, Option<f64>), _> = sqlx::query_as(
        "select count(*), sum(estimated_value) from modules where creator_id = any($1)",
    )
    .bind(&characters)
    .fetch_one(&state.pool)
    .await;
    let (total_modules, total_value) = match totals {
        Ok(totals) => totals,
        Err(error) => return super::api::database_error(error),
    };

    let total_spent: Result<Option<f64>, _> = sqlx::query_scalar(
        "with latest as (
             select distinct on (type_id) type_id, average
             from market_histories
             order by type_id, date desc
         )
         select sum(ls.average + lm.average)
         from modules m
         join latest ls on ls.type_id = m.source_type_id
         join latest lm on lm.type_id = m.mutaplasmid_id
         where m.creator_id = any($1)",
    )
    .bind(&characters)
    .fetch_one(&state.pool)
    .await;
    let total_spent = match total_spent {
        Ok(total_spent) => total_spent,
        Err(error) => return super::api::database_error(error),
    };

    let stats: Vec<serde_json::Value> = stats_rows
        .into_iter()
        .map(|(type_id, type_name, creator_id, creator_name, count)| {
            json!({
                "type": { "id": type_id, "name": type_name },
                "creator": { "id": creator_id, "name": creator_name },
                "count": count,
            })
        })
        .collect();

    axum::Json(json!({
        "stats": stats,
        "total_modules": total_modules,
        "total_value": total_value.unwrap_or(0.0),
        "total_spent": total_spent.unwrap_or(0.0),
    }))
    .into_response()
}
