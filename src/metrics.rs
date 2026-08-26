//! Time-series metrics for the admin dashboard, shaped like Laravel
//! Pulse (which the legacy admin nav links to): anything implementing
//! [`Recordable`] gets sampled into the narrow `metric_samples` table by
//! the metric-samples scheduler job and can be charted over time. New
//! series need only a new `Recordable` in [`REGISTRY`].

use futures_util::future::BoxFuture;
use sqlx::PgPool;

/// Samples kept per metric, pruned by the recording job (2 days at the
/// five-minute cadence).
const SAMPLE_KEEP: &str = "2 days";

/// The window served to the dashboard charts.
const HISTORY_WINDOW: &str = "1 day";

/// What a property must provide to be recorded over time.
pub trait Recordable: Send + Sync {
    /// Stable snake-case identifier, the `metric` column.
    fn metric(&self) -> &'static str;

    /// Reads the current value.
    fn sample<'a>(&'a self, pool: &'a PgPool) -> BoxFuture<'a, sqlx::Result<f64>>;
}

/// A metric that is one scalar SQL query — what almost every database
/// count boils down to.
pub struct ScalarQuery {
    pub metric: &'static str,
    pub sql: &'static str,
}

impl Recordable for ScalarQuery {
    fn metric(&self) -> &'static str {
        self.metric
    }

    fn sample<'a>(&'a self, pool: &'a PgPool) -> BoxFuture<'a, sqlx::Result<f64>> {
        Box::pin(async move {
            let value: i64 = sqlx::query_scalar(self.sql).fetch_one(pool).await?;
            Ok(value as f64)
        })
    }
}

/// Every recorded series. The dashboard's database tiles read the same
/// identifiers from the history payload.
pub static REGISTRY: &[&dyn Recordable] = &[
    &ScalarQuery { metric: "modules", sql: "select count(*) from modules" },
    &ScalarQuery {
        metric: "modules_without_estimate",
        sql: "select count(*) from modules where estimated_value is null",
    },
    &ScalarQuery { metric: "contracts", sql: "select count(*) from contracts" },
    &ScalarQuery { metric: "contract_items", sql: "select count(*) from contract_items" },
    &ScalarQuery { metric: "characters", sql: "select count(*) from characters" },
    &ScalarQuery { metric: "users", sql: "select count(*) from users" },
    &ScalarQuery { metric: "assets", sql: "select count(*) from assets" },
    &ScalarQuery {
        metric: "public_ownerships",
        sql: "select count(*) from public_module_ownerships",
    },
    &ScalarQuery { metric: "market_history_days", sql: "select count(*) from market_histories" },
];

/// Samples every registered metric into `metric_samples` and prunes the
/// window — the body of the metric-samples scheduler job. Returns the
/// number of samples written.
pub async fn record_all(pool: &PgPool) -> sqlx::Result<usize> {
    let mut written = 0;
    for recordable in REGISTRY {
        let value = recordable.sample(pool).await?;
        sqlx::query("insert into metric_samples (metric, value) values ($1, $2)")
            .bind(recordable.metric())
            .bind(value)
            .execute(pool)
            .await?;
        written += 1;
    }

    sqlx::query(&format!(
        "delete from metric_samples where taken_at < now() - interval '{SAMPLE_KEEP}'"
    ))
    .execute(pool)
    .await?;

    Ok(written)
}

/// One recorded sample of a metric's served history.
#[derive(Debug, serde::Serialize)]
pub struct Sample {
    /// Unix seconds.
    pub taken_at: i64,
    pub value: f64,
}

/// The charted window per metric, oldest first, keyed by metric name.
pub async fn history(
    pool: &PgPool,
) -> sqlx::Result<std::collections::BTreeMap<String, Vec<Sample>>> {
    let rows: Vec<(String, i64, f64)> = sqlx::query_as(&format!(
        "select metric, extract(epoch from taken_at)::bigint, value
         from metric_samples
         where taken_at >= now() - interval '{HISTORY_WINDOW}'
         order by taken_at",
    ))
    .fetch_all(pool)
    .await?;

    let mut series = std::collections::BTreeMap::new();
    for (metric, taken_at, value) in rows {
        series
            .entry(metric)
            .or_insert_with(Vec::new)
            .push(Sample { taken_at, value });
    }

    Ok(series)
}
