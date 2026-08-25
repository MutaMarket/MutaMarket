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

/// Recorded runs returned per job in the status payload.
const RUNS_SHOWN: i64 = 10;

/// One `scheduler_runs` row as selected for the status payload:
/// (started_at, finished_at, outcome, summary, error).
type RunRow = (String, Option<String>, Option<String>, Option<String>, Option<String>);

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
                "select started_at::text, finished_at::text, outcome, summary, error
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
            "last_runs": runs
                .into_iter()
                .map(|(started_at, finished_at, outcome, summary, error)| {
                    json!({
                        "started_at": started_at,
                        "finished_at": finished_at,
                        "outcome": outcome,
                        "summary": summary,
                        "error": error,
                    })
                })
                .collect::<Vec<_>>(),
        }));
    }

    Json(json!({
        "enabled": state.scheduler.enabled,
        "in_downtime": crate::scheduler::is_downtime(),
        "jobs": jobs,
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
