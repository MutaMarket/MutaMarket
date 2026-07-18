//! Background schedules replacing the legacy Laravel scheduler for the
//! ported ingestion: public contracts per region, auction bids, and the
//! PLEX market history. Enabled via `SCHEDULER_ENABLED=true` (heavy ESI
//! traffic; off by default for local development — use
//! `cargo run --bin contracts_sync` for one-shot runs).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use crate::contracts;
use crate::esi::EsiClient;
use crate::mutation::reference::ReferenceData;

/// Public contracts refresh cadence, like the legacy every-thirty-minutes.
const CONTRACTS_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Auction bid refresh cadence, like the legacy every-five-minutes.
const BIDS_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// PLEX market history refresh cadence, like the legacy daily schedule.
const MARKET_HISTORY_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// EVE's daily downtime window (UTC seconds of day, with margin) during
/// which ESI jobs pause, like the legacy notDuringDownTime.
const DOWNTIME_START: u64 = 10 * 3600 + 55 * 60;
const DOWNTIME_END: u64 = 11 * 3600 + 20 * 60;

pub fn enabled_by_env() -> bool {
    std::env::var("SCHEDULER_ENABLED").is_ok_and(|value| value == "true" || value == "1")
}

fn is_downtime() -> bool {
    let seconds_of_day = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|now| now.as_secs() % (24 * 3600))
        .unwrap_or(0);

    (DOWNTIME_START..=DOWNTIME_END).contains(&seconds_of_day)
}

/// Spawns the ingestion loops.
pub fn start(pool: PgPool, reference: Arc<ReferenceData>, esi: EsiClient) {
    {
        let pool = pool.clone();
        let esi = esi.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(MARKET_HISTORY_INTERVAL);
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
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(CONTRACTS_INTERVAL);
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
                    match contracts::sync_region(&pool, &reference, &esi, region_id).await {
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

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(BIDS_INTERVAL);
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
