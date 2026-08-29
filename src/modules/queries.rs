//! Read queries for modules, shared by the JSON API handlers and the Leptos
//! page server functions. The shapes mirror the legacy resources; see
//! `modules::view`.

use sqlx::{PgPool, Row};

use super::view::{
    CharacterRef, ContractRef, FilterAttribute, ModuleAttributeView, ModuleDetail, MutaplasmidRef,
    SourceTypeRef, TypeRef, UnitRef, module_slug,
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
        let name: String = row
            .get::<Option<String>, _>("creator_name")
            .unwrap_or_default();

        CharacterRef {
            id: creator_id,
            slug: module_slug(&name, creator_id),
            name,
            description: row.get("creator_description"),
            has_premium: row
                .get::<Option<bool>, _>("creator_has_premium")
                .unwrap_or(false),
            corporation_id: row.get("creator_corporation_id"),
        }
    });

    let source_type = row
        .get::<Option<i64>, _>("source_type_id")
        .map(|source_type_id| SourceTypeRef {
            id: source_type_id,
            name: row
                .get::<Option<String>, _>("source_type_name")
                .unwrap_or_default(),
            meta_group: row.get("source_meta_group"),
            meta_group_id: row.get("source_meta_group_id"),
            published: row
                .get::<Option<bool>, _>("source_published")
                .unwrap_or(false),
        });

    let mutaplasmid = row
        .get::<Option<i64>, _>("mutaplasmid_id")
        .map(|mutaplasmid_id| MutaplasmidRef {
            id: mutaplasmid_id,
            name: row
                .get::<Option<String>, _>("mutaplasmid_name")
                .unwrap_or_default(),
        });

    let contract = row.get::<Option<i64>, _>("contract_id").map(|contract_id| {
        let issuer = row.get::<Option<i64>, _>("issuer_id").map(|issuer_id| {
            let name: String = row
                .get::<Option<String>, _>("issuer_name")
                .unwrap_or_default();

            CharacterRef {
                id: issuer_id,
                slug: module_slug(&name, issuer_id),
                name,
                description: row.get("issuer_description"),
                has_premium: row
                    .get::<Option<bool>, _>("issuer_has_premium")
                    .unwrap_or(false),
                corporation_id: row.get("issuer_corporation_id"),
            }
        });

        ContractRef {
            id: contract_id,
            r#type: row
                .get::<Option<String>, _>("contract_type")
                .unwrap_or_default(),
            price: row.get("contract_price"),
            asking_for_items: row
                .get::<Option<bool>, _>("contract_asking")
                .unwrap_or(false),
            plex_count: i64::from(
                row.get::<Option<i32>, _>("contract_plex_count")
                    .unwrap_or(0),
            ),
            non_abyssal_modules_count: i64::from(
                row.get::<Option<i32>, _>("contract_non_abyssal_count")
                    .unwrap_or(0),
            ),
            abyssal_modules_count: i64::from(
                row.get::<Option<i32>, _>("contract_abyssal_count")
                    .unwrap_or(0),
            ),
            issuer,
            date_issued: row.get("contract_date_issued"),
            date_expired: row.get("contract_date_expired"),
        }
    });

    // The legacy withDefaultRelations loads publicAsset.character plus a
    // price subselect: the asking price the asset owner's user set for
    // the module (module_pricing joined through characters on the same
    // user). Legacy quirk, ported: PublicAssetResource casts with
    // `(float) $this->price`, so an unpriced asset emits 0, not null.
    let public_asset: Option<serde_json::Value> = sqlx::query_as::<_, (i64, String, Option<f64>)>(
        "select pa.character_id, c.name,
                (select mp.price from module_pricing mp
                 where mp.module_id = pa.module_id and mp.user_id = c.user_id
                 limit 1) as price
         from public_assets pa
         join characters c on c.id = pa.character_id
         where pa.module_id = $1
         order by pa.id limit 1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?
    .map(|(id, name, price)| {
        serde_json::json!({
            "owner": { "id": id, "name": name },
            "price": price.unwrap_or(0.0),
        })
    });

    Ok(Some(ModuleDetail {
        training_module: None,
        note: None,
        collection_note: None,
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
        public_asset,
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
         order by a.derived, a.display_name",
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
                name: row
                    .get::<Option<String>, _>("unit_name")
                    .unwrap_or_default(),
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
                    type_band(
                        reference,
                        mutaplasmid_id,
                        attribute_id,
                        fraction,
                        fraction_type,
                    )
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

    let clamp01 = |value: f64| {
        if value.is_nan() {
            0.0
        } else {
            value.clamp(0.0, 1.0)
        }
    };

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

/// The slider bounds for every mutated attribute of an abyssal type,
/// aggregated from the per-source-type roll statistics (the equivalent of
/// the legacy `abyssal_type_statistics` rows the filter UI reads).
/// The published source types feeding an abyssal type, with their base
/// values for the given filter attributes: the slider-pip and
/// related-type data of the filter panel (specs/browser-filters.md §7).
pub async fn type_filter_source_types(
    pool: &PgPool,
    type_id: i64,
    attribute_ids: &[i64],
) -> sqlx::Result<Vec<crate::modules::view::FilterSourceType>> {
    use crate::modules::view::{FilterSourceType, FilterSourceTypeValue};

    let types: Vec<(i64, String, Option<i64>, Option<f64>)> = sqlx::query_as(
        "select distinct t.id, t.name, t.meta_group_id, ml.value as meta_level
         from mutaplasmids m
         join mutaplasmid_input_types mit on mit.mutaplasmid_id = m.id
         join types t on t.id = mit.type_id and t.published
         left join type_attributes ml
             on ml.type_id = t.id and ml.attribute_id = $2
         where m.output_type_id = $1",
    )
    .bind(type_id)
    .bind(crate::modules::META_LEVEL_ATTRIBUTE_ID)
    .fetch_all(pool)
    .await?;

    let type_ids: Vec<i64> = types.iter().map(|(id, ..)| *id).collect();
    let values: Vec<(i64, i64, Option<f64>)> = sqlx::query_as(
        "select type_id, attribute_id, value from type_attributes
         where type_id = any($1) and attribute_id = any($2)
         order by attribute_id",
    )
    .bind(&type_ids)
    .bind(attribute_ids)
    .fetch_all(pool)
    .await?;

    let mut source_types: Vec<FilterSourceType> = types
        .into_iter()
        .map(|(id, name, meta_group_id, meta_level)| FilterSourceType {
            id,
            name,
            meta_group_id,
            meta_level: meta_level.map(|level| level as i64),
            attributes: values
                .iter()
                .filter(|(type_id, ..)| *type_id == id)
                .filter_map(|(_, attribute_id, value)| {
                    value.map(|value| FilterSourceTypeValue {
                        attribute_id: *attribute_id,
                        value,
                    })
                })
                .collect(),
        })
        .collect();

    // Meta rank (T1, T2, Storyline, Faction, Deadspace, Officer) then
    // name, like every legacy type list.
    let rank = |meta_group_id: Option<i64>| match meta_group_id {
        Some(1) => 1,
        Some(2) => 2,
        Some(3) => 3,
        Some(4) => 4,
        Some(6) => 5,
        Some(5) => 6,
        Some(other) => other,
        None => i64::MAX,
    };
    source_types.sort_by(|a, b| {
        rank(a.meta_group_id)
            .cmp(&rank(b.meta_group_id))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(source_types)
}

pub async fn type_filter_attributes(
    pool: &PgPool,
    type_id: i64,
) -> sqlx::Result<Vec<FilterAttribute>> {
    let rows = sqlx::query(
        "select s.attribute_id, a.name, a.display_name,
                nullif(u.name, '') as unit_name,
                nullif(u.display_name, '') as unit_display_name,
                bool_or(s.high_is_good) as high_is_good,
                bool_or(s.is_virtual) as is_virtual,
                case when bool_or(s.high_is_good) then max(s.best) else min(s.best) end as best,
                case when bool_or(s.high_is_good) then min(s.worst) else max(s.worst) end as worst
         from mutaplasmid_type_statistics s
         join mutaplasmids m on m.id = s.mutaplasmid_id
         join attributes a on a.id = s.attribute_id
         left join units u on u.id = a.unit_id
         where m.output_type_id = $1
         group by s.attribute_id, a.name, a.display_name, a.derived, u.name, u.display_name
         order by a.derived, a.display_name",
    )
    .bind(type_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| FilterAttribute {
            attribute_id: row.get("attribute_id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            unit_name: row.get("unit_name"),
            unit_display_name: row.get("unit_display_name"),
            high_is_good: row.get("high_is_good"),
            is_virtual: row.get("is_virtual"),
            best: row.get("best"),
            worst: row.get("worst"),
        })
        .collect())
}

/// The newest for-sale modules with full card data — the legacy
/// `PremiumController` sample query (`hasLatestContract`, latest by id).
pub async fn premium_sample_modules(
    pool: &PgPool,
    reference: &ReferenceData,
    limit: i64,
) -> sqlx::Result<Vec<ModuleDetail>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "select id from modules where latest_contract_id is not null order by id desc limit $1",
    )
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

/// Attaches the signed-in user's notes (the legacy `withUserNote` half of
/// `withDefaultRelations`): every module gains the `note` key, null when
/// the user has no note on it. Guests skip the call, leaving the key
/// absent like the legacy unloaded relation.
pub async fn attach_user_notes(
    pool: &PgPool,
    user_id: i64,
    modules: &mut [crate::modules::view::ModuleDetail],
) -> sqlx::Result<()> {
    let ids: Vec<i64> = modules.iter().map(|module| module.id).collect();
    let rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "select module_id, id, content from notes
         where user_id = $1 and module_id = any($2)",
    )
    .bind(user_id)
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    let mut by_module: std::collections::HashMap<i64, crate::modules::view::NoteRef> = rows
        .into_iter()
        .map(|(module_id, id, content)| (module_id, crate::modules::view::NoteRef { id, content }))
        .collect();
    for module in modules {
        module.note = Some(by_module.remove(&module.id));
    }
    Ok(())
}

/// Attaches a collection's notes to its page modules (the legacy
/// `withCollectionNote($collection)` loadout of `CollectionController::
/// show`, loaded for every viewer): every module gains the
/// `collection_note` key with the collection resource embedded.
pub async fn attach_collection_notes(
    pool: &PgPool,
    collection_id: i64,
    modules: &mut [crate::modules::view::ModuleDetail],
) -> sqlx::Result<()> {
    use crate::modules::view::{CollectionNoteRef, NoteCollectionRef};

    let ids: Vec<i64> = modules.iter().map(|module| module.id).collect();
    let rows = sqlx::query(
        "select n.module_id, n.id, n.content,
                c.id as collection_id, c.identifier, c.name, c.description, c.visibility,
                c.auto_sync,
                to_char(c.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                    as created_at,
                to_char(c.updated_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                    as updated_at,
                to_char(c.last_synced_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                    as last_synced_at
         from collection_notes n
         join collections c on c.id = n.collection_id
         where n.collection_id = $1 and n.module_id = any($2)",
    )
    .bind(collection_id)
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    let mut by_module: std::collections::HashMap<i64, CollectionNoteRef> = rows
        .into_iter()
        .map(|row| {
            let name: String = row.get("name");
            let identifier: String = row.get("identifier");
            let slug = format!("{}-{}", crate::modules::view::slugify(&name), identifier);
            (
                row.get::<i64, _>("module_id"),
                CollectionNoteRef {
                    collection: NoteCollectionRef {
                        id: row.get("collection_id"),
                        identifier,
                        slug,
                        name,
                        description: row.get("description"),
                        visibility: row.get("visibility"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                        auto_sync: row.get("auto_sync"),
                        last_synced_at: row.get("last_synced_at"),
                    },
                    id: row.get("id"),
                    content: row.get("content"),
                },
            )
        })
        .collect();
    for module in modules {
        module.collection_note = Some(by_module.remove(&module.id));
    }
    Ok(())
}

/// Attaches the recorded historic sale to each card (the legacy
/// `with('trainingModule.historicContract')` loadout of the
/// historic-sales page).
pub async fn attach_training(
    pool: &PgPool,
    modules: &mut [crate::modules::view::ModuleDetail],
) -> sqlx::Result<()> {
    let ids: Vec<i64> = modules.iter().map(|module| module.id).collect();
    let rows: Vec<(i64, i64, Option<f64>, Option<String>)> = sqlx::query_as(
        "select tm.module_id, hc.id, hc.unified_price, hc.date_issued::text
         from training_modules tm
         join historic_contracts hc on hc.id = tm.historic_contract_id
         where tm.module_id = any($1)",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    let by_module: std::collections::HashMap<i64, crate::modules::view::TrainingRef> = rows
        .into_iter()
        .map(|(module_id, contract_id, sold_for, sold_at)| {
            (
                module_id,
                crate::modules::view::TrainingRef {
                    contract_id,
                    sold_for,
                    sold_at,
                },
            )
        })
        .collect();
    for module in modules {
        module.training_module = by_module.get(&module.id).cloned();
    }
    Ok(())
}
