//! Port of `App\Services\OpenGraph\OpenGraphService`: which of a module's
//! mutated attributes reach the card, in which order, and the handful of
//! derived values the row draws.
//!
//! The legacy service recomputed the roll range, the percentage increase
//! and the fraction from the source type and the mutaplasmid. All of that
//! is already stored per mutated attribute here (`src/mutation`), so the
//! port keeps only the parts that are genuinely presentation: the filter,
//! the ordering, and the split of the stored signed fraction into the
//! magnitude and direction the bar is drawn from.

use crate::modules::view::ModuleAttributeView;

/// One attribute row of the module card: the legacy
/// `OpenGraphModuleAttribute` reduced to the fields `ModuleAttribute`
/// actually draws.
#[derive(Debug, Clone, PartialEq)]
pub struct CardAttribute {
    /// The attribute id, which is also the name of its icon file.
    pub id: i64,
    pub name: String,
    /// The rolled value with its unit, legacy `actual_value_formatted`.
    pub value: String,
    /// The signed change against the base value, legacy `difference_formatted`.
    pub difference: String,
    /// How much of the reachable roll range the roll used, 0..1. The legacy
    /// `fraction`, which is a magnitude; the direction is `is_positive`.
    pub fraction: f64,
    /// Whether the roll improved the module, legacy `is_positive`.
    pub is_positive: bool,
    pub derived: bool,
    /// The legacy `AttributeBar` as an integer: -1 brown, 0 none, 1 gold,
    /// 2 diamond.
    pub bar: i16,
}

/// The card rows of a module, legacy `getModuleInstanceAttributes`.
pub fn card_attributes(attributes: &[ModuleAttributeView]) -> Vec<CardAttribute> {
    let mut selected: Vec<&ModuleAttributeView> = attributes
        .iter()
        .filter(|attribute| !attribute.is_virtual)
        .collect();

    // Legacy `sortByAttributeNameAndDerived`: derived attributes after real
    // ones, then `strcmp` on the display name, which is a byte comparison
    // and so exactly `str::cmp`. Both sorts are stable.
    selected.sort_by(|a, b| {
        a.is_derived
            .cmp(&b.is_derived)
            .then_with(|| a.display_name.as_str().cmp(b.display_name.as_str()))
    });

    selected
        .into_iter()
        .map(|attribute| CardAttribute {
            id: attribute.attribute_id,
            name: attribute.display_name.clone(),
            value: attribute.formatted_value(),
            difference: attribute.formatted_difference(),
            // The stored fraction carries both halves of what the legacy
            // service computed separately: `src/mutation` clamps the share
            // of the roll range to 0..1 and then negates it when the roll
            // did not improve the module, which is the legacy `is_positive`
            // condition verbatim. The sign bit survives a zero magnitude,
            // so it is read rather than compared against zero.
            fraction: attribute.fraction.abs(),
            is_positive: attribute.fraction.is_sign_positive(),
            derived: attribute.is_derived,
            bar: attribute.bar,
        })
        .collect()
}
