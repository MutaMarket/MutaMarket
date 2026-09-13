//! Market-wide module statistics, ported from the legacy
//! `StatsService::getAllModulesStats` + `ModulesStats` DTO. Shown on the
//! home / all-modules browser header.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::PgPool;

pub use super::view::{ModulesStats, ScopedModuleStats};

/// How long the market-wide statistics are reused before one refresh
/// runs behind them.
///
/// Divergence from legacy, deliberate: legacy cached each count for an
/// hour, which would leave `added_last_hour_count` reading an hour
/// stale. Five minutes keeps the freshest counter honest and still
/// collapses a computation-per-page-view into one per window. It has to
/// be cached at all because the eleven count scans cost the better part
/// of a second of server time: on 2026-09-13 a distributed crawl of the
/// filter space at about 29 requests a second ran them concurrently
/// until the box had nothing left for anything else.
const STATS_TTL: Duration = Duration::from_secs(5 * 60);

/// The `bar` marker values on `mutated_attributes`, like the legacy roll
/// bar classifier: gold (best regular meta variant beaten), brown (worst
/// beaten), diamond (best recorded roll for the type).
const BAR_GOLD: i16 = 1;
const BAR_BROWN: i16 = -1;
const BAR_DIAMOND: i16 = 2;

/// The two cached readings of [`all_modules_stats`], one per `unlisted`
/// variant. Lives in the server state, so a router holds its own.
#[derive(Default)]
pub struct ModuleStatsCache {
    listed: StatsEntry,
    unlisted: StatsEntry,
}

#[derive(Default)]
struct StatsEntry {
    value: Mutex<Option<(Instant, ModulesStats)>>,
    /// Held while the counts are computed, so a cold cache under load
    /// runs one computation and hands the result to everyone waiting
    /// instead of starting one per request.
    computing: tokio::sync::Mutex<()>,
}

impl ModuleStatsCache {
    fn entry(&self, unlisted: bool) -> &StatsEntry {
        if unlisted {
            &self.unlisted
        } else {
            &self.listed
        }
    }
}

impl StatsEntry {
    /// The held reading and whether it has aged past [`STATS_TTL`].
    fn read(&self) -> Option<(bool, ModulesStats)> {
        self.value
            .lock()
            .expect("stats cache lock")
            .as_ref()
            .map(|(taken, stats)| (taken.elapsed() >= STATS_TTL, stats.clone()))
    }

    fn store(&self, stats: ModulesStats) {
        *self.value.lock().expect("stats cache lock") = Some((Instant::now(), stats));
    }
}

/// [`all_modules_stats`] through [`ModuleStatsCache`]: an aged reading is
/// answered immediately and refreshed behind the response, so only the
/// first request after a restart waits for the counts.
pub async fn cached_all_modules_stats(
    pool: &PgPool,
    cache: &Arc<ModuleStatsCache>,
    unlisted: bool,
) -> sqlx::Result<ModulesStats> {
    if let Some((aged, stats)) = cache.entry(unlisted).read() {
        if aged {
            let (pool, cache) = (pool.clone(), Arc::clone(cache));
            tokio::spawn(async move {
                // The refresh slot is the same lock the cold path takes,
                // so an already-running computation keeps this one out.
                let Ok(_guard) = cache.entry(unlisted).computing.try_lock() else {
                    return;
                };
                match all_modules_stats(&pool, unlisted).await {
                    Ok(stats) => cache.entry(unlisted).store(stats),
                    // The aged reading stays; the next request tries again.
                    Err(error) => tracing::warn!("refreshing the module stats failed: {error}"),
                }
            });
        }
        return Ok(stats);
    }

    let entry = cache.entry(unlisted);
    let _guard = entry.computing.lock().await;
    // Whoever held the lock has stored a reading by now.
    if let Some((_, stats)) = entry.read() {
        return Ok(stats);
    }
    let stats = all_modules_stats(pool, unlisted).await?;
    entry.store(stats.clone());
    Ok(stats)
}

/// Computes the market-wide statistics in one round trip.
/// [`cached_all_modules_stats`] is what the endpoint calls; this is the
/// uncached read behind it.
///
/// `unlisted` counts the bar totals across the whole archive instead of
/// only for-sale modules — a deliberate divergence from legacy, which
/// showed the visible-only counts even on its all-modules page (mostly
/// tiny numbers, since bars were only stamped on recently processed
/// modules there).
pub async fn all_modules_stats(pool: &PgPool, unlisted: bool) -> sqlx::Result<ModulesStats> {
    // A module counts toward a bar total when it is visible (has a live
    // contract, or `unlisted`) and carries at least one attribute with
    // that bar marker. Divergence from legacy `visible`: public assets
    // are not populated yet, so visibility is contract-only for now.
    let row = sqlx::query_as::<_, (
        i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64,
    )>(
        "select
            (select count(*) from modules),
            (select count(*) from modules where latest_contract_id is not null),
            (select count(*) from modules where created_at >= now() - interval '1 hour'),
            (select count(*) from modules where created_at >= now() - interval '1 day'),
            (select count(*) from modules where created_at >= now() - interval '7 days'),
            (select count(*) from contracts where abyssal_modules_count > 0),
            (select count(*) from contracts where type = 'item_exchange' and abyssal_modules_count > 0),
            (select count(*) from contracts where type = 'auction' and abyssal_modules_count > 0),
            (select count(distinct a.module_id) from mutated_attributes a
                join modules m on m.id = a.module_id
                where a.bar = $1 and (m.latest_contract_id is not null or $4)),
            (select count(distinct a.module_id) from mutated_attributes a
                join modules m on m.id = a.module_id
                where a.bar = $2 and (m.latest_contract_id is not null or $4)),
            (select count(distinct a.module_id) from mutated_attributes a
                join modules m on m.id = a.module_id
                where a.bar = $3 and (m.latest_contract_id is not null or $4))",
    )
    .bind(BAR_GOLD)
    .bind(BAR_BROWN)
    .bind(BAR_DIAMOND)
    .bind(unlisted)
    .fetch_one(pool)
    .await?;

    Ok(ModulesStats {
        total_count: row.0,
        listed_count: row.1,
        added_last_hour_count: row.2,
        added_last_day_count: row.3,
        added_last_week_count: row.4,
        contracts_count: row.5,
        item_exchanges_count: row.6,
        auctions_count: row.7,
        goldbars_count: row.8,
        brownbars_count: row.9,
        diamondbars_count: row.10,
    })
}

/// Totals over the module ids a `members(id)` CTE yields. `members` is the
/// complete WITH clause defining that CTE (plus whatever it builds on);
/// `binds` are its positional parameters in order. The bar counts join the
/// set against the partial bar index once instead of probing every
/// module's attributes three times, which took seconds for a station
/// holding 27k rolls.
pub async fn scoped_module_stats(
    pool: &PgPool,
    members: &str,
    binds: &[i64],
) -> sqlx::Result<ScopedModuleStats> {
    let sql = format!(
        "{members},
         bars as (select b.module_id, b.bar from mutated_attributes b
                  join members l on l.id = b.module_id where b.bar <> 0)
         select count(*) as total_count,
                coalesce(sum(m.estimated_value), 0)::float8 as total_value,
                coalesce(avg(m.estimated_value), 0)::float8 as average_value,
                (select count(distinct module_id) from bars where bar = {BAR_GOLD}) as goldbars_count,
                (select count(distinct module_id) from bars where bar = {BAR_BROWN}) as brownbars_count,
                (select count(distinct module_id) from bars where bar = {BAR_DIAMOND}) as diamondbars_count
         from modules m
         join members l on l.id = m.id"
    );
    let mut query = sqlx::query_as::<_, ScopedModuleStats>(sqlx::AssertSqlSafe(sql));
    for bind in binds {
        query = query.bind(*bind);
    }
    query.fetch_one(pool).await
}

/// Refreshes the /statistics materialized views (concurrently, so page
/// reads never block on the rebuild).
pub async fn refresh_statistics_views(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query("refresh materialized view concurrently statistics_overview")
        .execute(pool)
        .await?;
    sqlx::query("refresh materialized view concurrently statistics_creator_type_counts")
        .execute(pool)
        .await?;
    Ok(())
}
