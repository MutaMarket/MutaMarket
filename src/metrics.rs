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

/// Samples kept per metric, pruned by the recording job: the widest
/// dashboard window (7 days) plus slack.
const SAMPLE_KEEP: &str = "8 days";

/// The windows the dashboard's timeframe toggle may request, as
/// (label, hours, bucket seconds); wider windows are averaged into
/// coarser buckets so each stays near the native point count.
pub const HISTORY_WINDOWS: [(&str, i64, i64); 3] =
    [("24h", 24, 300), ("3d", 72, 900), ("7d", 168, 2100)];

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
            Ok(if self.errors {
                errors as f64
            } else {
                requests as f64
            })
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

/// Memory the machine has handed out, in bytes: `MemTotal` less
/// `MemAvailable` from `/proc/meminfo`, which is what `free` and `btop`
/// call used.
///
/// The cgroup readings above cover this process's container alone. On
/// this box that is under a third of the box's usage, because Postgres,
/// the SvelteKit renderer and Caddy sit beside it, so the console needs
/// both numbers to mean anything: the container's for our own footprint,
/// this one for how close the machine is to full.
pub fn host_memory_used_bytes() -> Option<i64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    parse_host_memory_used(&text)
}

fn parse_host_memory_used(meminfo: &str) -> Option<i64> {
    let kilobytes = |field: &str| -> Option<i64> {
        meminfo
            .lines()
            .find(|line| line.starts_with(field))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    Some((kilobytes("MemTotal:")? - kilobytes("MemAvailable:")?) * 1024)
}

/// Busy cpu seconds of the whole machine, from the aggregate line of
/// `/proc/stat`: every field except idle and iowait, which is the
/// denominator-free form of what `btop` shows per core. A counter, like
/// [`process_cpu_seconds`], charted as deltas.
///
/// `/proc/stat` is not namespaced by the container runtime, so this is
/// the host's figure even though it is read from inside the container.
pub fn host_cpu_seconds() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/stat").ok()?;
    parse_host_cpu_seconds(&text)
}

fn parse_host_cpu_seconds(stat: &str) -> Option<f64> {
    let line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let fields: Vec<f64> = line
        .split_whitespace()
        .skip(1)
        .map(|field| field.parse().unwrap_or(0.0))
        .collect();
    // user, nice, system, [idle], [iowait], irq, softirq, steal.
    let busy: f64 = fields.first()?
        + fields.get(1)?
        + fields.get(2)?
        + fields.get(5).copied().unwrap_or(0.0)
        + fields.get(6).copied().unwrap_or(0.0)
        + fields.get(7).copied().unwrap_or(0.0);
    Some(busy / CLOCK_TICKS_PER_SECOND)
}

/// Where the host's sysfs is bind-mounted (see docker-compose.yml). A
/// container's own `/proc/net/dev` and `/sys/class/net` describe its veth
/// alone; sysfs mounted from the host keeps the host's interfaces, and
/// unlike `/proc/<pid>/net` it needs no ptrace access to read.
const HOST_SYSFS_NET: &str = "/host/sys/class/net";

/// Interfaces whose counters are the machine's traffic with the outside.
///
/// The docker bridges (`docker0`, `br-*`) and the container ends (`veth*`)
/// carry the same bytes again as they hop between containers, so counting
/// them would report several times the real volume. `lo` is local by
/// definition.
fn is_uplink_interface(name: &str) -> bool {
    !(name == "lo"
        || name.starts_with("veth")
        || name.starts_with("docker")
        || name.starts_with("br-")
        || name.starts_with("virbr"))
}

/// (rx, tx) bytes of the machine's uplinks, from the bind-mounted host
/// sysfs. `None` when it is not mounted, which is every host that did not
/// opt in: the console then falls back to [`network_totals`].
pub fn host_network_totals() -> Option<(i64, i64)> {
    let mut totals: Option<(i64, i64)> = None;
    for entry in std::fs::read_dir(HOST_SYSFS_NET).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_uplink_interface(&name) {
            continue;
        }
        let counter = |field: &str| -> Option<i64> {
            read_number(&format!("{HOST_SYSFS_NET}/{name}/statistics/{field}"))
        };
        // An interface that disappears mid-read is skipped, not fatal.
        if let (Some(rx), Some(tx)) = (counter("rx_bytes"), counter("tx_bytes")) {
            let (sum_rx, sum_tx) = totals.unwrap_or((0, 0));
            totals = Some((sum_rx + rx, sum_tx + tx));
        }
    }
    totals
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
    // Only the vitals the dashboard charts remain; the database-count
    // and ESI-counter collectors were dropped with their sparklines
    // (the dashboard shows those as live numbers instead).
    // Storage (gauge).
    &ScalarQuery {
        metric: "database_size_bytes",
        sql: "select pg_database_size(current_database())",
    },
    // The machine's own vitals, which the container readings below do
    // not cover: everything beside this process lives here too.
    &SystemReading {
        metric: "host_memory_bytes",
        read: || host_memory_used_bytes().map(|bytes| bytes as f64),
    },
    &SystemReading {
        metric: "host_cpu_seconds",
        read: host_cpu_seconds,
    },
    &SystemReading {
        metric: "host_network_rx_bytes",
        read: || host_network_totals().map(|(rx, _)| rx as f64),
    },
    &SystemReading {
        metric: "host_network_tx_bytes",
        read: || host_network_totals().map(|(_, tx)| tx as f64),
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
    &SystemReading {
        metric: "cpu_seconds",
        read: || process_cpu_seconds(),
    },
    &SystemReading {
        metric: "network_rx_bytes",
        read: || network_totals().map(|(rx, _)| rx as f64),
    },
    &SystemReading {
        metric: "network_tx_bytes",
        read: || network_totals().map(|(_, tx)| tx as f64),
    },
    &SystemReading {
        metric: "disk_used_bytes",
        read: || disk_usage().map(|(used, _)| used as f64),
    },
];

/// The machine's total memory from /proc/meminfo, the utilization
/// capacity when no cgroup limit is set.
pub fn host_memory_total_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// (used, total) bytes of the filesystem the process runs on, via
/// `df` (no statvfs binding in std; the coreutils tool is present in
/// the container and on dev hosts alike).
pub fn disk_usage() -> Option<(u64, u64)> {
    let output = std::process::Command::new("df")
        .args(["-k", "/"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().nth(1)?;
    let mut fields = line.split_whitespace();
    let total_kb: u64 = fields.nth(1)?.parse().ok()?;
    let used_kb: u64 = fields.next()?.parse().ok()?;
    Some((used_kb * 1024, total_kb * 1024))
}

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

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "delete from metric_samples where taken_at < now() - interval '{SAMPLE_KEEP}'"
    )))
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
/// Samples are averaged into `step_seconds` buckets.
pub async fn history(
    pool: &PgPool,
    hours: i64,
    step_seconds: i64,
) -> sqlx::Result<std::collections::BTreeMap<String, Vec<Sample>>> {
    let rows: Vec<(String, i64, f64)> = sqlx::query_as(
        "select metric,
                (floor(extract(epoch from taken_at) / $2) * $2)::bigint as bucket,
                avg(value)
         from metric_samples
         where taken_at >= now() - make_interval(hours => $1::int)
         group by metric, bucket
         order by bucket",
    )
    .bind(hours)
    .bind(step_seconds)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The lines as the production box answers them (2026-09-11), where
    /// the container read 1.6 GB while the machine was using 4.6 GB.
    const MEMINFO: &str = "MemTotal:        7916036 kB
MemFree:          170056 kB
MemAvailable:    3099880 kB
Buffers:           12345 kB
";

    const STAT: &str = "cpu  103835890 44935 17909713 176956203 1380895 0 3904276 0 0 0
cpu0 25000000 11000 4000000 44000000 345000 0 976000 0 0 0
intr 123456
";

    #[test]
    fn host_memory_is_what_free_calls_used() {
        let used = parse_host_memory_used(MEMINFO).expect("a reading");
        assert_eq!(used, (7_916_036 - 3_099_880) * 1024);
        // 4.6 GB, against the 1.6 GB the container alone reported.
        assert_eq!(used / 1024 / 1024 / 1024, 4);
    }

    #[test]
    fn host_memory_needs_both_fields() {
        assert_eq!(
            parse_host_memory_used("MemTotal:        7916036 kB\n"),
            None
        );
        assert_eq!(parse_host_memory_used(""), None);
    }

    #[test]
    fn uplinks_exclude_the_docker_plumbing() {
        // The box runs eth0 plus a bridge, docker0 and four veths; the
        // veths mirror traffic that eth0 already counted.
        assert!(is_uplink_interface("eth0"));
        assert!(is_uplink_interface("enp1s0"));
        assert!(is_uplink_interface("ens3"));

        assert!(!is_uplink_interface("lo"));
        assert!(!is_uplink_interface("docker0"));
        assert!(!is_uplink_interface("br-3245e7677816"));
        assert!(!is_uplink_interface("veth25559bd"));
        assert!(!is_uplink_interface("virbr0"));
    }

    #[test]
    fn host_cpu_counts_every_busy_state_but_not_idle() {
        let seconds = parse_host_cpu_seconds(STAT).expect("a reading");
        let busy = 103_835_890.0 + 44_935.0 + 17_909_713.0 + 3_904_276.0;
        assert_eq!(seconds, busy / CLOCK_TICKS_PER_SECOND);
    }

    #[test]
    fn host_cpu_ignores_a_per_core_line_and_a_missing_aggregate() {
        assert_eq!(parse_host_cpu_seconds("cpu0 1 2 3 4 5 6 7 8\n"), None);
        assert_eq!(parse_host_cpu_seconds(""), None);
    }

    #[test]
    fn host_cpu_tolerates_a_short_line_from_an_older_kernel() {
        // irq/softirq/steal absent: the busy sum is what is there.
        let seconds = parse_host_cpu_seconds("cpu  100 200 300 400 500\n").expect("a reading");
        assert_eq!(seconds, 600.0 / CLOCK_TICKS_PER_SECOND);
    }
}
