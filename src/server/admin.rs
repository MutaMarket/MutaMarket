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

    // The table counts are expensive (full count scans) and change
    // slowly, while the dashboard polls every five seconds - serve them
    // from a short in-process cache. The metric history moved to its
    // own windowed endpoint, fetched only on load and toggle.
    let database = match cached_database_counts(&state.pool).await {
        Ok(database) => database,
        Err(error) => return super::api::database_error(error),
    };

    Json(json!({
        "enabled": state.scheduler.enabled,
        "in_downtime": crate::scheduler::is_downtime(),
        "database": database,
        "jobs": jobs,
    }))
    .into_response()
}

/// How long the counts fragment is reused between polls.
const SLOW_DATA_TTL: std::time::Duration = std::time::Duration::from_secs(60);

static SLOW_DATA_CACHE: std::sync::Mutex<Option<(std::time::Instant, serde_json::Value)>> =
    std::sync::Mutex::new(None);

async fn cached_database_counts(pool: &sqlx::PgPool) -> sqlx::Result<serde_json::Value> {
    if let Some((taken, value)) = SLOW_DATA_CACHE.lock().expect("cache lock").as_ref()
        && taken.elapsed() < SLOW_DATA_TTL
    {
        return Ok(value.clone());
    }

    let value = database_counts(pool).await?;
    *SLOW_DATA_CACHE.lock().expect("cache lock") = Some((std::time::Instant::now(), value.clone()));
    Ok(value)
}

/// `GET /api/admin/metrics?window=` — the vitals history for the
/// dashboard charts, in one of the toggle's windows (default 24h).
pub async fn metrics_history(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<MetricsParams>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let wanted = params.window.as_deref().unwrap_or("24h");
    let Some((label, hours, step)) = crate::metrics::HISTORY_WINDOWS
        .iter()
        .find(|(label, _, _)| *label == wanted)
        .copied()
    else {
        return super::api::error(StatusCode::UNPROCESSABLE_ENTITY, "The selected window is invalid.");
    };

    match crate::metrics::history(&state.pool, hours, step).await {
        Ok(series) => Json(json!({
            "window": label,
            "step_seconds": step,
            "series": series,
        }))
        .into_response(),
        Err(error) => super::api::database_error(error),
    }
}

#[derive(serde::Deserialize, Default)]
pub struct MetricsParams {
    window: Option<String>,
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
    let disk = crate::metrics::disk_usage();
    Json(json!({
        "disk_used_bytes": disk.map(|(used, _)| used),
        "disk_total_bytes": disk.map(|(_, total)| total),
        "memory_rss_bytes": crate::metrics::process_rss_bytes(),
        "memory_current_bytes": crate::metrics::read_number("/sys/fs/cgroup/memory.current"),
        "memory_limit_bytes": crate::metrics::read_number("/sys/fs/cgroup/memory.max"),
        "memory_total_bytes": crate::metrics::host_memory_total_bytes(),
        "cpu_seconds": crate::metrics::process_cpu_seconds(),
        "cpu_cores": std::thread::available_parallelism().map(|cores| cores.get()).ok(),
        "network_rx_bytes": network.map(|(rx, _)| rx),
        "network_tx_bytes": network.map(|(_, tx)| tx),
        "uptime_seconds": STARTED.get().map(|started| started.elapsed().as_secs()),
        "database_size_bytes": database_size_bytes,
    }))
    .into_response()
}

/// `GET /api/admin/service-character` — the character the background
/// features act through (structure resolution, future donation and
/// wallet processing): the admin-authorized setting, or the env
/// fallback, with its freshest token's scopes.
pub async fn service_character(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let setting = match crate::app_settings::get(
        &state.pool,
        crate::app_settings::SERVICE_CHARACTER_KEY,
    )
    .await
    {
        Ok(setting) => setting,
        Err(error) => return super::api::database_error(error),
    };
    let from_setting = setting.as_deref().and_then(|value| value.parse::<i64>().ok());
    let character_id = match from_setting {
        Some(id) => Some(id),
        None => std::env::var("EVE_STRUCTURES_CHARACTER_ID")
            .ok()
            .and_then(|value| value.parse().ok()),
    };
    let Some(character_id) = character_id else {
        return Json(json!({ "character": serde_json::Value::Null, "source": serde_json::Value::Null }))
            .into_response();
    };

    type CharacterRow = (String, Option<Vec<String>>);
    let row: Result<Option<CharacterRow>, _> = sqlx::query_as(
        "select c.name,
                (select t.scopes from esi_tokens t
                 where t.character_id = c.id order by t.id desc limit 1)
         from characters c where c.id = $1",
    )
    .bind(character_id)
    .fetch_optional(&state.pool)
    .await;
    let (name, scopes) = match row {
        Ok(Some((name, scopes))) => (Some(name), scopes.unwrap_or_default()),
        Ok(None) => (None, Vec::new()),
        Err(error) => return super::api::database_error(error),
    };

    Json(json!({
        "character": { "id": character_id, "name": name, "scopes": scopes },
        "source": if from_setting.is_some() { "authorized" } else { "env" },
    }))
    .into_response()
}

/// One gear item of the management list, the legacy
/// `Admin\GearItemController::index` row.
#[derive(sqlx::FromRow)]
struct GearItemRow {
    id: i64,
    name: String,
    description: Option<String>,
    image_url: Option<String>,
    link: String,
    active: bool,
    priority: i32,
}

impl GearItemRow {
    fn json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "image_url": self.image_url,
            "link": self.link,
            "active": self.active,
            "priority": self.priority,
        })
    }
}

/// `GET /api/admin/gear-items` — the management list, the legacy
/// `Admin\GearItemController::index`.
pub async fn gear_items(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let rows: Result<Vec<GearItemRow>, sqlx::Error> = sqlx::query_as(
        "select id, name, description, image_url, link, active, priority
         from gear_items
         order by priority desc, id desc",
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            axum::Json(rows.iter().map(GearItemRow::json).collect::<Vec<_>>()).into_response()
        }
        Err(error) => super::api::database_error(error),
    }
}

/// The legacy StoreGearItemRequest rules, minus the file upload: like
/// the advertisements port, the rewrite takes an image URL instead of a
/// multipart upload (no public-disk storage yet), a documented
/// divergence — and so the image stays required on update too, where
/// the legacy let an omitted file keep the stored one.
#[derive(serde::Deserialize, Default)]
pub struct GearItemPayload {
    name: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
    link: Option<String>,
    priority: Option<i32>,
    active: Option<bool>,
}

fn validate_gear_item(payload: &GearItemPayload) -> Result<(), Box<Response>> {
    let name_ok = payload.name.as_deref().is_some_and(|name| !name.is_empty() && name.len() <= 255);
    if !name_ok {
        return Err(Box::new(validation_error("name", "The name field is required.")));
    }
    let image_ok = payload
        .image_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !image_ok {
        return Err(Box::new(validation_error("image_url", "The image url field is required.")));
    }
    // Unlike advertisements, the gear link is required.
    let Some(link) = payload.link.as_deref().filter(|link| !link.is_empty()) else {
        return Err(Box::new(validation_error("link", "The link field is required.")));
    };
    if !(link.starts_with("http://") || link.starts_with("https://")) {
        return Err(Box::new(validation_error("link", "The link field must be a valid URL.")));
    }
    if payload.priority.is_some_and(|priority| priority < 0) {
        return Err(Box::new(validation_error("priority", "The priority field must be at least 0.")));
    }
    Ok(())
}

/// `POST /api/admin/gear-items` — create.
pub async fn create_gear_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let payload: GearItemPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_gear_item(&payload) {
        return *response;
    }

    let result = sqlx::query(
        "insert into gear_items (name, description, image_url, link, priority, active)
         values ($1, $2, $3, $4, $5, $6)",
    )
    .bind(payload.name.as_deref())
    .bind(payload.description.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref())
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.active.unwrap_or(true))
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `PUT /api/admin/gear-items/{gear_item}` — update (the legacy used
/// POST for its multipart form; JSON here).
pub async fn update_gear_item(
    State(state): State<AppState>,
    axum::extract::Path(gear_item): axum::extract::Path<i64>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let payload: GearItemPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_gear_item(&payload) {
        return *response;
    }

    let result = sqlx::query(
        "update gear_items
         set name = $1, description = $2, image_url = $3, link = $4,
             priority = $5, active = $6, updated_at = now()
         where id = $7",
    )
    .bind(payload.name.as_deref())
    .bind(payload.description.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref())
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.active.unwrap_or(true))
    .bind(gear_item)
    .execute(&state.pool)
    .await;

    match result {
        Ok(updated) if updated.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}

/// `PATCH /api/admin/gear-items/{gear_item}/toggle`.
pub async fn toggle_gear_item(
    State(state): State<AppState>,
    axum::extract::Path(gear_item): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let result =
        sqlx::query("update gear_items set active = not active, updated_at = now() where id = $1")
            .bind(gear_item)
            .execute(&state.pool)
            .await;
    match result {
        Ok(updated) if updated.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}

/// `DELETE /api/admin/gear-items/{gear_item}`.
pub async fn destroy_gear_item(
    State(state): State<AppState>,
    axum::extract::Path(gear_item): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let result = sqlx::query("delete from gear_items where id = $1")
        .bind(gear_item)
        .execute(&state.pool)
        .await;
    match result {
        Ok(deleted) if deleted.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}

/// One advertisement of the management list.
#[derive(sqlx::FromRow)]
struct AdvertisementRow {
    id: i64,
    name: String,
    description: Option<String>,
    image_url: Option<String>,
    link: Option<String>,
    size: String,
    active: bool,
    priority: i32,
    starts_at: Option<String>,
    expires_at: Option<String>,
    expired: bool,
    scheduled: bool,
}

impl AdvertisementRow {
    /// The legacy derived status: inactive / expired / scheduled / live.
    fn json(&self) -> serde_json::Value {
        let status = if !self.active {
            "inactive"
        } else if self.expired {
            "expired"
        } else if self.scheduled {
            "scheduled"
        } else {
            "live"
        };
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "image_url": self.image_url,
            "link": self.link,
            "size": self.size,
            "active": self.active,
            "priority": self.priority,
            "starts_at": self.starts_at,
            "expires_at": self.expires_at,
            "status": status,
        })
    }
}

/// `GET /api/admin/advertisements` — the management list, the legacy
/// `Admin\AdvertisementController::index`.
pub async fn advertisements(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    let rows: Result<Vec<AdvertisementRow>, sqlx::Error> = sqlx::query_as(
        "select id, name, description, image_url, link, size, active, priority,
                starts_at::text as starts_at, expires_at::text as expires_at,
                (expires_at is not null and expires_at <= now()) as expired,
                (starts_at is not null and starts_at > now()) as scheduled
         from advertisements
         order by priority desc, id desc",
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            axum::Json(rows.iter().map(AdvertisementRow::json).collect::<Vec<_>>()).into_response()
        }
        Err(error) => super::api::database_error(error),
    }
}

/// The legacy StoreAdvertisementRequest rules, minus the file upload:
/// the rewrite takes an image URL (no public-disk storage yet), a
/// documented divergence.
#[derive(serde::Deserialize, Default)]
pub struct AdvertisementPayload {
    name: Option<String>,
    description: Option<String>,
    image_url: Option<String>,
    link: Option<String>,
    size: Option<String>,
    priority: Option<i32>,
    starts_at: Option<String>,
    expires_at: Option<String>,
    active: Option<bool>,
}

fn validate_advertisement(payload: &AdvertisementPayload) -> Result<(), Box<Response>> {
    let name_ok = payload.name.as_deref().is_some_and(|name| !name.is_empty() && name.len() <= 255);
    if !name_ok {
        return Err(Box::new(validation_error("name", "The name field is required.")));
    }
    let image_ok = payload
        .image_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !image_ok {
        return Err(Box::new(validation_error("image_url", "The image url field is required.")));
    }
    if let Some(link) = payload.link.as_deref()
        && !link.is_empty()
        && !(link.starts_with("http://") || link.starts_with("https://"))
    {
        return Err(Box::new(validation_error("link", "The link field must be a valid URL.")));
    }
    if let Some(size) = payload.size.as_deref()
        && size != "sidebar"
    {
        return Err(Box::new(validation_error("size", "The selected size is invalid.")));
    }
    if payload.priority.is_some_and(|priority| priority < 0) {
        return Err(Box::new(validation_error("priority", "The priority field must be at least 0.")));
    }
    Ok(())
}

fn validation_error(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(json!({
            "message": "The given data was invalid.",
            "errors": { field: [message] },
        })),
    )
        .into_response()
}

/// `POST /api/admin/advertisements` — create.
pub async fn create_advertisement(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let payload: AdvertisementPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_advertisement(&payload) {
        return *response;
    }

    let result = sqlx::query(
        "insert into advertisements
             (name, description, image_url, link, size, priority, starts_at, expires_at, active)
         values ($1, $2, $3, $4, $5, $6, $7::timestamptz, $8::timestamptz, $9)",
    )
    .bind(payload.name.as_deref())
    .bind(payload.description.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.size.as_deref().unwrap_or("sidebar"))
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.starts_at.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.expires_at.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.active.unwrap_or(true))
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => StatusCode::CREATED.into_response(),
        Err(error) => super::api::database_error(error),
    }
}

/// `PUT /api/admin/advertisements/{advertisement}` — update (the legacy
/// used POST for its multipart form; JSON here).
pub async fn update_advertisement(
    State(state): State<AppState>,
    axum::extract::Path(advertisement): axum::extract::Path<i64>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let payload: AdvertisementPayload = serde_json::from_slice(&body).unwrap_or_default();
    if let Err(response) = validate_advertisement(&payload) {
        return *response;
    }

    let result = sqlx::query(
        "update advertisements
         set name = $1, description = $2, image_url = $3, link = $4, size = $5,
             priority = $6, starts_at = $7::timestamptz, expires_at = $8::timestamptz,
             active = $9, updated_at = now()
         where id = $10",
    )
    .bind(payload.name.as_deref())
    .bind(payload.description.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.size.as_deref().unwrap_or("sidebar"))
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.starts_at.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.expires_at.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.active.unwrap_or(true))
    .bind(advertisement)
    .execute(&state.pool)
    .await;

    match result {
        Ok(updated) if updated.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}

/// `PATCH /api/admin/advertisements/{advertisement}/toggle`.
pub async fn toggle_advertisement(
    State(state): State<AppState>,
    axum::extract::Path(advertisement): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let result = sqlx::query(
        "update advertisements set active = not active, updated_at = now() where id = $1",
    )
    .bind(advertisement)
    .execute(&state.pool)
    .await;
    match result {
        Ok(updated) if updated.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}

/// `DELETE /api/admin/advertisements/{advertisement}`.
pub async fn destroy_advertisement(
    State(state): State<AppState>,
    axum::extract::Path(advertisement): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }
    let result = sqlx::query("delete from advertisements where id = $1")
        .bind(advertisement)
        .execute(&state.pool)
        .await;
    match result {
        Ok(deleted) if deleted.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => super::api::error(StatusCode::NOT_FOUND, "Not found."),
        Err(error) => super::api::database_error(error),
    }
}
