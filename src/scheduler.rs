//! Background schedules replacing the legacy Laravel scheduler for the
//! ported ingestion: public contracts across every k-space region, auction
//! bids, the market history sweep, and the module value estimate refresh.
//! On by default like the legacy scheduler; set `SCHEDULER_ENABLED=false`
//! to opt out (e.g. to avoid the ESI traffic during
//! development — `cargo run --bin contracts_sync` and
//! `cargo run --bin estimate_values` cover one-shot runs).
//!
//! The jobs are a registry (`Scheduler`) rather than anonymous loops so
//! the admin API can observe and control them: every run is recorded in
//! `scheduler_runs`, jobs can be paused (persisted in `scheduler_jobs`)
//! and triggered manually — manual runs work even while the scheduled
//! loops are disabled.
//!
//! The legacy weekly estimator training schedule (`app:estimator:train`,
//! Mondays at downtime) maps to the interval-based `estimator-training`
//! job: weekly, without the downtime guard (legacy deliberately trained
//! during downtime while the ESI jobs pause). The admin page's run-now
//! covers ad-hoc training.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use crate::auth::sso::SsoClient;
use crate::esi::EsiClient;
use crate::estimator::{self, Estimator};
use crate::mutation::reference::ReferenceData;
use crate::{assets, contracts, structures};

/// Public contracts refresh cadence, like the legacy every-thirty-minutes.
const CONTRACTS_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Auction bid refresh cadence, like the legacy every-five-minutes.
const BIDS_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Market history sweep cadence, like the legacy daily
/// GetMarketHistoriesCommand schedule.
const MARKET_HISTORY_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Character name sync cadence, like the legacy every-minute schedule (only
/// characters without a fetch stamp are queried, so idle runs are free).
const CHARACTER_NAMES_INTERVAL: Duration = Duration::from_secs(60);

/// Character contracts fan-out cadence, like the legacy
/// every-five-minutes GetCharacterContractsCommand.
const CHARACTER_CONTRACTS_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Character assets fan-out cadence, like the legacy every-five-minutes
/// GetCharacterAssetsCommand.
const CHARACTER_ASSETS_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Stale asset-import sweeper cadence, like the legacy every-minute
/// FailStaleAssetImportsCommand (which runs without the downtime guard).
const STALE_ASSET_IMPORTS_INTERVAL: Duration = Duration::from_secs(60);

/// Donation ingestion cadence, like the legacy every-minute
/// GetWalletJournalCommand.
const WALLET_DONATIONS_INTERVAL: Duration = Duration::from_secs(60);

/// Premium expiry sweep cadence, like the legacy every-five-minutes
/// RemoveExpiredPremiumCommand (which runs without the downtime guard).
const PREMIUM_EXPIRY_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Patreon subscriber sync cadence, like the legacy every-ten-minutes
/// GetPatreonSubscribers.
const PATREON_SUBSCRIBERS_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// Public structure sweep cadence, like the legacy daily
/// GetPublicStructuresCommand.
const STRUCTURES_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Raffle draw cadence, like the legacy hourly DrawRaffleWinnerCommand
/// (whose winners expire at the top of the next hour).
const RAFFLE_DRAW_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// The hourly admin-scope check, the legacy `app:check-admin-scopes`
/// `->hourly()` schedule.
const ADMIN_SCOPES_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Alliance sweep cadence, like the legacy daily GetAlliancesCommand.
const ALLIANCES_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Estimate refresh cadence, like the legacy every-five-minutes
/// `app:estimate-values` schedule.
const ESTIMATES_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// How often the resident models are checked against `estimator_models`,
/// so forests trained by another process (the training bin) reach the
/// running API without a restart. A no-op when nothing changed.
const ESTIMATOR_MODELS_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Hourly like the legacy `app:search-training-modules` schedule.
const TRAINING_MODULES_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Unread-offer-message notifier cadence, like the legacy every-minute
/// `app:notify-users` schedule.
const OFFER_NOTIFICATIONS_INTERVAL: Duration = Duration::from_secs(60);

/// Outbox drain cadence; the legacy channels sent inline, our outbox
/// delivers within a minute of queueing.
const NOTIFICATION_DELIVERY_INTERVAL: Duration = Duration::from_secs(60);

/// EVE mail ingestion cadence, like the legacy every-thirty-seconds
/// `app:get-mails` schedule.
const EVE_MAILS_INTERVAL: Duration = Duration::from_secs(30);

/// The launcher-ad loop ticks hourly; the body only syncs in the
/// sale-drop hour or as a staleness catch-up.
const LAUNCHER_ADS_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// How often the /statistics materialized views are rebuilt. The page
/// tolerates staleness (it shows the refresh time); 15 minutes keeps
/// the activity counters honest without hammering the modules table.
const STATISTICS_VIEWS_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// New store sales usually drop with the post-downtime deploy; syncing
/// in the 12:00 UTC hour (one hour after downtime starts at 11:00)
/// catches them the day they appear.
const LAUNCHER_ADS_SYNC_HOUR_UTC: u64 = 12;

/// Whether an hourly launcher-ads tick should actually sync: in the
/// sale-drop hour, or when the last real sync is at least a day old
/// (covers restarts and the very first run).
fn launcher_sync_due(hour_utc: u64, last_sync_age_hours: Option<i64>) -> bool {
    hour_utc == LAUNCHER_ADS_SYNC_HOUR_UTC || last_sync_age_hours.is_none_or(|age| age >= 24)
}

/// Outbox rows drained per delivery run.
const NOTIFICATION_DELIVERY_BATCH: i64 = 50;

/// Weekly like the legacy Mondays-at-downtime `app:estimator:train`
/// schedule (interval-based here; the scheduler has no calendar).
const ESTIMATOR_TRAINING_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// OpenGraph card cache wipe, weekly like the legacy Mondays-at-downtime
/// `app:clear-og-cache` schedule (interval-based here as well).
const OG_CACHE_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Metric sampling for the admin dashboard charts, the legacy
/// SnapshotCommand's five-minute cadence.
const METRIC_SAMPLES_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Discord invite member-count refresh, matching the legacy
/// DiscordWidgetService 24-hour cache TTL (there a request-time cache;
/// here a job persisting into app_settings, see src/discord_invites.rs).
const DISCORD_MEMBER_COUNTS_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// EVE's daily downtime window (UTC seconds of day, with margin) during
/// which ESI jobs pause, like the legacy notDuringDownTime.
const DOWNTIME_START: u64 = 10 * 3600 + 55 * 60;
const DOWNTIME_END: u64 = 11 * 3600 + 20 * 60;

/// How often the request recorder's buffer is written to Postgres. A
/// restart loses at most this much of the counters; they are aggregates
/// for a monthly dashboard, so a minute of them is noise.
const ACTIVITY_FLUSH_INTERVAL: Duration = Duration::from_secs(60);

/// Recorded runs kept per job; older `scheduler_runs` rows are pruned.
pub const RUN_HISTORY_KEEP: i64 = 50;

pub fn enabled_by_env() -> bool {
    !std::env::var("SCHEDULER_ENABLED").is_ok_and(|value| value == "false" || value == "0")
}

pub fn is_downtime() -> bool {
    let seconds_of_day = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| now.as_secs() % (24 * 3600))
        .unwrap_or(0);

    (DOWNTIME_START..=DOWNTIME_END).contains(&seconds_of_day)
}

/// When a job's first tick after boot fires: its interval after the last
/// successful run, or right away when that is already due or it never
/// ran. Without this every loop ticked at startup, so the weekly
/// training and the big syncs re-ran on every deploy.
fn first_tick_delay(last_started_at: Option<i64>, interval: Duration, now: i64) -> Duration {
    match last_started_at {
        Some(started_at) => {
            let due_in = started_at + interval.as_secs() as i64 - now;
            Duration::from_secs(due_in.max(0) as u64)
        }
        None => Duration::ZERO,
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| now.as_secs() as i64)
        .unwrap_or(0)
}

/// Everything a job body may need; the same set `start()` always received.
#[derive(Clone)]
pub struct JobDeps {
    pub pool: PgPool,
    /// The in-memory request counters the flush job drains.
    pub activity: Arc<crate::activity::ActivityRecorder>,
    pub reference: Arc<ReferenceData>,
    pub esi: EsiClient,
    pub estimator: Estimator,
    pub sso: SsoClient,
}

/// What a finished run reports: the human summary line, the job's
/// headline metric, and optional named sub-metrics (same unit as each
/// other) the card charts as separate lines.
pub struct RunReport {
    pub summary: String,
    pub items: i64,
    pub metrics: Vec<(&'static str, i64)>,
}

impl RunReport {
    fn metrics_json(&self) -> Option<serde_json::Value> {
        if self.metrics.is_empty() {
            return None;
        }
        Some(serde_json::Value::Object(
            self.metrics
                .iter()
                .map(|(key, value)| ((*key).to_owned(), serde_json::Value::from(*value)))
                .collect(),
        ))
    }
}

type JobFuture<'a> = Pin<Box<dyn Future<Output = Result<RunReport, String>> + Send + 'a>>;
type JobBody = for<'a> fn(&'a JobDeps, &'a JobProgress) -> JobFuture<'a>;

/// The live progress line of an in-flight run, shown by the admin page.
/// Cleared automatically when the run finishes.
#[derive(Clone, Default)]
pub struct JobProgress(Arc<std::sync::Mutex<Option<String>>>);

impl JobProgress {
    pub fn set(&self, line: String) {
        *self.0.lock().expect("progress lock") = Some(line);
    }

    fn clear(&self) {
        *self.0.lock().expect("progress lock") = None;
    }

    fn current(&self) -> Option<String> {
        self.0.lock().expect("progress lock").clone()
    }
}

pub struct JobDefinition {
    /// Stable kebab-case identifier, also the URL segment of the admin API.
    pub name: &'static str,
    pub interval: Duration,
    /// Whether scheduled runs skip EVE's daily downtime window.
    pub downtime_guarded: bool,
    body: JobBody,
}

struct JobState {
    paused: AtomicBool,
    /// Serializes scheduled and manual runs of the same job.
    in_flight: Arc<tokio::sync::Mutex<()>>,
    /// Unix seconds of the next scheduled tick; 0 while unknown (loops
    /// not running).
    next_run_at: AtomicI64,
    progress: JobProgress,
}

/// The job registry: definitions, shared dependencies and live state.
pub struct Scheduler {
    /// Whether the scheduled loops run (`SCHEDULER_ENABLED`); manual runs
    /// work either way.
    pub enabled: bool,
    deps: JobDeps,
    jobs: Vec<(JobDefinition, JobState)>,
}

pub type SchedulerHandle = Arc<Scheduler>;

/// A job's observable state for the admin API.
pub struct JobSnapshot {
    pub name: &'static str,
    pub interval: Duration,
    pub downtime_guarded: bool,
    pub paused: bool,
    pub running: bool,
    /// Unix seconds of the next scheduled tick, when the loops run.
    pub next_run_at: Option<i64>,
    /// The in-flight run's live progress line, if it reported one.
    pub progress: Option<String>,
}

/// The outcome of a manual run request.
pub enum RunNowOutcome {
    Started,
    AlreadyRunning,
    UnknownJob,
}

impl Scheduler {
    /// Builds the registry, seeding the pause flags from `scheduler_jobs`.
    pub async fn load(deps: JobDeps, enabled: bool) -> sqlx::Result<SchedulerHandle> {
        let paused_jobs: Vec<String> =
            sqlx::query_scalar("select job from scheduler_jobs where paused")
                .fetch_all(&deps.pool)
                .await?;
        let last_started: Vec<(String, i64)> = sqlx::query_as(
            "select job, max(extract(epoch from started_at))::bigint
             from scheduler_runs where outcome = 'ok' group by job",
        )
        .fetch_all(&deps.pool)
        .await?;
        let now = unix_now();

        // Runs the previous process never finished would show as running
        // forever; mark them interrupted instead.
        sqlx::query(
            "update scheduler_runs
             set finished_at = now(), outcome = 'error',
                 error = 'interrupted (server restarted)'
             where finished_at is null",
        )
        .execute(&deps.pool)
        .await?;

        let jobs = definitions()
            .into_iter()
            .map(|definition| {
                let paused = paused_jobs.iter().any(|job| job == definition.name);
                let last_started_at = last_started
                    .iter()
                    .find(|(job, _)| job == definition.name)
                    .map(|(_, started_at)| *started_at);
                let delay = first_tick_delay(last_started_at, definition.interval, now);
                let state = JobState {
                    paused: AtomicBool::new(paused),
                    in_flight: Arc::new(tokio::sync::Mutex::new(())),
                    next_run_at: AtomicI64::new(now + delay.as_secs() as i64),
                    progress: JobProgress::default(),
                };
                (definition, state)
            })
            .collect();

        Ok(Arc::new(Self {
            enabled,
            deps,
            jobs,
        }))
    }

    /// A registry with no scheduled loops and default pause flags, for
    /// routers built in tests: jobs run only when triggered manually.
    pub fn disabled(deps: JobDeps) -> SchedulerHandle {
        let jobs = definitions()
            .into_iter()
            .map(|definition| {
                let state = JobState {
                    paused: AtomicBool::new(false),
                    in_flight: Arc::new(tokio::sync::Mutex::new(())),
                    next_run_at: AtomicI64::new(0),
                    progress: JobProgress::default(),
                };
                (definition, state)
            })
            .collect();

        Arc::new(Self {
            enabled: false,
            deps,
            jobs,
        })
    }

    fn job(&self, name: &str) -> Option<&(JobDefinition, JobState)> {
        self.jobs
            .iter()
            .find(|(definition, _)| definition.name == name)
    }

    /// The recorder the router counts into and `activity-flush` drains.
    pub fn activity(&self) -> Arc<crate::activity::ActivityRecorder> {
        self.deps.activity.clone()
    }

    pub fn snapshots(&self) -> Vec<JobSnapshot> {
        self.jobs
            .iter()
            .map(|(definition, state)| {
                let next_run_at = state.next_run_at.load(Ordering::Relaxed);
                JobSnapshot {
                    name: definition.name,
                    interval: definition.interval,
                    downtime_guarded: definition.downtime_guarded,
                    paused: state.paused.load(Ordering::Relaxed),
                    running: state.in_flight.try_lock().is_err(),
                    next_run_at: (next_run_at > 0).then_some(next_run_at),
                    progress: state.progress.current(),
                }
            })
            .collect()
    }

    /// Persists and applies a pause flag; `false` for an unknown job.
    pub async fn set_paused(&self, name: &str, paused: bool) -> sqlx::Result<bool> {
        let Some((definition, state)) = self.job(name) else {
            return Ok(false);
        };

        sqlx::query(
            "insert into scheduler_jobs (job, paused) values ($1, $2)
             on conflict (job) do update set paused = excluded.paused",
        )
        .bind(definition.name)
        .bind(paused)
        .execute(&self.deps.pool)
        .await?;

        state.paused.store(paused, Ordering::Relaxed);
        Ok(true)
    }

    /// Triggers a job outside its schedule; runs even while the scheduled
    /// loops are disabled.
    pub fn run_now(self: &Arc<Self>, name: &str) -> RunNowOutcome {
        let Some(index) = self
            .jobs
            .iter()
            .position(|(definition, _)| definition.name == name)
        else {
            return RunNowOutcome::UnknownJob;
        };

        let Ok(guard) = self.jobs[index].1.in_flight.clone().try_lock_owned() else {
            return RunNowOutcome::AlreadyRunning;
        };

        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_once(index, guard).await;
        });

        RunNowOutcome::Started
    }

    /// Executes one recorded run; the caller holds the in-flight guard.
    async fn run_once(&self, index: usize, _guard: tokio::sync::OwnedMutexGuard<()>) {
        let (definition, state) = &self.jobs[index];
        let pool = &self.deps.pool;
        state.progress.clear();

        let run_id: Result<i64, sqlx::Error> =
            sqlx::query_scalar("insert into scheduler_runs (job) values ($1) returning id")
                .bind(definition.name)
                .fetch_one(pool)
                .await;
        let run_id = match run_id {
            Ok(run_id) => run_id,
            Err(error) => {
                tracing::warn!(
                    "scheduler: recording {} run failed: {error}",
                    definition.name
                );
                return;
            }
        };

        // Any ESI failure inside the body is attributed to this job and
        // this recorded run.
        let outcome = crate::esi::failures::ESI_CALLER
            .scope(
                crate::esi::failures::EsiCaller::job(definition.name, Some(run_id)),
                (definition.body)(&self.deps, &state.progress),
            )
            .await;
        state.progress.clear();

        let (outcome_label, summary, error, items, metrics) = match &outcome {
            Ok(report) => {
                tracing::info!("scheduler: {}: {}", definition.name, report.summary);
                (
                    "success",
                    Some(report.summary.as_str()),
                    None,
                    Some(report.items),
                    report.metrics_json(),
                )
            }
            Err(error) => {
                tracing::warn!("scheduler: {} failed: {error}", definition.name);
                ("error", None, Some(error.as_str()), None, None)
            }
        };

        if let Err(db_error) = sqlx::query(
            "update scheduler_runs
             set finished_at = now(), outcome = $2, summary = $3, error = $4, items = $5,
                 metrics = $6
             where id = $1",
        )
        .bind(run_id)
        .bind(outcome_label)
        .bind(summary)
        .bind(error)
        .bind(items)
        .bind(metrics)
        .execute(pool)
        .await
        {
            tracing::warn!(
                "scheduler: finishing {} run failed: {db_error}",
                definition.name
            );
        }

        if let Err(db_error) = sqlx::query(
            "delete from scheduler_runs
             where job = $1 and id not in (
                 select id from scheduler_runs where job = $1 order by id desc limit $2
             )",
        )
        .bind(definition.name)
        .bind(RUN_HISTORY_KEEP)
        .execute(pool)
        .await
        {
            tracing::warn!(
                "scheduler: pruning {} runs failed: {db_error}",
                definition.name
            );
        }
    }
}

/// Spawns the scheduled loop of every job.
pub fn start(scheduler: SchedulerHandle) {
    for index in 0..scheduler.jobs.len() {
        let scheduler = scheduler.clone();
        tokio::spawn(async move {
            let (definition, state) = &scheduler.jobs[index];
            let first_tick = state.next_run_at.load(Ordering::Relaxed) - unix_now();
            let mut ticker = tokio::time::interval_at(
                tokio::time::Instant::now() + Duration::from_secs(first_tick.max(0) as u64),
                definition.interval,
            );
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                state.next_run_at.store(
                    unix_now() + definition.interval.as_secs() as i64,
                    Ordering::Relaxed,
                );

                if state.paused.load(Ordering::Relaxed) {
                    continue;
                }
                if definition.downtime_guarded && is_downtime() {
                    continue;
                }
                // A manual run in progress: skip this tick instead of
                // queueing behind it.
                let Ok(guard) = state.in_flight.clone().try_lock_owned() else {
                    continue;
                };

                scheduler.run_once(index, guard).await;
            }
        });
    }
}

fn definitions() -> Vec<JobDefinition> {
    vec![
        JobDefinition {
            name: "character-contracts",
            interval: CHARACTER_CONTRACTS_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(character_contracts(deps, progress)),
        },
        JobDefinition {
            name: "character-assets",
            interval: CHARACTER_ASSETS_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(character_assets(deps, progress)),
        },
        JobDefinition {
            name: "stale-asset-imports",
            interval: STALE_ASSET_IMPORTS_INTERVAL,
            // No downtime guard: the legacy sweeper runs through it.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(stale_asset_imports(deps)),
        },
        JobDefinition {
            name: "statistics-views",
            interval: STATISTICS_VIEWS_INTERVAL,
            // Pure Postgres work: runs straight through downtime.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(statistics_views(deps)),
        },
        JobDefinition {
            name: "wallet-donations",
            interval: WALLET_DONATIONS_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(wallet_donations(deps)),
        },
        JobDefinition {
            name: "admin-scopes",
            interval: ADMIN_SCOPES_INTERVAL,
            // Database reads plus at most one Discord webhook post.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(admin_scopes(deps)),
        },
        JobDefinition {
            name: "premium-expiry",
            interval: PREMIUM_EXPIRY_INTERVAL,
            // Pure database work (it only queues outbox rows).
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(premium_expiry(deps)),
        },
        JobDefinition {
            name: "raffle-draw",
            interval: RAFFLE_DRAW_INTERVAL,
            // Database only, like the legacy schedule without a guard.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(raffle_draw(deps)),
        },
        JobDefinition {
            name: "patreon-subscribers",
            interval: PATREON_SUBSCRIBERS_INTERVAL,
            // Not ESI, but the legacy schedule still guarded it.
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(patreon_subscribers(deps)),
        },
        JobDefinition {
            name: "structures",
            interval: STRUCTURES_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(structures_sweep(deps)),
        },
        JobDefinition {
            name: "alliances",
            interval: ALLIANCES_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(alliances_sweep(deps, progress)),
        },
        JobDefinition {
            name: "market-histories",
            interval: MARKET_HISTORY_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(market_histories(deps, progress)),
        },
        JobDefinition {
            name: "region-contracts",
            interval: CONTRACTS_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(region_contracts(deps, progress)),
        },
        JobDefinition {
            name: "character-names",
            interval: CHARACTER_NAMES_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(character_names(deps)),
        },
        JobDefinition {
            name: "auction-bids",
            interval: BIDS_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(auction_bids(deps)),
        },
        JobDefinition {
            name: "estimates",
            interval: ESTIMATES_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(estimates(deps)),
        },
        JobDefinition {
            name: "training-modules",
            interval: TRAINING_MODULES_INTERVAL,
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(training_modules(deps)),
        },
        JobDefinition {
            name: "activity-flush",
            interval: ACTIVITY_FLUSH_INTERVAL,
            // Pure database work; downtime is irrelevant.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(activity_flush(deps)),
        },
        JobDefinition {
            name: "metric-samples",
            interval: METRIC_SAMPLES_INTERVAL,
            // Pure database work; downtime is irrelevant.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(metric_samples(deps)),
        },
        JobDefinition {
            name: "offer-notifications",
            interval: OFFER_NOTIFICATIONS_INTERVAL,
            // Pure database work (it only queues outbox rows).
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(offer_notifications(deps)),
        },
        JobDefinition {
            name: "notification-delivery",
            interval: NOTIFICATION_DELIVERY_INTERVAL,
            // Real deliveries call ESI; skip the downtime window so
            // rows are not burned on guaranteed-failing sends.
            downtime_guarded: true,
            body: |deps, _progress| Box::pin(notification_delivery(deps)),
        },
        JobDefinition {
            name: "eve-mails",
            interval: EVE_MAILS_INTERVAL,
            downtime_guarded: true,
            body: |deps, progress| Box::pin(eve_mails(deps, progress)),
        },
        JobDefinition {
            name: "launcher-ads",
            interval: LAUNCHER_ADS_INTERVAL,
            // A public CDN feed, not ESI; downtime is irrelevant.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(launcher_ads(deps)),
        },
        JobDefinition {
            name: "discord-member-counts",
            interval: DISCORD_MEMBER_COUNTS_INTERVAL,
            // Discord's API, not ESI; downtime is irrelevant.
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(discord_member_counts(deps)),
        },
        JobDefinition {
            name: "estimator-training",
            interval: ESTIMATOR_TRAINING_INTERVAL,
            // Legacy trained AT downtime, so no guard.
            downtime_guarded: false,
            body: |deps, progress| Box::pin(estimator_training(deps, progress)),
        },
        JobDefinition {
            name: "estimator-models",
            interval: ESTIMATOR_MODELS_INTERVAL,
            downtime_guarded: false,
            body: |deps, _progress| Box::pin(estimator_models(deps)),
        },
        JobDefinition {
            name: "og-cache",
            interval: OG_CACHE_INTERVAL,
            // Deleting local files; downtime is irrelevant.
            downtime_guarded: false,
            body: |_deps, _progress| Box::pin(og_cache()),
        },
    ]
}

/// Legacy `ClearOGCacheCommand`: drop the rendered OpenGraph cards so a
/// card design change reaches links that were shared before it.
async fn og_cache() -> Result<RunReport, String> {
    crate::og::clear_cache()
        .map(|()| RunReport {
            metrics: Vec::new(),
            summary: "OG image cache cleared".to_owned(),
            items: 1,
        })
        .map_err(|error| error.to_string())
}

async fn character_contracts(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let characters = contracts::character::pending_contract_characters(&deps.pool)
        .await
        .map_err(|error| format!("contract character lookup failed: {error}"))?;

    let (mut total, mut items_synced, mut items_failed, mut failed_characters) = (0, 0, 0, 0);
    let character_count = characters.len();
    for (index, character_id) in characters.into_iter().enumerate() {
        progress.set(format!(
            "character {}/{character_count} (id {character_id}): {total} contracts so far",
            index + 1,
        ));
        match contracts::character::sync_character_contracts(
            &deps.pool,
            &deps.reference,
            &deps.esi,
            &deps.sso,
            character_id,
        )
        .await
        {
            Ok(stats) => {
                total += stats.total;
                items_synced += stats.items_synced;
                items_failed += stats.items_failed;
            }
            Err(error) => {
                failed_characters += 1;
                tracing::warn!("scheduler: contracts for character {character_id} failed: {error}");
            }
        }
    }

    Ok(RunReport {
        metrics: Vec::new(),
        summary: format!(
            "{character_count} characters: {total} contracts, {items_synced} item syncs, \
             {items_failed} item failures, {failed_characters} characters failed",
        ),
        items: total as i64,
    })
}

async fn character_assets(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let characters = assets::pending_asset_characters(&deps.pool)
        .await
        .map_err(|error| format!("asset character lookup failed: {error}"))?;

    let (mut kept, mut modules, mut imported, mut failed, mut failed_characters) = (0, 0, 0, 0, 0);
    let character_count = characters.len();
    for (index, character_id) in characters.into_iter().enumerate() {
        progress.set(format!(
            "character {}/{character_count} (id {character_id}): {imported} modules imported so far",
            index + 1,
        ));
        match assets::sync_character_assets(
            &deps.pool,
            &deps.reference,
            &deps.esi,
            &deps.sso,
            &deps.estimator,
            character_id,
        )
        .await
        {
            Ok(stats) => {
                kept += stats.assets;
                modules += stats.abyssal_modules;
                imported += stats.modules_imported;
                failed += stats.modules_failed;
            }
            Err(error) => {
                failed_characters += 1;
                tracing::warn!("scheduler: assets for character {character_id} failed: {error}");
            }
        }
    }

    Ok(RunReport {
        metrics: vec![
            ("found", modules as i64),
            ("imported", imported as i64),
            ("failed", failed as i64),
        ],
        summary: format!(
            "{character_count} characters: {kept} assets kept, {modules} modules \
             ({imported} imported, {failed} failed), {failed_characters} characters failed",
        ),
        items: imported as i64,
    })
}

async fn activity_flush(deps: &JobDeps) -> Result<RunReport, String> {
    let (routes, users) = crate::activity::flush::flush(&deps.pool, &deps.activity)
        .await
        .map_err(|error| error.to_string())?;

    Ok(RunReport {
        summary: format!("{routes} route buckets, {users} user days"),
        items: (routes + users) as i64,
        metrics: vec![("routes", routes as i64), ("users", users as i64)],
    })
}

async fn stale_asset_imports(deps: &JobDeps) -> Result<RunReport, String> {
    assets::fail_stale_asset_imports(&deps.pool)
        .await
        .map(|failed| RunReport {
            metrics: Vec::new(),
            summary: format!("{failed} stale asset imports failed"),
            items: failed as i64,
        })
        .map_err(|error| error.to_string())
}

async fn statistics_views(deps: &JobDeps) -> Result<RunReport, String> {
    crate::modules::stats::refresh_statistics_views(&deps.pool)
        .await
        .map(|()| RunReport {
            metrics: Vec::new(),
            summary: "statistics views refreshed".to_owned(),
            items: 1,
        })
        .map_err(|error| error.to_string())
}

async fn structures_sweep(deps: &JobDeps) -> Result<RunReport, String> {
    // The admin-authorized service character (env fallback), the legacy
    // services.eveonline.character_id.
    let character_id = crate::app_settings::service_character_id(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;
    let Some(character_id) = character_id else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no service character authorized".to_owned(),
            items: 0,
        });
    };

    structures::sync_public_structures(&deps.pool, &deps.esi, &deps.sso, character_id)
        .await
        .map(|stats| RunReport {
            metrics: Vec::new(),
            summary: format!(
                "{} public, {} resolved, {} unresolved, {} skipped",
                stats.total, stats.resolved, stats.unresolved, stats.skipped,
            ),
            items: stats.resolved as i64,
        })
        .map_err(|error| error.to_string())
}

/// The legacy `app:get-wallet-journal`: donation ingestion from the
/// service character's wallet.
async fn wallet_donations(deps: &JobDeps) -> Result<RunReport, String> {
    let character_id = crate::app_settings::service_character_id(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;
    let Some(character_id) = character_id else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no service character authorized".to_owned(),
            items: 0,
        });
    };

    crate::donations::sync_wallet_donations(&deps.pool, &deps.esi, &deps.sso, character_id)
        .await
        .map(|stats| RunReport {
            metrics: vec![
                ("donations", stats.donations as i64),
                ("new", stats.created as i64),
            ],
            summary: format!(
                "{} journal entries, {} donations, {} new",
                stats.entries, stats.donations, stats.created,
            ),
            items: stats.created as i64,
        })
        .map_err(|error| error.to_string())
}

/// The legacy hourly `app:check-admin-scopes`: the service character
/// must hold tokens covering every admin-login scope; missing scopes
/// alert the Discord webhook when one is configured.
async fn admin_scopes(deps: &JobDeps) -> Result<RunReport, String> {
    let character_id = crate::app_settings::service_character_id(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;
    let Some(character_id) = character_id else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no service character authorized".to_owned(),
            items: 0,
        });
    };

    let webhook = std::env::var(crate::admin_scopes::ALERT_WEBHOOK_ENV).ok();
    let outcome =
        crate::admin_scopes::check_admin_scopes(&deps.pool, character_id, webhook.as_deref())
            .await
            .map_err(|error| error.to_string())?;

    let summary = if outcome.missing.is_empty() {
        "all admin scopes present".to_owned()
    } else if outcome.alerted {
        format!(
            "{} admin scopes missing, Discord alerted",
            outcome.missing.len()
        )
    } else {
        format!(
            "{} admin scopes missing, no alert webhook configured",
            outcome.missing.len()
        )
    };
    Ok(RunReport {
        metrics: vec![("missing", outcome.missing.len() as i64)],
        summary,
        items: outcome.missing.len() as i64,
    })
}

/// The legacy hourly `app:draw-raffle-winner`.
async fn raffle_draw(deps: &JobDeps) -> Result<RunReport, String> {
    crate::raffles::draw_winners(&deps.pool)
        .await
        .map(|stats| RunReport {
            metrics: vec![("drawn", stats.drawn as i64), ("reset", stats.reset as i64)],
            summary: format!(
                "{} drawn, {} reset, {} without an eligible winner",
                stats.drawn, stats.reset, stats.unclaimed,
            ),
            items: stats.drawn as i64,
        })
        .map_err(|error| error.to_string())
}

/// The legacy `app:remove-expired-premium` sweep.
async fn premium_expiry(deps: &JobDeps) -> Result<RunReport, String> {
    crate::premium::remove_expired_premium(&deps.pool, crate::premium::PremiumCosts::from_env())
        .await
        .map(|expired| RunReport {
            metrics: Vec::new(),
            summary: format!("{expired} premium subscriptions expired"),
            items: expired,
        })
        .map_err(|error| error.to_string())
}

/// The legacy `app:get-patreon-subscribers` sync.
async fn patreon_subscribers(deps: &JobDeps) -> Result<RunReport, String> {
    let Some(client) = crate::patreon::PatreonCampaignClient::from_env() else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no Patreon access token configured".to_owned(),
            items: 0,
        });
    };

    let tiers = crate::patreon::premium_tiers_from_env();
    crate::patreon::sync_patreon_subscribers(&deps.pool, &client, &tiers)
        .await
        .map(|stats| RunReport {
            metrics: vec![("premium", stats.premium_members as i64)],
            summary: format!(
                "{} campaigns, {} members, {} premium",
                stats.campaigns, stats.members, stats.premium_members,
            ),
            items: stats.premium_members as i64,
        })
        .map_err(|error| error.to_string())
}

/// The legacy daily `app:get-alliances` sweep over every alliance ESI
/// lists.
async fn alliances_sweep(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let stats = crate::alliances::sync_alliances(&deps.pool, &deps.esi, |line| progress.set(line))
        .await
        .map_err(|error| error.to_string())?;

    Ok(RunReport {
        metrics: vec![
            ("upserted", stats.upserted as i64),
            ("failed", stats.failed as i64),
        ],
        summary: format!(
            "{} alliances: {} upserted, {} failed",
            stats.total, stats.upserted, stats.failed,
        ),
        items: stats.upserted as i64,
    })
}

/// The legacy daily `GetMarketHistoriesCommand` fan-out: every
/// mutaplasmid, published source type and support type (PLEX keeps its
/// full-history refresh).
async fn market_histories(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let stats = contracts::sync_market_histories(&deps.pool, &deps.esi, |line| progress.set(line))
        .await
        .map_err(|error| error.to_string())?;

    Ok(RunReport {
        metrics: vec![
            ("days", stats.days as i64),
            ("empty", stats.empty as i64),
            ("failed", stats.failed as i64),
        ],
        summary: format!(
            "{} types: {} days stored, {} without data, {} failed",
            stats.types, stats.days, stats.empty, stats.failed,
        ),
        items: stats.days as i64,
    })
}

async fn region_contracts(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let regions = contracts::kspace_region_ids(&deps.pool)
        .await
        .map_err(|error| format!("region lookup failed: {error}"))?;

    let (mut total, mut relevant, mut new, mut invalidated, mut failed_regions) = (0, 0, 0, 0, 0);
    let region_count = regions.len();
    for (index, region_id) in regions.into_iter().enumerate() {
        // The region's own phases ride on the line: a large region runs
        // its item fetches for minutes, which read as a frozen job with
        // one line per region.
        let (done_total, done_relevant) = (total, relevant);
        let report = move |phase: &str| {
            progress.set(format!(
                "region {}/{region_count} (id {region_id}): {phase}; \
                 {done_total} contracts, {done_relevant} relevant in earlier regions",
                index + 1,
            ));
        };
        report("starting");
        match contracts::sync_region(
            &deps.pool,
            &deps.reference,
            &deps.esi,
            &deps.estimator,
            region_id,
            &report,
        )
        .await
        {
            Ok(stats) => {
                total += stats.total;
                relevant += stats.relevant;
                new += stats.new;
                invalidated += stats.invalidated;
            }
            Err(error) => {
                failed_regions += 1;
                tracing::warn!("scheduler: contracts for region {region_id} failed: {error}");
            }
        }
    }

    Ok(RunReport {
        metrics: vec![("new", new as i64), ("invalidated", invalidated as i64)],
        summary: format!(
            "{region_count} regions: {total} contracts, {relevant} relevant, {new} new, \
             {invalidated} invalidated, {failed_regions} regions failed",
        ),
        items: new as i64,
    })
}

async fn character_names(deps: &JobDeps) -> Result<RunReport, String> {
    crate::characters::sync_character_names(&deps.pool, &deps.esi)
        .await
        .map(|named| RunReport {
            metrics: Vec::new(),
            summary: format!("{named} characters named"),
            items: named as i64,
        })
        .map_err(|error| error.to_string())
}

async fn auction_bids(deps: &JobDeps) -> Result<RunReport, String> {
    contracts::sync_auction_bids(&deps.pool, &deps.esi)
        .await
        .map(|auctions| RunReport {
            metrics: Vec::new(),
            summary: format!("{auctions} auctions refreshed"),
            items: auctions as i64,
        })
        .map_err(|error| error.to_string())
}

async fn training_modules(deps: &JobDeps) -> Result<RunReport, String> {
    contracts::sync_training_modules(&deps.pool)
        .await
        .map(|(deleted, upserted)| RunReport {
            metrics: Vec::new(),
            summary: format!("{upserted} training modules refreshed, {deleted} dropped"),
            items: upserted as i64,
        })
        .map_err(|error| error.to_string())
}

/// The legacy DiscordWidgetService fetch, moved to the scheduler: the
/// configured partner invites' member counts, persisted for the
/// sidebar payload.
async fn discord_member_counts(deps: &JobDeps) -> Result<RunReport, String> {
    let invite_urls: Vec<String> = crate::discord_invites::INVITES
        .iter()
        .filter_map(crate::discord_invites::invite_url)
        .collect();
    if invite_urls.is_empty() {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no Discord invites configured".to_owned(),
            items: 0,
        });
    }

    let stats = crate::discord_invites::refresh_member_counts(
        &deps.pool,
        crate::auth::linked::DEFAULT_DISCORD_API_BASE_URL,
        &invite_urls,
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(RunReport {
        metrics: vec![("unavailable", stats.unavailable as i64)],
        summary: format!(
            "{} invites: {} counts stored, {} unavailable",
            invite_urls.len(),
            stats.stored,
            stats.unavailable,
        ),
        items: stats.stored as i64,
    })
}

async fn metric_samples(deps: &JobDeps) -> Result<RunReport, String> {
    let context = crate::metrics::SampleContext {
        pool: &deps.pool,
        esi: &deps.esi,
    };
    crate::metrics::record_all(&context)
        .await
        .map(|(written, skipped)| RunReport {
            metrics: Vec::new(),
            summary: if skipped > 0 {
                format!("{written} metrics sampled, {skipped} unavailable")
            } else {
                format!("{written} metrics sampled")
            },
            items: written as i64,
        })
        .map_err(|error| error.to_string())
}

async fn estimator_models(deps: &JobDeps) -> Result<RunReport, String> {
    let load = deps
        .estimator
        .load_models(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(RunReport {
        metrics: Vec::new(),
        summary: format!(
            "{} models loaded, {} dropped, {} resident",
            load.loaded, load.dropped, load.resident,
        ),
        items: load.loaded as i64,
    })
}

async fn estimator_training(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let run = estimator::training::train_all(&deps.pool, |line| progress.set(line))
        .await
        .map_err(|error| error.to_string())?;

    // Freshly trained models replace the resident forests right away.
    deps.estimator
        .load_models(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;

    Ok(RunReport {
        metrics: Vec::new(),
        summary: format!(
            "{} types trained, {} skipped, {} module estimates cleared",
            run.trained, run.skipped, run.cleared,
        ),
        items: run.trained as i64,
    })
}

async fn estimates(deps: &JobDeps) -> Result<RunReport, String> {
    estimator::estimate_values(
        &deps.pool,
        &deps.estimator,
        estimator::estimate_count_from_env(),
        None,
    )
    .await
    .map(|run| RunReport {
        metrics: Vec::new(),
        summary: format!("{} of {} modules refreshed", run.updated, run.attempted),
        items: run.updated as i64,
    })
    .map_err(|error| error.to_string())
}

/// The legacy `app:notify-users`, delegating to the notifications
/// module's scan.
async fn offer_notifications(deps: &JobDeps) -> Result<RunReport, String> {
    crate::notifications::queue_unread_message_notifications(&deps.pool)
        .await
        .map(|notified| RunReport {
            metrics: Vec::new(),
            summary: format!("{notified} users notified about unread messages"),
            items: notified,
        })
        .map_err(|error| error.to_string())
}

/// Drains the notification outbox. `NOTIFY_DELIVERY=esi` sends real
/// EVE mails as the `NOTIFY_SENDER_CHARACTER_ID` character; any other
/// value (the dev default) marks rows `simulated` so the dev stack
/// never mails anyone yet everything stays inspectable.
async fn notification_delivery(deps: &JobDeps) -> Result<RunReport, String> {
    let esi_delivery = crate::notifications::esi_delivery_enabled();
    let pending = crate::notifications::pending(&deps.pool, NOTIFICATION_DELIVERY_BATCH)
        .await
        .map_err(|error| error.to_string())?;

    let mut sent = 0i64;
    let mut simulated = 0i64;
    let mut failed = 0i64;
    for row in &pending {
        if !esi_delivery {
            crate::notifications::mark_delivered(&deps.pool, row.id, "simulated", None)
                .await
                .map_err(|error| error.to_string())?;
            simulated += 1;
            continue;
        }

        let outcome = deliver_mail(deps, row).await;
        let (delivery, error) = match &outcome {
            Ok(()) => ("esi", None),
            Err(error) => ("esi", Some(error.as_str())),
        };
        crate::notifications::mark_delivered(&deps.pool, row.id, delivery, error)
            .await
            .map_err(|error| error.to_string())?;
        if outcome.is_ok() {
            sent += 1;
        } else {
            failed += 1;
        }
    }

    Ok(RunReport {
        metrics: vec![("sent", sent), ("simulated", simulated), ("failed", failed)],
        summary: format!("{sent} mailed, {simulated} simulated, {failed} failed"),
        items: sent + simulated + failed,
    })
}

/// One real EVE mail: the configured sender character's token (with the
/// mail scope) addresses the row's recipient character.
async fn deliver_mail(
    deps: &JobDeps,
    row: &crate::notifications::PendingNotification,
) -> Result<(), String> {
    let sender: i64 = std::env::var(crate::notifications::SENDER_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("{} unset", crate::notifications::SENDER_ENV))?;
    let recipient = row
        .recipient_character_id
        .ok_or("user has no character to notify")?;

    let token = crate::auth::tokens::valid_access_token(
        &deps.pool,
        &deps.sso,
        sender,
        crate::notifications::MAIL_SCOPE,
    )
    .await
    .map_err(|error| format!("sender token: {error:?}"))?
    .ok_or("sender has no token with the mail scope")?;

    deps.esi
        .send_mail(
            &token.access_token,
            sender,
            recipient,
            &row.subject,
            &row.body,
        )
        .await
        .map(|_| ())
        .map_err(|error| format!("esi mail: {error:?}"))
}

/// The legacy `app:get-mails` inbox scan for the service character (the
/// mail-based appraisal flow, `crate::mails`).
async fn eve_mails(deps: &JobDeps, progress: &JobProgress) -> Result<RunReport, String> {
    let character_id = crate::app_settings::service_character_id(&deps.pool)
        .await
        .map_err(|error| error.to_string())?;
    let Some(character_id) = character_id else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: no service character authorized".to_owned(),
            items: 0,
        });
    };

    let stats = crate::mails::sync_eve_mails(
        &deps.pool,
        &deps.reference,
        &deps.esi,
        &deps.sso,
        &deps.estimator,
        character_id,
        crate::notifications::esi_delivery_enabled(),
        |line| progress.set(line),
    )
    .await
    .map_err(|error| error.to_string())?;

    let Some(stats) = stats else {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: "skipped: service character has no mail-read token".to_owned(),
            items: 0,
        });
    };

    Ok(RunReport {
        metrics: vec![
            ("new", stats.new as i64),
            ("modules", stats.modules as i64),
            ("replies", stats.replies as i64),
        ],
        summary: format!(
            "{} mails seen: {} new, {} modules linked, {} replies queued, {} failed",
            stats.mails, stats.new, stats.modules, stats.replies, stats.failed,
        ),
        items: stats.new as i64,
    })
}

/// Mirrors the launcher's store campaigns into the ad rotation, timed
/// to the post-downtime sale drop.
async fn launcher_ads(deps: &JobDeps) -> Result<RunReport, String> {
    let hour_utc = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| (now.as_secs() % (24 * 3600)) / 3600)
        .unwrap_or(0);
    // Age of the last run that actually synced (skips record 0 items).
    let last_sync_age_hours: Option<i64> = sqlx::query_scalar(
        "select extract(epoch from now() - max(started_at))::bigint / 3600
         from scheduler_runs
         where job = 'launcher-ads' and outcome = 'ok' and items > 0",
    )
    .fetch_one(&deps.pool)
    .await
    .map_err(|error| error.to_string())?;

    if !launcher_sync_due(hour_utc, last_sync_age_hours) {
        return Ok(RunReport {
            metrics: Vec::new(),
            summary: format!("skipped: next sync at {LAUNCHER_ADS_SYNC_HOUR_UTC}:00 UTC"),
            items: 0,
        });
    }

    let feed_url = crate::advertisements::resolve_feed_url().await;
    let report = crate::advertisements::sync_launcher_store_ads(
        &deps.pool,
        &feed_url,
        std::path::Path::new(crate::advertisements::ADS_IMAGE_DIR),
    )
    .await?;
    Ok(RunReport {
        metrics: vec![
            ("added", report.upserted),
            ("removed", report.removed),
            ("downloaded", report.downloaded),
        ],
        summary: format!(
            "{} campaigns added, {} removed, {} creatives downloaded, generic store ad {}",
            report.upserted,
            report.removed,
            report.downloaded,
            if report.fallback { "shown" } else { "off" }
        ),
        // Always at least one item: a completed sync counts for the
        // staleness check even when the feed was unchanged.
        items: (report.upserted + report.removed).max(1),
    })
}

#[cfg(test)]
mod launcher_timing_tests {
    use super::launcher_sync_due;

    #[test]
    fn syncs_in_the_sale_hour_and_on_staleness() {
        assert!(
            launcher_sync_due(12, Some(3)),
            "the sale-drop hour always syncs"
        );
        assert!(
            !launcher_sync_due(13, Some(3)),
            "other hours skip fresh syncs"
        );
        assert!(
            launcher_sync_due(2, Some(24)),
            "a stale sync catches up anywhere"
        );
        assert!(launcher_sync_due(2, None), "the very first run syncs");
    }
}

#[cfg(test)]
mod tests {
    use super::first_tick_delay;
    use std::time::Duration;

    #[test]
    fn the_first_tick_waits_out_the_interval_since_the_last_run() {
        let week = Duration::from_secs(7 * 24 * 3600);
        assert_eq!(first_tick_delay(None, week, 1_000), Duration::ZERO);
        assert_eq!(
            first_tick_delay(Some(1_000), week, 1_000 + 3_600),
            week - Duration::from_secs(3_600)
        );
        assert_eq!(first_tick_delay(Some(0), week, 10_000_000), Duration::ZERO);
    }
}
