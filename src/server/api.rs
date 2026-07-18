//! The public JSON API, ported from the legacy `Api\ModuleController` and
//! statistics controllers. Contract- and estimator-dependent behavior
//! (price filters, sale listings, estimated values) arrives with those
//! milestones; the shapes here carry what exists so far.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use sqlx::{PgPool, Row};

#[derive(Serialize)]
struct ModuleSummary {
    id: i64,
    slug: String,
    type_id: i64,
    type_name: String,
    average_fraction: Option<f64>,
    creator_id: Option<i64>,
}

#[derive(Serialize)]
struct ModuleDetail {
    #[serde(flatten)]
    summary: ModuleSummary,
    source_type_id: Option<i64>,
    source_type_name: Option<String>,
    mutaplasmid_id: Option<i64>,
    mutaplasmid_name: Option<String>,
    attributes: Vec<ModuleAttribute>,
}

#[derive(Serialize)]
struct ModuleAttribute {
    attribute_id: i64,
    name: String,
    value: f64,
    base_value: f64,
    fraction: f64,
    fraction_type: f64,
    fraction_absolute: f64,
    bar: i16,
    is_virtual: bool,
}

/// Modules per index page, like the legacy cursor pagination.
const MODULES_PAGE_SIZE: i64 = 100;

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}

fn database_error(error: sqlx::Error) -> Response {
    tracing_stderr(&error);
    self::error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
}

fn tracing_stderr(error: &sqlx::Error) {
    eprintln!("api database error: {error}");
}

/// `GET /api/modules` — the legacy index requires a type option in the query
/// path, so the bare route always rejects.
pub async fn modules_index_root() -> Response {
    error(StatusCode::NOT_FOUND, "Please provide a valid type.")
}

/// `GET /api/modules/{query}`: a slug ending in digits is a module lookup,
/// anything else is the type-scoped module index with filter segments.
pub async fn modules_show_or_index(
    State(pool): State<PgPool>,
    Path(query): Path<String>,
) -> Response {
    match module_id_from_slug(&query) {
        Some(item_id) => show_module(&pool, item_id).await,
        None => module_index(&pool, &query).await,
    }
}

async fn show_module(pool: &PgPool, item_id: i64) -> Response {
    let row = sqlx::query(
        "select m.id, m.type_id, t.name as type_name, m.source_type_id,
                st.name as source_type_name, m.mutaplasmid_id, mp.name as mutaplasmid_name,
                m.creator_id, m.average_fraction
         from modules m
         join types t on t.id = m.type_id
         left join types st on st.id = m.source_type_id
         left join mutaplasmids mp on mp.id = m.mutaplasmid_id
         where m.id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await;

    let row = match row {
        Ok(Some(row)) => row,
        Ok(None) => {
            return error(
                StatusCode::NOT_FOUND,
                "No module with this item id is known to MutaMarket.",
            );
        }
        Err(error) => return database_error(error),
    };

    let attribute_rows = sqlx::query(
        "select ma.attribute_id, a.name, ma.value, ma.base_value, ma.fraction,
                ma.fraction_type, ma.fraction_absolute, ma.bar, ma.is_virtual
         from mutated_attributes ma
         join attributes a on a.id = ma.attribute_id
         where ma.module_id = $1
         order by ma.id",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await;

    let attribute_rows = match attribute_rows {
        Ok(rows) => rows,
        Err(error) => return database_error(error),
    };

    let type_name: String = row.get("type_name");

    let detail = ModuleDetail {
        summary: ModuleSummary {
            id: row.get("id"),
            slug: module_slug(&type_name, item_id),
            type_id: row.get("type_id"),
            type_name,
            average_fraction: row.get("average_fraction"),
            creator_id: row.get("creator_id"),
        },
        source_type_id: row.get("source_type_id"),
        source_type_name: row.get("source_type_name"),
        mutaplasmid_id: row.get("mutaplasmid_id"),
        mutaplasmid_name: row.get("mutaplasmid_name"),
        attributes: attribute_rows
            .iter()
            .map(|row| ModuleAttribute {
                attribute_id: row.get("attribute_id"),
                name: row.get("name"),
                value: row.get("value"),
                base_value: row.get("base_value"),
                fraction: row.get("fraction"),
                fraction_type: row.get("fraction_type"),
                fraction_absolute: row.get("fraction_absolute"),
                bar: row.get("bar"),
                is_virtual: row.get("is_virtual"),
            })
            .collect(),
    };

    Json(json!({ "data": detail })).into_response()
}

async fn module_index(pool: &PgPool, query: &str) -> Response {
    let Some(type_option) = type_option(query) else {
        return error(StatusCode::NOT_FOUND, "Please provide a valid type.");
    };

    let type_row = sqlx::query("select id, name from types where id = $1 or slug(name) = $2 limit 1")
        .bind(type_option.parse::<i64>().unwrap_or(-1))
        .bind(&type_option)
        .fetch_optional(pool)
        .await;

    let type_row = match type_row {
        Ok(Some(row)) => row,
        Ok(None) => return error(StatusCode::NOT_FOUND, "Please provide a valid type."),
        Err(error) => return database_error(error),
    };

    let type_id: i64 = type_row.get("id");
    let type_name: String = type_row.get("name");

    let rows = sqlx::query(
        "select m.id, m.average_fraction, m.creator_id
         from modules m
         where m.type_id = $1
         order by m.id desc
         limit $2",
    )
    .bind(type_id)
    .bind(MODULES_PAGE_SIZE)
    .fetch_all(pool)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return database_error(error),
    };

    let modules: Vec<ModuleSummary> = rows
        .iter()
        .map(|row| {
            let id: i64 = row.get("id");
            ModuleSummary {
                id,
                slug: module_slug(&type_name, id),
                type_id,
                type_name: type_name.clone(),
                average_fraction: row.get("average_fraction"),
                creator_id: row.get("creator_id"),
            }
        })
        .collect();

    Json(json!({ "data": modules })).into_response()
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

/// The legacy module route pattern: an all-alphanumeric-and-dashes single
/// segment ending in digits is a module id (slug or bare id).
fn module_id_from_slug(query: &str) -> Option<i64> {
    if query.is_empty()
        || query.contains('/')
        || !query.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return None;
    }

    let digits: String = query
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
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

fn module_slug(type_name: &str, item_id: i64) -> String {
    let mut slug = String::with_capacity(type_name.len() + 16);

    for c in type_name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }

    let slug = slug.trim_end_matches('-');
    format!("{slug}-{item_id}")
}

#[cfg(test)]
mod tests {
    use super::{module_id_from_slug, module_slug, type_option};

    #[test]
    fn module_ids_parse_from_slugs_and_bare_ids() {
        assert_eq!(
            module_id_from_slug("50mn-abyssal-microwarpdrive-1037153455177"),
            Some(1037153455177),
        );
        assert_eq!(module_id_from_slug("1037153455177"), Some(1037153455177));
        assert_eq!(module_id_from_slug("type/47408"), None);
        assert_eq!(module_id_from_slug("damage-control"), None);
        assert_eq!(module_id_from_slug(""), None);
    }

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

    #[test]
    fn module_slugs_normalize_type_names() {
        assert_eq!(
            module_slug("50MN Abyssal Microwarpdrive", 123),
            "50mn-abyssal-microwarpdrive-123",
        );
        assert_eq!(module_slug("Gistum C-Type Web", 5), "gistum-c-type-web-5");
    }
}
