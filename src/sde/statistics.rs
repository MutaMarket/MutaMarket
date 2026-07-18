//! The best/worst roll statistics per (source type, mutaplasmid, attribute),
//! ported from the legacy `MutaplasmidTypeStatisticsService` and its seeder.
//! These power the gold/brown bar markers; they are computed by the app —
//! the SDE does not ship them.

use std::collections::HashMap;

use crate::mutation::fractions::round5;
use crate::mutation::reference::{ReferenceTables, StatisticRow};

/// Computes all statistics rows for the given reference tables. For every
/// mutaplasmid and every published source type it accepts, each rollable
/// attribute gets its extreme achievable values.
pub fn compute_statistics(tables: &ReferenceTables) -> Vec<StatisticRow> {
    let attribute_high_is_good: HashMap<i64, bool> = tables
        .attributes
        .iter()
        .map(|attribute| (attribute.id, attribute.high_is_good))
        .collect();

    let published: HashMap<i64, bool> = tables
        .types
        .iter()
        .map(|row| (row.id, row.published))
        .collect();

    let mut type_values: HashMap<(i64, i64), Option<f64>> = HashMap::new();
    for row in &tables.type_attributes {
        type_values
            .entry((row.type_id, row.attribute_id))
            .or_insert(row.value);
    }

    let mut attributes_by_mutaplasmid: HashMap<i64, Vec<usize>> = HashMap::new();
    for (index, row) in tables.mutaplasmid_attributes.iter().enumerate() {
        attributes_by_mutaplasmid
            .entry(row.mutaplasmid_id)
            .or_default()
            .push(index);
    }

    let mut mutaplasmids: Vec<_> = tables.mutaplasmids.iter().collect();
    mutaplasmids.sort_by_key(|mutaplasmid| mutaplasmid.id);

    let mut rows = Vec::new();
    let mut next_id = 0i64;

    for mutaplasmid in mutaplasmids {
        let attribute_indexes = attributes_by_mutaplasmid
            .get(&mutaplasmid.id)
            .map(Vec::as_slice)
            .unwrap_or_default();

        for input_type in &tables.input_types {
            if input_type.mutaplasmid_id != mutaplasmid.id
                || !published.get(&input_type.type_id).copied().unwrap_or(false)
            {
                continue;
            }

            for &index in attribute_indexes {
                let attribute = &tables.mutaplasmid_attributes[index];

                // The legacy service treats a missing record, a null value
                // and a zero value all as base value 1 (PHP truthiness).
                let source_value = match type_values
                    .get(&(input_type.type_id, attribute.attribute_id))
                    .copied()
                    .flatten()
                {
                    Some(value) if value != 0.0 => value,
                    _ => 1.0,
                };

                let mut max_value = round5(source_value * attribute.value_max);
                let mut min_value = round5(source_value * attribute.value_min);

                // A negative base flips which multiplier extreme is larger.
                if source_value < 0.0 {
                    std::mem::swap(&mut min_value, &mut max_value);
                }

                let high_is_good = attribute
                    .high_is_good
                    .unwrap_or_else(|| {
                        attribute_high_is_good
                            .get(&attribute.attribute_id)
                            .copied()
                            .unwrap_or(false)
                    });

                next_id += 1;
                rows.push(StatisticRow {
                    id: next_id,
                    type_id: input_type.type_id,
                    mutaplasmid_id: mutaplasmid.id,
                    attribute_id: attribute.attribute_id,
                    best: if high_is_good { max_value } else { min_value },
                    worst: if high_is_good { min_value } else { max_value },
                    high_is_good,
                    is_virtual: attribute.is_virtual,
                });
            }
        }
    }

    rows
}
