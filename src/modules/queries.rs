//! Read queries for modules, shared by the JSON API handlers and the Leptos
//! page server functions.

use sqlx::{PgPool, Row};

use super::view::{ModuleAttributeView, ModuleDetail, ModuleSummary, module_slug};
use crate::mutation::reference::ReferenceData;

/// A module with its computed attributes and related names, if it exists.
pub async fn module_detail(
    pool: &PgPool,
    reference: &ReferenceData,
    item_id: i64,
) -> sqlx::Result<Option<ModuleDetail>> {
    let row = sqlx::query(
        "select m.id, m.type_id, t.name as type_name, m.source_type_id,
                st.name as source_type_name, st.meta_group_id as source_meta_group_id,
                m.mutaplasmid_id, mp.name as mutaplasmid_name,
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

    let attributes =
        module_attributes(pool, reference, item_id, row.get("mutaplasmid_id")).await?;
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
        source_meta_group_id: row.get("source_meta_group_id"),
        mutaplasmid_id: row.get("mutaplasmid_id"),
        mutaplasmid_name: row.get("mutaplasmid_name"),
        attributes,
    }))
}

async fn module_attributes(
    pool: &PgPool,
    reference: &ReferenceData,
    item_id: i64,
    mutaplasmid_id: Option<i64>,
) -> sqlx::Result<Vec<ModuleAttributeView>> {
    let rows = sqlx::query(
        "select ma.attribute_id, a.name, a.display_name, a.derived,
                u.name as unit_name, u.display_name as unit_display_name,
                ma.value, ma.base_value, ma.fraction, ma.fraction_type, ma.fraction_absolute,
                ma.bar, ma.is_virtual
         from mutated_attributes ma
         join attributes a on a.id = ma.attribute_id
         left join units u on u.id = a.unit_id
         where ma.module_id = $1
         order by ma.id",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let attribute_id: i64 = row.get("attribute_id");
            let fraction: f64 = row.get("fraction");
            let fraction_type: f64 = row.get("fraction_type");

            ModuleAttributeView {
                attribute_id,
                name: row.get("name"),
                display_name: row.get("display_name"),
                unit_name: row.get("unit_name"),
                unit_display_name: row.get("unit_display_name"),
                value: row.get("value"),
                base_value: row.get("base_value"),
                fraction,
                fraction_type,
                fraction_absolute: row.get("fraction_absolute"),
                type_band: mutaplasmid_id.and_then(|mutaplasmid_id| {
                    type_band(reference, mutaplasmid_id, attribute_id, fraction, fraction_type)
                }),
                bar: row.get("bar"),
                is_derived: row.get("derived"),
                is_virtual: row.get("is_virtual"),
            }
        })
        .collect())
}

/// The highlight band of the type-normalized bar: how much of the type-wide
/// roll range the module's own mutaplasmid covers, as (min, max) half-width
/// fractions. Ported from the legacy BarTypeNormalized component; absent
/// when the mutaplasmid's range is the whole range.
fn type_band(
    reference: &ReferenceData,
    mutaplasmid_id: i64,
    attribute_id: i64,
    fraction: f64,
    fraction_type: f64,
) -> Option<(f64, f64)> {
    if fraction_type == fraction {
        return None;
    }

    let (value_min, value_max) = reference.roll_range(mutaplasmid_id, attribute_id)?;
    let (extreme_min, extreme_max) = reference.type_roll_extremes(mutaplasmid_id, attribute_id)?;
    let high_is_good = reference.roll_high_is_good(mutaplasmid_id, attribute_id)?;

    let clamp01 = |value: f64| if value.is_nan() { 0.0 } else { value.clamp(0.0, 1.0) };

    let fraction_max = clamp01((value_max - 1.0) / (extreme_max - 1.0));
    let fraction_min = clamp01((1.0 - value_min) / (1.0 - extreme_min));

    Some(if high_is_good {
        (fraction_min, fraction_max)
    } else {
        (fraction_max, fraction_min)
    })
}

/// The newest modules with full card data (details including attributes).
pub async fn recent_module_cards(
    pool: &PgPool,
    reference: &ReferenceData,
    limit: i64,
) -> sqlx::Result<Vec<ModuleDetail>> {
    let ids: Vec<i64> = sqlx::query_scalar("select id from modules order by id desc limit $1")
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let mut cards = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(detail) = module_detail(pool, reference, id).await? {
            cards.push(detail);
        }
    }

    Ok(cards)
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
