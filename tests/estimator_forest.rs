//! Characterization of the native regression tree against scikit-learn.
//!
//! `tests/fixtures/estimator/tree_fixture.json` holds two datasets plus
//! the full structure and predictions of real sklearn
//! `DecisionTreeRegressor`s fitted on them with the legacy
//! hyperparameters (`gen_tree_fixture.py` next to the fixture is the
//! generator). The datasets are designed tie-free — where two splits score
//! exactly equal sklearn breaks the tie with its shuffled feature order,
//! which no port can reproduce — so our tree must match sklearn node for
//! node: same features, bit-identical thresholds, same predictions.

use serde::Deserialize;

use mutamarket::estimator::forest::{Dataset, Forest, cross_validate, fit_single_tree};

#[derive(Deserialize)]
struct TreeCase {
    max_depth: usize,
    x_train: Vec<Vec<f32>>,
    y_train: Vec<f32>,
    x_test: Vec<Vec<f32>>,
    predictions_train: Vec<f64>,
    predictions_test: Vec<f64>,
    node_count: usize,
    feature: Vec<i64>,
    threshold: Vec<f64>,
    children_left: Vec<i64>,
    children_right: Vec<i64>,
}

#[derive(Deserialize)]
struct TreeFixture {
    deep: TreeCase,
    wide: TreeCase,
}

fn fixture() -> TreeFixture {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/estimator/tree_fixture.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture readable"))
        .expect("fixture parses")
}

fn dataset(case: &TreeCase) -> Dataset {
    Dataset {
        n_features: case.x_train[0].len(),
        features: case.x_train.iter().flatten().copied().collect(),
        targets: case.y_train.clone(),
    }
}

/// Walks our tree and sklearn's arrays in lockstep, asserting identical
/// split features and bit-identical thresholds at every node.
fn assert_structure_matches(case: &TreeCase, tree: &mutamarket::estimator::forest::Tree) {
    fn walk(tree: &mutamarket::estimator::forest::Tree, ours: u32, case: &TreeCase, theirs: i64) {
        let (feature, threshold, left, right) = tree.node(ours);
        let their_feature = case.feature[theirs as usize];
        assert_eq!(
            feature < 0,
            their_feature < 0,
            "leaf/split mismatch at sklearn node {theirs}",
        );
        if feature < 0 {
            return;
        }
        assert_eq!(feature as i64, their_feature, "feature at sklearn node {theirs}");
        assert_eq!(
            threshold.to_bits(),
            case.threshold[theirs as usize].to_bits(),
            "threshold at sklearn node {theirs}: ours {threshold}, sklearn {}",
            case.threshold[theirs as usize],
        );
        walk(tree, left, case, case.children_left[theirs as usize]);
        walk(tree, right, case, case.children_right[theirs as usize]);
    }
    assert_eq!(tree.node_count(), case.node_count);
    walk(tree, 0, case, 0);
}

fn assert_predictions_match(case: &TreeCase, tree: &mutamarket::estimator::forest::Tree) {
    for (rows, expected, label) in [
        (&case.x_train, &case.predictions_train, "train"),
        (&case.x_test, &case.predictions_test, "test"),
    ] {
        for (index, (row, want)) in rows.iter().zip(expected.iter()).enumerate() {
            let got = tree.predict(row);
            let scale = want.abs().max(1.0);
            assert!(
                (got - want).abs() <= scale * 1e-12,
                "{label} row {index}: ours {got} vs sklearn {want}",
            );
        }
    }
}

#[test]
fn single_feature_tree_reproduces_sklearn_exactly() {
    let case = fixture().deep;
    let tree = fit_single_tree(&dataset(&case), case.max_depth);

    assert_structure_matches(&case, &tree);
    assert_predictions_match(&case, &tree);
}

#[test]
fn multi_feature_depth_capped_tree_reproduces_sklearn_exactly() {
    let case = fixture().wide;
    let tree = fit_single_tree(&dataset(&case), case.max_depth);

    assert_structure_matches(&case, &tree);
    assert_predictions_match(&case, &tree);
}

#[test]
fn forest_learns_the_fixture_relationship() {
    // The forest has no bit-exact sklearn counterpart (bootstrap RNG
    // differs), so pin its quality instead: cross-validation on the
    // near-noiseless single-feature fixture must explain almost all the
    // variance, and a forest fitted on the wide fixture must track its
    // training targets closely.
    let deep = fixture().deep;
    let metrics = cross_validate(&dataset(&deep), 42);
    assert!(metrics.r2 > 0.95, "cv r2 too low: {}", metrics.r2);
    assert!(metrics.mae < 2.0, "cv mae too high: {}", metrics.mae);

    let case = fixture().wide;
    let data = dataset(&case);

    let forest = Forest::fit(&data, vec![], 42);
    let truth_mean =
        case.y_train.iter().map(|y| *y as f64).sum::<f64>() / case.y_train.len() as f64;
    let (mut ss_residual, mut ss_total) = (0.0, 0.0);
    for (row, target) in case.x_train.iter().zip(&case.y_train) {
        let prediction = forest.predict(row);
        ss_residual += (prediction - *target as f64).powi(2);
        ss_total += (*target as f64 - truth_mean).powi(2);
    }
    let train_r2 = 1.0 - ss_residual / ss_total;
    assert!(train_r2 > 0.9, "train r2 too low: {train_r2}");
}
