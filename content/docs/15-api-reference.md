---
section: API
---

# API: Reference data

Two endpoints describe the machinery behind the numbers on module pages:
how good each price model is, and what a given attribute can actually roll.
Both are small, stable, and change on the order of weeks — cache them.

## Price estimator quality

```
GET /api/estimator-statistics
```

Quality metrics for every per-type price model: how much historic sales
data it was trained on, how well it fits that data, and its average error.

Use it to decide how much weight to put on a module's `estimated_value`. A
model with a null `r2` has not been trained yet, and modules of that type
carry no estimate at all.

| Field | Meaning |
|---|---|
| `type_id`, `name` | The abyssal type the model predicts. |
| `data_count` | Recorded sales it was trained on. Below the training threshold, the model is not built. |
| `data_statistics` | Training sales broken down by the source module's meta group. A model trained mostly on one meta group predicts the others less well. |
| `r2` | Fit against the training data, where 1 is perfect. Null means untrained. |
| `mae` | Mean absolute error, in ISK. |
| `nmae` | The same error normalized by the mean sale price, as a percentage. Comparable across types, unlike `mae`. |
| `last_trained_at` | When the model was last rebuilt. |

Returns a bare array:

```json
[
  {
    "id": 1,
    "type_id": 47408,
    "name": "50MN Abyssal Microwarpdrive",
    "data_count": 2,
    "data_statistics": {
      "Tech I": 0, "Tech II": 0, "Storyline": 0,
      "Faction": 0, "Deadspace": 0, "Officer": 0
    },
    "r2": null,
    "mae": null,
    "nmae": null,
    "last_trained_at": "2026-08-31 09:38:07.078659+00",
    "created_at": "2026-08-29 12:35:29.307246+00",
    "updated_at": "2026-08-31 09:38:07.078659+00"
  }
]
```

## Attribute roll ranges

```
GET /api/abyssal-type-statistics
```

The best and worst possible rolled value of every mutated attribute of
every abyssal type, with the attribute definition and the type.

This is the data behind the `fraction_type` roll-quality metric and the
attribute bars on module pages. Use it to normalize a module's rolls
against the theoretical range of its type, rather than hard-coding ranges
that shift when CCP changes a mutaplasmid.

| Field | Meaning |
|---|---|
| `type_id`, `type` | The abyssal type. |
| `attribute_id`, `attribute` | The dogma attribute, with its display name and unit. |
| `best`, `worst` | The extremes of the possible roll. |
| `high_is_good` | Whether a larger value is better. When false, `best` is the smaller number. |
| `is_derived` | The attribute is computed by MutaMarket rather than rolled by EVE. |
| `is_virtual` | The attribute is not on the module itself but on something it grants. |

```json
[
  {
    "id": 1,
    "type_id": 47408,
    "type": {
      "id": 47408,
      "name": "50MN Abyssal Microwarpdrive",
      "meta_group": "Abyssal",
      "meta_group_id": 15,
      "published": true
    },
    "attribute_id": 20,
    "attribute": {
      "id": 20,
      "name": "speedFactor",
      "display_name": "Maximum Velocity Bonus",
      "high_is_good": true,
      "is_derived": false,
      "unit": { "id": 124, "name": "Modifier Relative Percent", "display_name": "%" }
    },
    "best": 597.99999,
    "worst": 425.00001,
    "high_is_good": true,
    "is_derived": false,
    "is_virtual": false
  }
]
```

Note `high_is_good` and `is_derived` appear both at the top level and
inside `attribute`. They carry the same value; the duplication is inherited
from the original API and kept so existing clients keep working.
