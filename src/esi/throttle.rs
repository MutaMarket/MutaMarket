//! A self-imposed cap on the rate of outgoing ESI requests, so a burst of
//! background jobs cannot hammer ESI past its floating-window limits (the
//! `x-rate-limit` groups in the spec, e.g. `char-contract` at 600 tokens
//! per 15 minutes per character). Every request passes [`RateLimiter::acquire`]
//! before it executes, which spaces the whole client's requests at least
//! `1 / max_rps` apart regardless of how many job lanes fan out.
//!
//! This is a global request-rate cap, not a per-endpoint token accountant:
//! it smooths bursts so the per-endpoint limiters are reached far less
//! often, and it leaves the per-character 429 (which the caller already
//! skips) to be handled where the job knows the character.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The default cap when `ESI_MAX_RPS` is unset: gentle enough to stay well
/// under the busiest groups during a post-cutover sync, ample for steady
/// background work.
const DEFAULT_MAX_RPS: f64 = 20.0;

pub struct RateLimiter {
    /// The minimum spacing between two requests; `None` disables the cap.
    interval: Option<Duration>,
    /// The earliest instant the next request may start.
    next_slot: Mutex<Instant>,
}

impl RateLimiter {
    /// A limiter at `max_rps` requests per second; `max_rps <= 0` disables it.
    pub fn new(max_rps: f64) -> Self {
        let interval = (max_rps > 0.0).then(|| Duration::from_secs_f64(1.0 / max_rps));
        Self {
            interval,
            next_slot: Mutex::new(Instant::now()),
        }
    }

    /// The production limiter, `ESI_MAX_RPS` requests per second (default
    /// [`DEFAULT_MAX_RPS`]). An unparsable value falls back to the default;
    /// `0` disables the cap.
    pub fn from_env() -> Self {
        let max_rps = std::env::var("ESI_MAX_RPS")
            .ok()
            .and_then(|value| value.trim().parse::<f64>().ok())
            .unwrap_or(DEFAULT_MAX_RPS);
        Self::new(max_rps)
    }

    /// An uncapped limiter, for the many test clients that talk to a local
    /// mock and must not be paced.
    pub fn disabled() -> Self {
        Self::new(0.0)
    }

    /// Claims the next slot and waits until it is due. Reserving the slot
    /// under the lock and sleeping outside it lets concurrent callers line
    /// up one interval apart instead of all waking together.
    pub async fn acquire(&self) {
        let Some(interval) = self.interval else {
            return;
        };
        let slot = {
            let mut next = self.next_slot.lock().expect("rate limiter lock");
            let now = Instant::now();
            let slot = (*next).max(now);
            *next = slot + interval;
            slot
        };
        let now = Instant::now();
        if slot > now {
            tokio::time::sleep(slot - now).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimiter;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn spaces_requests_by_the_interval() {
        // 200 rps => 5ms apart; three back-to-back acquires span >= 10ms.
        let limiter = RateLimiter::new(200.0);
        let started = Instant::now();
        limiter.acquire().await;
        limiter.acquire().await;
        limiter.acquire().await;
        assert!(started.elapsed() >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn disabled_never_waits() {
        let limiter = RateLimiter::disabled();
        let started = Instant::now();
        for _ in 0..10_000 {
            limiter.acquire().await;
        }
        assert!(started.elapsed() < Duration::from_millis(50));
    }
}
