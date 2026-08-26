//! Time-series metrics for the admin dashboard, shaped like Laravel
//! Pulse (which the legacy admin nav links to): anything implementing
//! [`Recordable`] gets sampled into the narrow `metric_samples` table by
//! the metric-samples scheduler job and can be charted over time —
//! database counts, container vitals and the ESI request stream alike.
//! New series need only a new `Recordable` in [`REGISTRY`].
//!
//! Two shapes of series: gauges (a level, charted as-is) and counters
//! (monotonic totals like cpu seconds or network bytes; the dashboard
//! charts per-sample deltas, which also absorbs restarts resetting them
//! to zero).

use futures_util::future::BoxFuture;
use sqlx::PgPool;

use crate::esi::EsiClient;

/// Samples kept per metric, pruned by the recording job (2 days at the
/// five-minute cadence).
const SAMPLE_KEEP: &str = "2 days";

/// The window served to the dashboard charts.
const HISTORY_WINDOW: &str = "1 day";

/// What a sampler may read from.
pub struct SampleContext<'a> {
    pub pool: &'a PgPool,
    pub esi: &'a EsiClient,
}

/// A failed reading carries why; the recorder skips it (system readings
/// are unavailable outside Linux, everything else still records).
pub type SampleResult = Result<f64, String>;

/// What a property must provide to be recorded over time.
pub trait Recordable: Send + Sync {
    /// Stable snake-case identifier, the `metric` column.
    fn metric(&self) -> &'static str;

    /// Reads the current value.
    fn sample<'a>(&'a self, context: &'a SampleContext<'a>) -> BoxFuture<'a, SampleResult>;
}

/// A metric that is one scalar SQL query — what every database count
/// boils down to.
pub struct ScalarQuery {
    pub metric: &'static str,
    pub sql: &'static str,
}

impl Recordable for ScalarQuery {
    fn metric(&self) -> &'static str {
        self.metric
    }

    fn sample<'a>(&'a self, context: &'a SampleContext<'a>) -> BoxFuture<'a, SampleResult> {
        Box::pin(async move {
            let value: i64 = sqlx::query_scalar(self.sql)
                .fetch_one(context.pool)
                .await
                .map_err(|error| error.to_string())?;
            Ok(value as f64)
        })
    }
}

/// A metric read synchronously from the host (`/proc`, cgroups);
/// unavailable outside Linux.
pub struct SystemReading {
    pub metric: &'static str,
    pub read: fn() -> Option<f64>,
}

impl Recordable for SystemReading {
    fn metric(&self) -> &'static str {
        self.metric
    }

    fn sample<'a>(&'a self, _context: &'a SampleContext<'a>) -> BoxFuture<'a, SampleResult> {
        Box::pin(async move { (self.read)().ok_or_else(|| "unavailable on this host".to_owned()) })
    }
}

/// The ESI request stream's cumulative counters.
pub struct EsiCounter {
    pub metric: &'static str,
    pub errors: bool,
}

impl Recordable for EsiCounter {
    fn metric(&self) -> &'static str {
        self.metric
    }

    fn sample<'a>(&'a self, context: &'a SampleContext<'a>) -> BoxFuture<'a, SampleResult> {
        Box::pin(async move {
            let (requests, errors) = context.esi.telemetry().totals();
            Ok(if self.errors { errors as f64 } else { requests as f64 })
        })
    }
}

// --- Host readers (shared with the /api/admin/system endpoint) ---------

/// `USER_HZ` for the cpu fields of `/proc/self/stat`; 100 on every
/// mainstream kernel build.
const CLOCK_TICKS_PER_SECOND: f64 = 100.0;

pub fn read_number(path: &str) -> Option<i64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// VmRSS from `/proc/self/status`, in bytes.
pub fn process_rss_bytes() -> Option<i64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kilobytes: i64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kilobytes * 1024)
}

/// utime+stime of `/proc/self/stat`, in seconds.
pub fn process_cpu_seconds() -> Option<f64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    // The comm field can carry spaces; fields count from after its
    // closing parenthesis (state = index 0, utime = 11, stime = 12).
    let after = stat.rsplit(')').next()?;
    let fields: Vec<&str> = after.split_whitespace().collect();
    let utime: f64 = fields.get(11)?.parse().ok()?;
    let stime: f64 = fields.get(12)?.parse().ok()?;
    Some((utime + stime) / CLOCK_TICKS_PER_SECOND)
}

/// Sum of rx/tx bytes over `/proc/net/dev`, loopback excluded.
pub fn network_totals() -> Option<(i64, i64)> {
    let dev = std::fs::read_to_string("/proc/net/dev").ok()?;
    let (mut rx, mut tx) = (0_i64, 0_i64);
    for line in dev.lines().skip(2) {
        let (name, rest) = line.split_once(':')?;
        if name.trim() == "lo" {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        rx += fields.first()?.parse::<i64>().unwrap_or(0);
        tx += fields.get(8)?.parse::<i64>().unwrap_or(0);
    }
    Some((rx, tx))
}

/// Every recorded series. The dashboard reads the same identifiers from
/// the history payload; the `*_bytes`/`*_seconds`/`esi_*` counters are
/// charted as per-sample deltas there.
pub static REGISTRY: &[&dyn Recordable] = &[
    // Database counts (gauges).
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
    &ScalarQuery { metric: "offers", sql: "select count(*) from offers where deleted_at is null" },
    &ScalarQuery {
        metric: "notifications_pending",
        sql: "select count(*) from notification_outbox where delivered_at is null",
    },
    // Storage (gauge).
    &ScalarQuery {
        metric: "database_size_bytes",
        sql: "select pg_database_size(current_database())",
    },
    // Container vitals (memory a gauge, cpu/network counters).
    &SystemReading {
        metric: "memory_bytes",
        read: || {
            read_number("/sys/fs/cgroup/memory.current")
                .or_else(process_rss_bytes)
                .map(|bytes| bytes as f64)
        },
    },
    &SystemReading { metric: "cpu_seconds", read: || process_cpu_seconds() },
    &SystemReading {
        metric: "network_rx_bytes",
        read: || network_totals().map(|(rx, _)| rx as f64),
    },
    &SystemReading {
        metric: "network_tx_bytes",
        read: || network_totals().map(|(_, tx)| tx as f64),
    },
    // The ESI stream (counters).
    &EsiCounter { metric: "esi_requests", errors: false },
    &EsiCounter { metric: "esi_errors", errors: true },
];

/// Samples every registered metric into `metric_samples` and prunes the
/// window — the body of the metric-samples scheduler job. Unreadable
/// metrics are skipped (system readings outside Linux); returns
/// (written, skipped).
pub async fn record_all(context: &SampleContext<'_>) -> sqlx::Result<(usize, usize)> {
    let (mut written, mut skipped) = (0, 0);
    for recordable in REGISTRY {
        let value = match recordable.sample(context).await {
            Ok(value) => value,
            Err(reason) => {
                tracing::debug!("metrics: {} skipped: {reason}", recordable.metric());
                skipped += 1;
                continue;
            }
        };
        sqlx::query("insert into metric_samples (metric, value) values ($1, $2)")
            .bind(recordable.metric())
            .bind(value)
            .execute(context.pool)
            .await?;
        written += 1;
    }

    sqlx::query(&format!(
        "delete from metric_samples where taken_at < now() - interval '{SAMPLE_KEEP}'"
    ))
    .execute(context.pool)
    .await?;

    Ok((written, skipped))
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
