//! Read queries for modules, shared by the JSON API handlers and the Leptos
//! page server functions.

use sqlx::{PgPool, Row};

use super::view::{ModuleAttributeView, ModuleDetail, ModuleSummary, module_slug};

/// A module with its computed attributes and related names, if it exists.
pub async fn module_detail(pool: &PgPool, item_id: i64) -> sqlx::Result<Option<ModuleDetail>> {
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
    .await?;

    let Some(row) = row else {
        return Ok(None);
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
    .await?;

    let type_name: String = row.get("type_name");

    Ok(Some(ModuleDetail {
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
            .map(|row| ModuleAttributeView {
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
    }))
}

/// Resolves a type by EVE id or name slug.
pub async fn find_type(pool: &PgPool, id_or_slug: &str) -> sqlx::Result<Option<(i64, String)>> {
    let row = sqlx::query("select id, name from types where id = $1 or slug(name) = $2 limit 1")
        .bind(id_or_slug.parse::<i64>().unwrap_or(-1))
        .bind(id_or_slug)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|row| (row.get("id"), row.get("name"))))
}

/// The newest modules of one type.
pub async fn modules_of_type(
    pool: &PgPool,
    type_id: i64,
    type_name: &str,
    limit: i64,
) -> sqlx::Result<Vec<ModuleSummary>> {
    let rows = sqlx::query(
        "select m.id, m.average_fraction, m.creator_id
         from modules m
         where m.type_id = $1
         order by m.id desc
         limit $2",
    )
    .bind(type_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let id: i64 = row.get("id");
            ModuleSummary {
                id,
                slug: module_slug(type_name, id),
                type_id,
                type_name: type_name.to_owned(),
                average_fraction: row.get("average_fraction"),
                creator_id: row.get("creator_id"),
            }
        })
        .collect())
}

/// The newest modules across all types.
pub async fn recent_modules(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<ModuleSummary>> {
    let rows = sqlx::query(
        "select m.id, m.type_id, t.name as type_name, m.average_fraction, m.creator_id
         from modules m
         join types t on t.id = m.type_id
         order by m.id desc
         limit $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let type_name: String = row.get("type_name");
            ModuleSummary {
                id,
                slug: module_slug(&type_name, id),
                type_id: row.get("type_id"),
                type_name,
                average_fraction: row.get("average_fraction"),
                creator_id: row.get("creator_id"),
            }
        })
        .collect())
}
