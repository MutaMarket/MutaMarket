//! The module value estimator, ported from the legacy estimator stack
//! (`App\Actions\Estimators\EstimateModuleValue`, the `app:estimate` /
//! `app:estimate-values` commands and the `App\Http\Integrations\AI`
//! client plus the FastAPI `estimators/query.py` server).
//!
//! Divergence from legacy: estimation is in-process. The legacy stack
//! posted the feature map to a Python server holding one scikit-learn
//! RandomForest per abyssal type; here the [`Estimator`] loads the native
//! forests trained by [`training`] straight from `estimator_models` (with
//! a small in-memory cache) and predicts locally. A type is estimable
//! only while its `estimator_statistics` row has a non-null `r2`, exactly
//! like legacy; the feature engineering (the `EstimatorQueryResource`) is
//! unchanged.

pub mod forest;
pub mod seed;
pub mod training;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use sqlx::PgPool;

use forest::Forest;

/// Modules refreshed per estimate pass, like the legacy `config/ai.php`
/// `AI_COUNT` default used by `app:estimate-values`.
pub const DEFAULT_ESTIMATE_COUNT: i64 = 4000;

/// `AI_COUNT` from the environment, like the legacy `config('ai.COUNT')`.
pub fn estimate_count_from_env() -> i64 {
    std::env::var("AI_COUNT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ESTIMATE_COUNT)
}

struct Loaded {
    forest: Arc<Forest>,
    /// `estimator_models.trained_at` in microseconds; a newer row
    /// replaces the resident forest on the next load.
    trained_at: i64,
}

/// What a `load_models` pass changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelLoad {
    /// Models decoded because they were missing or retrained.
    pub loaded: usize,
    /// Resident models whose row disappeared.
    pub dropped: usize,
    /// Models in memory afterwards.
    pub resident: usize,
}

/// In-process model store. Every trained forest stays resident (about
/// 1.5 GB for the full legacy set) so estimates never wait on a decode:
/// `load_models` fills it at boot and after training, and a type that
/// is asked for before that loads on demand. Cloning shares the store.
#[derive(Clone, Default)]
pub struct Estimator {
    models: Arc<tokio::sync::RwLock<HashMap<i64, Loaded>>>,
    /// Serializes `load_models` passes: the boot load and the scheduler's
    /// first check start together and would otherwise decode every
    /// forest twice.
    loading: Arc<tokio::sync::Mutex<()>>,
}

impl Estimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Brings the store in line with `estimator_models`: decodes every
    /// model that is missing or has a newer `trained_at`, drops the ones
    /// whose row is gone. Decodes run one at a time off the async
    /// runtime, so the transient cost is one model's bytes.
    pub async fn load_models(&self, pool: &PgPool) -> sqlx::Result<ModelLoad> {
        let _pass = self.loading.lock().await;
        let stored: Vec<(i64, i64)> = sqlx::query_as(
            "select type_id, (extract(epoch from trained_at) * 1000000)::bigint
             from estimator_models order by type_id",
        )
        .fetch_all(pool)
        .await?;

        let mut report = ModelLoad::default();
        let stale: Vec<i64> = {
            let models = self.models.read().await;
            stored
                .iter()
                .filter(|(type_id, trained_at)| {
                    models
                        .get(type_id)
                        .is_none_or(|loaded| loaded.trained_at != *trained_at)
                })
                .map(|(type_id, _)| *type_id)
                .collect()
        };
        for type_id in stale {
            if self.load_model(pool, type_id).await?.is_some() {
                report.loaded += 1;
            }
        }

        let mut models = self.models.write().await;
        let kept: std::collections::HashSet<i64> = stored.iter().map(|(id, _)| *id).collect();
        let before = models.len();
        models.retain(|type_id, _| kept.contains(type_id));
        report.dropped = before - models.len();
        report.resident = models.len();
        Ok(report)
    }

    /// The type's trained forest, resident or loaded on demand; `None`
    /// when no model is stored.
    async fn model(&self, pool: &PgPool, type_id: i64) -> sqlx::Result<Option<Arc<Forest>>> {
        if let Some(loaded) = self.models.read().await.get(&type_id) {
            return Ok(Some(loaded.forest.clone()));
        }
        self.load_model(pool, type_id).await
    }

    /// Reads and decodes one model from `estimator_models` into the
    /// store, replacing whatever was resident for the type.
    async fn load_model(&self, pool: &PgPool, type_id: i64) -> sqlx::Result<Option<Arc<Forest>>> {
        let row: Option<(Vec<u8>, i64)> = sqlx::query_as(
            "select model, (extract(epoch from trained_at) * 1000000)::bigint
             from estimator_models where type_id = $1",
        )
        .bind(type_id)
        .fetch_optional(pool)
        .await?;
        let Some((bytes, trained_at)) = row else {
            self.models.write().await.remove(&type_id);
            return Ok(None);
        };

        // Deserializing a large forest takes long enough to keep off the
        // async runtime.
        let forest = tokio::task::spawn_blocking(move || Forest::from_bytes(&bytes))
            .await
            .expect("model decode task");
        let forest = match forest {
            Ok(forest) => Arc::new(forest),
            Err(error) => {
                tracing::warn!("estimator: model for type {type_id} does not deserialize: {error}");
                self.models.write().await.remove(&type_id);
                return Ok(None);
            }
        };

        self.models.write().await.insert(
            type_id,
            Loaded {
                forest: forest.clone(),
                trained_at,
            },
        );
        Ok(Some(forest))
    }
}

/// The AI model name of a type, like the legacy
/// `EstimateModuleValue::resolveModelName`: lowercase, spaces to
/// underscores (`50MN Abyssal Microwarpdrive` -> `50mn_abyssal_microwarpdrive`).
/// Still the naming of the `estimator_statistics.name` column.
pub fn model_name(type_name: &str) -> String {
    type_name.to_lowercase().replace(' ', "_")
}

/// One estimator attribute of a module's type together with the module's
/// candidate values for it.
#[derive(Debug, Clone)]
pub struct FeatureSource {
    /// The dogma attribute name; it becomes the feature key.
    pub name: String,
    /// App-derived attributes are never model features.
    pub derived: bool,
    /// The module's rolled value, if the attribute was mutated.
    pub mutated_value: Option<f64>,
    /// The source type's base value, if the source type carries it.
    pub source_value: Option<f64>,
}

/// Builds the model input exactly like the legacy `EstimatorQueryResource`:
/// derived attributes are dropped, features are keyed by attribute name in
/// ascending name order (the `BTreeMap`), and each value is the mutated
/// value, falling back to the source type's value, falling back to zero.
pub fn feature_map(sources: &[FeatureSource]) -> BTreeMap<String, f64> {
    sources
        .iter()
        .filter(|source| !source.derived)
        .map(|source| {
            let value = source.mutated_value.or(source.source_value).unwrap_or(0.0);
            (source.name.clone(), value)
        })
        .collect()
}

/// Lays the feature map out in the model's column order, or `None` when
/// the key sets differ — the equivalent of the legacy query server
/// rejecting missing or unexpected features with a 422.
pub fn feature_vector(
    feature_names: &[String],
    features: &BTreeMap<String, f64>,
) -> Option<Vec<f32>> {
    if features.len() != feature_names.len() {
        return None;
    }
    feature_names
        .iter()
        .map(|name| features.get(name).map(|value| *value as f32))
        .collect()
}

/// Loads a module's feature sources: every estimator attribute of its type
/// with the mutated and source-type values, like the relation loadout of
/// the legacy `EstimateModuleValue` query.
async fn load_feature_sources(pool: &PgPool, module_id: i64) -> sqlx::Result<Vec<FeatureSource>> {
    let rows: Vec<(String, bool, Option<f64>, Option<f64>)> = sqlx::query_as(
        "select a.name, a.derived, ma.value as mutated_value, ta.value as source_value
         from modules m
         join estimator_attributes ea on ea.type_id = m.type_id
         join attributes a on a.id = ea.attribute_id
         left join mutated_attributes ma
             on ma.module_id = m.id and ma.attribute_id = ea.attribute_id
         left join type_attributes ta
             on ta.type_id = m.source_type_id and ta.attribute_id = ea.attribute_id
         where m.id = $1",
    )
    .bind(module_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(name, derived, mutated_value, source_value)| FeatureSource {
                name,
                derived,
                mutated_value,
                source_value,
            },
        )
        .collect())
}

/// Estimates a module's value through the trained per-type model and stores
/// it, the legacy `EstimateModuleValue::handle`:
///
/// - no `estimator_statistics` row for the type, or a null `r2`: the stored
///   estimate is cleared but the timestamp still advances; returns false.
/// - no stored model, or its features not matching the type's current
///   estimator attributes (the legacy AI server answering a failure
///   status): nothing is stored; returns false.
/// - otherwise the estimate and timestamp are stored; returns true.
///
/// A missing module returns false without touching anything (legacy throws
/// a ModelNotFoundException there; every caller resolves the module first).
pub async fn estimate_module_value(
    pool: &PgPool,
    estimator: &Estimator,
    module_id: i64,
) -> sqlx::Result<bool> {
    let module: Option<(i64, Option<f64>)> = sqlx::query_as(
        "select m.type_id, es.r2
         from modules m
         left join estimator_statistics es on es.type_id = m.type_id
         where m.id = $1",
    )
    .bind(module_id)
    .fetch_optional(pool)
    .await?;

    let Some((type_id, r2)) = module else {
        return Ok(false);
    };

    if r2.is_none() {
        // Eloquent's update() also touches updated_at.
        sqlx::query(
            "update modules
             set estimated_value = null, estimated_value_updated_at = now(), updated_at = now()
             where id = $1",
        )
        .bind(module_id)
        .execute(pool)
        .await?;

        return Ok(false);
    }

    let Some(forest) = estimator.model(pool, type_id).await? else {
        return Ok(false);
    };

    let features = feature_map(&load_feature_sources(pool, module_id).await?);
    let Some(vector) = feature_vector(&forest.feature_names, &features) else {
        tracing::warn!(
            "estimator: features of module {module_id} do not match the model of type {type_id}",
        );
        return Ok(false);
    };

    let estimated_value = forest.predict(&vector);

    sqlx::query(
        "update modules
         set estimated_value = $2, estimated_value_updated_at = now(), updated_at = now()
         where id = $1",
    )
    .bind(module_id)
    .bind(estimated_value)
    .execute(pool)
    .await?;

    Ok(true)
}

/// Outcome of an estimate pass.
#[derive(Debug, Clone, Copy)]
pub struct EstimateRun {
    /// Modules picked for the pass.
    pub attempted: usize,
    /// Modules that received a fresh estimate.
    pub updated: usize,
}

/// One estimate pass over the stalest modules, the legacy
/// `app:estimate-values` command: up to `count` modules whose type has a
/// trained model (non-null `r2`), oldest estimate first, optionally
/// restricted to one abyssal type matched by name fragment (the legacy
/// `--type` option; case-insensitive like Laravel's whereLike, resolved
/// against the mutaplasmid output types since the legacy `abyssal_types`
/// table is not ported). No type matching the fragment is an error, like
/// the legacy firstOrFail.
///
/// Legacy orders by `estimated_value_updated_at` on MySQL, where nulls sort
/// first; Postgres defaults to nulls last, so the never-estimated modules
/// need an explicit `nulls first` to keep their priority. The id tiebreak
/// makes the order deterministic (legacy leaves ties to the database).
pub async fn estimate_values(
    pool: &PgPool,
    estimator: &Estimator,
    count: i64,
    type_filter: Option<&str>,
) -> sqlx::Result<EstimateRun> {
    let type_id = match type_filter {
        Some(fragment) => {
            let type_id: Option<i64> = sqlx::query_scalar(
                "select id from types
                 where name ilike '%' || $1 || '%'
                   and id in (select distinct output_type_id from mutaplasmids)
                 order by id
                 limit 1",
            )
            .bind(fragment)
            .fetch_optional(pool)
            .await?;

            Some(type_id.ok_or(sqlx::Error::RowNotFound)?)
        }
        None => None,
    };

    let stalest: Vec<(i64, i64)> = sqlx::query_as(
        "select m.id, m.type_id
         from modules m
         where exists (
             select 1 from estimator_statistics es
             where es.type_id = m.type_id and es.r2 is not null
         )
         and ($2::bigint is null or m.type_id = $2)
         order by m.estimated_value_updated_at asc nulls first, m.id
         limit $1",
    )
    .bind(count)
    .bind(type_id)
    .fetch_all(pool)
    .await?;

    let ids = grouped_by_type(stalest);
    let mut updated = 0;
    for id in &ids {
        if estimate_module_value(pool, estimator, *id).await? {
            updated += 1;
        }
    }

    Ok(EstimateRun {
        attempted: ids.len(),
        updated,
    })
}

/// The sweep's `(module, type)` batch reordered so each type's modules
/// run back to back. The batch is still chosen by staleness; only the
/// processing order changes, so every model decodes once per sweep
/// instead of once per module when stale rows interleave types.
fn grouped_by_type(batch: Vec<(i64, i64)>) -> Vec<i64> {
    let mut ordered = batch;
    ordered.sort_by_key(|(_, type_id)| *type_id);
    ordered.into_iter().map(|(id, _)| id).collect()
}

#[cfg(test)]
mod tests {
    use super::{FeatureSource, feature_map, feature_vector, grouped_by_type, model_name};

    #[test]
    fn the_sweep_runs_each_type_back_to_back_in_staleness_order() {
        let batch = vec![(10, 2), (11, 1), (12, 2), (13, 3), (14, 1)];
        assert_eq!(grouped_by_type(batch), [11, 14, 10, 12, 13]);
    }

    fn source(
        name: &str,
        derived: bool,
        mutated_value: Option<f64>,
        source_value: Option<f64>,
    ) -> FeatureSource {
        FeatureSource {
            name: name.to_owned(),
            derived,
            mutated_value,
            source_value,
        }
    }

    #[test]
    fn features_mirror_the_legacy_estimator_query_resource() {
        // mutated ?? source ?? 0, derived attributes dropped, name order.
        let sources = [
            source("speedFactor", false, Some(512.5), Some(500.0)),
            source("capacitorNeed", false, None, Some(45.0)),
            source("overloadSpeedFactorBonus", false, None, None),
            source("mass", true, Some(1_000_000.0), Some(500_000.0)),
        ];

        let features = feature_map(&sources);

        assert_eq!(
            serde_json::to_string(&features).expect("serializes"),
            r#"{"capacitorNeed":45.0,"overloadSpeedFactorBonus":0.0,"speedFactor":512.5}"#,
        );
    }

    #[test]
    fn mutated_zero_values_win_over_source_values() {
        // PHP's ?? keeps a mutated 0.0 (unlike ?:), so 0.0 must not fall
        // through to the source value.
        let features = feature_map(&[source("cpu", false, Some(0.0), Some(25.0))]);

        assert_eq!(features.get("cpu"), Some(&0.0));
    }

    #[test]
    fn feature_vectors_follow_the_model_order_and_reject_mismatches() {
        let names = ["speedFactor".to_owned(), "capacitorNeed".to_owned()];
        let features = feature_map(&[
            source("capacitorNeed", false, Some(45.0), None),
            source("speedFactor", false, Some(512.5), None),
        ]);

        assert_eq!(feature_vector(&names, &features), Some(vec![512.5, 45.0]));

        // A missing feature and an unexpected feature both reject, like
        // the legacy query server's 422s.
        let missing = feature_map(&[source("speedFactor", false, Some(512.5), None)]);
        assert_eq!(feature_vector(&names, &missing), None);
        let unexpected = feature_map(&[
            source("capacitorNeed", false, Some(45.0), None),
            source("speedFactor", false, Some(512.5), None),
            source("cpu", false, Some(25.0), None),
        ]);
        assert_eq!(feature_vector(&names, &unexpected), None);
    }

    #[test]
    fn model_names_are_lowercased_and_underscored() {
        assert_eq!(
            model_name("50MN Abyssal Microwarpdrive"),
            "50mn_abyssal_microwarpdrive",
        );
        assert_eq!(
            model_name("Mutated 'Excavator' Mining Drone"),
            "mutated_'excavator'_mining_drone",
        );
    }
}
