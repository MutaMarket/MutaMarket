//! Characterization test for the mutation roll-quality math using the
//! file-loaded reference fixtures. A mismatch means the Rust port diverges
//! from the legacy `ModuleAttributeCalculator` pipeline.

mod common;

use std::path::Path;

use mutamarket::mutation::reference::ReferenceData;

#[test]
fn calculates_module_attributes_matching_the_legacy_fixture_snapshots() {
    let reference = ReferenceData::load_from_dir(Path::new("tests/fixtures/reference"))
        .expect("reference fixtures load");

    common::assert_reference_matches_fixtures(&reference);
}
