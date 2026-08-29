//! The estimator training pipeline, ported from the legacy
//! `app:estimator:train-type` / `app:estimator:train` commands and
//! `estimators/train.py` — natively, with `crate::estimator::forest`
//! instead of the Python subprocess and joblib artifacts.
//!
//! Per type: the training set is every sold abyssal module (rows from
//! `training_modules`, priced by the historic contract's unified price)
//! plus, as anchors, the published low-meta input types of the type's
//! mutaplasmids that have market history (priced by the market history
//! average). Features are the type's non-derived `estimator_attributes`
//! in attribute-name order; module values are mutated ?? source-type ?? 0
//! like the legacy `EstimatorTrainResource`.
//!
//! Divergence from legacy: for the anchor rows the legacy trainer only
//! filled attributes mutated by the type's mutaplasmids and left the rest
//! NaN for sklearn's missing-value splits; our trees have no NaN support,
//! so anchors carry the input type's real attribute value (?? 0) for every
//! feature — exactly what inference computes for an unmutated module.

use sqlx::PgPool;

use super::forest::{self, CvMetrics, Dataset, Forest};

/// Minimum sold-module count to train a type, the legacy
/// `TrainEstimatorCommand::$minimum_data_count`.
pub const MINIMUM_DATA_COUNT: i64 = 50;

/// Meta levels counting as ordinary (non-abyssal) anchor candidates, the
/// legacy `$low_meta_levels`.
const LOW_META_LEVELS: [f64; 5] = [0.0, 1.0, 2.0, 3.0, 4.0];

/// What training one type did.
#[derive(Debug)]
pub enum TrainOutcome {
    Trained {
        /// Sold abyssal modules (the statistics row's `data_count`; the
        /// anchor rows are not counted, like legacy).
        data_count: i64,
        /// Total training rows including the anchors.
        rows: usize,
        metrics: CvMetrics,
    },
    /// Fewer than [`MINIMUM_DATA_COUNT`] sold modules: metrics are nulled
    /// but `last_trained_at`/`data_count` still advance, like legacy.
    NotEnoughData { data_count: i64 },
    /// The type has no non-derived estimator attributes; nothing is
    /// touched, like the legacy trainer failing on a featureless frame.
    NoFeatures,
}

/// A feature column: the estimator attribute and its display position.
struct FeatureColumn {
    attribute_id: i64,
    name: String,
}

/// The type's non-derived estimator attributes in attribute-name order,
/// the training column order (legacy `sortBy('attribute.name')`).
async fn feature_columns(pool: &PgPool, type_id: i64) -> sqlx::Result<Vec<FeatureColumn>> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "select a.id, a.name
         from estimator_attributes ea
         join attributes a on a.id = ea.attribute_id
         where ea.type_id = $1 and not a.derived
         order by a.name",
    )
    .bind(type_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(attribute_id, name)| FeatureColumn { attribute_id, name })
        .collect())
}

/// Loads the training rows: sold abyssal modules in module-id order, then
/// anchor types in type-id order (the legacy jsonl row order, which the
/// consecutive CV folds depend on).
async fn training_dataset(
    pool: &PgPool,
    type_id: i64,
    columns: &[FeatureColumn],
) -> sqlx::Result<Dataset> {
    let attribute_ids: Vec<i64> = columns.iter().map(|column| column.attribute_id).collect();

    // One row per (module, feature) in module-id + column order; the
    // price repeats per feature row.
    let module_rows: Vec<(i64, f64, f64)> = sqlx::query_as(
        "select m.id, hc.unified_price, coalesce(ma.value, ta.value, 0) as value
         from modules m
         join training_modules tm on tm.module_id = m.id
         join historic_contracts hc on hc.id = tm.historic_contract_id
         cross join unnest($2::bigint[]) with ordinality as feature (attribute_id, position)
         left join mutated_attributes ma
             on ma.module_id = m.id and ma.attribute_id = feature.attribute_id
         left join type_attributes ta
             on ta.type_id = m.source_type_id and ta.attribute_id = feature.attribute_id
         where m.type_id = $1
         order by m.id, feature.position",
    )
    .bind(type_id)
    .bind(&attribute_ids)
    .fetch_all(pool)
    .await?;

    // Anchor rows: published low-meta mutaplasmid input types with market
    // history, anchored to the newest history row per type. The legacy
    // hasOne marketHistory was always fresh because ProcessMarketHistory
    // kept exactly one updateOrCreate row per (type, region); our daily
    // sweep accumulates one row per day instead, so every consumer picks
    // the newest row (the sweep's divergence note). Feature values come
    // from the type's own attributes (?? 0) — see the module docs for
    // the NaN divergence.
    let anchor_rows: Vec<(i64, f64, f64)> = sqlx::query_as(
        "select t.id, mh.average, coalesce(ta.value, 0) as value
         from types t
         join lateral (
             select average from market_histories
             where type_id = t.id order by date desc limit 1
         ) mh on true
         cross join unnest($2::bigint[]) with ordinality as feature (attribute_id, position)
         left join type_attributes ta
             on ta.type_id = t.id and ta.attribute_id = feature.attribute_id
         where t.published
           and exists (
               select 1 from type_attributes ml
               where ml.type_id = t.id and ml.attribute_id = $3 and ml.value = any($4)
           )
           and exists (
               select 1 from mutaplasmid_input_types mit
               join mutaplasmids mp on mp.id = mit.mutaplasmid_id
               where mp.output_type_id = $1 and mit.type_id = t.id
           )
         order by t.id, feature.position",
    )
    .bind(type_id)
    .bind(&attribute_ids)
    .bind(crate::modules::META_LEVEL_ATTRIBUTE_ID)
    .bind(&LOW_META_LEVELS[..])
    .fetch_all(pool)
    .await?;

    let n_features = columns.len();
    let mut dataset = Dataset {
        n_features,
        features: Vec::new(),
        targets: Vec::new(),
    };
    for rows in [&module_rows, &anchor_rows] {
        assert!(rows.len() % n_features == 0, "ragged training rows");
        for chunk in rows.chunks(n_features) {
            dataset.targets.push(chunk[0].1 as f32);
            dataset
                .features
                .extend(chunk.iter().map(|(_, _, value)| *value as f32));
        }
    }

    Ok(dataset)
}

/// Rebuilds the statistics row's `data_statistics` like the legacy
/// `createDataStatistics`: the existing keys (meta group names) are kept
/// and remapped to the count of sold modules whose source type belongs to
/// that meta group (0 for unknown names).
async fn data_statistics(pool: &PgPool, type_id: i64) -> sqlx::Result<Option<serde_json::Value>> {
    let existing: Option<Option<serde_json::Value>> =
        sqlx::query_scalar("select data_statistics from estimator_statistics where type_id = $1")
            .bind(type_id)
            .fetch_optional(pool)
            .await?;
    let Some(Some(serde_json::Value::Object(existing))) = existing else {
        return Ok(None);
    };

    let counts: Vec<(String, i64)> = sqlx::query_as(
        "select mg.name, count(*)
         from modules m
         join training_modules tm on tm.module_id = m.id
         join types source_type on source_type.id = m.source_type_id
         join meta_groups mg on mg.id = source_type.meta_group_id
         where m.type_id = $1
         group by mg.name",
    )
    .bind(type_id)
    .fetch_all(pool)
    .await?;

    let rebuilt: serde_json::Map<String, serde_json::Value> = existing
        .keys()
        .map(|name| {
            let count = counts
                .iter()
                .find(|(group, _)| group == name)
                .map(|(_, count)| *count)
                .unwrap_or(0);
            (name.clone(), serde_json::Value::from(count))
        })
        .collect();

    Ok(Some(serde_json::Value::Object(rebuilt)))
}

/// Trains one type end to end: gate, dataset, cross-validation, final
/// fit, model upsert, statistics update. The legacy
/// `app:estimator:train-type` plus `train.py`.
pub async fn train_type(pool: &PgPool, type_id: i64) -> sqlx::Result<TrainOutcome> {
    let data_count: i64 = sqlx::query_scalar(
        "select count(*) from modules m
         join training_modules tm on tm.module_id = m.id
         where m.type_id = $1",
    )
    .bind(type_id)
    .fetch_one(pool)
    .await?;

    if data_count < MINIMUM_DATA_COUNT {
        sqlx::query(
            "update estimator_statistics
             set last_trained_at = now(), data_count = $2,
                 r2 = null, mae = null, nmae = null, updated_at = now()
             where type_id = $1",
        )
        .bind(type_id)
        .bind(data_count)
        .execute(pool)
        .await?;

        return Ok(TrainOutcome::NotEnoughData { data_count });
    }

    let columns = feature_columns(pool, type_id).await?;
    if columns.is_empty() {
        return Ok(TrainOutcome::NoFeatures);
    }
    let dataset = training_dataset(pool, type_id, &columns).await?;
    let rows = dataset.n_rows();
    let feature_names: Vec<String> = columns.into_iter().map(|column| column.name).collect();

    // CPU-bound: 6 forest fits (5 CV folds + the final model).
    let (metrics, forest) = tokio::task::spawn_blocking(move || {
        let metrics = forest::cross_validate(&dataset, forest::RANDOM_STATE);
        let forest = Forest::fit(&dataset, feature_names, forest::RANDOM_STATE);
        (metrics, forest)
    })
    .await
    .expect("training task");

    sqlx::query(
        "insert into estimator_models (type_id, feature_names, model, trained_at)
         values ($1, $2, $3, now())
         on conflict (type_id) do update
         set feature_names = excluded.feature_names, model = excluded.model,
             trained_at = excluded.trained_at",
    )
    .bind(type_id)
    .bind(serde_json::to_value(&forest.feature_names).expect("feature names serialize"))
    .bind(forest.to_bytes())
    .execute(pool)
    .await?;

    let statistics = data_statistics(pool, type_id).await?;
    sqlx::query(
        "update estimator_statistics
         set r2 = $2, mae = $3, nmae = $4, last_trained_at = now(), data_count = $5,
             data_statistics = coalesce($6, data_statistics), updated_at = now()
         where type_id = $1",
    )
    .bind(type_id)
    .bind(metrics.r2)
    .bind(metrics.mae)
    .bind(metrics.nmae)
    .bind(data_count)
    .bind(statistics)
    .execute(pool)
    .await?;

    Ok(TrainOutcome::Trained {
        data_count,
        rows,
        metrics,
    })
}

/// Summary of a full training sweep.
#[derive(Debug, Default)]
pub struct TrainRun {
    pub trained: usize,
    pub skipped: usize,
    /// Modules whose estimate was cleared because their type stayed
    /// untrained.
    pub cleared: u64,
}

/// Trains every mutaplasmid output type, then clears the stored estimates
/// of modules whose type has no trained model — the legacy
/// `app:estimator:train` sweep. `progress` receives a line per type.
pub async fn train_all(pool: &PgPool, mut progress: impl FnMut(String)) -> sqlx::Result<TrainRun> {
    let types: Vec<(i64, String)> = sqlx::query_as(
        "select id, name from types
         where id in (select distinct output_type_id from mutaplasmids)
         order by id",
    )
    .fetch_all(pool)
    .await?;

    let mut run = TrainRun::default();
    let type_count = types.len();
    for (index, (type_id, name)) in types.into_iter().enumerate() {
        progress(format!("type {}/{type_count}: {name}", index + 1));
        match train_type(pool, type_id).await? {
            TrainOutcome::Trained { metrics, .. } => {
                run.trained += 1;
                tracing::info!(
                    "estimator: trained {name}: r2 {:.2}, mae {:.2}, nmae {:.2}",
                    metrics.r2,
                    metrics.mae,
                    metrics.nmae,
                );
            }
            TrainOutcome::NotEnoughData { data_count } => {
                run.skipped += 1;
                tracing::info!(
                    "estimator: not enough data for {name} ({data_count} < {MINIMUM_DATA_COUNT})",
                );
            }
            TrainOutcome::NoFeatures => {
                run.skipped += 1;
                tracing::warn!("estimator: {name} has no estimator attributes");
            }
        }
    }

    // Every module of an untrained type, whether or not it currently has
    // an estimate — the legacy sweep updates them all unconditionally
    // (Eloquent's mass update also touches updated_at).
    run.cleared = sqlx::query(
        "update modules
         set estimated_value = null, estimated_value_updated_at = now(), updated_at = now()
         where type_id in (select type_id from estimator_statistics where r2 is null)",
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(run)
}
