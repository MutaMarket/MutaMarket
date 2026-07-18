//! Read queries for modules, shared by the JSON API handlers and the Leptos
//! page server functions. The shapes mirror the legacy resources; see
//! `modules::view`.

use sqlx::{PgPool, Row};

use super::view::{
    CharacterRef, ContractRef, ModuleAttributeView, ModuleDetail, MutaplasmidRef, SourceTypeRef,
    TypeRef, UnitRef, module_slug,
};
use crate::mutation::reference::ReferenceData;

/// A module with its computed attributes and related names, if it exists.
pub async fn module_detail(
    pool: &PgPool,
    reference: &ReferenceData,
    item_id: i64,
) -> sqlx::Result<Option<ModuleDetail>> {
    let row = sqlx::query(
        "select m.id, m.type_id, t.name as type_name,
                m.source_type_id, st.name as source_type_name,
                st.meta_group_id as source_meta_group_id, st.published as source_published,
                smg.name as source_meta_group,
                m.mutaplasmid_id, mp.name as mutaplasmid_name,
                m.creator_id, c.name as creator_name, c.description as creator_description,
                c.corporation_id as creator_corporation_id,
                (c.premium_paid_until is not null and c.premium_paid_until > now())
                    as creator_has_premium,
                m.estimated_value, m.estimated_value_updated_at::text as estimated_value_updated_at,
                m.average_fraction,
                ct.id as contract_id, ct.type as contract_type,
                ct.unified_price as contract_price, ct.asking_for_items as contract_asking,
                ct.plex_count as contract_plex_count,
                ct.non_abyssal_modules_count as contract_non_abyssal_count,
                ct.abyssal_modules_count as contract_abyssal_count,
                ct.date_issued::text as contract_date_issued,
                ct.date_expired::text as contract_date_expired,
                ic.id as issuer_id, ic.name as issuer_name,
                ic.description as issuer_description,
                ic.corporation_id as issuer_corporation_id,
                (ic.premium_paid_until is not null and ic.premium_paid_until > now())
                    as issuer_has_premium
         from modules m
         join types t on t.id = m.type_id
         left join types st on st.id = m.source_type_id
         left join meta_groups smg on smg.id = st.meta_group_id
         left join mutaplasmids mp on mp.id = m.mutaplasmid_id
         left join characters c on c.id = m.creator_id
         left join contracts ct on ct.id = m.latest_contract_id
         left join characters ic on ic.id = ct.issuer_id
         where m.id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let mutated_attributes =
        module_attributes(pool, reference, item_id, row.get("mutaplasmid_id")).await?;
    let type_name: String = row.get("type_name");

    let creator = row.get::<Option<i64>, _>("creator_id").map(|creator_id| {
        let name: String = row.get::<Option<String>, _>("creator_name").unwrap_or_default();

        CharacterRef {
            id: creator_id,
            slug: module_slug(&name, creator_id),
            name,
            description: row.get("creator_description"),
            has_premium: row.get::<Option<bool>, _>("creator_has_premium").unwrap_or(false),
            corporation_id: row.get("creator_corporation_id"),
        }
    });

    let source_type = row.get::<Option<i64>, _>("source_type_id").map(|source_type_id| {
        SourceTypeRef {
            id: source_type_id,
            name: row.get::<Option<String>, _>("source_type_name").unwrap_or_default(),
            meta_group: row.get("source_meta_group"),
            meta_group_id: row.get("source_meta_group_id"),
            published: row.get::<Option<bool>, _>("source_published").unwrap_or(false),
        }
    });

    let mutaplasmid = row.get::<Option<i64>, _>("mutaplasmid_id").map(|mutaplasmid_id| {
        MutaplasmidRef {
            id: mutaplasmid_id,
            name: row.get::<Option<String>, _>("mutaplasmid_name").unwrap_or_default(),
        }
    });

    let contract = row.get::<Option<i64>, _>("contract_id").map(|contract_id| {
        let issuer = row.get::<Option<i64>, _>("issuer_id").map(|issuer_id| {
            let name: String = row.get::<Option<String>, _>("issuer_name").unwrap_or_default();

            CharacterRef {
                id: issuer_id,
                slug: module_slug(&name, issuer_id),
                name,
                description: row.get("issuer_description"),
                has_premium: row.get::<Option<bool>, _>("issuer_has_premium").unwrap_or(false),
                corporation_id: row.get("issuer_corporation_id"),
            }
        });

        ContractRef {
            id: contract_id,
            r#type: row.get::<Option<String>, _>("contract_type").unwrap_or_default(),
            price: row.get("contract_price"),
            asking_for_items: row.get::<Option<bool>, _>("contract_asking").unwrap_or(false),
            plex_count: i64::from(row.get::<Option<i32>, _>("contract_plex_count").unwrap_or(0)),
            non_abyssal_modules_count: i64::from(
                row.get::<Option<i32>, _>("contract_non_abyssal_count").unwrap_or(0),
            ),
            abyssal_modules_count: i64::from(
                row.get::<Option<i32>, _>("contract_abyssal_count").unwrap_or(0),
            ),
            issuer,
            date_issued: row.get("contract_date_issued"),
            date_expired: row.get("contract_date_expired"),
        }
    });

    Ok(Some(ModuleDetail {
        id: row.get("id"),
        r#type: TypeRef {
            id: row.get("type_id"),
            name: type_name.clone(),
        },
        creator,
        mutated_attributes,
        source_type,
        mutaplasmid,
        contract,
        estimated_value: row.get("estimated_value"),
        estimated_value_updated_at: row.get("estimated_value_updated_at"),
        public_asset: None,
        slug: module_slug(&type_name, item_id),
        average_fraction: row.get("average_fraction"),
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
                u.id as unit_id, u.name as unit_name, u.display_name as unit_display_name,
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

            let unit = row.get::<Option<i64>, _>("unit_id").map(|unit_id| UnitRef {
                id: unit_id,
                name: row.get::<Option<String>, _>("unit_name").unwrap_or_default(),
                display_name: row
                    .get::<Option<String>, _>("unit_display_name")
                    .unwrap_or_default(),
            });

            ModuleAttributeView {
                attribute_id,
                name: row.get("name"),
                display_name: row.get("display_name"),
                value: row.get("value"),
                base_value: row.get("base_value"),
                fraction,
                fraction_type,
                fraction_absolute: row.get("fraction_absolute"),
                bar: row.get("bar"),
                is_derived: row.get("derived"),
                unit,
                is_virtual: row.get("is_virtual"),
                type_band: mutaplasmid_id.and_then(|mutaplasmid_id| {
                    type_band(reference, mutaplasmid_id, attribute_id, fraction, fraction_type)
                }),
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

/// The newest modules across all types, with full card data.
pub async fn recent_module_cards(
    pool: &PgPool,
    reference: &ReferenceData,
    limit: i64,
) -> sqlx::Result<Vec<ModuleDetail>> {
    let ids: Vec<i64> = sqlx::query_scalar("select id from modules order by id desc limit $1")
        .bind(limit)
        .fetch_all(pool)
        .await?;

    details_for(pool, reference, ids).await
}

/// Full module resources for the given ids, in order.
pub async fn details_for(
    pool: &PgPool,
    reference: &ReferenceData,
    ids: Vec<i64>,
) -> sqlx::Result<Vec<ModuleDetail>> {
    let mut details = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(detail) = module_detail(pool, reference, id).await? {
            details.push(detail);
        }
    }

    Ok(details)
}
