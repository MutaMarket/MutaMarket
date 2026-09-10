//! Persisted detail of failed incoming requests, so an error count on
//! the console's route roll-up can be opened and read.
//!
//! [`super`] keeps the exact counts per hour and route; this keeps a
//! bounded sample of the failures behind them with the concrete path,
//! the account (when the request carried a session) and our own error
//! body. `src/esi/failures.rs` does the same for outgoing ESI calls.
//!
//! # What is not stored
//!
//! No request headers, and deliberately no field for them: every request
//! carries the session cookie, and a struct that cannot hold a header
//! cannot leak one. No request bodies either, so a password-less site's
//! one secret-bearing body, the EVE mail a user sends, stays out of the
//! table. Query parameters naming a secret are redacted before the path
//! is written; the response body is only ever our own error JSON or an
//! error page.

use std::collections::HashMap;
use std::time::Duration;

use sqlx::PgPool;

/// Response body kept per failure. Our error bodies are one-line JSON;
/// this still catches an error page or a proxy interstitial with the
/// useful part intact.
pub const BODY_CAPTURE_BYTES: usize = 8 * 1024;

/// Failures kept in the table, newest first (the `FAILURE_HISTORY_KEEP`
/// idiom of the ESI failure log).
pub const FAILURE_HISTORY_KEEP: i64 = 2000;

/// Age beyond which a failure is dropped regardless of the row cap.
pub const FAILURE_RETENTION_DAYS: i64 = 7;

/// Failures stored per route and status per minute. A scanner sweeping
/// the site is thousands of identical 404s a minute; without this one
/// sweep would evict everything else from the table. The true counts stay
/// exact in `activity_hours`.
pub const CAPTURES_PER_MINUTE_PER_KIND: u32 = 3;

/// Query parameters whose value is replaced before the path is stored.
/// `code` and `state` are the EVE SSO callback's: a failed callback is
/// exactly the kind of failure worth capturing, and its code is exactly
/// what must not be kept.
const REDACTED_QUERY_PARAMS: [&str; 6] = [
    "code",
    "state",
    "token",
    "access_token",
    "refresh_token",
    "secret",
];

/// Per-minute capture budget, keyed by route and status.
#[derive(Default)]
pub struct Sampler {
    minute: i64,
    taken: HashMap<(String, u16), u32>,
}

impl Sampler {
    pub fn allow(&mut self, minute: i64, route: &str, status: u16) -> bool {
        if minute != self.minute {
            self.minute = minute;
            self.taken.clear();
        }
        let taken = self.taken.entry((route.to_owned(), status)).or_default();
        if *taken >= CAPTURES_PER_MINUTE_PER_KIND {
            return false;
        }
        *taken += 1;
        true
    }
}

/// One failed request, as the log stores it.
pub struct RequestFailure {
    /// Method plus the matched route pattern, the `activity_hours` key.
    pub route: String,
    pub method: String,
    /// Concrete path with its query, secrets redacted.
    pub path: String,
    pub status: u16,
    /// The signed-in account behind the request, if any.
    pub user_id: Option<i64>,
    pub duration: Duration,
    /// The response body and its length before truncation.
    pub body: Option<(String, i64)>,
}

/// Writes the failure and bounds the table. Never returns an error: the
/// log must not mask the failure it describes, let alone fail the
/// response carrying it.
pub async fn record(pool: &PgPool, failure: &RequestFailure) {
    let body = failure.body.as_ref();
    let result = sqlx::query(
        "insert into request_failures
             (route, method, path, status, error_message, response_body, response_bytes,
              duration_ms, user_id)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&failure.route)
    .bind(&failure.method)
    .bind(&failure.path)
    .bind(failure.status as i32)
    .bind(body.and_then(|(text, _)| error_message(text)))
    .bind(body.map(|(text, _)| text.as_str()))
    .bind(body.map(|(_, bytes)| *bytes))
    .bind(failure.duration.as_millis() as i64)
    .bind(failure.user_id)
    .execute(pool)
    .await;

    if let Err(error) = result {
        tracing::warn!("recording a request failure failed: {error}");
        return;
    }
    prune(pool).await;
}

async fn prune(pool: &PgPool) {
    let result = sqlx::query(
        "delete from request_failures
         where id <= coalesce(
                   (select id from request_failures order by id desc offset $1 limit 1), 0)
            or occurred_at < now() - make_interval(days => $2::int)",
    )
    .bind(FAILURE_HISTORY_KEEP)
    .bind(FAILURE_RETENTION_DAYS as i32)
    .execute(pool)
    .await;

    if let Err(error) = result {
        tracing::warn!("pruning request failures failed: {error}");
    }
}

/// Replaces the value of any secret-bearing query parameter.
pub fn redact_path(path: &str) -> String {
    let Some((base, query)) = path.split_once('?') else {
        return path.to_owned();
    };
    let redacted: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((key, _)) if REDACTED_QUERY_PARAMS.contains(&key) => format!("{key}=[redacted]"),
            _ => pair.to_owned(),
        })
        .collect();
    format!("{base}?{}", redacted.join("&"))
}

/// The body as text capped at [`BODY_CAPTURE_BYTES`], with its length
/// before truncation so the console can say what it is not showing.
pub fn truncate_body(bytes: &[u8]) -> (String, i64) {
    let full = bytes.len() as i64;
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= BODY_CAPTURE_BYTES {
        return (text.into_owned(), full);
    }
    // Cut on a char boundary; a multibyte sequence spanning the cap
    // would otherwise panic the slice.
    let mut end = BODY_CAPTURE_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), full)
}

/// Our own message, from the `{"message": "..."}` body every API error
/// answers with. An error page body has none and keeps null.
fn error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("message")?
        .as_str()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_the_sso_callback_and_leaves_the_rest() {
        assert_eq!(
            redact_path("/eve/callback?code=secret&state=nonce"),
            "/eve/callback?code=[redacted]&state=[redacted]"
        );
        assert_eq!(
            redact_path("/api/module-cards/type/47408?unlisted=true"),
            "/api/module-cards/type/47408?unlisted=true"
        );
        assert_eq!(redact_path("/api/search-alerts"), "/api/search-alerts");
    }

    #[test]
    fn keeps_short_bodies_whole_and_reports_the_full_length() {
        let (text, bytes) = truncate_body(b"{\"message\": \"Unauthenticated.\"}");
        assert_eq!(text, "{\"message\": \"Unauthenticated.\"}");
        assert_eq!(bytes, 31);
    }

    #[test]
    fn truncates_a_long_body_on_a_char_boundary() {
        let body = "é".repeat(BODY_CAPTURE_BYTES);
        let (text, bytes) = truncate_body(body.as_bytes());
        assert!(text.len() <= BODY_CAPTURE_BYTES);
        assert!(text.chars().all(|c| c == 'é'));
        assert_eq!(bytes, (BODY_CAPTURE_BYTES * 2) as i64);
    }

    #[test]
    fn reads_our_own_error_message() {
        assert_eq!(
            error_message("{\"message\": \"Unauthenticated.\"}").as_deref(),
            Some("Unauthenticated.")
        );
        assert_eq!(error_message("{\"other\": 1}"), None);
        assert_eq!(error_message("<html>nope</html>"), None);
    }

    #[test]
    fn the_sampler_bounds_each_route_and_status_per_minute() {
        let mut sampler = Sampler::default();
        for _ in 0..CAPTURES_PER_MINUTE_PER_KIND {
            assert!(sampler.allow(100, "GET /api/search-alerts", 401));
        }
        assert!(!sampler.allow(100, "GET /api/search-alerts", 401));

        // A different status and a different route each get their own
        // budget, so one storm cannot hide another failure.
        assert!(sampler.allow(100, "GET /api/search-alerts", 500));
        assert!(sampler.allow(100, "GET /api/modules/{module}", 401));

        // The next minute starts over.
        assert!(sampler.allow(101, "GET /api/search-alerts", 401));
    }
}
