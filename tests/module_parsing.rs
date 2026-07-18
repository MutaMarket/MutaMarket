//! Characterization tests for the mutation roll-quality math: 445 real
//! modules with their exact expected outputs, exported from the legacy app
//! (`app:generate-module-parsing-fixtures`). A mismatch means the Rust port
//! diverges from the legacy `ModuleAttributeCalculator` pipeline.

use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;

use mutamarket::mutation::calculator::{DogmaAttribute, average_fraction, calculate};
use mutamarket::mutation::reference::{ContextCache, ReferenceData};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    type_id: i64,
    modules: Vec<ModuleFixture>,
}

#[derive(Deserialize)]
struct ModuleFixture {
    module_id: i64,
    source_type_id: i64,
    mutaplasmid_id: i64,
    input_attributes: Vec<InputAttribute>,
    expected: Expected,
}

#[derive(Deserialize)]
struct InputAttribute {
    attribute_id: i64,
    value: f64,
}

#[derive(Deserialize)]
struct Expected {
    average_fraction: f64,
    attributes: Vec<ExpectedAttribute>,
}

#[derive(Deserialize)]
struct ExpectedAttribute {
    attribute_id: i64,
    value: f64,
    base_value: f64,
    fraction: f64,
    fraction_type: f64,
    fraction_absolute: f64,
    bar: i64,
    is_derived: bool,
    is_virtual: bool,
}

fn reference() -> &'static ReferenceData {
    static REFERENCE: OnceLock<ReferenceData> = OnceLock::new();
    REFERENCE.get_or_init(|| {
        ReferenceData::load_from_dir(Path::new("tests/fixtures/reference"))
            .expect("reference fixtures load")
    })
}

/// Same tolerance as the legacy characterization test.
fn matches(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= f64::max(1e-9, expected.abs() * 1e-9)
}

#[test]
fn calculates_module_attributes_matching_the_legacy_fixture_snapshots() {
    let mut paths: Vec<_> = std::fs::read_dir("tests/fixtures/module_parsing")
        .expect("fixture directory")
        .map(|entry| entry.expect("fixture entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no module parsing fixtures found");

    let mut failures = Vec::new();
    let mut modules_checked = 0usize;
    let mut contexts = ContextCache::new(reference());

    for path in &paths {
        let fixture: Fixture =
            serde_json::from_reader(File::open(path).expect("fixture file")).expect("fixture JSON");

        for module in &fixture.modules {
            modules_checked += 1;
            let context = format!("module {} (type {})", module.module_id, fixture.type_id);

            let Some(mutation_context) =
                contexts.context(module.mutaplasmid_id, module.source_type_id)
            else {
                failures.push(format!("{context}: no mutation context"));
                continue;
            };

            let dogma: Vec<DogmaAttribute> = module
                .input_attributes
                .iter()
                .map(|attribute| DogmaAttribute {
                    attribute_id: attribute.attribute_id,
                    value: attribute.value,
                })
                .collect();

            let results = calculate(mutation_context, &dogma);

            if results.len() != module.expected.attributes.len() {
                failures.push(format!(
                    "{context}: expected {} attributes, got {}",
                    module.expected.attributes.len(),
                    results.len(),
                ));
            }

            for expected in &module.expected.attributes {
                let attribute_context = format!("{context}, attribute {}", expected.attribute_id);

                let Some(result) = results
                    .iter()
                    .find(|result| result.attribute_id == expected.attribute_id)
                else {
                    failures.push(format!("{attribute_context}: missing result"));
                    continue;
                };

                let checks: [(&str, f64, f64); 5] = [
                    ("value", expected.value, result.value),
                    ("base_value", expected.base_value, result.base_value),
                    ("fraction", expected.fraction, result.fraction),
                    ("fraction_type", expected.fraction_type, result.fraction_type),
                    (
                        "fraction_absolute",
                        expected.fraction_absolute,
                        result.fraction_absolute,
                    ),
                ];

                for (field, expected_value, actual_value) in checks {
                    if !matches(expected_value, actual_value) {
                        failures.push(format!(
                            "{attribute_context}, {field}: expected {expected_value}, got {actual_value}"
                        ));
                    }
                }

                if expected.bar != result.bar.as_int() {
                    failures.push(format!(
                        "{attribute_context}, bar: expected {}, got {}",
                        expected.bar,
                        result.bar.as_int(),
                    ));
                }
                if expected.is_derived != result.is_derived {
                    failures.push(format!("{attribute_context}: is_derived mismatch"));
                }
                if expected.is_virtual != result.is_virtual {
                    failures.push(format!("{attribute_context}: is_virtual mismatch"));
                }
            }

            let actual_average = average_fraction(&results);
            if !actual_average.is_some_and(|actual| matches(module.expected.average_fraction, actual)) {
                failures.push(format!(
                    "{context}, average_fraction: expected {}, got {actual_average:?}",
                    module.expected.average_fraction,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {modules_checked} modules diverge from the legacy snapshots (showing up to 40):\n{}",
        failures.len(),
        failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"),
    );

    // The committed legacy fixture set contains exactly 445 modules; anything
    // else means the fixtures were not fully loaded (or were regenerated
    // without updating this expectation).
    assert_eq!(modules_checked, 445);
}
