//! Persisted detail of failed outgoing ESI requests, so a count on the
//! admin console's error chart can be opened and read.
//!
//! [`super::telemetry`] keeps the exact per-minute counts; this keeps a
//! bounded sample of the failures behind them with the status, the URL,
//! ESI's message and the response body.
//!
//! # Secrets
//!
//! Request headers are never stored, and there is deliberately no field
//! for them: authenticated calls carry `Authorization: Bearer <token>`,
//! and a struct that cannot hold a header cannot leak one. Only
//! [`RequestContext::authenticated`] records that a token was sent.
//! Request bodies are default-deny — only [`CAPTURED_REQUEST_BODIES`]
//! endpoints store one, which is what keeps the mail a user sends out of
//! the database. Response bodies are only ever read on a failure, where
//! ESI answers with `{"error": ...}` rather than a payload.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

/// Response body kept per failure. An ESI error body is under 200 bytes;
/// this still catches an HTML error page or a proxy interstitial with
/// the useful part intact.
pub const BODY_CAPTURE_BYTES: usize = 8 * 1024;

/// Failures kept in the table, newest first (the `RUN_HISTORY_KEEP`
/// idiom of the scheduler's run history).
pub const FAILURE_HISTORY_KEEP: i64 = 2000;

/// Age beyond which a failure is dropped regardless of the row cap.
pub const FAILURE_RETENTION_DAYS: i64 = 7;

/// Failures stored per endpoint and status class per minute. A bad day
/// on ESI's side is thousands of identical 500s in a minute; without
/// this one burst would evict everything else from the table. The true
/// counts stay exact in the telemetry buckets, so the console can show
/// "12 errors this minute · 3 captured".
pub const CAPTURES_PER_MINUTE_PER_KIND: u32 = 3;

/// Query parameters whose value is replaced before the URL is stored.
/// We authenticate with a bearer header, but ESI's older token-in-query
/// form would otherwise land in the table verbatim.
const REDACTED_QUERY_PARAMS: [&str; 2] = ["token", "access_token"];

/// The only endpoints whose request body is stored. Each posts a bare
/// array of ids, which is exactly what makes a failure diagnosable.
/// Everything else stores nothing — notably `characters/mail`, whose
/// body is a player's message.
const CAPTURED_REQUEST_BODIES: [&str; 3] =
    ["characters/affiliation", "universe/names", "assets/names"];

/// Response headers worth keeping: the error-limit budget explains a
/// 420 storm, the request id is what CCP asks for, the rest frame the
/// body.
const CAPTURED_RESPONSE_HEADERS: [&str; 7] = [
    "content-type",
    "retry-after",
    "warning",
    "x-esi-error-limit-remain",
    "x-esi-error-limit-reset",
    "x-esi-request-id",
    "x-pages",
];

tokio::task_local! {
    /// The job or handler the current ESI call is running under.
    pub static ESI_CALLER: EsiCaller;
}

#[derive(Clone)]
pub struct EsiCaller {
    /// `job:region-contracts` or `http:GET /api/modules/{module}`.
    pub label: String,
    /// The `scheduler_runs` row this call belongs to, for a job.
    pub scheduler_run_id: Option<i64>,
}

impl EsiCaller {
    pub fn job(name: &str, scheduler_run_id: Option<i64>) -> Self {
        Self {
            label: format!("job:{name}"),
            scheduler_run_id,
        }
    }

    pub fn http(label: String) -> Self {
        Self {
            label: format!("http:{label}"),
            scheduler_run_id: None,
        }
    }

    fn current() -> Option<Self> {
        ESI_CALLER.try_with(Clone::clone).ok()
    }
}

/// What is known about a request before its response arrives.
pub struct RequestContext {
    endpoint: &'static str,
    method: String,
    /// Full URL with its query, token-ish parameters redacted.
    url: String,
    /// Body and its byte length, for allowlisted endpoints only.
    request_body: Option<(String, i64)>,
    /// An `Authorization` header was sent. The header itself is not kept.
    authenticated: bool,
}

impl RequestContext {
    pub fn capture(endpoint: &'static str, request: &reqwest::Request) -> Self {
        let request_body = if CAPTURED_REQUEST_BODIES.contains(&endpoint) {
            request
                .body()
                .and_then(reqwest::Body::as_bytes)
                .map(truncate_body)
        } else {
            None
        };

        Self {
            endpoint,
            method: request.method().as_str().to_owned(),
            url: redact_url(request.url().as_str()),
            request_body,
            authenticated: request
                .headers()
                .contains_key(reqwest::header::AUTHORIZATION),
        }
    }
}

/// Replaces the value of any token-ish query parameter.
fn redact_url(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_owned();
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
fn truncate_body(bytes: &[u8]) -> (String, i64) {
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

/// The transport failure kind, for a request that got no response.
fn error_kind(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_decode() {
        "decode"
    } else if error.is_body() {
        "body"
    } else {
        "request"
    }
}

/// ESI's own message, from the `{"error": "..."}` body it answers with.
fn esi_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("error")?
        .as_str()
        .map(str::to_owned)
}

/// Per-minute capture budget, keyed by endpoint and status class.
#[derive(Default)]
struct Sampler {
    minute: i64,
    taken: HashMap<(&'static str, Option<u16>), u32>,
}

impl Sampler {
    fn allow(&mut self, minute: i64, endpoint: &'static str, status: Option<u16>) -> bool {
        if minute != self.minute {
            self.minute = minute;
            self.taken.clear();
        }
        let taken = self.taken.entry((endpoint, status)).or_default();
        if *taken >= CAPTURES_PER_MINUTE_PER_KIND {
            return false;
        }
        *taken += 1;
        true
    }
}

pub struct EsiFailureLog {
    pool: PgPool,
    sampler: Mutex<Sampler>,
}

impl EsiFailureLog {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            sampler: Mutex::new(Sampler::default()),
        }
    }

    fn allow(&self, endpoint: &'static str, status: Option<u16>) -> bool {
        let minute = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|now| (now.as_secs() / 60) as i64)
            .unwrap_or(0);
        self.sampler
            .lock()
            .expect("sampler lock")
            .allow(minute, endpoint, status)
    }

    /// Records a response the client refused to accept.
    pub async fn record_response(
        &self,
        context: &RequestContext,
        status: reqwest::StatusCode,
        headers: &reqwest::header::HeaderMap,
        body: &[u8],
        elapsed: Duration,
    ) {
        tracing::warn!(%status, url = context.url, "ESI request failed");
        if !self.allow(context.endpoint, Some(status.as_u16())) {
            return;
        }

        let (body_text, body_bytes) = truncate_body(body);
        let kept: serde_json::Map<String, serde_json::Value> = CAPTURED_RESPONSE_HEADERS
            .iter()
            .filter_map(|name| {
                let value = headers.get(*name)?.to_str().ok()?;
                Some(((*name).to_owned(), serde_json::Value::from(value)))
            })
            .collect();

        self.insert(
            context,
            Some(status.as_u16() as i32),
            None,
            esi_error_message(&body_text),
            elapsed,
            Some(serde_json::Value::Object(kept)),
            Some((body_text, body_bytes)),
        )
        .await;
    }

    /// Records a request that never got a response at all.
    pub async fn record_transport(
        &self,
        context: &RequestContext,
        error: &reqwest::Error,
        elapsed: Duration,
    ) {
        tracing::warn!(url = context.url, "ESI request failed: {error}");
        if !self.allow(context.endpoint, None) {
            return;
        }

        self.insert(
            context,
            None,
            Some(error_kind(error)),
            Some(error.to_string()),
            elapsed,
            None,
            None,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert(
        &self,
        context: &RequestContext,
        status: Option<i32>,
        error_kind: Option<&str>,
        error_message: Option<String>,
        elapsed: Duration,
        response_headers: Option<serde_json::Value>,
        response_body: Option<(String, i64)>,
    ) {
        let caller = EsiCaller::current();
        let result = sqlx::query(
            "insert into esi_failures
                 (endpoint, method, url, status, error_kind, error_message, duration_ms,
                  authenticated, caller, scheduler_run_id, response_headers, response_body,
                  response_bytes, request_body, request_bytes)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(context.endpoint)
        .bind(&context.method)
        .bind(&context.url)
        .bind(status)
        .bind(error_kind)
        .bind(error_message)
        .bind(elapsed.as_millis() as i64)
        .bind(context.authenticated)
        .bind(caller.as_ref().map(|caller| caller.label.as_str()))
        .bind(caller.as_ref().and_then(|caller| caller.scheduler_run_id))
        .bind(response_headers)
        .bind(response_body.as_ref().map(|(text, _)| text.as_str()))
        .bind(response_body.as_ref().map(|(_, bytes)| *bytes))
        .bind(context.request_body.as_ref().map(|(text, _)| text.as_str()))
        .bind(context.request_body.as_ref().map(|(_, bytes)| *bytes))
        .execute(&self.pool)
        .await;

        // The failure log must never mask the ESI error it describes.
        if let Err(error) = result {
            tracing::warn!("recording an ESI failure failed: {error}");
            return;
        }
        self.prune().await;
    }

    async fn prune(&self) {
        let result = sqlx::query(
            "delete from esi_failures
             where id <= coalesce(
                       (select id from esi_failures order by id desc offset $1 limit 1), 0)
                or occurred_at < now() - make_interval(days => $2::int)",
        )
        .bind(FAILURE_HISTORY_KEEP)
        .bind(FAILURE_RETENTION_DAYS as i32)
        .execute(&self.pool)
        .await;

        if let Err(error) = result {
            tracing::warn!("pruning ESI failures failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_token_query_parameters_and_leaves_the_rest() {
        assert_eq!(
            redact_url("https://esi/latest/x/?token=secret&region_id=10000002"),
            "https://esi/latest/x/?token=[redacted]&region_id=10000002"
        );
        assert_eq!(
            redact_url("https://esi/latest/x/?access_token=secret"),
            "https://esi/latest/x/?access_token=[redacted]"
        );
        assert_eq!(
            redact_url("https://esi/latest/x/?page=2"),
            "https://esi/latest/x/?page=2"
        );
        assert_eq!(redact_url("https://esi/latest/x/"), "https://esi/latest/x/");
    }

    #[test]
    fn keeps_short_bodies_whole_and_reports_the_full_length() {
        let (text, bytes) = truncate_body(b"{\"error\": \"nope\"}");
        assert_eq!(text, "{\"error\": \"nope\"}");
        assert_eq!(bytes, 17);
    }

    #[test]
    fn truncates_a_long_body_on_a_char_boundary() {
        // A multibyte character straddling the cap must not panic the
        // slice, and the reported length is the untruncated one.
        let body = "é".repeat(BODY_CAPTURE_BYTES);
        let (text, bytes) = truncate_body(body.as_bytes());
        assert!(text.len() <= BODY_CAPTURE_BYTES);
        assert!(text.chars().all(|c| c == 'é'));
        assert_eq!(bytes, (BODY_CAPTURE_BYTES * 2) as i64);
    }

    #[test]
    fn reads_esi_own_error_message() {
        assert_eq!(
            esi_error_message("{\"error\": \"Character not found\"}").as_deref(),
            Some("Character not found")
        );
        assert_eq!(esi_error_message("{\"other\": 1}"), None);
        assert_eq!(esi_error_message("not json at all"), None);
    }

    #[test]
    fn the_sampler_bounds_each_kind_per_minute() {
        let mut sampler = Sampler::default();
        for _ in 0..CAPTURES_PER_MINUTE_PER_KIND {
            assert!(sampler.allow(100, "contracts/public", Some(500)));
        }
        assert!(!sampler.allow(100, "contracts/public", Some(500)));

        // A different status class and a different endpoint each get
        // their own budget, so one storm cannot hide another failure.
        assert!(sampler.allow(100, "contracts/public", Some(404)));
        assert!(sampler.allow(100, "characters/assets", Some(500)));
        assert!(sampler.allow(100, "contracts/public", None));

        // The next minute starts over.
        assert!(sampler.allow(101, "contracts/public", Some(500)));
    }
}
