//! In-memory telemetry of outgoing ESI requests: per-minute buckets per
//! endpoint group with status-class counts and total latency, kept for
//! the last hour and served by the admin API. Shared across clones of
//! [`super::EsiClient`], so every caller (handlers, scheduler jobs)
//! lands in one stream.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Minutes of history kept (and returned by the snapshot).
pub const WINDOW_MINUTES: usize = 60;

/// Counts of one endpoint group within one minute.
#[derive(Clone, Default, PartialEq, Debug, serde::Serialize)]
pub struct BucketCounts {
    pub requests: u64,
    /// 2xx/3xx responses.
    pub success: u64,
    /// 4xx responses.
    pub client_errors: u64,
    /// 5xx responses.
    pub server_errors: u64,
    /// No response at all (connect/timeout/transport failures).
    pub transport_errors: u64,
    /// Summed request duration, for average latency.
    pub total_ms: u64,
}

struct Bucket {
    /// Unix minute (unix seconds / 60).
    minute: i64,
    per_endpoint: BTreeMap<&'static str, BucketCounts>,
}

/// One minute of the snapshot, serialized for the admin API.
#[derive(serde::Serialize)]
pub struct BucketSnapshot {
    /// Unix seconds of the minute's start.
    pub minute_start: i64,
    pub endpoints: BTreeMap<&'static str, BucketCounts>,
}

#[derive(Default)]
pub struct EsiTelemetry {
    buckets: Mutex<VecDeque<Bucket>>,
    /// Since process start, for the recorded metric series (counters:
    /// the dashboard charts per-sample deltas).
    requests_total: std::sync::atomic::AtomicU64,
    errors_total: std::sync::atomic::AtomicU64,
}

impl EsiTelemetry {
    pub fn record(&self, endpoint: &'static str, status: Option<u16>, elapsed: Duration) {
        let minute = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|now| (now.as_secs() / 60) as i64)
            .unwrap_or(0);
        self.record_at(minute, endpoint, status, elapsed);
    }

    fn record_at(
        &self,
        minute: i64,
        endpoint: &'static str,
        status: Option<u16>,
        elapsed: Duration,
    ) {
        let mut buckets = self.buckets.lock().expect("telemetry lock");

        if buckets.back().is_none_or(|bucket| bucket.minute != minute) {
            buckets.push_back(Bucket {
                minute,
                per_endpoint: BTreeMap::new(),
            });
            while buckets.len() > WINDOW_MINUTES {
                buckets.pop_front();
            }
        }

        let counts = buckets
            .back_mut()
            .expect("bucket just ensured")
            .per_endpoint
            .entry(endpoint)
            .or_default();

        counts.requests += 1;
        counts.total_ms += elapsed.as_millis() as u64;
        match status {
            Some(status) if (200..400).contains(&status) => counts.success += 1,
            Some(status) if (400..500).contains(&status) => counts.client_errors += 1,
            Some(_) => counts.server_errors += 1,
            None => counts.transport_errors += 1,
        }

        use std::sync::atomic::Ordering;
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if !matches!(status, Some(status) if (200..400).contains(&status)) {
            self.errors_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// (requests, errors) since process start.
    pub fn totals(&self) -> (u64, u64) {
        use std::sync::atomic::Ordering;
        (
            self.requests_total.load(Ordering::Relaxed),
            self.errors_total.load(Ordering::Relaxed),
        )
    }

    /// The kept window, oldest first.
    pub fn snapshot(&self) -> Vec<BucketSnapshot> {
        self.buckets
            .lock()
            .expect("telemetry lock")
            .iter()
            .map(|bucket| BucketSnapshot {
                minute_start: bucket.minute * 60,
                endpoints: bucket.per_endpoint.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_group_by_minute_endpoint_and_status_class() {
        let telemetry = EsiTelemetry::default();
        telemetry.record_at(
            100,
            "contracts/public",
            Some(200),
            Duration::from_millis(120),
        );
        telemetry.record_at(
            100,
            "contracts/public",
            Some(304),
            Duration::from_millis(30),
        );
        telemetry.record_at(
            100,
            "contracts/public",
            Some(404),
            Duration::from_millis(20),
        );
        telemetry.record_at(100, "universe/names", Some(500), Duration::from_millis(50));
        telemetry.record_at(101, "universe/names", None, Duration::from_millis(1000));

        let snapshot = telemetry.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].minute_start, 6000);
        assert_eq!(
            snapshot[0].endpoints["contracts/public"],
            BucketCounts {
                requests: 3,
                success: 2,
                client_errors: 1,
                server_errors: 0,
                transport_errors: 0,
                total_ms: 170,
            },
        );
        assert_eq!(snapshot[0].endpoints["universe/names"].server_errors, 1);
        assert_eq!(snapshot[1].endpoints["universe/names"].transport_errors, 1);
    }

    #[test]
    fn the_window_holds_only_the_newest_minutes() {
        let telemetry = EsiTelemetry::default();
        for minute in 0..(WINDOW_MINUTES as i64 + 10) {
            telemetry.record_at(minute, "universe/names", Some(200), Duration::ZERO);
        }

        let snapshot = telemetry.snapshot();
        assert_eq!(snapshot.len(), WINDOW_MINUTES);
        assert_eq!(snapshot[0].minute_start, 10 * 60);
    }
}
