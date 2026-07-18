//! Native import of EVE reference data, replacing the legacy Laravel
//! SDE commands and seeders end to end:
//!
//! - [`data`]: parsed form of CCP's official JSONL Static Data Export plus
//!   the community dynamic-item-attributes conversion (mutaplasmids).
//! - [`enrich`]: the manual corrections and app-defined derived/virtual
//!   attributes the legacy seeders applied on top of the raw SDE.
//! - [`statistics`]: the best/worst roll statistics per (source type,
//!   mutaplasmid, attribute) — computed by the app, not shipped in the SDE.
//! - [`client`]: downloading the SDE zip and the dynamic-item JSON.
//!
//! The output is a plain [`ReferenceTables`], the same structure the fixture
//! dumps and Postgres produce, so everything downstream is source-agnostic.

pub mod client;
pub mod data;
pub mod enrich;
pub mod statistics;

use crate::mutation::context::{AttributeDef, Mutaplasmid};
use crate::mutation::reference::{
    InputTypeRow, MutaplasmidAttributeRow, ReferenceTables, TypeAttributeRow,
};

use data::SdeData;

/// Turns parsed SDE data into fully enriched reference tables, including the
/// computed roll statistics.
pub fn build_reference_tables(sde: SdeData) -> ReferenceTables {
    let mut tables = base_tables(sde);

    enrich::apply_ccp_corrections(&mut tables);
    enrich::add_virtual_attributes(&mut tables);
    enrich::add_derived_attributes(&mut tables);

    tables.statistics = statistics::compute_statistics(&tables);

    tables
}

fn base_tables(sde: SdeData) -> ReferenceTables {
    let type_names: std::collections::HashMap<i64, &str> = sde
        .types
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();

    let mut mutaplasmids = Vec::new();
    let mut mutaplasmid_attributes = Vec::new();
    let mut input_types = Vec::new();

    for item in &sde.dynamic_items {
        // A mutaplasmid without a type entry cannot be named; the legacy
        // seeder would have crashed on it, so it should not occur.
        let Some(name) = type_names.get(&item.type_id) else {
            continue;
        };

        mutaplasmids.push(Mutaplasmid {
            id: item.type_id,
            name: (*name).to_owned(),
            output_type_id: item.resulting_type,
        });

        for attribute in &item.attributes {
            mutaplasmid_attributes.push(MutaplasmidAttributeRow {
                id: mutaplasmid_attributes.len() as i64 + 1,
                mutaplasmid_id: item.type_id,
                attribute_id: attribute.attribute_id,
                value_min: attribute.min,
                value_max: attribute.max,
                high_is_good: attribute.high_is_good,
                is_virtual: false,
            });
        }

        for &type_id in &item.applicable_types {
            input_types.push(InputTypeRow {
                id: input_types.len() as i64 + 1,
                mutaplasmid_id: item.type_id,
                type_id,
            });
        }
    }

    ReferenceTables {
        attributes: sde
            .attributes
            .into_iter()
            .map(|attribute| AttributeDef {
                id: attribute.id,
                name: attribute.name,
                display_name: attribute.display_name,
                unit_id: attribute.unit_id,
                high_is_good: attribute.high_is_good,
                derived: false,
                derived_operation: None,
                derived_attributes: Vec::new(),
            })
            .collect(),
        units: sde.units,
        meta_groups: sde.meta_groups,
        regions: sde.regions,
        type_attributes: sde
            .type_dogma
            .into_iter()
            .enumerate()
            .map(|(index, (type_id, attribute_id, value))| TypeAttributeRow {
                id: index as i64 + 1,
                type_id,
                attribute_id,
                value: Some(value),
            })
            .collect(),
        types: sde.types,
        mutaplasmids,
        mutaplasmid_attributes,
        input_types,
        statistics: Vec::new(),
    }
}
