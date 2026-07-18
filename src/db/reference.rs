//! Seeding and loading of the EVE reference tables. `seed_reference` writes
//! any [`ReferenceTables`] to Postgres (used with the fixture dumps until the
//! native SDE import lands); `load_reference` reads them back.

use sqlx::{PgPool, Row};

use crate::mutation::context::{AttributeDef, Mutaplasmid};
use crate::mutation::reference::{
    InputTypeRow, MetaGroupRow, MutaplasmidAttributeRow, ReferenceTables, RegionRow, StatisticRow,
    TypeAttributeRow, TypeRow, UnitRow,
};

/// Replaces the reference tables with the given rows, in one transaction.
pub async fn seed_reference(pool: &PgPool, tables: &ReferenceTables) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    // CASCADE also clears dependent data (modules and their attributes) —
    // reference reseeding is a dev/test operation.
    sqlx::query(
        "truncate mutaplasmid_type_statistics, mutaplasmid_input_types, mutaplasmid_attributes,
         mutaplasmids, type_attributes, types, attributes, units, meta_groups cascade",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into units (id, name, display_name)
         select * from unnest($1::bigint[], $2::text[], $3::text[])",
    )
    .bind(tables.units.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.units.iter().map(|row| row.name.clone()).collect::<Vec<_>>())
    .bind(tables.units.iter().map(|row| row.display_name.clone()).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into meta_groups (id, name)
         select * from unnest($1::bigint[], $2::text[])",
    )
    .bind(tables.meta_groups.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.meta_groups.iter().map(|row| row.name.clone()).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    // Regions are upserted rather than truncated: contracts reference them.
    sqlx::query(
        "insert into regions (id, name)
         select * from unnest($1::bigint[], $2::text[])
         on conflict (id) do update set name = excluded.name",
    )
    .bind(tables.regions.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.regions.iter().map(|row| row.name.clone()).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    for attribute in &tables.attributes {
        sqlx::query(
            "insert into attributes
             (id, name, display_name, unit_id, high_is_good, derived, derived_operation,
              derived_attributes)
             values ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(attribute.id)
        .bind(&attribute.name)
        .bind(&attribute.display_name)
        .bind(attribute.unit_id)
        .bind(attribute.high_is_good)
        .bind(attribute.derived)
        .bind(&attribute.derived_operation)
        .bind(&attribute.derived_attributes)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "insert into types (id, name, published, meta_group_id)
         select * from unnest($1::bigint[], $2::text[], $3::boolean[], $4::bigint[])",
    )
    .bind(tables.types.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.types.iter().map(|row| row.name.clone()).collect::<Vec<_>>())
    .bind(tables.types.iter().map(|row| row.published).collect::<Vec<_>>())
    .bind(tables.types.iter().map(|row| row.meta_group_id).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into type_attributes (id, type_id, attribute_id, value)
         select * from unnest($1::bigint[], $2::bigint[], $3::bigint[], $4::float8[])",
    )
    .bind(tables.type_attributes.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.type_attributes.iter().map(|row| row.type_id).collect::<Vec<_>>())
    .bind(tables.type_attributes.iter().map(|row| row.attribute_id).collect::<Vec<_>>())
    .bind(tables.type_attributes.iter().map(|row| row.value).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into mutaplasmids (id, name, output_type_id)
         select * from unnest($1::bigint[], $2::text[], $3::bigint[])",
    )
    .bind(tables.mutaplasmids.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.mutaplasmids.iter().map(|row| row.name.clone()).collect::<Vec<_>>())
    .bind(tables.mutaplasmids.iter().map(|row| row.output_type_id).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into mutaplasmid_attributes
         (id, mutaplasmid_id, attribute_id, value_min, value_max, high_is_good, is_virtual)
         select * from unnest($1::bigint[], $2::bigint[], $3::bigint[], $4::float8[], $5::float8[],
                              $6::boolean[], $7::boolean[])",
    )
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.mutaplasmid_id).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.attribute_id).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.value_min).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.value_max).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.high_is_good).collect::<Vec<_>>())
    .bind(tables.mutaplasmid_attributes.iter().map(|row| row.is_virtual).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into mutaplasmid_input_types (id, mutaplasmid_id, type_id)
         select * from unnest($1::bigint[], $2::bigint[], $3::bigint[])",
    )
    .bind(tables.input_types.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.input_types.iter().map(|row| row.mutaplasmid_id).collect::<Vec<_>>())
    .bind(tables.input_types.iter().map(|row| row.type_id).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "insert into mutaplasmid_type_statistics
         (id, type_id, mutaplasmid_id, attribute_id, best, worst, high_is_good, is_virtual)
         select * from unnest($1::bigint[], $2::bigint[], $3::bigint[], $4::bigint[],
                              $5::float8[], $6::float8[], $7::boolean[], $8::boolean[])",
    )
    .bind(tables.statistics.iter().map(|row| row.id).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.type_id).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.mutaplasmid_id).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.attribute_id).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.best).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.worst).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.high_is_good).collect::<Vec<_>>())
    .bind(tables.statistics.iter().map(|row| row.is_virtual).collect::<Vec<_>>())
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

/// Reads the reference tables back out of Postgres, in stable id order.
pub async fn load_reference(pool: &PgPool) -> sqlx::Result<ReferenceTables> {
    let mut tables = ReferenceTables::default();

    for row in sqlx::query(
        "select id, name, display_name, unit_id, high_is_good, derived, derived_operation,
                derived_attributes
         from attributes order by id",
    )
    .fetch_all(pool)
    .await?
    {
        tables.attributes.push(AttributeDef {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            unit_id: row.get("unit_id"),
            high_is_good: row.get("high_is_good"),
            derived: row.get("derived"),
            derived_operation: row.get("derived_operation"),
            derived_attributes: row
                .get::<Option<Vec<i64>>, _>("derived_attributes")
                .unwrap_or_default(),
        });
    }

    for row in sqlx::query("select id, name, display_name from units order by id")
        .fetch_all(pool)
        .await?
    {
        tables.units.push(UnitRow {
            id: row.get("id"),
            name: row.get("name"),
            display_name: row.get("display_name"),
        });
    }

    for row in sqlx::query("select id, name from meta_groups order by id")
        .fetch_all(pool)
        .await?
    {
        tables.meta_groups.push(MetaGroupRow {
            id: row.get("id"),
            name: row.get("name"),
        });
    }

    for row in sqlx::query("select id, name from regions order by id")
        .fetch_all(pool)
        .await?
    {
        tables.regions.push(RegionRow {
            id: row.get("id"),
            name: row.get("name"),
        });
    }

    for row in sqlx::query("select id, name, published, meta_group_id from types order by id")
        .fetch_all(pool)
        .await?
    {
        tables.types.push(TypeRow {
            id: row.get("id"),
            name: row.get("name"),
            published: row.get("published"),
            meta_group_id: row.get("meta_group_id"),
        });
    }

    for row in sqlx::query("select id, type_id, attribute_id, value from type_attributes order by id")
        .fetch_all(pool)
        .await?
    {
        tables.type_attributes.push(TypeAttributeRow {
            id: row.get("id"),
            type_id: row.get("type_id"),
            attribute_id: row.get("attribute_id"),
            value: row.get("value"),
        });
    }

    for row in sqlx::query("select id, name, output_type_id from mutaplasmids order by id")
        .fetch_all(pool)
        .await?
    {
        tables.mutaplasmids.push(Mutaplasmid {
            id: row.get("id"),
            name: row.get("name"),
            output_type_id: row.get("output_type_id"),
        });
    }

    for row in sqlx::query(
        "select id, mutaplasmid_id, attribute_id, value_min, value_max, high_is_good, is_virtual
         from mutaplasmid_attributes order by id",
    )
    .fetch_all(pool)
    .await?
    {
        tables.mutaplasmid_attributes.push(MutaplasmidAttributeRow {
            id: row.get("id"),
            mutaplasmid_id: row.get("mutaplasmid_id"),
            attribute_id: row.get("attribute_id"),
            value_min: row.get("value_min"),
            value_max: row.get("value_max"),
            high_is_good: row.get("high_is_good"),
            is_virtual: row.get("is_virtual"),
        });
    }

    for row in sqlx::query("select id, mutaplasmid_id, type_id from mutaplasmid_input_types order by id")
        .fetch_all(pool)
        .await?
    {
        tables.input_types.push(InputTypeRow {
            id: row.get("id"),
            mutaplasmid_id: row.get("mutaplasmid_id"),
            type_id: row.get("type_id"),
        });
    }

    for row in sqlx::query(
        "select id, type_id, mutaplasmid_id, attribute_id, best, worst, high_is_good, is_virtual
         from mutaplasmid_type_statistics order by id",
    )
    .fetch_all(pool)
    .await?
    {
        tables.statistics.push(StatisticRow {
            id: row.get("id"),
            type_id: row.get("type_id"),
            mutaplasmid_id: row.get("mutaplasmid_id"),
            attribute_id: row.get("attribute_id"),
            best: row.get("best"),
            worst: row.get("worst"),
            high_is_good: row.get("high_is_good"),
            is_virtual: row.get("is_virtual"),
        });
    }

    Ok(tables)
}
