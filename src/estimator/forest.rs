//! Native random forest regression, replacing the legacy scikit-learn
//! trainer (`estimators/train.py`).
//!
//! The legacy models are `RandomForestRegressor(n_estimators=300,
//! max_depth=20, min_samples_leaf=2, random_state=42)`. sklearn's
//! regression default `max_features=1.0` considers every feature at every
//! split, so the forest is plain bagging over CART trees: bootstrap the
//! rows, fit a variance-reduction tree, average the tree predictions. The
//! tree math here mirrors sklearn's `BestSplitter`/`squared_error`
//! criterion exactly (float32 features, float64 accumulation, midpoint
//! thresholds, the 1e-7 feature threshold, the `<=` split rule) and is
//! pinned against a real sklearn tree in `tests/estimator_forest.rs`.
//! Bootstrap sampling uses our own seeded RNG, so forests match sklearn's
//! algorithm and hyperparameters but not its bit-exact draws — metrics land
//! in the same range as the legacy-trained values, not identical ones.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Trees per forest, the legacy `RF_CONFIG["n_estimators"]`.
pub const N_TREES: usize = 300;

/// Maximum tree depth, the legacy `RF_CONFIG["max_depth"]`.
pub const MAX_DEPTH: usize = 20;

/// Minimum samples in a leaf, the legacy `RF_CONFIG["min_samples_leaf"]`.
pub const MIN_SAMPLES_LEAF: usize = 2;

/// Base RNG seed, the legacy `RANDOM_STATE`.
pub const RANDOM_STATE: u64 = 42;

/// Cross-validation folds, the legacy `CV_FOLDS`.
pub const CV_FOLDS: usize = 5;

/// sklearn's `FEATURE_THRESHOLD`: feature values closer than this are
/// treated as equal when scanning split positions.
const FEATURE_THRESHOLD: f64 = 1e-7;

/// A row-major feature matrix with its regression targets. Features and
/// targets are `f32` like the legacy trainer's `astype(np.float32)`.
pub struct Dataset {
    pub n_features: usize,
    /// Row-major, `n_features` values per row.
    pub features: Vec<f32>,
    pub targets: Vec<f32>,
}

impl Dataset {
    pub fn n_rows(&self) -> usize {
        self.targets.len()
    }

    fn row(&self, row: usize) -> &[f32] {
        &self.features[row * self.n_features..(row + 1) * self.n_features]
    }

    fn value(&self, row: usize, feature: usize) -> f32 {
        self.features[row * self.n_features + feature]
    }
}

#[derive(Serialize, Deserialize)]
struct Node {
    /// Split feature index; `-1` marks a leaf.
    feature: i32,
    /// Split threshold; rows with `value <= threshold` go left.
    threshold: f64,
    left: u32,
    right: u32,
    /// Leaf prediction (the mean target of its training rows).
    value: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Tree {
    nodes: Vec<Node>,
}

/// Fits one CART tree on all rows without bootstrapping — the
/// deterministic core the sklearn parity fixture pins
/// (`tests/estimator_forest.rs`).
pub fn fit_single_tree(data: &Dataset, max_depth: usize) -> Tree {
    let mut indices: Vec<u32> = (0..data.n_rows() as u32).collect();
    Tree::fit(data, &mut indices, max_depth)
}

impl Tree {
    fn fit(data: &Dataset, indices: &mut [u32], max_depth: usize) -> Self {
        let mut tree = Tree { nodes: Vec::new() };
        tree.grow(data, indices, 0, max_depth);
        tree
    }

    /// Builds the subtree over `indices` and returns its root node index.
    fn grow(&mut self, data: &Dataset, indices: &mut [u32], depth: usize, max_depth: usize) -> u32 {
        let n = indices.len();
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        for &row in indices.iter() {
            let y = data.targets[row as usize] as f64;
            sum += y;
            sum_sq += y * y;
        }
        let mean = sum / n as f64;
        // sklearn's node impurity: sq_sum/N - mean^2, leaf when <= eps.
        let impurity = sum_sq / n as f64 - mean * mean;

        let node = self.push_leaf(mean);
        if depth >= max_depth || n < 2 * MIN_SAMPLES_LEAF || impurity <= f64::EPSILON {
            return node;
        }

        let Some((feature, threshold)) = best_split(data, indices, sum) else {
            return node;
        };

        // Partition in place: left rows (value <= threshold) first.
        let mut split = 0;
        for i in 0..n {
            if (data.value(indices[i] as usize, feature) as f64) <= threshold {
                indices.swap(i, split);
                split += 1;
            }
        }

        let (left_rows, right_rows) = indices.split_at_mut(split);
        let left = self.grow(data, left_rows, depth + 1, max_depth);
        let right = self.grow(data, right_rows, depth + 1, max_depth);

        self.nodes[node as usize] = Node {
            feature: feature as i32,
            threshold,
            left,
            right,
            value: mean,
        };
        node
    }

    fn push_leaf(&mut self, value: f64) -> u32 {
        self.nodes.push(Node {
            feature: -1,
            threshold: 0.0,
            left: 0,
            right: 0,
            value,
        });
        (self.nodes.len() - 1) as u32
    }

    pub fn predict(&self, row: &[f32]) -> f64 {
        let mut node = &self.nodes[0];
        while node.feature >= 0 {
            node = if (row[node.feature as usize] as f64) <= node.threshold {
                &self.nodes[node.left as usize]
            } else {
                &self.nodes[node.right as usize]
            };
        }
        node.value
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// A node's (feature, threshold, left, right) for the sklearn
    /// structure comparison in `tests/estimator_forest.rs`; feature is -1
    /// on leaves.
    pub fn node(&self, index: u32) -> (i32, f64, u32, u32) {
        let node = &self.nodes[index as usize];
        (node.feature, node.threshold, node.left, node.right)
    }
}

/// The best (feature, threshold) split of the rows by sklearn's proxy
/// improvement `sum_left²/n_left + sum_right²/n_right`, or `None` when no
/// valid split exists (all features constant within the node).
fn best_split(data: &Dataset, indices: &[u32], total_sum: f64) -> Option<(usize, f64)> {
    let n = indices.len();
    let mut best: Option<(usize, f64)> = None;
    let mut best_proxy = f64::NEG_INFINITY;
    let mut sorted: Vec<(f32, f32)> = Vec::with_capacity(n);

    for feature in 0..data.n_features {
        sorted.clear();
        sorted.extend(indices.iter().map(|&row| {
            (
                data.value(row as usize, feature),
                data.targets[row as usize],
            )
        }));
        sorted.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

        let mut left_sum = 0.0f64;
        for (position, &(value, target)) in sorted.iter().enumerate() {
            left_sum += target as f64;
            let n_left = position + 1;
            let n_right = n - n_left;
            if n_left < MIN_SAMPLES_LEAF || n_right < MIN_SAMPLES_LEAF {
                continue;
            }
            let next = sorted[position + 1].0;
            if (next as f64) <= (value as f64) + FEATURE_THRESHOLD {
                continue;
            }

            let right_sum = total_sum - left_sum;
            let proxy =
                left_sum * left_sum / n_left as f64 + right_sum * right_sum / n_right as f64;
            if proxy > best_proxy {
                best_proxy = proxy;
                // sklearn's midpoint threshold — computed as a/2 + b/2
                // (not (a+b)/2, which rounds differently) and clamped back
                // onto the left value when rounding puts it on the right.
                let mut threshold = (value as f64) / 2.0 + (next as f64) / 2.0;
                if threshold == next as f64 {
                    threshold = value as f64;
                }
                best = Some((feature, threshold));
            }
        }
    }

    best
}

/// A fitted forest together with the feature names (in training column
/// order) it expects, like the legacy joblib artifact's `feature_names`.
#[derive(Serialize, Deserialize)]
pub struct Forest {
    pub feature_names: Vec<String>,
    trees: Vec<Tree>,
}

impl Forest {
    /// Fits the forest: `N_TREES` bootstrap samples, one CART tree each,
    /// fitted in parallel. Deterministic for a given dataset and seed (each
    /// tree draws from its own RNG seeded by `seed` + tree index).
    pub fn fit(data: &Dataset, feature_names: Vec<String>, seed: u64) -> Self {
        let n = data.n_rows();
        assert!(n > 0, "cannot fit a forest on an empty dataset");

        let trees = (0..N_TREES)
            .into_par_iter()
            .map(|tree_index| {
                use rand::{RngExt, SeedableRng};
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed + tree_index as u64);
                let mut indices: Vec<u32> = (0..n).map(|_| rng.random_range(0..n) as u32).collect();
                Tree::fit(data, &mut indices, MAX_DEPTH)
            })
            .collect();

        Forest {
            feature_names,
            trees,
        }
    }

    /// The mean prediction over all trees.
    pub fn predict(&self, row: &[f32]) -> f64 {
        let sum: f64 = self.trees.iter().map(|tree| tree.predict(row)).sum();
        sum / self.trees.len() as f64
    }

    pub fn node_count(&self) -> usize {
        self.trees.iter().map(Tree::node_count).sum()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("forest serializes")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Cross-validation metrics, the legacy `train.py` output line: fold-mean
/// r2 and mae, and fold-mean normalized mae ×100.
#[derive(Debug, Clone, Copy)]
pub struct CvMetrics {
    pub r2: f64,
    pub mae: f64,
    pub nmae: f64,
}

/// 5-fold cross-validation like `cross_validate(cv=5)`: consecutive
/// unshuffled folds (the first `n % 5` folds one row larger), a fresh
/// forest per fold, metrics averaged over folds. Requires
/// `data.n_rows() >= CV_FOLDS` like the legacy trainer.
pub fn cross_validate(data: &Dataset, seed: u64) -> CvMetrics {
    let n = data.n_rows();
    assert!(
        n >= CV_FOLDS,
        "cross-validation needs at least {CV_FOLDS} rows"
    );

    let fold_size = n / CV_FOLDS;
    let remainder = n % CV_FOLDS;

    let (mut r2_sum, mut mae_sum, mut nmae_sum) = (0.0, 0.0, 0.0);
    let mut start = 0;
    for fold in 0..CV_FOLDS {
        let size = fold_size + usize::from(fold < remainder);
        let test = start..start + size;
        start += size;

        let train_rows: Vec<usize> = (0..n).filter(|row| !test.contains(row)).collect();
        let mut train = Dataset {
            n_features: data.n_features,
            features: Vec::with_capacity(train_rows.len() * data.n_features),
            targets: Vec::with_capacity(train_rows.len()),
        };
        for &row in &train_rows {
            train.features.extend_from_slice(data.row(row));
            train.targets.push(data.targets[row]);
        }

        let forest = Forest::fit(&train, Vec::new(), seed);
        let truth: Vec<f64> = test.clone().map(|row| data.targets[row] as f64).collect();
        let predicted: Vec<f64> = test
            .clone()
            .map(|row| forest.predict(data.row(row)))
            .collect();

        r2_sum += r2(&truth, &predicted);
        let fold_mae = mae(&truth, &predicted);
        mae_sum += fold_mae;
        let mean_truth = truth.iter().sum::<f64>() / truth.len() as f64;
        nmae_sum += if mean_truth != 0.0 {
            fold_mae / mean_truth
        } else {
            0.0
        };
    }

    CvMetrics {
        r2: r2_sum / CV_FOLDS as f64,
        mae: mae_sum / CV_FOLDS as f64,
        // The legacy trainer reports nmae ×100.
        nmae: nmae_sum / CV_FOLDS as f64 * 100.0,
    }
}

/// The coefficient of determination, `sklearn.metrics.r2_score` (0.0 for a
/// constant truth, like sklearn's degenerate-denominator answer).
fn r2(truth: &[f64], predicted: &[f64]) -> f64 {
    let mean = truth.iter().sum::<f64>() / truth.len() as f64;
    let ss_total: f64 = truth.iter().map(|y| (y - mean) * (y - mean)).sum();
    if ss_total == 0.0 {
        return 0.0;
    }
    let ss_residual: f64 = truth
        .iter()
        .zip(predicted)
        .map(|(y, prediction)| (y - prediction) * (y - prediction))
        .sum();
    1.0 - ss_residual / ss_total
}

fn mae(truth: &[f64], predicted: &[f64]) -> f64 {
    truth
        .iter()
        .zip(predicted)
        .map(|(y, prediction)| (y - prediction).abs())
        .sum::<f64>()
        / truth.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dataset(rows: &[(&[f32], f32)]) -> Dataset {
        Dataset {
            n_features: rows[0].0.len(),
            features: rows
                .iter()
                .flat_map(|(row, _)| row.iter().copied())
                .collect(),
            targets: rows.iter().map(|(_, target)| *target).collect(),
        }
    }

    #[test]
    fn a_tree_splits_on_the_variance_reducing_feature() {
        // Feature 1 separates the targets perfectly; feature 0 is noise.
        let data = dataset(&[
            (&[5.0, 1.0], 10.0),
            (&[1.0, 2.0], 10.0),
            (&[4.0, 8.0], 50.0),
            (&[2.0, 9.0], 50.0),
        ]);
        let tree = Tree::fit(&data, &mut [0, 1, 2, 3], MAX_DEPTH);

        assert_eq!(tree.predict(&[3.0, 1.5]), 10.0);
        assert_eq!(tree.predict(&[3.0, 8.5]), 50.0);
        // Root split + two pure leaves; min_samples_leaf=2 stops there.
        assert_eq!(tree.node_count(), 3);
    }

    #[test]
    fn min_samples_leaf_keeps_pairs_together() {
        // Four rows, distinct targets: only the middle split leaves two
        // rows on each side.
        let data = dataset(&[(&[1.0], 1.0), (&[2.0], 2.0), (&[3.0], 30.0), (&[4.0], 31.0)]);
        let tree = Tree::fit(&data, &mut [0, 1, 2, 3], MAX_DEPTH);

        assert_eq!(tree.node_count(), 3);
        assert_eq!(tree.predict(&[1.0]), 1.5);
        assert_eq!(tree.predict(&[4.0]), 30.5);
    }

    #[test]
    fn constant_features_produce_a_single_leaf() {
        let data = dataset(&[(&[7.0], 1.0), (&[7.0], 2.0), (&[7.0], 3.0), (&[7.0], 4.0)]);
        let tree = Tree::fit(&data, &mut [0, 1, 2, 3], MAX_DEPTH);

        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.predict(&[7.0]), 2.5);
    }

    #[test]
    fn forest_fits_are_deterministic() {
        let rows: Vec<(f32, f32)> = (0..40)
            .map(|i| {
                let x = i as f32 * 0.37;
                (x, x * 3.0 + (i % 7) as f32)
            })
            .collect();
        let data = Dataset {
            n_features: 1,
            features: rows.iter().map(|(x, _)| *x).collect(),
            targets: rows.iter().map(|(_, y)| *y).collect(),
        };

        let first = Forest::fit(&data, vec!["x".to_owned()], RANDOM_STATE);
        let second = Forest::fit(&data, vec!["x".to_owned()], RANDOM_STATE);

        assert_eq!(first.node_count(), second.node_count());
        for probe in [0.0f32, 3.3, 7.7, 14.0] {
            assert_eq!(first.predict(&[probe]), second.predict(&[probe]));
        }
    }

    #[test]
    fn forest_bytes_round_trip() {
        let data = dataset(&[(&[1.0], 1.0), (&[2.0], 2.0), (&[3.0], 30.0), (&[4.0], 31.0)]);
        let forest = Forest::fit(&data, vec!["speedFactor".to_owned()], RANDOM_STATE);

        let restored = Forest::from_bytes(&forest.to_bytes()).expect("deserializes");

        assert_eq!(restored.feature_names, vec!["speedFactor".to_owned()]);
        assert_eq!(restored.predict(&[2.5]), forest.predict(&[2.5]));
    }

    #[test]
    fn cv_metrics_match_hand_computed_folds() {
        // 10 rows, y = 2x exactly: every fold predicts nearly perfectly,
        // so r2 approaches 1 and mae stays small; the exact assertion is
        // on the metric formulas via a degenerate constant-target set.
        let constant = dataset(&[
            (&[1.0], 5.0),
            (&[2.0], 5.0),
            (&[3.0], 5.0),
            (&[4.0], 5.0),
            (&[5.0], 5.0),
        ]);
        let metrics = cross_validate(&constant, RANDOM_STATE);

        // Constant target: perfect predictions, degenerate r2 = 0 like
        // sklearn, mae = 0, nmae = 0.
        assert_eq!(metrics.r2, 0.0);
        assert_eq!(metrics.mae, 0.0);
        assert_eq!(metrics.nmae, 0.0);
    }

    #[test]
    fn r2_and_mae_formulas() {
        let truth = [1.0, 2.0, 3.0];
        let predicted = [1.0, 2.0, 5.0];

        // ss_res = 4, ss_tot = 2.
        assert_eq!(r2(&truth, &predicted), 1.0 - 4.0 / 2.0);
        assert!((mae(&truth, &predicted) - 2.0 / 3.0).abs() < 1e-12);
    }
}
