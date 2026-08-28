//! The shared JSON base of the legacy `ContractResource`: the key set
//! every contract source serializes before its `whenHas`-conditional
//! extras (status, modules, types, acceptor, ...), which stay at the
//! call sites because they follow each source table's columns.

use serde_json::json;
use sqlx::Row;
use sqlx::postgres::PgRow;

/// The legacy `CharacterResource` guest fragment (like the module cards'
/// creator), read from `{prefix}_id`, `{prefix}_name`, ... columns.
/// `None` when the (left-joined) character is absent, serializing as the
/// legacy null relation.
pub fn character_fragment(row: &PgRow, prefix: &str) -> Option<serde_json::Value> {
    let id: i64 = row.get::<Option<i64>, _>(format!("{prefix}_id").as_str())?;
    let name: String = row
        .get::<Option<String>, _>(format!("{prefix}_name").as_str())
        .unwrap_or_default();
    Some(json!({
        "id": id,
        "slug": crate::modules::view::module_slug(&name, id),
        "name": name,
        "description": row.get::<Option<String>, _>(format!("{prefix}_description").as_str()),
        "has_premium": row
            .get::<Option<bool>, _>(format!("{prefix}_has_premium").as_str())
            .unwrap_or(false),
        "corporation_id": row.get::<Option<i64>, _>(format!("{prefix}_corporation_id").as_str()),
    }))
}

/// The unconditional `ContractResource` keys. The row must alias its
/// price column as `price`, cast the dates to text, and select the
/// issuer through the `issuer_*` fragment columns.
pub fn contract_base(row: &PgRow) -> serde_json::Value {
    json!({
        "id": row.get::<i64, _>("id"),
        "type": row.get::<String, _>("type"),
        "price": row.get::<Option<f64>, _>("price"),
        "asking_for_items": row.get::<bool, _>("asking_for_items"),
        "plex_count": row.get::<i32, _>("plex_count"),
        "non_abyssal_modules_count": row.get::<i32, _>("non_abyssal_modules_count"),
        "abyssal_modules_count": row.get::<i32, _>("abyssal_modules_count"),
        "issuer": character_fragment(row, "issuer"),
        "date_issued": row.get::<Option<String>, _>("date_issued"),
        "date_expired": row.get::<Option<String>, _>("date_expired"),
    })
}
