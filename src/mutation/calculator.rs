//! Orchestration of the attribute math, ported from the legacy
//! `ModuleAttributeCalculator`: resolve each rollable attribute's value and
//! base value (falling back to derived formulas), then attach the three
//! roll-quality fractions and the best/worst-roll bar.

use std::collections::HashMap;

use super::bars::resolve_bar;
pub use super::bars::AttributeBar;
use super::context::MutationContext;
use super::derived::calculate_derived;
use super::fractions::{absolute_fraction, roll_fraction, type_fraction};

/// A raw dogma attribute of a mutated item as returned by ESI.
#[derive(Debug, Clone, Copy)]
pub struct DogmaAttribute {
    pub attribute_id: i64,
    pub value: f64,
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

/// Turns the raw dogma attributes of a mutated module into one result per
/// mutaplasmid attribute, carrying the rolled value, its base value and the
/// three roll-quality fractions. Results keep the context's attribute order.
pub fn calculate(context: &MutationContext, dogma: &[DogmaAttribute]) -> Vec<AttributeMutationResult> {
    let rolled = rolled_values(context, dogma);
    let derived = calculate_derived(context, &rolled);

    context
        .mutaplasmid_attributes
        .iter()
        .map(|attribute| {
            let attribute_id = attribute.attribute_id;
            let derived_values = derived.get(&attribute_id);

            // ESI reports real attributes directly; synthetic ones (and their
            // base values) come from the derived formulas.
            let value = rolled
                .get(&attribute_id)
                .copied()
                .or(derived_values.map(|derived| derived.value))
                .unwrap_or(0.0);
            let base_value = context
                .source_value(attribute_id)
                .or(derived_values.map(|derived| derived.base_value))
                .unwrap_or(0.0);

            AttributeMutationResult {
                attribute_id,
                value,
                base_value,
                fraction: roll_fraction(context, attribute, value),
                fraction_type: type_fraction(context, attribute, value),
                fraction_absolute: absolute_fraction(context, attribute, value),
                is_derived: attribute.attribute.derived,
                is_virtual: attribute.is_virtual,
                bar: resolve_bar(context, attribute_id, value),
            }
        })
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

/// The module's rolled values, restricted to the attributes this mutaplasmid
/// can actually mutate. On duplicate dogma entries the first occurrence wins,
/// matching the legacy `firstWhere` lookups.
fn rolled_values(context: &MutationContext, dogma: &[DogmaAttribute]) -> HashMap<i64, f64> {
    let mut rolled = HashMap::with_capacity(context.mutaplasmid_attributes.len());

    for attribute in dogma {
        if context.mutaplasmid_attribute(attribute.attribute_id).is_some() {
            rolled.entry(attribute.attribute_id).or_insert(attribute.value);
        }
    }

    rolled
}
