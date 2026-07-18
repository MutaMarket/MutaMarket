//! App-level enrichment applied on top of the raw SDE, ported from the
//! legacy `MutaplasmidSeeder` and `DerivedAttributeSeeder`:
//!
//! - corrections for known CCP data mistakes,
//! - virtual attributes (fixed 1x "rolls" shown for context in the UI),
//! - derived attributes (synthetic stats like "shield boost per second"),
//!   including their computed roll-multiplier ranges per mutaplasmid and
//!   their computed base values per source type.
//!
//! Verified by reconstructing the legacy fixture dumps exactly.

use std::collections::{HashMap, HashSet};

use crate::mutation::context::AttributeDef;
use crate::mutation::reference::{MutaplasmidAttributeRow, ReferenceTables, TypeAttributeRow};

/// Attribute values for attribute 2346 of these mutaplasmids are swapped in
/// CCP's data.
pub fn apply_ccp_corrections(tables: &mut ReferenceTables) {
    for row in &mut tables.mutaplasmid_attributes {
        if [56299, 56300, 56301].contains(&row.mutaplasmid_id) && row.attribute_id == 2346 {
            std::mem::swap(&mut row.value_min, &mut row.value_max);
            row.high_is_good = Some(false);
        }
    }
}

/// Fixed 1x-1x virtual attributes the legacy app attaches to some
/// mutaplasmid families.
pub fn add_virtual_attributes(tables: &mut ReferenceTables) {
    add_virtual_attribute(tables, "%Ballistic Control%", 1255);
    add_virtual_attribute(tables, "%Microwarpdrive%", 147);
}

fn add_virtual_attribute(tables: &mut ReferenceTables, pattern: &str, attribute_id: i64) {
    let mutaplasmid_ids: Vec<i64> = tables
        .mutaplasmids
        .iter()
        .filter(|mutaplasmid| sql_like(&mutaplasmid.name, pattern))
        .map(|mutaplasmid| mutaplasmid.id)
        .collect();

    for mutaplasmid_id in mutaplasmid_ids {
        push_mutaplasmid_attribute(
            tables,
            MutaplasmidAttributeRow {
                id: 0,
                mutaplasmid_id,
                attribute_id,
                value_min: 1.0,
                value_max: 1.0,
                high_is_good: Some(true),
                is_virtual: true,
            },
        );
    }
}

/// The id range reserved for app-defined derived attributes.
pub const DERIVED_ATTRIBUTE_ID_START: i64 = 5_000_000;

/// A derived attribute of the form `numerator / denominator`.
struct RatioDerived {
    name: &'static str,
    numerator: i64,
    denominator: i64,
    mutaplasmid_patterns: &'static [&'static str],
    /// Whether source types with a zero denominator are skipped (the
    /// per-capacitor variants) instead of divided anyway.
    skip_zero_denominator_types: bool,
}

const SHIELD_BOOST: i64 = 68;
const ARMOR_REPAIR: i64 = 84;
const DURATION: i64 = 73;
const CAPACITOR: i64 = 6;
const MISSILE_DAMAGE: i64 = 213;
const TURRET_DAMAGE: i64 = 64;
const RATE_OF_FIRE: i64 = 204;
const MINING_AMOUNT: i64 = 77;
const MINING_CRIT_CHANCE: i64 = 5967;
const MINING_CRIT_BONUS_YIELD: i64 = 5969;

/// The first six derived attributes, in legacy creation order (their ids are
/// assigned sequentially from [`DERIVED_ATTRIBUTE_ID_START`]).
const RATIO_DERIVED: [RatioDerived; 6] = [
    RatioDerived {
        name: "shieldBoostPerTime",
        numerator: SHIELD_BOOST,
        denominator: DURATION,
        mutaplasmid_patterns: &["%Shield Booster%"],
        skip_zero_denominator_types: false,
    },
    RatioDerived {
        name: "shieldBoostPerCapacitor",
        numerator: SHIELD_BOOST,
        denominator: CAPACITOR,
        mutaplasmid_patterns: &["%Shield Booster%"],
        skip_zero_denominator_types: true,
    },
    RatioDerived {
        name: "armorRepairPerTime",
        numerator: ARMOR_REPAIR,
        denominator: DURATION,
        mutaplasmid_patterns: &["%Armor Repairer%"],
        skip_zero_denominator_types: false,
    },
    RatioDerived {
        name: "armorRepairPerCapacitor",
        numerator: ARMOR_REPAIR,
        denominator: CAPACITOR,
        mutaplasmid_patterns: &["%Armor Repairer%"],
        skip_zero_denominator_types: true,
    },
    RatioDerived {
        name: "dpsIncreaseMissiles",
        numerator: MISSILE_DAMAGE,
        denominator: RATE_OF_FIRE,
        mutaplasmid_patterns: &["%Ballistic Control System%"],
        skip_zero_denominator_types: false,
    },
    RatioDerived {
        name: "dpsIncreaseTurrets",
        numerator: TURRET_DAMAGE,
        denominator: RATE_OF_FIRE,
        mutaplasmid_patterns: &[
            "%Gyrostabilizer%",
            "%Heat Sink%",
            "%Entropic Radiation Sink%",
            "%Magnetic Field Stabilizer%",
            "%Vorton Tuning System%",
        ],
        skip_zero_denominator_types: false,
    },
];

const EFFECTIVE_MINING_PATTERNS: [&str; 6] = [
    "%Mining Laser%",
    "%Deep Core Mining Laser%",
    "%Ice Mining Laser%",
    "%Strip Miner%",
    "%Ice Harvester Mutaplasmid",
    "%Modulated Deep Core Miner Mutaplasmid",
];

const MINING_SPEED_PATTERNS: [&str; 6] = [
    "%Gas Cloud Harvester%",
    "%Gas Cloud Scoop%",
    "%Mining Drone%",
    "%Excavator%Mining%",
    "%Ice Harvesting Drone%",
    "%Excavator%Ice%",
];

/// Adds the eight app-defined derived attributes: the attribute definitions,
/// their roll-multiplier ranges on the applicable mutaplasmids, and their
/// computed base values on the applicable source types.
pub fn add_derived_attributes(tables: &mut ReferenceTables) {
    let mut next_id = DERIVED_ATTRIBUTE_ID_START;

    for spec in &RATIO_DERIVED {
        let attribute_id = next_id;
        next_id += 1;

        define_attribute(tables, attribute_id, spec.name, "{1}/{2}", vec![
            spec.numerator,
            spec.denominator,
        ]);

        for mutaplasmid_id in matching_mutaplasmids(tables, spec.mutaplasmid_patterns) {
            let Some(numerator) = mutaplasmid_range(tables, mutaplasmid_id, spec.numerator) else {
                continue;
            };
            let Some(denominator) = mutaplasmid_range(tables, mutaplasmid_id, spec.denominator)
            else {
                continue;
            };

            push_mutaplasmid_attribute(tables, MutaplasmidAttributeRow {
                id: 0,
                mutaplasmid_id,
                attribute_id,
                value_min: numerator.0 / denominator.1,
                value_max: numerator.1 / denominator.0,
                high_is_good: None,
                is_virtual: false,
            });
        }

        for (type_id, values) in types_with(tables, &[spec.numerator, spec.denominator]) {
            let numerator = values[0].unwrap_or(0.0);
            let denominator = values[1].unwrap_or(0.0);

            if spec.skip_zero_denominator_types && denominator == 0.0 {
                continue;
            }

            push_type_attribute(tables, type_id, attribute_id, numerator / denominator);
        }
    }

    add_effective_mining_speed(tables, next_id);
    add_mining_speed(tables, next_id + 1);
}

/// `miningAmount * (1 + critChance * critBonusYield) / duration`, for mining
/// modules that can roll critical strikes.
fn add_effective_mining_speed(tables: &mut ReferenceTables, attribute_id: i64) {
    define_attribute(
        tables,
        attribute_id,
        "effectiveMiningSpeed",
        "{1}*(1+{2}*{3})/{4}",
        vec![MINING_AMOUNT, MINING_CRIT_CHANCE, MINING_CRIT_BONUS_YIELD, DURATION],
    );

    // Typical base values used to normalize the crit factor of the
    // multiplier range: crit chance 0.01, crit bonus yield 2.
    let base_crit_chance = 0.01;
    let base_crit_bonus = 2.0;
    let base_factor = 1.0 + base_crit_chance * base_crit_bonus;

    for mutaplasmid_id in matching_mutaplasmids(tables, &EFFECTIVE_MINING_PATTERNS) {
        if mutaplasmid_range(tables, mutaplasmid_id, MINING_CRIT_CHANCE).is_none() {
            continue;
        }

        let ranges = [
            mutaplasmid_range(tables, mutaplasmid_id, MINING_AMOUNT),
            mutaplasmid_range(tables, mutaplasmid_id, MINING_CRIT_CHANCE),
            mutaplasmid_range(tables, mutaplasmid_id, MINING_CRIT_BONUS_YIELD),
            mutaplasmid_range(tables, mutaplasmid_id, DURATION),
        ];
        let [Some(amount), Some(crit), Some(bonus), Some(duration)] = ranges else {
            continue;
        };

        let best = (amount.1 * (1.0 + base_crit_chance * crit.1 * base_crit_bonus * bonus.1))
            / (base_factor * duration.0);
        let worst = (amount.0 * (1.0 + base_crit_chance * crit.0 * base_crit_bonus * bonus.0))
            / (base_factor * duration.1);

        push_mutaplasmid_attribute(tables, MutaplasmidAttributeRow {
            id: 0,
            mutaplasmid_id,
            attribute_id,
            value_min: worst,
            value_max: best,
            high_is_good: None,
            is_virtual: false,
        });
    }

    for (type_id, values) in types_with(tables, &[
        MINING_AMOUNT,
        MINING_CRIT_CHANCE,
        MINING_CRIT_BONUS_YIELD,
        DURATION,
    ]) {
        let amount = values[0].unwrap_or(0.0);
        let crit_chance = values[1].unwrap_or(0.0);
        let crit_bonus_yield = values[2].unwrap_or(0.0);
        let duration = values[3].unwrap_or(1.0);

        if duration == 0.0 {
            continue;
        }

        // Value stays in m3/ms; formatting converts to m3/s.
        let value = amount * (1.0 + crit_chance * crit_bonus_yield) / duration;
        push_type_attribute(tables, type_id, attribute_id, value);
    }
}

/// `miningAmount / duration`, for mining modules and drones without crit.
fn add_mining_speed(tables: &mut ReferenceTables, attribute_id: i64) {
    define_attribute(tables, attribute_id, "miningSpeed", "{1}/{2}", vec![
        MINING_AMOUNT,
        DURATION,
    ]);

    for mutaplasmid_id in matching_mutaplasmids(tables, &MINING_SPEED_PATTERNS) {
        if mutaplasmid_range(tables, mutaplasmid_id, MINING_CRIT_CHANCE).is_some() {
            continue;
        }

        let (Some(amount), Some(duration)) = (
            mutaplasmid_range(tables, mutaplasmid_id, MINING_AMOUNT),
            mutaplasmid_range(tables, mutaplasmid_id, DURATION),
        ) else {
            continue;
        };

        push_mutaplasmid_attribute(tables, MutaplasmidAttributeRow {
            id: 0,
            mutaplasmid_id,
            attribute_id,
            value_min: amount.0 / duration.1,
            value_max: amount.1 / duration.0,
            high_is_good: None,
            is_virtual: false,
        });
    }

    let with_crit: HashSet<i64> = types_with(tables, &[MINING_CRIT_CHANCE])
        .into_iter()
        .map(|(type_id, _)| type_id)
        .collect();

    for (type_id, values) in types_with(tables, &[MINING_AMOUNT, DURATION]) {
        if with_crit.contains(&type_id) {
            continue;
        }

        let amount = values[0].unwrap_or(0.0);
        let duration = values[1].unwrap_or(1.0);

        if duration == 0.0 {
            continue;
        }

        push_type_attribute(tables, type_id, attribute_id, amount / duration);
    }
}

// --- Helpers ---------------------------------------------------------------

fn define_attribute(
    tables: &mut ReferenceTables,
    id: i64,
    name: &str,
    operation: &str,
    operands: Vec<i64>,
) {
    tables.attributes.push(AttributeDef {
        id,
        name: name.to_owned(),
        high_is_good: true,
        derived: true,
        derived_operation: Some(operation.to_owned()),
        derived_attributes: operands,
    });
}

fn matching_mutaplasmids(tables: &ReferenceTables, patterns: &[&str]) -> Vec<i64> {
    tables
        .mutaplasmids
        .iter()
        .filter(|mutaplasmid| patterns.iter().any(|pattern| sql_like(&mutaplasmid.name, pattern)))
        .map(|mutaplasmid| mutaplasmid.id)
        .collect()
}

/// (value_min, value_max) of a mutaplasmid's attribute, if it has it.
fn mutaplasmid_range(
    tables: &ReferenceTables,
    mutaplasmid_id: i64,
    attribute_id: i64,
) -> Option<(f64, f64)> {
    tables
        .mutaplasmid_attributes
        .iter()
        .find(|row| row.mutaplasmid_id == mutaplasmid_id && row.attribute_id == attribute_id)
        .map(|row| (row.value_min, row.value_max))
}

/// Every type carrying all the given attributes, with their values in the
/// same order, sorted by type id for deterministic output.
fn types_with(tables: &ReferenceTables, attribute_ids: &[i64]) -> Vec<(i64, Vec<Option<f64>>)> {
    let mut per_type: HashMap<i64, HashMap<i64, Option<f64>>> = HashMap::new();
    for row in &tables.type_attributes {
        if attribute_ids.contains(&row.attribute_id) {
            per_type.entry(row.type_id).or_default().insert(row.attribute_id, row.value);
        }
    }

    let mut matching: Vec<(i64, Vec<Option<f64>>)> = per_type
        .into_iter()
        .filter(|(_, values)| attribute_ids.iter().all(|id| values.contains_key(id)))
        .map(|(type_id, values)| {
            (
                type_id,
                attribute_ids.iter().map(|id| values[id]).collect(),
            )
        })
        .collect();

    matching.sort_by_key(|(type_id, _)| *type_id);
    matching
}

/// Appends a mutaplasmid attribute with the next free row id, unless the
/// (mutaplasmid, attribute) pair already exists.
fn push_mutaplasmid_attribute(tables: &mut ReferenceTables, row: MutaplasmidAttributeRow) {
    let exists = tables.mutaplasmid_attributes.iter().any(|existing| {
        existing.mutaplasmid_id == row.mutaplasmid_id && existing.attribute_id == row.attribute_id
    });
    if exists {
        return;
    }

    let id = tables
        .mutaplasmid_attributes
        .iter()
        .map(|existing| existing.id)
        .max()
        .unwrap_or(0)
        + 1;

    tables.mutaplasmid_attributes.push(MutaplasmidAttributeRow { id, ..row });
}

/// Appends a type attribute with the next free row id, unless the
/// (type, attribute) pair already exists.
fn push_type_attribute(tables: &mut ReferenceTables, type_id: i64, attribute_id: i64, value: f64) {
    let exists = tables
        .type_attributes
        .iter()
        .any(|existing| existing.type_id == type_id && existing.attribute_id == attribute_id);
    if exists {
        return;
    }

    let id = tables
        .type_attributes
        .iter()
        .map(|existing| existing.id)
        .max()
        .unwrap_or(0)
        + 1;

    tables.type_attributes.push(TypeAttributeRow {
        id,
        type_id,
        attribute_id,
        value: Some(value),
    });
}

/// Case-insensitive SQL LIKE with `%` wildcards, matching MySQL's default
/// collation behavior that the legacy name queries relied on.
fn sql_like(value: &str, pattern: &str) -> bool {
    let value = value.to_lowercase();
    let pattern = pattern.to_lowercase();

    let starts_anchored = !pattern.starts_with('%');
    let ends_anchored = !pattern.ends_with('%');
    let segments: Vec<&str> = pattern.split('%').filter(|segment| !segment.is_empty()).collect();

    let Some((&last, leading)) = segments.split_last() else {
        return true;
    };

    let mut position = 0usize;

    for (index, &segment) in leading.iter().enumerate() {
        if index == 0 && starts_anchored {
            if !value.starts_with(segment) {
                return false;
            }
            position = segment.len();
            continue;
        }

        match value[position..].find(segment) {
            Some(found) => position += found + segment.len(),
            None => return false,
        }
    }

    let remainder = &value[position..];

    if ends_anchored {
        if leading.is_empty() && starts_anchored {
            // Single anchored segment: the pattern has no wildcards at all.
            return value == last;
        }
        return remainder.ends_with(last);
    }

    if leading.is_empty() && starts_anchored {
        return value.starts_with(last);
    }

    remainder.contains(last)
}

#[cfg(test)]
mod tests {
    use super::sql_like;

    #[test]
    fn sql_like_matches_contains_prefix_suffix_and_sequences() {
        assert!(sql_like("Decayed 50MN Microwarpdrive Mutaplasmid", "%Microwarpdrive%"));
        assert!(sql_like("ORE Ice Harvester Mutaplasmid", "%Ice Harvester Mutaplasmid"));
        assert!(!sql_like("Ice Harvester Mutaplasmid II", "%Ice Harvester Mutaplasmid"));
        assert!(sql_like("Unstable Excavator Deluxe Mining Drone", "%Excavator%Mining%"));
        assert!(!sql_like("Unstable Mining Excavator Drone", "%Excavator%Mining%"));
        assert!(sql_like("gravid heat sink mutaplasmid", "%Heat Sink%"));
    }
}
