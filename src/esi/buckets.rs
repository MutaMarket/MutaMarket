//! Per-(rate-limit-group, subject) pacing for ESI's floating-window token
//! buckets (the `X-Ratelimit-*` headers described in the ESI spec, e.g.
//! `char-contract` at 600 tokens per 15 minutes per character).
//! [`crate::esi::throttle::RateLimiter`] spaces every outgoing request at a
//! flat global rate regardless of endpoint; this instead mirrors the
//! *actual* remaining budget ESI reports per group and subject, and only
//! slows down the specific bucket that is close to empty.
//!
//! ESI does not publish a static endpoint-to-group map worth vendoring;
//! instead every successful response to a limited route names its own
//! group via `X-Ratelimit-Group`. So the door works in two passes: the
//! first request to a not-yet-seen endpoint is let through (there is
//! nothing to check yet), and its response teaches this limiter which
//! group that endpoint belongs to. Every later request to the same
//! endpoint looks the group up and, if that group's remembered budget for
//! the same subject is low and its window has not rolled over yet, waits
//! out the window before firing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Whom a request's token budget is billed to: ESI counts per
/// (application, character) for authenticated routes, and by source IP
/// for public ones (a single shared subject, since every request leaves
/// from this one app).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateSubject {
    Character(i64),
    Public,
}

/// Seconds in a minute/hour/day, for expanding `X-Ratelimit-Limit`'s
/// window suffix (e.g. `"600/15m"`) into a [`Duration`].
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * 60;
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * 24;

/// Fraction of a bucket's token limit reserved as safety margin: the door
/// starts waiting a little before the mirrored `remaining` count would
/// actually hit zero, so a request in flight when headers are read does
/// not tip the bucket into ESI's own 429.
const DEFAULT_BUCKET_MARGIN_FRACTION: f64 = 0.05;

/// Floor of the margin in tokens, so a bucket small enough that 5% rounds
/// to nothing still keeps a couple of tokens of headroom.
const DEFAULT_BUCKET_MARGIN_FLOOR: u32 = 3;

/// Park duration used for a 429 whose `Retry-After` header is missing or
/// unparsable. Should not happen per the spec, but parking briefly beats
/// hammering the same bucket in a tight loop.
const DEFAULT_RETRY_AFTER_SECS: u64 = 60;

#[derive(Debug, Clone, Copy)]
struct BucketState {
    limit: u32,
    remaining: u32,
    reset_at: Instant,
}

/// Arc-shared across every [`crate::esi::EsiClient`] clone, so all callers
/// draw from the same learned groups and mirrored budgets.
pub struct BucketLimiter {
    enabled: bool,
    /// `ESI_BUCKET_MARGIN`: an absolute token count applied to every
    /// bucket instead of the computed percentage-of-limit default.
    margin_override: Option<u32>,
    endpoint_groups: Mutex<HashMap<&'static str, String>>,
    buckets: Mutex<HashMap<(String, RateSubject), BucketState>>,
}

impl BucketLimiter {
    fn new(enabled: bool, margin_override: Option<u32>) -> Self {
        Self {
            enabled,
            margin_override,
            endpoint_groups: Mutex::new(HashMap::new()),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// A no-op limiter: mock-backed test clients must not be paced.
    pub fn disabled() -> Self {
        Self::new(false, None)
    }

    /// The production limiter, built from `ESI_BUCKET_MARGIN` (see
    /// [`BucketLimiter::margin_override`] doc on the field).
    pub fn from_env() -> Self {
        let margin_override = std::env::var("ESI_BUCKET_MARGIN")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok());
        Self::new(true, margin_override)
    }

    fn margin_for(&self, limit: u32) -> u32 {
        self.margin_override.unwrap_or_else(|| {
            let fraction = (f64::from(limit) * DEFAULT_BUCKET_MARGIN_FRACTION).ceil() as u32;
            fraction.max(DEFAULT_BUCKET_MARGIN_FLOOR)
        })
    }

    /// Waits, if needed, before a request to `endpoint` on behalf of
    /// `subject` fires. A no-op when the limiter is disabled, the
    /// endpoint's group is not learned yet, or the bucket has budget (or
    /// its window has already rolled over since the last observation, in
    /// which case ESI has refilled it).
    pub async fn wait_before(&self, endpoint: &'static str, subject: RateSubject) {
        if !self.enabled {
            return;
        }
        let group = {
            let groups = self.endpoint_groups.lock().expect("bucket groups lock");
            groups.get(endpoint).cloned()
        };
        let Some(group) = group else {
            return;
        };
        let deadline = {
            let buckets = self.buckets.lock().expect("bucket state lock");
            buckets.get(&(group, subject)).and_then(|state| {
                let now = Instant::now();
                if now >= state.reset_at {
                    return None;
                }
                (state.remaining <= self.margin_for(state.limit)).then_some(state.reset_at)
            })
        };
        if let Some(deadline) = deadline {
            let now = Instant::now();
            if deadline > now {
                tokio::time::sleep(deadline - now).await;
            }
        }
    }

    /// Mirrors a response's rate-limit headers into the bucket for
    /// `(group learned for endpoint, subject)`. Also the only place an
    /// endpoint's group is learned, from `X-Ratelimit-Group`.
    pub fn record_response(
        &self,
        endpoint: &'static str,
        subject: RateSubject,
        status: reqwest::StatusCode,
        headers: &reqwest::header::HeaderMap,
    ) {
        if !self.enabled {
            return;
        }

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            self.record_429(endpoint, subject, headers);
            return;
        }

        let Some(group) = header_str(headers, "x-ratelimit-group") else {
            return;
        };
        let Some((limit, window)) =
            header_str(headers, "x-ratelimit-limit").and_then(|value| parse_limit_header(&value))
        else {
            return;
        };
        let Some(remaining) = header_str(headers, "x-ratelimit-remaining")
            .and_then(|value| value.trim().parse::<u32>().ok())
        else {
            return;
        };

        self.endpoint_groups
            .lock()
            .expect("bucket groups lock")
            .insert(endpoint, group.clone());
        self.buckets.lock().expect("bucket state lock").insert(
            (group, subject),
            BucketState {
                limit,
                remaining,
                reset_at: Instant::now() + window,
            },
        );
    }

    /// A 429 carries only `Retry-After`, no group header, so it only parks
    /// a bucket whose group this endpoint has already taught us: an
    /// endpoint's very first response cannot be a 429 we know how to key,
    /// and letting it through teaches the group like any other response.
    fn record_429(
        &self,
        endpoint: &'static str,
        subject: RateSubject,
        headers: &reqwest::header::HeaderMap,
    ) {
        let Some(group) = self
            .endpoint_groups
            .lock()
            .expect("bucket groups lock")
            .get(endpoint)
            .cloned()
        else {
            return;
        };
        let retry_after = header_str(headers, "retry-after")
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_RETRY_AFTER_SECS);

        let mut buckets = self.buckets.lock().expect("bucket state lock");
        let key = (group, subject);
        // Keep the last known limit (only used to size the margin on the
        // next door check); a bucket parked before ever seeing a 2xx has
        // no limit to remember, so 0 (always below margin) is safe.
        let limit = buckets.get(&key).map_or(0, |state| state.limit);
        buckets.insert(
            key,
            BucketState {
                limit,
                remaining: 0,
                reset_at: Instant::now() + Duration::from_secs(retry_after),
            },
        );
    }
}

fn header_str(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers.get(name)?.to_str().ok().map(str::to_owned)
}

/// Parses `X-Ratelimit-Limit`'s `"<count>/<window>"` form (e.g.
/// `"600/15m"`) into the token count and the window as a [`Duration`].
fn parse_limit_header(value: &str) -> Option<(u32, Duration)> {
    let (count, window) = value.split_once('/')?;
    let count: u32 = count.trim().parse().ok()?;
    let window = parse_window(window.trim())?;
    Some((count, window))
}

/// Parses a window like `"15m"`: digits followed by a single unit letter
/// (`s`, `m`, `h` or `d`).
fn parse_window(value: &str) -> Option<Duration> {
    let split_at = value.find(|c: char| !c.is_ascii_digit())?;
    let (digits, unit) = value.split_at(split_at);
    let amount: u64 = digits.parse().ok()?;
    let seconds = match unit {
        "s" => amount,
        "m" => amount * SECONDS_PER_MINUTE,
        "h" => amount * SECONDS_PER_HOUR,
        "d" => amount * SECONDS_PER_DAY,
        _ => return None,
    };
    Some(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderMap;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).expect("header name"),
                value.parse().expect("header value"),
            );
        }
        headers
    }

    #[test]
    fn parses_the_limit_header_count_and_window() {
        assert_eq!(
            parse_limit_header("600/15m"),
            Some((600, Duration::from_secs(15 * SECONDS_PER_MINUTE)))
        );
        assert_eq!(
            parse_limit_header("400/60s"),
            Some((400, Duration::from_secs(60)))
        );
        assert_eq!(parse_limit_header("garbage"), None);
        assert_eq!(parse_limit_header("600/15x"), None);
    }

    #[tokio::test]
    async fn door_waits_when_remaining_is_low_and_not_before_reset() {
        let limiter = BucketLimiter::new(true, Some(2));
        limiter
            .endpoint_groups
            .lock()
            .unwrap()
            .insert("ep", "grp".to_owned());
        limiter.buckets.lock().unwrap().insert(
            ("grp".to_owned(), RateSubject::Public),
            BucketState {
                limit: 10,
                remaining: 1,
                reset_at: Instant::now() + Duration::from_millis(60),
            },
        );

        let started = Instant::now();
        limiter.wait_before("ep", RateSubject::Public).await;
        assert!(
            started.elapsed() >= Duration::from_millis(50),
            "should have waited out the window, elapsed {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn door_does_not_wait_once_the_window_has_rolled_over() {
        let limiter = BucketLimiter::new(true, Some(5));
        limiter
            .endpoint_groups
            .lock()
            .unwrap()
            .insert("ep", "grp".to_owned());
        limiter.buckets.lock().unwrap().insert(
            ("grp".to_owned(), RateSubject::Public),
            BucketState {
                limit: 10,
                remaining: 0,
                // Already in the past: ESI would have refilled by now.
                reset_at: Instant::now() - Duration::from_millis(1),
            },
        );

        let started = Instant::now();
        limiter.wait_before("ep", RateSubject::Public).await;
        assert!(started.elapsed() < Duration::from_millis(20));
    }

    #[tokio::test]
    async fn unlearned_endpoint_and_healthy_bucket_do_not_wait() {
        let limiter = BucketLimiter::new(true, Some(5));
        let started = Instant::now();
        limiter.wait_before("never-seen", RateSubject::Public).await;
        assert!(started.elapsed() < Duration::from_millis(20));

        limiter
            .endpoint_groups
            .lock()
            .unwrap()
            .insert("ep", "grp".to_owned());
        limiter.buckets.lock().unwrap().insert(
            ("grp".to_owned(), RateSubject::Public),
            BucketState {
                limit: 10,
                remaining: 9,
                reset_at: Instant::now() + Duration::from_secs(60),
            },
        );
        let started = Instant::now();
        limiter.wait_before("ep", RateSubject::Public).await;
        assert!(started.elapsed() < Duration::from_millis(20));
    }

    #[test]
    fn a_429_parks_the_bucket_until_retry_after() {
        let limiter = BucketLimiter::new(true, None);
        limiter
            .endpoint_groups
            .lock()
            .unwrap()
            .insert("ep", "grp".to_owned());

        limiter.record_response(
            "ep",
            RateSubject::Public,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            &headers(&[("retry-after", "5")]),
        );

        let buckets = limiter.buckets.lock().unwrap();
        let state = buckets
            .get(&("grp".to_owned(), RateSubject::Public))
            .expect("bucket parked");
        assert_eq!(state.remaining, 0);
        assert!(state.reset_at > Instant::now() + Duration::from_secs(4));
    }

    #[test]
    fn a_429_on_an_unlearned_endpoint_parks_nothing() {
        let limiter = BucketLimiter::new(true, None);
        limiter.record_response(
            "never-seen",
            RateSubject::Public,
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            &headers(&[("retry-after", "5")]),
        );
        assert!(limiter.buckets.lock().unwrap().is_empty());
    }

    #[test]
    fn a_2xx_learns_the_group_and_mirrors_the_budget() {
        let limiter = BucketLimiter::new(true, None);
        limiter.record_response(
            "characters/contracts",
            RateSubject::Character(42),
            reqwest::StatusCode::OK,
            &headers(&[
                ("x-ratelimit-group", "char-contract"),
                ("x-ratelimit-limit", "600/15m"),
                ("x-ratelimit-remaining", "598"),
                ("x-ratelimit-used", "2"),
            ]),
        );

        assert_eq!(
            limiter
                .endpoint_groups
                .lock()
                .unwrap()
                .get("characters/contracts")
                .cloned(),
            Some("char-contract".to_owned())
        );
        let buckets = limiter.buckets.lock().unwrap();
        let state = buckets
            .get(&("char-contract".to_owned(), RateSubject::Character(42)))
            .expect("bucket recorded");
        assert_eq!((state.limit, state.remaining), (600, 598));
    }

    #[tokio::test]
    async fn independent_group_subject_buckets_do_not_interfere() {
        let limiter = BucketLimiter::new(true, Some(5));
        limiter
            .endpoint_groups
            .lock()
            .unwrap()
            .extend([("ep-a", "grp-a".to_owned()), ("ep-b", "grp-b".to_owned())]);
        {
            let mut buckets = limiter.buckets.lock().unwrap();
            // A character low on grp-a's budget...
            buckets.insert(
                ("grp-a".to_owned(), RateSubject::Character(1)),
                BucketState {
                    limit: 10,
                    remaining: 1,
                    reset_at: Instant::now() + Duration::from_millis(60),
                },
            );
            // ...does not slow down a different character on the same
            // group, nor the same character on a different group.
            buckets.insert(
                ("grp-a".to_owned(), RateSubject::Character(2)),
                BucketState {
                    limit: 10,
                    remaining: 9,
                    reset_at: Instant::now() + Duration::from_secs(60),
                },
            );
            buckets.insert(
                ("grp-b".to_owned(), RateSubject::Character(1)),
                BucketState {
                    limit: 10,
                    remaining: 9,
                    reset_at: Instant::now() + Duration::from_secs(60),
                },
            );
        }

        let started = Instant::now();
        limiter.wait_before("ep-a", RateSubject::Character(2)).await;
        limiter.wait_before("ep-b", RateSubject::Character(1)).await;
        assert!(started.elapsed() < Duration::from_millis(20));

        let started = Instant::now();
        limiter.wait_before("ep-a", RateSubject::Character(1)).await;
        assert!(started.elapsed() >= Duration::from_millis(50));
    }
}
