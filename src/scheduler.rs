//! Background schedules replacing the legacy Laravel scheduler for the
//! ported ingestion: public contracts across every k-space region, auction
//! bids, the PLEX market history, and the module value estimate refresh.
//! On by default like the legacy scheduler; set `SCHEDULER_ENABLED=false`
//! to opt out (e.g. to avoid the ESI and AI-server traffic during
//! development — `cargo run --bin contracts_sync` and
//! `cargo run --bin estimate_values` cover one-shot runs).
//!
//! The legacy weekly estimator training schedule (`app:estimator:train`,
//! Mondays at downtime) is not mirrored: training is not ported yet, see
//! `crate::estimator`.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use crate::auth::sso::SsoClient;
use crate::{assets, contracts, structures};
use crate::esi::EsiClient;
use crate::estimator::{self, EstimatorClient};
use crate::mutation::reference::ReferenceData;

/// Public contracts refresh cadence, like the legacy every-thirty-minutes.
const CONTRACTS_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Auction bid refresh cadence, like the legacy every-five-minutes.
const BIDS_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// PLEX market history refresh cadence, like the legacy daily schedule.
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

/// Public structure sweep cadence, like the legacy daily
/// GetPublicStructuresCommand.
const STRUCTURES_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Estimate refresh cadence, like the legacy every-five-minutes
/// `app:estimate-values` schedule.
const ESTIMATES_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// EVE's daily downtime window (UTC seconds of day, with margin) during
/// which ESI jobs pause, like the legacy notDuringDownTime.
const DOWNTIME_START: u64 = 10 * 3600 + 55 * 60;
const DOWNTIME_END: u64 = 11 * 3600 + 20 * 60;

pub fn enabled_by_env() -> bool {
    !std::env::var("SCHEDULER_ENABLED").is_ok_and(|value| value == "false" || value == "0")
}

fn is_downtime() -> bool {
    let seconds_of_day = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| now.as_secs() % (24 * 3600))
        .unwrap_or(0);

    (DOWNTIME_START..=DOWNTIME_END).contains(&seconds_of_day)
}

/// Spawns the ingestion loops.
pub fn start(
    pool: PgPool,
    reference: Arc<ReferenceData>,
    esi: EsiClient,
    estimator: EstimatorClient,
    sso: SsoClient,
) {
    {
        let pool = pool.clone();
        let reference = reference.clone();
        let esi = esi.clone();
        let sso = sso.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(CHARACTER_CONTRACTS_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }

                let characters = match contracts::character::pending_contract_characters(&pool).await
                {
                    Ok(characters) => characters,
                    Err(error) => {
                        eprintln!("scheduler: contract character lookup failed: {error}");
                        continue;
                    }
                };

                for character_id in characters {
                    match contracts::character::sync_character_contracts(
                        &pool, &reference, &esi, &sso, character_id,
                    )
                    .await
                    {
                        Ok(stats) => println!(
                            "scheduler: character {character_id} contracts: {} total, {} item syncs, {} failed",
                            stats.total, stats.items_synced, stats.items_failed,
                        ),
                        Err(error) => eprintln!(
                            "scheduler: contracts for character {character_id} failed: {error}",
                        ),
                    }
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let reference = reference.clone();
        let esi = esi.clone();
        let sso = sso.clone();
        let estimator = estimator.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(CHARACTER_ASSETS_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }

                let characters = match assets::pending_asset_characters(&pool).await {
                    Ok(characters) => characters,
                    Err(error) => {
                        eprintln!("scheduler: asset character lookup failed: {error}");
                        continue;
                    }
                };

                for character_id in characters {
                    match assets::sync_character_assets(&pool, &reference, &esi, &sso, &estimator, character_id)
                        .await
                    {
                        Ok(stats) => println!(
                            "scheduler: character {character_id} assets: {} kept, {} modules ({} imported, {} failed)",
                            stats.assets, stats.abyssal_modules, stats.modules_imported,
                            stats.modules_failed,
                        ),
                        Err(error) => eprintln!(
                            "scheduler: assets for character {character_id} failed: {error}",
                        ),
                    }
                }
            }
        });
    }

    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(STALE_ASSET_IMPORTS_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                // No downtime guard: the legacy sweeper runs through it.
                ticker.tick().await;
                match assets::fail_stale_asset_imports(&pool).await {
                    Ok(0) => {}
                    Ok(failed) => println!("scheduler: {failed} stale asset imports failed"),
                    Err(error) => eprintln!("scheduler: stale asset import sweep failed: {error}"),
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let esi = esi.clone();
        let sso = sso.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(STRUCTURES_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }

                // The sweep needs the configured resolver character (the
                // legacy services.eveonline.character_id).
                let Some(character_id) = structures::sweep_character_from_env() else {
                    println!(
                        "scheduler: EVE_STRUCTURES_CHARACTER_ID unset, skipping structure sweep",
                    );
                    continue;
                };

                match structures::sync_public_structures(&pool, &esi, &sso, character_id).await {
                    Ok(stats) => println!(
                        "scheduler: structures: {} public, {} resolved, {} unresolved, {} skipped",
                        stats.total, stats.resolved, stats.unresolved, stats.skipped,
                    ),
                    Err(error) => eprintln!("scheduler: structure sweep failed: {error}"),
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let esi = esi.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(MARKET_HISTORY_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }
                match contracts::sync_plex_market_history(&pool, &esi).await {
                    Ok(days) => println!("scheduler: PLEX market history refreshed ({days} days)"),
                    Err(error) => eprintln!("scheduler: PLEX market history failed: {error}"),
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let reference = reference.clone();
        let esi = esi.clone();
        let estimator = estimator.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(CONTRACTS_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }

                let regions = match contracts::kspace_region_ids(&pool).await {
                    Ok(regions) => regions,
                    Err(error) => {
                        eprintln!("scheduler: region lookup failed: {error}");
                        continue;
                    }
                };

                for region_id in regions {
                    match contracts::sync_region(&pool, &reference, &esi, &estimator, region_id).await {
                        Ok(stats) => println!(
                            "scheduler: region {region_id} contracts: {} total, {} relevant, {} new, {} invalidated",
                            stats.total, stats.relevant, stats.new, stats.invalidated,
                        ),
                        Err(error) => {
                            eprintln!("scheduler: contracts for region {region_id} failed: {error}");
                        }
                    }
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let esi = esi.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(CHARACTER_NAMES_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }
                match crate::characters::sync_character_names(&pool, &esi).await {
                    Ok(0) => {}
                    Ok(named) => println!("scheduler: named {named} characters"),
                    Err(error) => eprintln!("scheduler: character names failed: {error}"),
                }
            }
        });
    }

    {
        let pool = pool.clone();
        let esi = esi.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(BIDS_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                if is_downtime() {
                    continue;
                }
                if let Err(error) = contracts::sync_auction_bids(&pool, &esi).await {
                    eprintln!("scheduler: auction bids failed: {error}");
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(ESTIMATES_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if is_downtime() {
                continue;
            }
            match estimator::estimate_values(
                &pool,
                &estimator,
                estimator::estimate_count_from_env(),
                None,
            )
            .await
            {
                Ok(run) => println!(
                    "scheduler: estimates refreshed ({} of {} modules)",
                    run.updated, run.attempted,
                ),
                Err(error) => eprintln!("scheduler: estimate pass failed: {error}"),
            }
        }
    });
}
