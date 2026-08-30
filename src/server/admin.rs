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
use super::support::validation_error;
use crate::auth::session::session_from_headers;
use crate::scheduler::RunNowOutcome;

/// Recorded runs returned per job in the status payload (the per-job
/// cards chart them).
pub const RUNS_SHOWN: i64 = 20;

/// One `scheduler_runs` row as selected for the status payload:
/// (job, started_at, finished_at, outcome, summary, error, items,
/// duration_seconds, metrics).
type RunRow = (
    String,
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
        Ok(None) => {
            return Err(super::api::error(
                StatusCode::UNAUTHORIZED,
                "Unauthenticated.",
            ));
        }
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

    let jobs = match jobs_section(&state).await {
        Ok(jobs) => jobs,
        Err(response) => return response,
    };
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

/// Every registered job with its live state and its newest recorded
/// runs. One windowed query serves all of them: the per-job `limit`
/// loop this replaced issued one round trip per job on every five-second
/// poll of an open console.
async fn jobs_section(state: &AppState) -> Result<serde_json::Value, Response> {
    let rows: Vec<RunRow> = sqlx::query_as(
        "select job, started_at::text, finished_at::text, outcome, summary, error, items,
                extract(epoch from finished_at - started_at)::bigint as duration_seconds,
                metrics
         from (select *, row_number() over (partition by job order by id desc) as run_rank
               from scheduler_runs) ranked
         where run_rank <= $1
         order by job, id desc",
    )
    .bind(RUNS_SHOWN)
    .fetch_all(&state.pool)
    .await
    .map_err(super::api::database_error)?;

    let mut runs_by_job: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for (job, started_at, finished_at, outcome, summary, error, items, duration_seconds, metrics) in
        rows
    {
        runs_by_job.entry(job).or_default().push(json!({
            "started_at": started_at,
            "finished_at": finished_at,
            "outcome": outcome,
            "summary": summary,
            "error": error,
            "items": items,
            "duration_seconds": duration_seconds,
            "metrics": metrics,
        }));
    }

    Ok(serde_json::Value::Array(
        state
            .scheduler
            .snapshots()
            .into_iter()
            .map(|snapshot| {
                json!({
                    "name": snapshot.name,
                    "interval_seconds": snapshot.interval.as_secs(),
                    "downtime_guarded": snapshot.downtime_guarded,
                    "paused": snapshot.paused,
                    "running": snapshot.running,
                    "next_run_at": snapshot.next_run_at,
                    "progress": snapshot.progress,
                    "last_runs": runs_by_job.remove(snapshot.name).unwrap_or_default(),
                })
            })
            .collect(),
    ))
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
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The selected window is invalid.",
        );
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
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The given data was invalid.",
        );
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
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The given data was invalid.",
        );
    };
    if payload
        .non_abyssal_modules_count
        .is_some_and(|count| count < 0)
    {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The given data was invalid.",
        );
    }
    if let Some(status) = &payload.status
        && !["outstanding", "completed", "failed", "unknown"].contains(&status.as_str())
    {
        return super::api::error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "The given data was invalid.",
        );
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

    let setting =
        match crate::app_settings::get(&state.pool, crate::app_settings::SERVICE_CHARACTER_KEY)
            .await
        {
            Ok(setting) => setting,
            Err(error) => return super::api::database_error(error),
        };
    let from_setting = setting
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok());
    let character_id = match from_setting {
        Some(id) => Some(id),
        None => std::env::var("EVE_STRUCTURES_CHARACTER_ID")
            .ok()
            .and_then(|value| value.parse().ok()),
    };
    let Some(character_id) = character_id else {
        return Json(
            json!({ "character": serde_json::Value::Null, "source": serde_json::Value::Null }),
        )
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

/// Longest gear item name, the legacy `max:255`.
const GEAR_ITEM_NAME_MAX: usize = 255;

fn validate_gear_item(payload: &GearItemPayload) -> Result<(), Box<Response>> {
    let name_ok = payload
        .name
        .as_deref()
        .is_some_and(|name| !name.is_empty() && name.len() <= GEAR_ITEM_NAME_MAX);
    if !name_ok {
        return Err(Box::new(validation_error(
            "name",
            "The name field is required.",
        )));
    }
    let image_ok = payload
        .image_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !image_ok {
        return Err(Box::new(validation_error(
            "image_url",
            "The image url field is required.",
        )));
    }
    // Unlike advertisements, the gear link is required.
    let Some(link) = payload.link.as_deref().filter(|link| !link.is_empty()) else {
        return Err(Box::new(validation_error(
            "link",
            "The link field is required.",
        )));
    };
    if !(link.starts_with("http://") || link.starts_with("https://")) {
        return Err(Box::new(validation_error(
            "link",
            "The link field must be a valid URL.",
        )));
    }
    if payload.priority.is_some_and(|priority| priority < 0) {
        return Err(Box::new(validation_error(
            "priority",
            "The priority field must be at least 0.",
        )));
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
    .bind(
        payload
            .description
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
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

    // The legacy update applied only $request->safe() fields, so an
    // omitted priority or active keeps the stored value; coalesce
    // mirrors that (JSON null and an absent key both preserve, since
    // the legacy form never sent null).
    let result = sqlx::query(
        "update gear_items
         set name = $1, description = $2, image_url = $3, link = $4,
             priority = coalesce($5, priority), active = coalesce($6, active),
             updated_at = now()
         where id = $7",
    )
    .bind(payload.name.as_deref())
    .bind(
        payload
            .description
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref())
    .bind(payload.priority)
    .bind(payload.active)
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

/// One raffle item of the admin management list, the legacy
/// `Admin\RaffleController::index` row: the winner column shows the
/// notify character (name and portrait) and falls back to the account
/// name.
#[derive(sqlx::FromRow)]
struct RaffleItemRow {
    id: i64,
    name: Option<String>,
    description: Option<String>,
    code: String,
    status: i32,
    type_id: Option<i64>,
    type_name: Option<String>,
    winner_id: Option<i64>,
    winner_name: Option<String>,
    winner_character_id: Option<i64>,
    expires_at: Option<String>,
    created_at: Option<String>,
}

impl RaffleItemRow {
    fn json(&self) -> serde_json::Value {
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "code": self.code,
            "status": self.status,
            "type": self.type_id.map(|id| json!({ "id": id, "name": self.type_name })),
            "winner": self.winner_id.map(|id| json!({
                "id": id,
                "name": self.winner_name,
                "character_id": self.winner_character_id,
            })),
            "expires_at": self.expires_at,
            "created_at": self.created_at,
        })
    }
}

/// Longest type list the create form's search returns, the legacy
/// `limit(50)`.
const RAFFLE_TYPE_SEARCH_LIMIT: i64 = 50;

#[derive(serde::Deserialize, Default)]
pub struct RaffleIndexParams {
    type_search: Option<String>,
}

/// `GET /api/admin/raffles` — the management page data, the legacy
/// `Admin\RaffleController::index` Inertia props (snake_case here like
/// the rest of the API): every item ordered active, pending, then the
/// finished ones by recency, plus the create form's type search.
pub async fn raffles(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<RaffleIndexParams>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_admin(&state, &headers).await {
        return response;
    }

    // The legacy CASE ranks: active 1, pending 2, claimed/paid-out 3.
    let order = format!(
        "case r.status
             when {active} then 1
             when {pending} then 2
             when {claimed} then 3
             when {paid_out} then 3
             else 4
         end",
        active = crate::raffles::STATUS_ACTIVE,
        pending = crate::raffles::STATUS_PENDING,
        claimed = crate::raffles::STATUS_CLAIMED,
        paid_out = crate::raffles::STATUS_PAID_OUT,
    );
    let rows: Result<Vec<RaffleItemRow>, sqlx::Error> = sqlx::query_as(&format!(
        "select r.id, r.name, r.description, r.code, r.status,
                t.id as type_id, t.name as type_name,
                w.id as winner_id,
                coalesce(nchar.name, w.name) as winner_name,
                nc.character_id as winner_character_id,
                to_char(r.expires_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"+00:00\"')
                    as expires_at,
                to_char(r.created_at at time zone 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"+00:00\"')
                    as created_at
         from raffle_items r
         left join types t on t.id = r.type_id
         left join users w on w.id = r.winner_id
         left join notify_characters nc on nc.user_id = w.id
         left join characters nchar on nchar.id = nc.character_id
         order by {order}, r.updated_at desc",
    ))
    .fetch_all(&state.pool)
    .await;
    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return super::api::database_error(error),
    };

    // The legacy trimmed string() helper: absent becomes empty, and
    // only a non-empty search queries types (ilike for the MySQL
    // case-insensitive like).
    let type_search = params
        .type_search
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let types: Vec<(i64, String)> = if type_search.is_empty() {
        Vec::new()
    } else {
        match sqlx::query_as("select id, name from types where name ilike $1 limit $2")
            .bind(format!("%{type_search}%"))
            .bind(RAFFLE_TYPE_SEARCH_LIMIT)
            .fetch_all(&state.pool)
            .await
        {
            Ok(types) => types,
            Err(error) => return super::api::database_error(error),
        }
    };

    Json(json!({
        "raffle_items": rows.iter().map(RaffleItemRow::json).collect::<Vec<_>>(),
        "types": types
            .iter()
            .map(|(id, name)| json!({ "id": id, "name": name }))
            .collect::<Vec<_>>(),
        "type_search": type_search,
    }))
    .into_response()
}

/// Longest raffle item name and description, the legacy `max:255`.
const RAFFLE_TEXT_MAX: usize = 255;

#[derive(serde::Deserialize, Default)]
pub struct RaffleStorePayload {
    name: Option<String>,
    description: Option<String>,
    type_id: Option<i64>,
    codes: Option<String>,
}

/// `POST /raffles` — the legacy `Admin\RaffleController::store` on its
/// legacy path: one item per code line, pending, with the type icon
/// stored when a type is attached. Sits behind the legacy auth-then-
/// admin middleware pair: guests bounce to the login page, non-admins
/// get the AdminMiddleware 403 text.
pub async fn create_raffle_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match crate::auth::session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return axum::response::Redirect::to("/login").into_response(),
        Err(error) => return super::api::database_error(error),
    };
    let is_admin: bool = match sqlx::query_scalar("select is_admin from users where id = $1")
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(is_admin) => is_admin.unwrap_or(false),
        Err(error) => return super::api::database_error(error),
    };
    if !is_admin {
        return super::api::error(StatusCode::FORBIDDEN, "Unauthorized access.");
    }

    let payload: RaffleStorePayload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(name) = payload.name.as_deref().filter(|name| !name.is_empty()) else {
        return validation_error("name", "The name field is required.");
    };
    if name.len() > RAFFLE_TEXT_MAX {
        return validation_error(
            "name",
            "The name field must not be greater than 255 characters.",
        );
    }
    if payload
        .description
        .as_deref()
        .is_some_and(|text| text.len() > RAFFLE_TEXT_MAX)
    {
        return validation_error(
            "description",
            "The description field must not be greater than 255 characters.",
        );
    }
    if let Some(type_id) = payload.type_id {
        let exists: bool =
            match sqlx::query_scalar("select exists (select 1 from types where id = $1)")
                .bind(type_id)
                .fetch_one(&state.pool)
                .await
            {
                Ok(exists) => exists,
                Err(error) => return super::api::database_error(error),
            };
        if !exists {
            return validation_error("type_id", "The selected type id is invalid.");
        }
    }
    let Some(codes) = payload.codes.as_deref().filter(|codes| !codes.is_empty()) else {
        return validation_error("codes", "The codes field is required.");
    };

    // The legacy array_filter(array_map('trim', explode("\n", ...))):
    // besides blank lines, PHP truthiness also drops a literal "0" code.
    let codes: Vec<&str> = codes
        .split('\n')
        .map(str::trim)
        .filter(|code| !code.is_empty() && *code != "0")
        .collect();

    // One insert per code without a transaction, like the legacy loop of
    // creates (a duplicate code mid-batch keeps the earlier rows).
    for code in &codes {
        let result = sqlx::query(
            "insert into raffle_items (name, description, type_id, icon_url, code, status)
             values ($1, $2, $3, $4, $5, $6)",
        )
        .bind(name)
        // The legacy ConvertEmptyStringsToNull middleware nulls an
        // empty description before it is stored.
        .bind(
            payload
                .description
                .as_deref()
                .filter(|text| !text.is_empty()),
        )
        .bind(payload.type_id)
        .bind(payload.type_id.map(crate::raffles::icon_url))
        .bind(code)
        .bind(crate::raffles::STATUS_PENDING)
        .execute(&state.pool)
        .await;
        if let Err(error) = result {
            return super::api::database_error(error);
        }
    }

    // The legacy back() with the "Raffle items created!" flash; the
    // frontend shows the toast.
    super::support::back(&headers).into_response()
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
    let name_ok = payload
        .name
        .as_deref()
        .is_some_and(|name| !name.is_empty() && name.len() <= 255);
    if !name_ok {
        return Err(Box::new(validation_error(
            "name",
            "The name field is required.",
        )));
    }
    let image_ok = payload
        .image_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://") || url.starts_with("https://"));
    if !image_ok {
        return Err(Box::new(validation_error(
            "image_url",
            "The image url field is required.",
        )));
    }
    if let Some(link) = payload.link.as_deref()
        && !link.is_empty()
        && !(link.starts_with("http://") || link.starts_with("https://"))
    {
        return Err(Box::new(validation_error(
            "link",
            "The link field must be a valid URL.",
        )));
    }
    if let Some(size) = payload.size.as_deref()
        && size != "sidebar"
    {
        return Err(Box::new(validation_error(
            "size",
            "The selected size is invalid.",
        )));
    }
    if payload.priority.is_some_and(|priority| priority < 0) {
        return Err(Box::new(validation_error(
            "priority",
            "The priority field must be at least 0.",
        )));
    }
    Ok(())
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
    .bind(
        payload
            .description
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.size.as_deref().unwrap_or("sidebar"))
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.starts_at.as_deref().filter(|text| !text.is_empty()))
    .bind(
        payload
            .expires_at
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
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
    .bind(
        payload
            .description
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
    .bind(payload.image_url.as_deref())
    .bind(payload.link.as_deref().filter(|text| !text.is_empty()))
    .bind(payload.size.as_deref().unwrap_or("sidebar"))
    .bind(payload.priority.unwrap_or(0))
    .bind(payload.starts_at.as_deref().filter(|text| !text.is_empty()))
    .bind(
        payload
            .expires_at
            .as_deref()
            .filter(|text| !text.is_empty()),
    )
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
