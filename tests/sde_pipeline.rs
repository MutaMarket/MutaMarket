//! Hermetic verification of the SDE pipeline against the legacy fixture
//! dumps. The dumps are outputs of the legacy seeders, so the ported
//! enrichment and statistics computations must reconstruct them exactly:
//!
//! - stripping the derived/virtual rows and re-running enrichment must
//!   restore the original tables, and
//! - recomputing all statistics from the base tables must match the dumped
//!   statistics rows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mutamarket::mutation::reference::ReferenceTables;
use mutamarket::sde::enrich::{
    DERIVED_ATTRIBUTE_ID_START, add_derived_attributes, add_virtual_attributes,
};
use mutamarket::sde::statistics::{compute_abyssal_statistics, compute_statistics};

fn fixture_tables() -> ReferenceTables {
    ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference"))
        .expect("reference fixtures load")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= f64::max(1e-9, a.abs() * 1e-9)
}

#[test]
fn statistics_computation_reconstructs_the_fixture_rows() {
    let tables = fixture_tables();
    let computed = compute_statistics(&tables);

    let expected: BTreeMap<(i64, i64, i64), _> = tables
        .statistics
        .iter()
        .map(|row| ((row.type_id, row.mutaplasmid_id, row.attribute_id), row))
        .collect();
    let actual: BTreeMap<(i64, i64, i64), _> = computed
        .iter()
        .map(|row| ((row.type_id, row.mutaplasmid_id, row.attribute_id), row))
        .collect();

    let expected_keys: BTreeSet<_> = expected.keys().collect();
    let actual_keys: BTreeSet<_> = actual.keys().collect();

    let missing: Vec<_> = expected_keys.difference(&actual_keys).take(10).collect();
    let extra: Vec<_> = actual_keys.difference(&expected_keys).take(10).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "statistic keys diverge; missing: {missing:?}, extra: {extra:?}",
    );

    let mut mismatches = Vec::new();
    for (key, expected_row) in &expected {
        let actual_row = actual[key];
        if !close(expected_row.best, actual_row.best)
            || !close(expected_row.worst, actual_row.worst)
            || expected_row.high_is_good != actual_row.high_is_good
            || expected_row.is_virtual != actual_row.is_virtual
        {
            mismatches.push(format!(
                "{key:?}: expected best {} worst {} hig {} virtual {}, got best {} worst {} hig {} virtual {}",
                expected_row.best,
                expected_row.worst,
                expected_row.high_is_good,
                expected_row.is_virtual,
                actual_row.best,
                actual_row.worst,
                actual_row.high_is_good,
                actual_row.is_virtual,
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {} statistics diverge (showing up to 20):\n{}",
        mismatches.len(),
        expected.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// The legacy fixture export has no abyssal_type_statistics dump, so the
/// aggregation is verified by independent properties over the dumped
/// statistics rows instead of a snapshot: exact group key set, first-seen
/// group order, direction flags from the lowest-id row, and extremes that
/// are attained by and bound every row of the group.
#[test]
fn abyssal_statistics_aggregate_the_fixture_statistics() {
    let tables = fixture_tables();
    let computed = compute_abyssal_statistics(&tables);

    let output_types: BTreeMap<i64, i64> = tables
        .mutaplasmids
        .iter()
        .map(|mutaplasmid| (mutaplasmid.id, mutaplasmid.output_type_id))
        .collect();

    // Group the dumped statistics rows by (abyssal type, attribute).
    let mut groups: BTreeMap<(i64, i64), Vec<&mutamarket::mutation::reference::StatisticRow>> =
        BTreeMap::new();
    for row in &tables.statistics {
        let output_type_id = output_types[&row.mutaplasmid_id];
        groups
            .entry((output_type_id, row.attribute_id))
            .or_default()
            .push(row);
    }

    // Every group appears exactly once, and ids are sequential from 1.
    assert!(!computed.is_empty(), "no abyssal statistics computed");
    let computed_keys: BTreeSet<(i64, i64)> = computed
        .iter()
        .map(|row| (row.type_id, row.attribute_id))
        .collect();
    assert_eq!(
        computed_keys.len(),
        computed.len(),
        "duplicate group emitted"
    );
    assert_eq!(
        computed_keys,
        groups.keys().copied().collect::<BTreeSet<_>>(),
        "group key set diverges",
    );
    assert_eq!(
        computed.iter().map(|row| row.id).collect::<Vec<_>>(),
        (1..=computed.len() as i64).collect::<Vec<_>>(),
        "ids are not sequential",
    );

    // Groups come out ordered like the legacy seeder saw them: by the
    // first (lowest-id) statistic row of each group.
    let mut expected_order: Vec<((i64, i64), i64)> = groups
        .iter()
        .map(|(key, rows)| {
            (
                *key,
                rows.iter().map(|row| row.id).min().expect("non-empty"),
            )
        })
        .collect();
    expected_order.sort_by_key(|(_, first_id)| *first_id);
    assert_eq!(
        computed
            .iter()
            .map(|row| (row.type_id, row.attribute_id))
            .collect::<Vec<_>>(),
        expected_order
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>(),
        "group order diverges from first-seen statistic order",
    );

    let mut multi_mutaplasmid_groups = 0usize;
    for aggregate in &computed {
        let rows = &groups[&(aggregate.type_id, aggregate.attribute_id)];

        // Direction flags come from the group's first row by id.
        let first = rows.iter().min_by_key(|row| row.id).expect("non-empty");
        assert_eq!(
            aggregate.high_is_good, first.high_is_good,
            "high_is_good source"
        );
        assert_eq!(aggregate.is_virtual, first.is_virtual, "is_virtual source");

        // The aggregate extremes bound every row and are attained by one.
        for row in rows {
            if aggregate.high_is_good {
                assert!(row.best <= aggregate.best && row.worst >= aggregate.worst);
            } else {
                assert!(row.best >= aggregate.best && row.worst <= aggregate.worst);
            }
        }
        assert!(
            rows.iter().any(|row| row.best == aggregate.best),
            "best not attained"
        );
        assert!(
            rows.iter().any(|row| row.worst == aggregate.worst),
            "worst not attained"
        );

        if rows
            .iter()
            .map(|row| row.mutaplasmid_id)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
        {
            multi_mutaplasmid_groups += 1;
        }
    }

    // The aggregation must actually merge across mutaplasmids somewhere,
    // or this test proves nothing about the cross-mutaplasmid extremes.
    assert!(
        multi_mutaplasmid_groups > 0,
        "no group spans several mutaplasmids"
    );
}

#[test]
fn enrichment_reconstructs_the_derived_and_virtual_fixture_rows() {
    let original = fixture_tables();

    let mut stripped = original.clone();
    stripped.attributes.retain(|attribute| !attribute.derived);
    stripped
        .units
        .retain(|unit| unit.id < DERIVED_ATTRIBUTE_ID_START);
    stripped
        .mutaplasmid_attributes
        .retain(|row| !row.is_virtual && row.attribute_id < DERIVED_ATTRIBUTE_ID_START);
    stripped
        .type_attributes
        .retain(|row| row.attribute_id < DERIVED_ATTRIBUTE_ID_START);

    add_virtual_attributes(&mut stripped);
    add_derived_attributes(&mut stripped);

    // The app-defined units must come back exactly.
    let expected_units: BTreeMap<i64, _> = original
        .units
        .iter()
        .filter(|unit| unit.id >= DERIVED_ATTRIBUTE_ID_START)
        .map(|unit| (unit.id, unit))
        .collect();
    let actual_units: BTreeMap<i64, _> = stripped
        .units
        .iter()
        .filter(|unit| unit.id >= DERIVED_ATTRIBUTE_ID_START)
        .map(|unit| (unit.id, unit))
        .collect();

    assert_eq!(
        expected_units.keys().collect::<Vec<_>>(),
        actual_units.keys().collect::<Vec<_>>(),
        "derived unit ids diverge",
    );
    for (id, expected) in &expected_units {
        let actual = actual_units[id];
        assert_eq!(expected.name, actual.name, "unit {id} name");
        assert_eq!(
            expected.display_name, actual.display_name,
            "unit {id} display name"
        );
    }

    // Derived attribute definitions must come back with the same ids,
    // formulas and operands.
    let expected_attributes: BTreeMap<i64, _> = original
        .attributes
        .iter()
        .filter(|attribute| attribute.derived)
        .map(|attribute| (attribute.id, attribute))
        .collect();
    let actual_attributes: BTreeMap<i64, _> = stripped
        .attributes
        .iter()
        .filter(|attribute| attribute.derived)
        .map(|attribute| (attribute.id, attribute))
        .collect();

    assert_eq!(
        expected_attributes.keys().collect::<Vec<_>>(),
        actual_attributes.keys().collect::<Vec<_>>(),
        "derived attribute ids diverge",
    );

    for (id, expected) in &expected_attributes {
        let actual = actual_attributes[id];
        assert_eq!(expected.name, actual.name, "attribute {id} name");
        assert_eq!(
            expected.display_name, actual.display_name,
            "attribute {id} display name"
        );
        assert_eq!(expected.unit_id, actual.unit_id, "attribute {id} unit");
        assert_eq!(
            expected.derived_operation, actual.derived_operation,
            "attribute {id} operation",
        );
        assert_eq!(
            expected.derived_attributes, actual.derived_attributes,
            "attribute {id} operands",
        );
        assert_eq!(
            expected.high_is_good, actual.high_is_good,
            "attribute {id} high_is_good"
        );
    }

    // Mutaplasmid attribute rows (including the virtual and derived ones)
    // must reconstruct with identical ranges.
    let expected_rows: BTreeMap<(i64, i64), _> = original
        .mutaplasmid_attributes
        .iter()
        .map(|row| ((row.mutaplasmid_id, row.attribute_id), row))
        .collect();
    let actual_rows: BTreeMap<(i64, i64), _> = stripped
        .mutaplasmid_attributes
        .iter()
        .map(|row| ((row.mutaplasmid_id, row.attribute_id), row))
        .collect();

    assert_eq!(
        expected_rows.keys().collect::<Vec<_>>(),
        actual_rows.keys().collect::<Vec<_>>(),
        "mutaplasmid attribute keys diverge",
    );

    let mut mismatches = Vec::new();
    for (key, expected) in &expected_rows {
        let actual = actual_rows[key];
        if !close(expected.value_min, actual.value_min)
            || !close(expected.value_max, actual.value_max)
            || expected.high_is_good != actual.high_is_good
            || expected.is_virtual != actual.is_virtual
        {
            mismatches.push(format!(
                "{key:?}: expected [{}, {}] hig {:?} virtual {}, got [{}, {}] hig {:?} virtual {}",
                expected.value_min,
                expected.value_max,
                expected.high_is_good,
                expected.is_virtual,
                actual.value_min,
                actual.value_max,
                actual.high_is_good,
                actual.is_virtual,
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mutaplasmid attribute rows diverge (showing up to 20):\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Derived type attribute values must reconstruct as well.
    let expected_values: BTreeMap<(i64, i64), Option<f64>> = original
        .type_attributes
        .iter()
        .map(|row| ((row.type_id, row.attribute_id), row.value))
        .collect();
    let actual_values: BTreeMap<(i64, i64), Option<f64>> = stripped
        .type_attributes
        .iter()
        .map(|row| ((row.type_id, row.attribute_id), row.value))
        .collect();

    assert_eq!(
        expected_values.keys().collect::<Vec<_>>(),
        actual_values.keys().collect::<Vec<_>>(),
        "type attribute keys diverge",
    );

    let mut mismatches = Vec::new();
    for (key, expected) in &expected_values {
        let actual = actual_values[key];
        let matches = match (expected, actual) {
            (Some(expected), Some(actual)) => close(*expected, actual),
            (None, None) => true,
            _ => false,
        };
        if !matches {
            mismatches.push(format!("{key:?}: expected {expected:?}, got {actual:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} type attribute values diverge (showing up to 20):\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    );
}
