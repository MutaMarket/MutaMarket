//! The OpenGraph attribute selection (src/og/service.rs, port of
//! `App\Services\OpenGraph\OpenGraphService`) against the legacy
//! characterization fixtures: 445 real modules with their exact stored
//! attributes, plus the reference dumps for the display names the card
//! orders by.
//!
//! No database and no rendering here — this pins which rows reach a card
//! and in which order.

use std::collections::HashMap;
use std::path::Path;

use mutamarket::modules::view::{ModuleAttributeView, UnitRef};
use mutamarket::mutation::reference::ReferenceTables;
use mutamarket::og::card_attributes;

use crate::common;

/// The reference dumps, for attribute display names and units.
fn reference() -> ReferenceTables {
    ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse")
}

/// The fixtures' expected attributes as the view rows a card is built from.
fn views(module: &common::ModuleFixture, tables: &ReferenceTables) -> Vec<ModuleAttributeView> {
    let definitions: HashMap<i64, _> = tables
        .attributes
        .iter()
        .map(|attribute| (attribute.id, attribute))
        .collect();
    let units: HashMap<i64, _> = tables.units.iter().map(|unit| (unit.id, unit)).collect();

    module
        .expected
        .attributes
        .iter()
        .map(|attribute| {
            let definition = definitions
                .get(&attribute.attribute_id)
                .expect("fixture attribute is in the reference dump");

            ModuleAttributeView {
                attribute_id: attribute.attribute_id,
                name: definition.name.clone(),
                display_name: definition.display_name.clone(),
                value: attribute.value,
                base_value: attribute.base_value,
                fraction: attribute.fraction,
                fraction_type: attribute.fraction_type,
                fraction_absolute: attribute.fraction_absolute,
                bar: attribute.bar as i16,
                is_derived: attribute.is_derived,
                unit: definition.unit_id.and_then(|unit_id| {
                    units.get(&unit_id).map(|unit| UnitRef {
                        id: unit.id,
                        name: unit.name.clone(),
                        display_name: unit.display_name.clone(),
                    })
                }),
                is_virtual: attribute.is_virtual,
                type_band: None,
            }
        })
        .collect()
}

#[test]
fn every_fixture_module_drops_its_virtual_attributes_and_orders_derived_last() {
    let tables = reference();
    let fixtures = common::load_module_fixtures();
    assert!(!fixtures.is_empty(), "the fixtures are the spec");

    let mut modules_checked = 0;
    let mut virtual_attributes_dropped = 0;

    for fixture in &fixtures {
        for module in &fixture.modules {
            let attributes = views(module, &tables);
            let card = card_attributes(&attributes);

            // The legacy filter is `is_virtual` only, not the stricter
            // `is_visual` the module cards use: a zero-valued real
            // attribute still gets a row.
            let expected: Vec<i64> = attributes
                .iter()
                .filter(|attribute| !attribute.is_virtual)
                .map(|attribute| attribute.attribute_id)
                .collect();
            let mut sorted_ids: Vec<i64> = card.iter().map(|row| row.id).collect();
            sorted_ids.sort_unstable();
            let mut expected_sorted = expected.clone();
            expected_sorted.sort_unstable();
            assert_eq!(
                sorted_ids, expected_sorted,
                "module {} keeps exactly the non-virtual attributes",
                module.module_id,
            );
            virtual_attributes_dropped += attributes.len() - card.len();

            // Legacy sortByAttributeNameAndDerived: derived last, then the
            // display name byte for byte.
            let keys: Vec<(bool, &str)> = card
                .iter()
                .map(|row| (row.derived, row.name.as_str()))
                .collect();
            let mut ordered = keys.clone();
            ordered.sort();
            assert_eq!(
                keys, ordered,
                "module {} orders derived attributes last, then by display name",
                module.module_id,
            );

            for row in &card {
                assert!(
                    (0.0..=1.0).contains(&row.fraction),
                    "module {} attribute {} has a 0..1 magnitude fraction, got {}",
                    module.module_id,
                    row.id,
                    row.fraction,
                );
                assert!(!row.name.is_empty(), "every row is labelled");
                assert!(!row.value.is_empty(), "every row shows its rolled value");
            }

            modules_checked += 1;
        }
    }

    assert!(modules_checked >= 400, "all 445 fixture modules run");
    assert!(
        virtual_attributes_dropped > 0,
        "the fixtures do contain virtual attributes, so the filter is exercised",
    );
}

#[test]
fn the_fraction_sign_becomes_the_bar_direction() {
    let tables = reference();
    let fixtures = common::load_module_fixtures();

    let (mut positives, mut negatives) = (0, 0);

    for fixture in &fixtures {
        for module in &fixture.modules {
            let attributes = views(module, &tables);
            let by_id: HashMap<i64, &ModuleAttributeView> = attributes
                .iter()
                .map(|attribute| (attribute.attribute_id, attribute))
                .collect();

            for row in card_attributes(&attributes) {
                let stored = by_id[&row.id];

                assert_eq!(
                    row.is_positive,
                    stored.fraction.is_sign_positive(),
                    "the legacy is_positive is the sign of the stored fraction",
                );
                assert!(
                    (row.fraction - stored.fraction.abs()).abs() < 1e-12,
                    "the legacy fraction is the magnitude of the stored one",
                );
                assert_eq!(row.derived, stored.is_derived);
                assert_eq!(row.bar, stored.bar);

                if row.is_positive {
                    positives += 1;
                } else {
                    negatives += 1;
                }
            }
        }
    }

    assert!(positives > 0 && negatives > 0, "both directions occur");
}

#[test]
fn ordering_and_filtering_follow_the_legacy_comparator_exactly() {
    let attribute =
        |id: i64, display_name: &str, derived: bool, is_virtual: bool, fraction: f64| {
            ModuleAttributeView {
                attribute_id: id,
                name: display_name.to_owned(),
                display_name: display_name.to_owned(),
                value: 10.0,
                base_value: 8.0,
                fraction,
                fraction_type: 0.0,
                fraction_absolute: 0.0,
                bar: 0,
                is_derived: derived,
                unit: None,
                is_virtual,
                type_band: None,
            }
        };

    let attributes = vec![
        attribute(1, "Zeta", false, false, 0.5),
        attribute(2, "Alpha Derived", true, false, -0.25),
        attribute(3, "Alpha", false, false, 0.75),
        attribute(4, "Hidden", false, true, 0.5),
        // Uppercase sorts before lowercase under strcmp, which is a byte
        // comparison, not a locale-aware one.
        attribute(5, "alpha", false, false, 0.1),
    ];

    let card = card_attributes(&attributes);

    assert_eq!(
        card.iter().map(|row| row.id).collect::<Vec<_>>(),
        [3, 1, 5, 2],
        "virtual dropped, derived last, byte order within each group",
    );
    assert!(card[0].is_positive);
    assert!(!card[3].is_positive, "a negative fraction points left");
    assert_eq!(card[3].fraction, 0.25);
}
