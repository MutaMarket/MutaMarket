//! Market-wide module statistics, ported from the legacy
//! `StatsService::getAllModulesStats` + `ModulesStats` DTO. Shown on the
//! home / all-modules browser header.

use sqlx::PgPool;

pub use super::view::ModulesStats;

/// The `bar` marker values on `mutated_attributes`, like the legacy roll
/// bar classifier: gold (best regular meta variant beaten), brown (worst
/// beaten), diamond (best recorded roll for the type).
const BAR_GOLD: i16 = 1;
const BAR_BROWN: i16 = -1;
const BAR_DIAMOND: i16 = 2;

/// Computes the market-wide statistics in one round trip. Legacy caches
/// each count for an hour; we compute them together (cheap enough) and can
/// add a cache layer if it ever shows up in profiling.
pub async fn all_modules_stats(pool: &PgPool) -> sqlx::Result<ModulesStats> {
    // A module counts toward a bar total when it is visible (has a live
    // contract) and carries at least one attribute with that bar marker.
    // Divergence from legacy `visible`: public assets are not populated
    // yet, so visibility is contract-only for now.
    let row = sqlx::query_as::<_, (
        i64, i64, i64, i64, i64, i64, i64, i64, i64, i64,
    )>(
        "select
            (select count(*) from modules),
            (select count(*) from modules where created_at >= now() - interval '1 hour'),
            (select count(*) from modules where created_at >= now() - interval '1 day'),
            (select count(*) from modules where created_at >= now() - interval '7 days'),
            (select count(*) from contracts where abyssal_modules_count > 0),
            (select count(*) from contracts where type = 'item_exchange' and abyssal_modules_count > 0),
            (select count(*) from contracts where type = 'auction' and abyssal_modules_count > 0),
            (select count(*) from modules m where m.latest_contract_id is not null
                and exists (select 1 from mutated_attributes a where a.module_id = m.id and a.bar = $1)),
            (select count(*) from modules m where m.latest_contract_id is not null
                and exists (select 1 from mutated_attributes a where a.module_id = m.id and a.bar = $2)),
            (select count(*) from modules m where m.latest_contract_id is not null
                and exists (select 1 from mutated_attributes a where a.module_id = m.id and a.bar = $3))",
    )
    .bind(BAR_GOLD)
    .bind(BAR_BROWN)
    .bind(BAR_DIAMOND)
    .fetch_one(pool)
    .await?;

    Ok(ModulesStats {
        total_count: row.0,
        added_last_hour_count: row.1,
        added_last_day_count: row.2,
        added_last_week_count: row.3,
        contracts_count: row.4,
        item_exchanges_count: row.5,
        auctions_count: row.6,
        goldbars_count: row.7,
        brownbars_count: row.8,
        diamondbars_count: row.9,
    })
}
