//! Port of the legacy `ModuleAttributeCalculator`, `FractionCalculator`,
//! `DerivedAttributeCalculator` and `AttributeBarResolver`. The exact
//! arithmetic (including the 5-decimal rounding and the PHP null-coalescing
//! fallbacks) is characterization-tested against the legacy fixture snapshots.

use super::context::{MutaplasmidAttribute, MutationContext};

/// A raw dogma attribute of a mutated item as returned by ESI.
#[derive(Debug, Clone, Copy)]
pub struct DogmaAttribute {
    pub attribute_id: i64,
    pub value: f64,
}

/// Roll-quality marker: gold/diamond for the best possible roll of the
/// source type + mutaplasmid combination, brown for the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeBar {
    BrownBar,
    NoBar,
    GoldBar,
    DiamondBar,
}

impl AttributeBar {
    pub fn as_int(self) -> i64 {
        match self {
            AttributeBar::BrownBar => -1,
            AttributeBar::NoBar => 0,
            AttributeBar::GoldBar => 1,
            AttributeBar::DiamondBar => 2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttributeMutationResult {
    pub attribute_id: i64,
    pub value: f64,
    pub base_value: f64,
    /// Roll quality within the roll range of the module's own mutaplasmid, -1..1.
    pub fraction: f64,
    /// Roll quality within the combined roll range of all mutaplasmids
    /// producing the abyssal type, -1..1.
    pub fraction_type: f64,
    /// Position within all (source type x mutaplasmid) combinations for the
    /// abyssal type, mapped so that higher is always better, 0..1.
    pub fraction_absolute: f64,
    pub is_derived: bool,
    pub is_virtual: bool,
    pub bar: AttributeBar,
}

struct DerivedResult {
    attribute_id: i64,
    value: f64,
    base_value: f64,
}

/// Turns the raw dogma attributes of a mutated module into one result per
/// mutaplasmid attribute, carrying the rolled value, its base value and the
/// three roll-quality fractions.
pub fn calculate(context: &MutationContext, dogma: &[DogmaAttribute]) -> Vec<AttributeMutationResult> {
    let mutated: Vec<DogmaAttribute> = dogma
        .iter()
        .filter(|attribute| context.mutaplasmid_attribute(attribute.attribute_id).is_some())
        .copied()
        .collect();

    let derived_results = calculate_derived(context, &mutated);

    context
        .mutaplasmid_attributes
        .iter()
        .map(|attribute| calculate_attribute(context, attribute, &mutated, &derived_results))
        .collect()
}

/// Mean `fraction` over the real (non-virtual, non-derived) attributes;
/// `None` when there are no such attributes.
pub fn average_fraction(results: &[AttributeMutationResult]) -> Option<f64> {
    let fractions: Vec<f64> = results
        .iter()
        .filter(|result| !result.is_virtual && !result.is_derived)
        .map(|result| result.fraction)
        .collect();

    if fractions.is_empty() {
        return None;
    }

    Some(fractions.iter().sum::<f64>() / fractions.len() as f64)
}

fn calculate_attribute(
    context: &MutationContext,
    mutaplasmid_attribute: &MutaplasmidAttribute,
    mutated: &[DogmaAttribute],
    derived_results: &[DerivedResult],
) -> AttributeMutationResult {
    let attribute_id = mutaplasmid_attribute.attribute_id;

    let dogma_value = first_value(mutated, attribute_id);
    let derived_result = derived_results
        .iter()
        .find(|result| result.attribute_id == attribute_id);

    let value = dogma_value
        .or(derived_result.map(|result| result.value))
        .unwrap_or(0.0);
    let base_value = context
        .source_value(attribute_id)
        .or(derived_result.map(|result| result.base_value))
        .unwrap_or(0.0);

    AttributeMutationResult {
        attribute_id,
        value,
        base_value,
        fraction: roll_fraction(context, mutaplasmid_attribute, value),
        fraction_type: type_fraction(context, mutaplasmid_attribute, value),
        fraction_absolute: absolute_fraction(context, mutaplasmid_attribute, value),
        is_derived: mutaplasmid_attribute.attribute.derived,
        is_virtual: mutaplasmid_attribute.is_virtual,
        bar: resolve_bar(context, attribute_id, value),
    }
}

fn first_value(attributes: &[DogmaAttribute], attribute_id: i64) -> Option<f64> {
    attributes
        .iter()
        .find(|attribute| attribute.attribute_id == attribute_id)
        .map(|attribute| attribute.value)
}

// --- Derived (synthetic) attributes ---------------------------------------

fn calculate_derived(context: &MutationContext, mutated: &[DogmaAttribute]) -> Vec<DerivedResult> {
    context
        .mutaplasmid_attributes
        .iter()
        .filter(|attribute| attribute.attribute.derived)
        .map(|attribute| {
            let operand_ids = &attribute.attribute.derived_attributes;
            let operation = attribute.attribute.derived_operation.as_deref();

            let values: Vec<Option<f64>> = operand_ids
                .iter()
                .map(|&id| first_value(mutated, id))
                .collect();
            let base_values: Vec<Option<f64>> = operand_ids
                .iter()
                .map(|&id| context.source_value(id))
                .collect();

            DerivedResult {
                attribute_id: attribute.attribute_id,
                value: evaluate(operation, &values),
                base_value: evaluate(operation, &base_values),
            }
        })
        .collect()
}

fn evaluate(operation: Option<&str>, operands: &[Option<f64>]) -> f64 {
    let operand = |index: usize| operands.get(index).copied().flatten();

    match operation {
        Some("{1}/{2}") => {
            if operand(1) == Some(0.0) {
                0.0
            } else {
                operand(0).unwrap_or(0.0) / operand(1).unwrap_or(0.0)
            }
        }
        // Effective mining amount: miningAmount * (1 + critChance * critBonusYield)
        // / duration. Result is in m3/ms; formatting converts to m3/s.
        Some("{1}*(1+{2}*{3})/{4}") => {
            let divisor = operand(3).unwrap_or(1.0);
            if divisor == 0.0 {
                0.0
            } else {
                operand(0).unwrap_or(0.0) * (1.0 + operand(1).unwrap_or(0.0) * operand(2).unwrap_or(0.0))
                    / divisor
            }
        }
        _ => 0.0,
    }
}

// --- Fractions -------------------------------------------------------------

/// How good the roll is within the roll range of the module's own mutaplasmid.
fn roll_fraction(context: &MutationContext, attribute: &MutaplasmidAttribute, value: f64) -> f64 {
    // A missing source attribute record (as opposed to a record without a
    // value) means the fraction cannot be computed at all.
    let Some(source_attribute) = context.source_type_attributes.get(&attribute.attribute_id) else {
        return 0.0;
    };

    bounded_fraction(
        source_attribute.unwrap_or(1.0),
        value,
        attribute.value_max,
        attribute.value_min,
        high_is_good(attribute),
    )
}

/// How good the roll is within the combined roll range of all mutaplasmids
/// producing the same abyssal type.
fn type_fraction(context: &MutationContext, attribute: &MutaplasmidAttribute, value: f64) -> f64 {
    let ranges = context.ranges(attribute.attribute_id);

    bounded_fraction(
        context.source_value(attribute.attribute_id).unwrap_or(1.0),
        value,
        ranges.mutator_max.unwrap_or(0.0),
        ranges.mutator_min.unwrap_or(0.0),
        high_is_good(attribute),
    )
}

/// Where the value sits within all (source type x mutaplasmid) combinations
/// for the abyssal type, mapped so that higher is always better.
fn absolute_fraction(context: &MutationContext, attribute: &MutaplasmidAttribute, value: f64) -> f64 {
    let ranges = context.ranges(attribute.attribute_id);

    let source_value_min = ranges.source_value_min.unwrap_or(0.0);
    let source_value_max = ranges.source_value_max.unwrap_or(0.0);
    let mutator_min = ranges.mutator_min.unwrap_or(0.0);
    let mutator_max = ranges.mutator_max.unwrap_or(0.0);

    let permutations = [
        source_value_min * mutator_min,
        source_value_max * mutator_max,
        mutator_min * source_value_max,
        source_value_min * mutator_max,
    ];

    let min = permutations.iter().copied().fold(f64::INFINITY, f64::min);
    let max = permutations.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let mapped = (value - min) / (max - min);

    if high_is_good(attribute) { mapped } else { 1.0 - mapped }
}

fn bounded_fraction(
    source_value: f64,
    actual_value: f64,
    value_max: f64,
    value_min: f64,
    high_is_good: bool,
) -> f64 {
    let max_value = round5(source_value * value_max);
    let min_value = round5(source_value * value_min);

    let source_value = if source_value == 0.0 { 1.0 } else { source_value };

    let percentage_increase = if source_value.abs() > actual_value.abs() {
        -100.0 + (actual_value / source_value) * 100.0
    } else {
        ((actual_value - source_value) / source_value) * 100.0
    };

    let mut percentage_increase_min = ((min_value - source_value) / source_value) * 100.0;
    let mut percentage_increase_max = ((max_value - source_value) / source_value) * 100.0;

    if percentage_increase_min == 0.0 {
        percentage_increase_min = 1.0;
    }
    if percentage_increase_max == 0.0 {
        percentage_increase_max = 1.0;
    }

    let mut fraction = if percentage_increase >= 0.0 {
        percentage_increase / percentage_increase_max
    } else {
        percentage_increase / percentage_increase_min
    }
    .clamp(0.0, 1.0);

    let is_positive = if high_is_good {
        actual_value > source_value
    } else {
        actual_value < source_value
    };

    if !is_positive {
        fraction = -fraction;
    }

    round5(fraction)
}

/// The mutaplasmid attribute's own `high_is_good` override, falling back to
/// the attribute definition.
fn high_is_good(attribute: &MutaplasmidAttribute) -> bool {
    attribute.high_is_good.unwrap_or(attribute.attribute.high_is_good)
}

/// PHP `round($value * 100000) / 100000`: five decimals, half away from zero.
fn round5(value: f64) -> f64 {
    (value * 100000.0).round() / 100000.0
}

// --- Bars ------------------------------------------------------------------

/// Mutaplasmid grades that can never yield the overall best roll of a type,
/// so their perfect rolls don't get a bar.
const WEAK_MUTATORS: [&str; 6] = [
    "Decayed",
    "Glorified Decayed",
    "Gravid",
    "Glorified Gravid",
    "Radical",
    "Exigent",
];

fn resolve_bar(context: &MutationContext, attribute_id: i64, value: f64) -> AttributeBar {
    let name = context.mutaplasmid.name.as_str();

    if WEAK_MUTATORS.iter().any(|weak| name.starts_with(weak)) {
        return AttributeBar::NoBar;
    }

    let Some(statistic) = context.bar_statistic(attribute_id) else {
        return AttributeBar::NoBar;
    };

    if statistic.best == statistic.worst {
        return AttributeBar::NoBar;
    }

    if approximately_same(statistic.best, value) {
        if name.starts_with("Glorified") {
            return AttributeBar::DiamondBar;
        }

        return AttributeBar::GoldBar;
    }

    if approximately_same(statistic.worst, value) {
        return AttributeBar::BrownBar;
    }

    AttributeBar::NoBar
}

fn approximately_same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 0.0000001 * a.abs().max(b.abs())
}
