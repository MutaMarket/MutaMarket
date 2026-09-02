//! Per-client request limits on the routes that fan out to ESI or start
//! background work for the caller (module appraisal, asset and contract
//! refreshes, re-estimation). Without them a script posting random ids
//! burns the app's ESI error budget, which throttles every job and the
//! login callback with it. Clients are told apart by the proxy's
//! `X-Forwarded-For` (Caddy sets it); requests without one, such as the
//! test router's, are not counted.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use super::AppState;

/// Requests per client per window on the limited routes.
pub const ESI_FANOUT_LIMIT: u32 = 30;

/// The counting window.
pub const WINDOW: Duration = Duration::from_secs(60);

/// The limited routes (method, path prefix).
const LIMITED: &[(&str, &str)] = &[
    ("POST", "/modules"),
    ("POST", "/api/modules"),
    ("POST", "/personal/modules"),
    ("POST", "/personal/contracts"),
    ("POST", "/estimate/"),
];

/// Laravel's throttle response text.
const TOO_MANY: &str = "Too Many Attempts.";

#[derive(Default)]
pub struct RateLimits {
    windows: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimits {
    /// Counts one request for the client; false once the window is full.
    pub fn allow(&self, client: &str, now: Instant) -> bool {
        let mut windows = self.windows.lock().expect("rate limit lock");
        // Forgotten clients fall out on the next pass so the map stays
        // proportional to the active set.
        windows.retain(|_, (started, _)| now.duration_since(*started) < WINDOW);
        let (started, count) = windows.entry(client.to_owned()).or_insert((now, 0));
        if now.duration_since(*started) >= WINDOW {
            *started = now;
            *count = 0;
        }
        *count += 1;
        *count <= ESI_FANOUT_LIMIT
    }
}

fn limited(method: &Method, path: &str) -> bool {
    LIMITED.iter().any(|(m, prefix)| {
        method.as_str() == *m
            && (path == *prefix || (prefix.ends_with('/') && path.starts_with(prefix)))
    })
}

/// The client as the proxy reports it: the first `X-Forwarded-For` hop,
/// else `X-Real-IP`.
pub fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|list| list.split(',').next())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
        })
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .map(str::to_owned)
}

pub async fn enforce(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if limited(request.method(), request.uri().path())
        && let Some(client) = client_ip(request.headers())
        && !state.limits.allow(&client, Instant::now())
    {
        return super::api::error(StatusCode::TOO_MANY_REQUESTS, TOO_MANY);
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_is_cut_off_after_the_limit_until_the_window_passes() {
        let limits = RateLimits::default();
        let start = Instant::now();
        for _ in 0..ESI_FANOUT_LIMIT {
            assert!(limits.allow("10.0.0.1", start));
        }
        assert!(!limits.allow("10.0.0.1", start));
        assert!(
            limits.allow("10.0.0.2", start),
            "other clients are unaffected"
        );
        assert!(
            limits.allow("10.0.0.1", start + WINDOW),
            "the window resets"
        );
    }

    #[test]
    fn only_the_fan_out_routes_are_limited() {
        assert!(limited(&Method::POST, "/modules"));
        assert!(limited(&Method::POST, "/estimate/42"));
        assert!(!limited(&Method::GET, "/api/modules/type/1"));
        assert!(!limited(&Method::POST, "/offers"));
    }
}
