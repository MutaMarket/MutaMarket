//! Request activity: who is using the site, how much, and how many of
//! them come back.
//!
//! Every request is counted in memory and flushed to two aggregate
//! tables once a minute by the `activity-flush` job. There is no
//! per-request row: at a few requests a second that would be hundreds of
//! thousands of rows a day, and the aggregates answer every question the
//! console asks at a fraction of the volume. A restart loses at most one
//! flush interval of counters, which is noise on a monthly dashboard.
//!
//! # What is not stored
//!
//! No IP addresses, user agents, referrers, query strings or concrete
//! URLs — only the matched route pattern, a time bucket and counts. An
//! IP would be near-useless here anyway: SSR requests originate from the
//! SvelteKit node process, so most traffic would collapse onto one
//! address.
//!
//! [`UserDay`] deliberately carries no route. Adding one would turn an
//! activity counter into a browsing history for a named person.

pub mod flush;
pub mod middleware;
pub mod reports;

use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Minutes of live history kept for the console's chart, matching the
/// ESI telemetry window so the two read on the same scale.
pub const WINDOW_MINUTES: usize = 60;

/// The route label of a request that matched no route. Scanner noise
/// stays visible without giving the route column unbounded cardinality.
pub const NOT_FOUND_ROUTE: &str = "(not found)";

/// The route whose count is the closest thing to a page view: the root
/// layout load issues exactly one per render and per client-side
/// navigation, while a single page view costs several API requests.
pub const PAGE_VIEW_ROUTE: &str = "GET /api/nav-state";

/// How long a resolved session token is trusted before it is looked up
/// again. A logout inside the window misattributes only that user's own
/// requests, and only until it expires.
const SESSION_CACHE_TTL: Duration = Duration::from_secs(60);

/// Cached tokens before the map is swept of expired entries.
const SESSION_CACHE_MAX: usize = 5_000;

/// Path prefixes that are never counted.
///
/// The admin console is the important one: it polls every five seconds,
/// so one open tab is 720 requests an hour. Counting it would make
/// whoever has the console open the permanent top user and roughly
/// double the site's apparent traffic. The legacy Pulse config ignores
/// its own dashboard for the same reason.
const IGNORED_PREFIXES: [&str; 2] = ["/api/admin/", "/img/"];

/// Paths that are never counted, matched exactly.
///
/// `/ws` is one long-lived upgrade per open tab, not a request; a
/// reconnect loop would otherwise look like traffic.
const IGNORED_PATHS: [&str; 2] = ["/ws", "/api/admin"];

/// Whether a path is excluded from the activity counts.
pub fn ignored(path: &str) -> bool {
    IGNORED_PATHS.contains(&path)
        || IGNORED_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

/// Counts for one route within one hour.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct RouteCounts {
    pub requests: u64,
    /// 4xx and 5xx responses.
    pub errors: u64,
    /// Summed handling time, for average latency.
    pub total_ms: u64,
}

/// One minute of the live window.
#[derive(Default)]
struct MinuteBucket {
    minute: i64,
    signed_in: u64,
    anonymous: u64,
    users: BTreeSet<i64>,
}

/// One minute of the snapshot, serialized for the admin API.
#[derive(serde::Serialize)]
pub struct MinuteSnapshot {
    /// Unix seconds of the minute's start.
    pub minute_start: i64,
    pub signed_in: u64,
    pub anonymous: u64,
}

/// The live window plus its totals, as the console's poll reads it.
#[derive(serde::Serialize)]
pub struct ActivitySnapshot {
    pub window_minutes: usize,
    pub buckets: Vec<MinuteSnapshot>,
    pub hour: HourTotals,
}

#[derive(serde::Serialize)]
pub struct HourTotals {
    pub requests: u64,
    pub signed_in: u64,
    pub anonymous: u64,
    /// Distinct signed-in users seen across the window.
    pub users: usize,
}

/// One hour of one route, keyed for the upsert.
pub type RouteKey = (i64, String, bool);
/// One user's day, keyed for the upsert.
pub type UserDay = (i64, i64);

/// The counts drained for a flush.
#[derive(Default)]
pub struct Pending {
    /// (unix hour, route, signed in) -> counts.
    pub routes: HashMap<RouteKey, RouteCounts>,
    /// (unix day, user id) -> requests.
    pub users: HashMap<UserDay, u64>,
}

impl Pending {
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty() && self.users.is_empty()
    }
}

#[derive(Default)]
struct State {
    /// The last `WINDOW_MINUTES`, for the live chart. Never drained.
    window: Vec<MinuteBucket>,
    /// Unflushed delta, drained by the `activity-flush` job.
    pending: Pending,
}

/// Shared across clones of the router state and the job dependencies, so
/// every request lands in one stream.
#[derive(Default)]
pub struct ActivityRecorder {
    state: Mutex<State>,
    sessions: Mutex<HashMap<String, (Instant, Option<i64>)>>,
}

impl ActivityRecorder {
    pub fn record(&self, route: &str, user_id: Option<i64>, status: u16, elapsed: Duration) {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|now| now.as_secs() as i64)
            .unwrap_or(0);
        self.record_at(seconds, route, user_id, status, elapsed);
    }

    /// The testable seam: the wall clock arrives as a parameter.
    pub fn record_at(
        &self,
        seconds: i64,
        route: &str,
        user_id: Option<i64>,
        status: u16,
        elapsed: Duration,
    ) {
        let minute = seconds / 60;
        let mut state = self.state.lock().expect("activity lock");

        if state
            .window
            .last()
            .is_none_or(|bucket| bucket.minute != minute)
        {
            state.window.push(MinuteBucket {
                minute,
                ..MinuteBucket::default()
            });
            let overflow = state.window.len().saturating_sub(WINDOW_MINUTES);
            state.window.drain(..overflow);
        }
        let bucket = state.window.last_mut().expect("bucket just ensured");
        match user_id {
            Some(id) => {
                bucket.signed_in += 1;
                bucket.users.insert(id);
            }
            None => bucket.anonymous += 1,
        }

        let counts = state
            .pending
            .routes
            .entry((seconds / 3600, route.to_owned(), user_id.is_some()))
            .or_default();
        counts.requests += 1;
        counts.total_ms += elapsed.as_millis() as u64;
        if status >= 400 {
            counts.errors += 1;
        }

        if let Some(id) = user_id {
            *state
                .pending
                .users
                .entry((seconds / 86_400, id))
                .or_default() += 1;
        }
    }

    /// The live window, oldest first, with its totals.
    pub fn snapshot(&self) -> ActivitySnapshot {
        let state = self.state.lock().expect("activity lock");
        let mut users = BTreeSet::new();
        let mut signed_in = 0;
        let mut anonymous = 0;
        for bucket in &state.window {
            users.extend(bucket.users.iter().copied());
            signed_in += bucket.signed_in;
            anonymous += bucket.anonymous;
        }

        ActivitySnapshot {
            window_minutes: WINDOW_MINUTES,
            buckets: state
                .window
                .iter()
                .map(|bucket| MinuteSnapshot {
                    minute_start: bucket.minute * 60,
                    signed_in: bucket.signed_in,
                    anonymous: bucket.anonymous,
                })
                .collect(),
            hour: HourTotals {
                requests: signed_in + anonymous,
                signed_in,
                anonymous,
                users: users.len(),
            },
        }
    }

    /// Takes the unflushed counts, leaving the live window intact.
    pub fn drain(&self) -> Pending {
        std::mem::take(&mut self.state.lock().expect("activity lock").pending)
    }

    /// The user behind a session token, cached so a signed-in visitor
    /// costs one lookup a minute rather than one per request.
    pub async fn resolve_user(&self, pool: &sqlx::PgPool, token: &str) -> Option<i64> {
        if let Some((seen, user_id)) = self.sessions.lock().expect("session lock").get(token)
            && seen.elapsed() < SESSION_CACHE_TTL
        {
            return *user_id;
        }

        let user_id = crate::auth::session::session_user_id(pool, token)
            .await
            .unwrap_or(None);

        let mut cache = self.sessions.lock().expect("session lock");
        if cache.len() >= SESSION_CACHE_MAX {
            cache.retain(|_, (seen, _)| seen.elapsed() < SESSION_CACHE_TTL);
        }
        cache.insert(token.to_owned(), (Instant::now(), user_id));
        user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: i64 = 60;
    const HOUR: i64 = 3_600;
    const DAY: i64 = 86_400;

    #[test]
    fn the_console_polls_and_static_assets_are_not_counted() {
        // The admin console polls every five seconds; counting it would
        // make it the loudest "user" on the site.
        assert!(ignored("/api/admin/live"));
        assert!(ignored("/api/admin/esi-failures/12"));
        assert!(ignored("/api/admin"));
        assert!(ignored("/img/icons/633.png"));
        assert!(ignored("/ws"));

        // Real traffic, including the crawlers and unfurlers.
        assert!(!ignored("/api/nav-state"));
        assert!(!ignored("/api/module-page/x-1"));
        assert!(!ignored("/og/module/1"));
        assert!(!ignored("/sitemap.xml"));
        // Neither prefix nor exact match: a route that merely starts the
        // same way is still counted.
        assert!(!ignored("/wsomething"));
        assert!(!ignored("/api/administrators"));
    }

    #[test]
    fn requests_split_by_whether_they_carried_a_session() {
        let activity = ActivityRecorder::default();
        activity.record_at(
            100 * MINUTE,
            "GET /api/nav-state",
            Some(7),
            200,
            Duration::ZERO,
        );
        activity.record_at(
            100 * MINUTE,
            "GET /api/nav-state",
            Some(7),
            200,
            Duration::ZERO,
        );
        activity.record_at(
            100 * MINUTE,
            "GET /api/nav-state",
            None,
            200,
            Duration::ZERO,
        );

        let snapshot = activity.snapshot();
        assert_eq!(snapshot.buckets.len(), 1);
        assert_eq!(snapshot.buckets[0].minute_start, 100 * MINUTE);
        assert_eq!(snapshot.buckets[0].signed_in, 2);
        assert_eq!(snapshot.buckets[0].anonymous, 1);
        assert_eq!(snapshot.hour.requests, 3);
        assert_eq!(snapshot.hour.users, 1, "the same user counts once");
    }

    #[test]
    fn the_window_keeps_only_the_newest_minutes() {
        let activity = ActivityRecorder::default();
        for minute in 0..(WINDOW_MINUTES as i64 + 10) {
            activity.record_at(minute * MINUTE, "GET /x", None, 200, Duration::ZERO);
        }

        let snapshot = activity.snapshot();
        assert_eq!(snapshot.buckets.len(), WINDOW_MINUTES);
        assert_eq!(
            snapshot.buckets[0].minute_start,
            10 * MINUTE,
            "oldest first, the first ten minutes dropped",
        );
    }

    #[test]
    fn pending_counts_bucket_by_hour_route_and_session() {
        let activity = ActivityRecorder::default();
        activity.record_at(HOUR, "GET /a", Some(7), 200, Duration::from_millis(10));
        activity.record_at(HOUR + 59, "GET /a", Some(7), 500, Duration::from_millis(30));
        activity.record_at(HOUR, "GET /a", None, 200, Duration::ZERO);
        activity.record_at(HOUR, "GET /b", Some(7), 404, Duration::ZERO);
        // The next hour is its own bucket.
        activity.record_at(2 * HOUR, "GET /a", Some(7), 200, Duration::ZERO);

        let pending = activity.drain();
        assert_eq!(
            pending.routes.get(&(1, "GET /a".to_owned(), true)),
            Some(&RouteCounts {
                requests: 2,
                errors: 1,
                total_ms: 40,
            }),
        );
        assert_eq!(
            pending.routes.get(&(1, "GET /a".to_owned(), false)),
            Some(&RouteCounts {
                requests: 1,
                errors: 0,
                total_ms: 0,
            }),
        );
        assert_eq!(
            pending
                .routes
                .get(&(1, "GET /b".to_owned(), true))
                .map(|c| c.errors),
            Some(1),
            "a 404 is an error for the route roll-up",
        );
        assert!(pending.routes.contains_key(&(2, "GET /a".to_owned(), true)));
    }

    #[test]
    fn user_days_count_only_signed_in_requests() {
        let activity = ActivityRecorder::default();
        activity.record_at(0, "GET /a", Some(7), 200, Duration::ZERO);
        activity.record_at(DAY - 1, "GET /a", Some(7), 200, Duration::ZERO);
        activity.record_at(DAY, "GET /a", Some(7), 200, Duration::ZERO);
        activity.record_at(DAY, "GET /a", Some(8), 200, Duration::ZERO);
        activity.record_at(DAY, "GET /a", None, 200, Duration::ZERO);

        let pending = activity.drain();
        assert_eq!(pending.users.get(&(0, 7)), Some(&2));
        assert_eq!(
            pending.users.get(&(1, 7)),
            Some(&1),
            "a day boundary splits"
        );
        assert_eq!(pending.users.get(&(1, 8)), Some(&1));
        assert_eq!(pending.users.len(), 3, "anonymous requests add no user day");
    }

    #[test]
    fn draining_empties_the_pending_counts_and_keeps_the_window() {
        let activity = ActivityRecorder::default();
        activity.record_at(100 * MINUTE, "GET /a", Some(7), 200, Duration::ZERO);

        assert!(!activity.drain().is_empty());
        assert!(
            activity.drain().is_empty(),
            "a second drain has nothing left to flush",
        );
        assert_eq!(
            activity.snapshot().buckets.len(),
            1,
            "the live chart is not drained with the counters",
        );
    }
}
