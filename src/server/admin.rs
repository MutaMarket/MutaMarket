//! `/api/admin` — internal observability and control endpoints, gated on
//! `users.is_admin` (no legacy counterpart; flip the flag manually:
//! `update users set is_admin = true where id = ...`).

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::AppState;
use crate::auth::session::session_from_headers;
use crate::scheduler::RunNowOutcome;

/// Recorded runs returned per job in the status payload (the per-job
/// cards chart them).
const RUNS_SHOWN: i64 = 20;

/// One `scheduler_runs` row as selected for the status payload:
/// (started_at, finished_at, outcome, summary, error, items,
/// duration_seconds, metrics).
type RunRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<serde_json::Value>,
);

/// The admin gate: 401 for guests, 403 for non-admin users.
async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    let session = match session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Err(super::api::error(StatusCode::UNAUTHORIZED, "Unauthenticated.")),
        Err(error) => return Err(super::api::database_error(error)),
    };

    let is_admin: bool = sqlx::query_scalar("select is_admin from users where id = $1")
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(super::api::database_error)?
        .unwrap_or(false);

    if is_admin {
        Ok(())
    } else {
        Err(super::api::error(StatusCode::FORBIDDEN, "Forbidden."))
    }
}

/// `GET /api/admin/scheduler` — every job with its live state and the
/// newest recorded runs.
pub async fn scheduler_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let mut jobs = Vec::new();
    for snapshot in state.scheduler.snapshots() {
        let runs: Result<Vec<RunRow>, _> = sqlx::query_as(
                "select started_at::text, finished_at::text, outcome, summary, error, items,
                        extract(epoch from finished_at - started_at)::bigint as duration_seconds,
                        metrics
                 from scheduler_runs where job = $1 order by id desc limit $2",
            )
            .bind(snapshot.name)
            .bind(RUNS_SHOWN)
            .fetch_all(&state.pool)
            .await;
        let runs = match runs {
            Ok(runs) => runs,
            Err(error) => return super::api::database_error(error),
        };

        jobs.push(json!({
            "name": snapshot.name,
            "interval_seconds": snapshot.interval.as_secs(),
            "downtime_guarded": snapshot.downtime_guarded,
            "paused": snapshot.paused,
            "running": snapshot.running,
            "next_run_at": snapshot.next_run_at,
            "progress": snapshot.progress,
            "last_runs": runs
                .into_iter()
                .map(
                    |(
                        started_at,
                        finished_at,
                        outcome,
                        summary,
                        error,
                        items,
                        duration_seconds,
                        metrics,
                    )| {
                        json!({
                            "started_at": started_at,
                            "finished_at": finished_at,
                            "outcome": outcome,
                            "summary": summary,
                            "error": error,
                            "items": items,
                            "duration_seconds": duration_seconds,
                            "metrics": metrics,
                        })
                    },
                )
                .collect::<Vec<_>>(),
        }));
    }

    // The table counts and metric history are expensive (full count
    // scans; thousands of sample rows) and change slowly, while the
    // dashboard polls every five seconds - serve them from a short
    // in-process cache.
    let slow_data = match cached_slow_data(&state.pool).await {
        Ok(slow_data) => slow_data,
        Err(error) => return super::api::database_error(error),
    };

    Json(json!({
        "enabled": state.scheduler.enabled,
        "in_downtime": crate::scheduler::is_downtime(),
        "database": slow_data["database"],
        "metrics": slow_data["metrics"],
        "jobs": jobs,
    }))
    .into_response()
}

/// How long the counts + history fragment is reused between polls.
const SLOW_DATA_TTL: std::time::Duration = std::time::Duration::from_secs(60);

static SLOW_DATA_CACHE: std::sync::Mutex<Option<(std::time::Instant, serde_json::Value)>> =
    std::sync::Mutex::new(None);

async fn cached_slow_data(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    if let Some((taken, value)) = SLOW_DATA_CACHE.lock().expect("cache lock").as_ref()
        && taken.elapsed() < SLOW_DATA_TTL
    {
        return Ok(value.clone());
    }

    let database = database_counts(pool).await?;
    let metrics = crate::metrics::history(pool).await?;
    let value = json!({ "database": database, "metrics": metrics });
    *SLOW_DATA_CACHE.lock().expect("cache lock") = Some((std::time::Instant::now(), value.clone()));
    Ok(value)
}

/// Live row counts of the ingestion-facing tables, so the page shows
/// what background work is actually landing in the database.
async fn database_counts(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    let (
        modules,
        modules_without_estimate,
        contracts,
        contract_items,
        characters,
        users,
        assets,
        public_ownerships,
        market_history_days,
    ): (i64, i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
             (select count(*) from modules),
             (select count(*) from modules where estimated_value is null),
             (select count(*) from contracts),
             (select count(*) from contract_items),
             (select count(*) from characters),
             (select count(*) from users),
             (select count(*) from assets),
             (select count(*) from public_module_ownerships),
             (select count(*) from market_histories)",
    )
    .fetch_one(pool)
    .await?;

    Ok(json!({
        "modules": modules,
        "modules_without_estimate": modules_without_estimate,
        "contracts": contracts,
        "contract_items": contract_items,
        "characters": characters,
        "users": users,
        "assets": assets,
        "public_ownerships": public_ownerships,
        "market_history_days": market_history_days,
    }))
}

/// `GET /api/admin/telemetry` — the last hour of outgoing ESI requests
/// as per-minute buckets per endpoint group.
pub async fn telemetry(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    Json(json!({
        "window_minutes": crate::esi::telemetry::WINDOW_MINUTES,
        "buckets": state.esi.telemetry().snapshot(),
    }))
    .into_response()
}

/// `POST /api/admin/scheduler/{job}/run` — trigger a job outside its
/// schedule (works while the scheduled loops are disabled too).
pub async fn scheduler_run(
    State(state): State<AppState>,
    Path(job): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    match state.scheduler.run_now(&job) {
        RunNowOutcome::Started => (
            StatusCode::ACCEPTED,
            Json(json!({ "message": "Run started." })),
        )
            .into_response(),
        RunNowOutcome::AlreadyRunning => {
            super::api::error(StatusCode::CONFLICT, "This job is already running.")
        }
        RunNowOutcome::UnknownJob => super::api::error(StatusCode::NOT_FOUND, "Unknown job."),
    }
}

/// `PUT /api/admin/scheduler/{job}` — persist the pause flag.
pub async fn scheduler_update(
    State(state): State<AppState>,
    Path(job): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    #[derive(serde::Deserialize)]
    struct Payload {
        paused: bool,
    }
    let Ok(payload) = serde_json::from_slice::<Payload>(&body) else {
        return super::api::error(StatusCode::UNPROCESSABLE_ENTITY, "The given data was invalid.");
    };

    match state.scheduler.set_paused(&job, payload.paused).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => super::api::error(StatusCode::NOT_FOUND, "Unknown job."),
        Err(error) => super::api::database_error(error),
    }
}

/// `PUT /api/historic-contracts/{id}` — the legacy
/// `HistoricContractsController::update`, reduced to the fields the
/// contract-actions dropdown sends. A contract that no longer qualifies
/// (not completed, extra items, or ignored) loses its training module.
pub async fn historic_contract_update(
    State(state): State<AppState>,
    Path(contract_id): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    #[derive(serde::Deserialize)]
    struct Payload {
        ignore_for_training: Option<bool>,
        non_abyssal_modules_count: Option<i32>,
        status: Option<String>,
    }
    let Ok(payload) = serde_json::from_slice::<Payload>(&body) else {
        return super::api::error(StatusCode::UNPROCESSABLE_ENTITY, "The given data was invalid.");
    };
    if payload.non_abyssal_modules_count.is_some_and(|count| count < 0) {
        return super::api::error(StatusCode::UNPROCESSABLE_ENTITY, "The given data was invalid.");
    }
    if let Some(status) = &payload.status
        && !["outstanding", "completed", "failed", "unknown"].contains(&status.as_str())
    {
        return super::api::error(StatusCode::UNPROCESSABLE_ENTITY, "The given data was invalid.");
    }

    let updated = sqlx::query(
        "update historic_contracts set
             ignore_for_training = coalesce($2, ignore_for_training),
             non_abyssal_modules_count = coalesce($3, non_abyssal_modules_count),
             status = coalesce($4, status),
             updated_at = now()
         where id = $1",
    )
    .bind(contract_id)
    .bind(payload.ignore_for_training)
    .bind(payload.non_abyssal_modules_count)
    .bind(&payload.status)
    .execute(&state.pool)
    .await;
    match updated {
        Ok(result) if result.rows_affected() == 0 => {
            return super::api::error(StatusCode::NOT_FOUND, "Not found.");
        }
        Ok(_) => {}
        Err(error) => return super::api::database_error(error),
    }

    let cleanup = sqlx::query(
        "delete from training_modules tm using historic_contracts hc
         where tm.historic_contract_id = $1 and hc.id = $1
           and (hc.status <> 'completed'
                or hc.non_abyssal_modules_count > 0
                or hc.ignore_for_training)",
    )
    .bind(contract_id)
    .execute(&state.pool)
    .await;
    if let Err(error) = cleanup {
        return super::api::database_error(error);
    }

    StatusCode::NO_CONTENT.into_response()
}

/// Process start marker for the uptime stat, stamped by `router()`.
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn mark_started() {
    let _ = STARTED.set(std::time::Instant::now());
}

/// `GET /api/admin/system` — process and container telemetry: cgroup v2
/// memory, process rss/cpu, interface byte counters (the client derives
/// rates from consecutive polls) and the database size, read through the
/// shared `metrics` host readers. The /proc and cgroup fields are null
/// outside Linux (native dev on macOS).
pub async fn system(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let database_size_bytes: Option<i64> =
        sqlx::query_scalar("select pg_database_size(current_database())")
            .fetch_one(&state.pool)
            .await
            .ok();

    let network = crate::metrics::network_totals();
    Json(json!({
        "memory_rss_bytes": crate::metrics::process_rss_bytes(),
        "memory_current_bytes": crate::metrics::read_number("/sys/fs/cgroup/memory.current"),
        "memory_limit_bytes": crate::metrics::read_number("/sys/fs/cgroup/memory.max"),
        "cpu_seconds": crate::metrics::process_cpu_seconds(),
        "cpu_cores": std::thread::available_parallelism().map(|cores| cores.get()).ok(),
        "network_rx_bytes": network.map(|(rx, _)| rx),
        "network_tx_bytes": network.map(|(_, tx)| tx),
        "uptime_seconds": STARTED.get().map(|started| started.elapsed().as_secs()),
        "database_size_bytes": database_size_bytes,
    }))
    .into_response()
}

