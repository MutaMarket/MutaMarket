//! Shared helpers around the legacy characterization fixtures: 445 real
//! modules with their exact expected outputs. Any `ReferenceData` — however
//! it was loaded — must reproduce them.

// Compiled once per test binary; not every binary uses every helper.
#![allow(dead_code)]

use std::fs::File;

use mutamarket::mutation::calculator::{DogmaAttribute, average_fraction, calculate};
use mutamarket::mutation::reference::{ContextCache, ReferenceData};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Fixture {
    pub type_id: i64,
    pub modules: Vec<ModuleFixture>,
}

#[derive(Deserialize)]
pub struct ModuleFixture {
    pub module_id: i64,
    pub source_type_id: i64,
    pub mutaplasmid_id: i64,
    pub creator_id: i64,
    pub input_attributes: Vec<InputAttribute>,
    pub expected: Expected,
}

#[derive(Deserialize)]
pub struct InputAttribute {
    pub attribute_id: i64,
    pub value: f64,
}

#[derive(Deserialize)]
pub struct Expected {
    pub average_fraction: f64,
    pub attributes: Vec<ExpectedAttribute>,
}

#[derive(Deserialize)]
pub struct ExpectedAttribute {
    pub attribute_id: i64,
    pub value: f64,
    pub base_value: f64,
    pub fraction: f64,
    pub fraction_type: f64,
    pub fraction_absolute: f64,
    pub bar: i64,
    pub is_derived: bool,
    pub is_virtual: bool,
}

/// All module-parsing fixtures, sorted by file name.
pub fn load_module_fixtures() -> Vec<Fixture> {
    let mut paths: Vec<_> = std::fs::read_dir("tests/fixtures/module_parsing")
        .expect("fixture directory")
        .map(|entry| entry.expect("fixture entry").path())
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no module parsing fixtures found");

    paths
        .iter()
        .map(|path| {
            serde_json::from_reader(File::open(path).expect("fixture file")).expect("fixture JSON")
        })
        .collect()
}

pub fn fixture_dogma(module: &ModuleFixture) -> Vec<DogmaAttribute> {
    module
        .input_attributes
        .iter()
        .map(|attribute| DogmaAttribute {
            attribute_id: attribute.attribute_id,
            value: attribute.value,
        })
        .collect()
}

/// Same tolerance as the legacy characterization test.
pub fn matches(expected: f64, actual: f64) -> bool {
    (expected - actual).abs() <= f64::max(1e-9, expected.abs() * 1e-9)
}

pub fn assert_reference_matches_fixtures(reference: &ReferenceData) {
    let fixtures = load_module_fixtures();

    let mut failures = Vec::new();
    let mut modules_checked = 0usize;
    let mut contexts = ContextCache::new(reference);

    for fixture in &fixtures {
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
