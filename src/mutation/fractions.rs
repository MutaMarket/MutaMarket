//! The three roll-quality fractions of a mutated attribute value, port of
//! the legacy `FractionCalculator`:
//!
//! - [`roll_fraction`]: within the roll range of the module's own mutaplasmid
//! - [`type_fraction`]: within the combined roll range of all mutaplasmids
//!   producing the abyssal type
//! - [`absolute_fraction`]: within all (source type x mutaplasmid)
//!   combinations, normalized to 0..1
//!
//! The arithmetic reproduces the legacy PHP float-for-float — including its
//! 5-decimal rounding and some quirky-looking guards — because the fixture
//! snapshots assert at 1e-9 relative tolerance.

use super::context::{MutaplasmidAttribute, MutationContext};

/// How good the roll is within the roll range of the module's own mutaplasmid.
pub(super) fn roll_fraction(
    context: &MutationContext,
    attribute: &MutaplasmidAttribute,
    value: f64,
) -> f64 {
    // A missing source attribute record (as opposed to a record without a
    // value) means the fraction cannot be computed at all.
    let Some(source_value) = context.source_type_attributes.get(&attribute.attribute_id) else {
        return 0.0;
    };

    bounded_fraction(
        source_value.unwrap_or(1.0),
        value,
        attribute.value_max,
        attribute.value_min,
        high_is_good(attribute),
    )
}

/// How good the roll is within the combined roll range of all mutaplasmids
/// producing the same abyssal type.
pub(super) fn type_fraction(
    context: &MutationContext,
    attribute: &MutaplasmidAttribute,
    value: f64,
) -> f64 {
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
pub(super) fn absolute_fraction(
    context: &MutationContext,
    attribute: &MutaplasmidAttribute,
    value: f64,
) -> f64 {
    let ranges = context.ranges(attribute.attribute_id);

    let source_value_min = ranges.source_value_min.unwrap_or(0.0);
    let source_value_max = ranges.source_value_max.unwrap_or(0.0);
    let mutator_min = ranges.mutator_min.unwrap_or(0.0);
    let mutator_max = ranges.mutator_max.unwrap_or(0.0);

    // The extreme achievable values are products of a base-value extreme and
    // a roll-multiplier extreme; with negative values any pairing can be the
    // overall min or max, so consider all of them.
    let permutations = [
        source_value_min * mutator_min,
        source_value_max * mutator_max,
        mutator_min * source_value_max,
        source_value_min * mutator_max,
    ];

    let min = permutations.iter().copied().fold(f64::INFINITY, f64::min);
    let max = permutations
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    let mapped = (value - min) / (max - min);

    if high_is_good(attribute) {
        mapped
    } else {
        1.0 - mapped
    }
}

/// The share of the maximum possible percentage change that the roll
/// achieved, clamped to 0..1 and signed: positive when the roll improves the
/// module, negative when it worsens it.
fn bounded_fraction(
    source_value: f64,
    actual_value: f64,
    value_max: f64,
    value_min: f64,
    high_is_good: bool,
) -> f64 {
    // The range bounds are rounded to 5 decimals before use — the legacy
    // behavior, kept because it shifts results right at the range edges.
    let max_value = round5(source_value * value_max);
    let min_value = round5(source_value * value_min);

    let source_value = if source_value == 0.0 {
        1.0
    } else {
        source_value
    };

    // Both branches are algebraically identical, but not in floating point;
    // the legacy code chose per magnitude, so we must too.
    let percentage_increase = if source_value.abs() > actual_value.abs() {
        -100.0 + (actual_value / source_value) * 100.0
    } else {
        ((actual_value - source_value) / source_value) * 100.0
    };

    // A bound equal to the source value would make the share division
    // meaningless; the legacy code substitutes 1 (i.e. "per percent").
    let mut percentage_increase_min = ((min_value - source_value) / source_value) * 100.0;
    let mut percentage_increase_max = ((max_value - source_value) / source_value) * 100.0;

    if percentage_increase_min == 0.0 {
        percentage_increase_min = 1.0;
    }
    if percentage_increase_max == 0.0 {
        percentage_increase_max = 1.0;
    }

    // Positive changes are measured against the upper bound, negative ones
    // against the lower.
    let mut fraction = if percentage_increase >= 0.0 {
        percentage_increase / percentage_increase_max
    } else {
        percentage_increase / percentage_increase_min
    }
    .clamp(0.0, 1.0);

    let improves_module = if high_is_good {
        actual_value > source_value
    } else {
        actual_value < source_value
    };

    if !improves_module {
        fraction = -fraction;
    }

    round5(fraction)
}

/// The mutaplasmid attribute's own `high_is_good` override, falling back to
/// the attribute definition.
pub(super) fn high_is_good(attribute: &MutaplasmidAttribute) -> bool {
    attribute
        .high_is_good
        .unwrap_or(attribute.attribute.high_is_good)
}

/// PHP `round($value * 100000) / 100000`: five decimals, half away from zero.
/// Also used by the SDE statistics computation, which shares the legacy
/// rounding behavior.
pub(crate) fn round5(value: f64) -> f64 {
    (value * 100000.0).round() / 100000.0
}
