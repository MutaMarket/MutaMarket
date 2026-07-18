//! The module value estimator, ported from the legacy estimator stack
//! (`App\Actions\Estimators\EstimateModuleValue`, the `app:estimate` /
//! `app:estimate-values` commands and the `App\Http\Integrations\AI`
//! client).
//!
//! Estimates are produced by an external AI service: the FastAPI server
//! from the legacy `estimators/` project, which serves one scikit-learn
//! RandomForest model per abyssal type over
//! `POST /estimate/{model_name}` with a flat `{feature: value}` JSON body
//! and responds `{"estimated_value": <float>}`. A type is estimable only
//! while its `estimator_statistics` row has a non-null `r2` (written by the
//! training pipeline).
//!
//! Divergence from legacy: the training side (`app:estimator:train`,
//! `app:search-training-modules` and the weekly Monday schedule) is not
//! ported yet — it exports training data from historic contracts, which
//! arrive with their own milestone, and shells out to the Python
//! `estimators/train.py`. Until then `estimator_statistics` rows keep the
//! seeded null `r2` (see [`seed`]) and no AI calls happen.

pub mod seed;

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use sqlx::PgPool;

/// Default host of the AI estimation server, like the legacy
/// `config/ai.php` `AI_HOST` default.
pub const DEFAULT_AI_HOST: &str = "0.0.0.0";

/// Default port of the AI estimation server, like the legacy
/// `config/ai.php` `AI_PORT` default.
pub const DEFAULT_AI_PORT: u16 = 6969;

/// Modules refreshed per estimate pass, like the legacy `config/ai.php`
/// `AI_COUNT` default used by `app:estimate-values`.
pub const DEFAULT_ESTIMATE_COUNT: i64 = 4000;

/// Total request attempts against the AI server on 5xx responses, like the
/// legacy connector's `retry(5, 1000)`.
const RETRY_ATTEMPTS: u32 = 5;

/// Pause between AI server retries, like the legacy connector's 1000ms.
const RETRY_DELAY: Duration = Duration::from_millis(1000);

/// `AI_COUNT` from the environment, like the legacy `config('ai.COUNT')`.
pub fn estimate_count_from_env() -> i64 {
    std::env::var("AI_COUNT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ESTIMATE_COUNT)
}

#[derive(Debug)]
pub enum EstimatorError {
    /// The AI server could not be reached or the response body was invalid.
    Http(reqwest::Error),
    /// The AI server answered with a failure status (e.g. 404 for a type
    /// without a trained model).
    Status(reqwest::StatusCode),
}

impl fmt::Display for EstimatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EstimatorError::Http(error) => write!(f, "AI request failed: {error}"),
            EstimatorError::Status(status) => write!(f, "AI server answered {status}"),
        }
    }
}

impl std::error::Error for EstimatorError {}

impl From<reqwest::Error> for EstimatorError {
    fn from(error: reqwest::Error) -> Self {
        EstimatorError::Http(error)
    }
}

#[derive(Deserialize)]
struct EstimateResponse {
    estimated_value: f64,
}

/// HTTP client for the AI estimation server, the legacy
/// `App\Http\Integrations\AI` connector.
#[derive(Clone)]
pub struct EstimatorClient {
    base_url: String,
    http: reqwest::Client,
}

impl EstimatorClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::builder()
                .user_agent("MutaMarket (https://mutamarket.com)")
                .build()
                .expect("reqwest client"),
        }
    }

    /// Base URL from `AI_HOST`/`AI_PORT`, like the legacy `config/ai.php`.
    pub fn from_env() -> Self {
        let host = std::env::var("AI_HOST").unwrap_or_else(|_| DEFAULT_AI_HOST.to_owned());
        let port = std::env::var("AI_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(DEFAULT_AI_PORT);

        Self::new(&format!("http://{host}:{port}"))
    }

    /// `POST /estimate/{model_name}` with the feature map as JSON body.
    /// 5xx responses are retried like the legacy connector (5 attempts,
    /// one second apart); connection failures and 4xx responses fail
    /// immediately (the legacy retry callback only matches server errors).
    pub async fn estimate(
        &self,
        model_name: &str,
        features: &BTreeMap<String, f64>,
    ) -> Result<f64, EstimatorError> {
        let url = format!("{}/estimate/{model_name}", self.base_url);

        let mut attempt = 1;
        loop {
            let response = self.http.post(&url).json(features).send().await?;
            let status = response.status();

            if status.is_success() {
                let body: EstimateResponse = response.json().await?;
                return Ok(body.estimated_value);
            }

            if status.is_server_error() && attempt < RETRY_ATTEMPTS {
                attempt += 1;
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }

            return Err(EstimatorError::Status(status));
        }
    }
}

/// The AI model name of a type, like the legacy
/// `EstimateModuleValue::resolveModelName`: lowercase, spaces to
/// underscores (`50MN Abyssal Microwarpdrive` -> `50mn_abyssal_microwarpdrive`).
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
        .map(|(name, derived, mutated_value, source_value)| FeatureSource {
            name,
            derived,
            mutated_value,
            source_value,
        })
        .collect())
}

/// Estimates a module's value through the trained per-type model and stores
/// it, the legacy `EstimateModuleValue::handle`:
///
/// - no `estimator_statistics` row for the type, or a null `r2`: the stored
///   estimate is cleared but the timestamp still advances; returns false.
/// - the AI server unreachable or answering a failure status: nothing is
///   stored; returns false.
/// - otherwise the estimate and timestamp are stored; returns true.
///
/// A missing module returns false without touching anything (legacy throws
/// a ModelNotFoundException there; every caller resolves the module first).
pub async fn estimate_module_value(
    pool: &PgPool,
    client: &EstimatorClient,
    module_id: i64,
) -> sqlx::Result<bool> {
    let module: Option<(String, Option<f64>)> = sqlx::query_as(
        "select t.name, es.r2
         from modules m
         join types t on t.id = m.type_id
         left join estimator_statistics es on es.type_id = m.type_id
         where m.id = $1",
    )
    .bind(module_id)
    .fetch_optional(pool)
    .await?;

    let Some((type_name, r2)) = module else {
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

    let features = feature_map(&load_feature_sources(pool, module_id).await?);

    let Ok(estimated_value) = client.estimate(&model_name(&type_name), &features).await else {
        return Ok(false);
    };

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
    client: &EstimatorClient,
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

    let ids: Vec<i64> = sqlx::query_scalar(
        "select m.id
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

    let mut updated = 0;
    for id in &ids {
        if estimate_module_value(pool, client, *id).await? {
            updated += 1;
        }
    }

    Ok(EstimateRun {
        attempted: ids.len(),
        updated,
    })
}

#[cfg(test)]
mod tests {
    use super::{FeatureSource, feature_map, model_name};

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
