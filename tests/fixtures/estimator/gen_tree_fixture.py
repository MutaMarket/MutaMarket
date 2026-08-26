"""Generates the sklearn parity fixtures for the Rust regression tree.

A DecisionTreeRegressor with the legacy hyperparameters (min_samples_leaf=2,
squared_error) is deterministic only where no two splits score exactly equal
(sklearn breaks such ties by a shuffled feature order). The fixtures are
designed to be tie-free:

- "deep": one feature, full legacy depth 20 — cross-feature ties are
  impossible, and single-feature candidate positions never tie.
- "wide": four features, 3000 rows, depth-limited to 4 so every split
  node keeps >100 rows and two features never induce the same partition.

Both are verified tie-free by matching structurally in the Rust suite.
"""

import json

import numpy as np
from sklearn.tree import DecisionTreeRegressor


def fit(x, y, max_depth):
    tree = DecisionTreeRegressor(max_depth=max_depth, min_samples_leaf=2, random_state=42)
    tree.fit(x, y)
    return tree


def dump(tree, x_train, y_train, x_test):
    return {
        "max_depth": int(tree.get_params()["max_depth"]),
        "x_train": x_train.tolist(),
        "y_train": y_train.tolist(),
        "x_test": x_test.tolist(),
        "predictions_train": tree.predict(x_train).tolist(),
        "predictions_test": tree.predict(x_test).tolist(),
        "node_count": int(tree.tree_.node_count),
        "feature": tree.tree_.feature.tolist(),
        "threshold": tree.tree_.threshold.tolist(),
        "children_left": tree.tree_.children_left.tolist(),
        "children_right": tree.tree_.children_right.tolist(),
    }


rng = np.random.RandomState(0)

deep_x = rng.uniform(-10, 10, size=(240, 1)).astype(np.float32)
deep_y = (3.0 * deep_x[:, 0] + np.sin(deep_x[:, 0] * 0.7) * 4.0 + rng.normal(0, 0.5, 240)).astype(
    np.float32
)
deep_test = rng.uniform(-12, 12, size=(60, 1)).astype(np.float32)
deep = fit(deep_x, deep_y, 20)

wide_x = rng.uniform(-10, 10, size=(3000, 4)).astype(np.float32)
wide_y = (
    3.0 * wide_x[:, 0]
    - 2.0 * np.abs(wide_x[:, 1])
    + 0.5 * wide_x[:, 2] * wide_x[:, 3]
    + np.sin(wide_x[:, 0] * 0.7) * 4.0
    + rng.normal(0, 0.5, 3000)
).astype(np.float32)
wide_test = rng.uniform(-12, 12, size=(80, 4)).astype(np.float32)
wide = fit(wide_x, wide_y, 4)

fixture = {
    "sklearn_version": __import__("sklearn").__version__,
    "deep": dump(deep, deep_x, deep_y, deep_test),
    "wide": dump(wide, wide_x, wide_y, wide_test),
}

with open("tree_fixture.json", "w") as handle:
    json.dump(fixture, handle)

print("deep nodes:", deep.tree_.node_count, "depth:", deep.tree_.max_depth)
print("wide nodes:", wide.tree_.node_count, "depth:", wide.tree_.max_depth)
