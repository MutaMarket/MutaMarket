//! Derived (synthetic) attributes such as "shield boost per second",
//! computed from the real dogma attributes with the formula stored on the
//! attribute definition (`derived_operation` over `derived_attributes` as
//! operands). Port of the legacy `DerivedAttributeCalculator`.

use std::collections::HashMap;

use super::context::MutationContext;

/// The evaluated formula for one derived attribute: once over the module's
/// rolled values and once over the source type's base values.
pub(super) struct DerivedValues {
    pub value: f64,
    pub base_value: f64,
}

/// Evaluates every derived attribute of the context's mutaplasmid, keyed by
/// attribute id.
pub(super) fn calculate_derived(
    context: &MutationContext,
    rolled: &HashMap<i64, f64>,
) -> HashMap<i64, DerivedValues> {
    context
        .mutaplasmid_attributes
        .iter()
        .filter(|attribute| attribute.attribute.derived)
        .map(|attribute| {
            let definition = &attribute.attribute;
            let operation = definition.derived_operation.as_deref();

            // Operands come from the rolled values (not raw dogma), so an
            // operand outside the mutaplasmid's attributes reads as missing —
            // exactly like the legacy calculator.
            let rolled_operands = operand_values(definition, |id| rolled.get(&id).copied());
            let base_operands = operand_values(definition, |id| context.source_value(id));

            (
                attribute.attribute_id,
                DerivedValues {
                    value: evaluate(operation, &rolled_operands),
                    base_value: evaluate(operation, &base_operands),
                },
            )
        })
        .collect()
}

fn operand_values(
    definition: &super::context::AttributeDef,
    lookup: impl Fn(i64) -> Option<f64>,
) -> Vec<Option<f64>> {
    definition.derived_attributes.iter().map(|&id| lookup(id)).collect()
}

/// Evaluates a derivation formula. Missing operands fall back the same way
/// PHP's null coercion did in the legacy implementation, and a zero divisor
/// yields 0 instead of infinity.
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
