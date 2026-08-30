//! One-shot public contract ingestion for development and operations:
//! refreshes the PLEX market history, syncs one region's contracts (The
//! Forge by default, or the region id given as argument, or `all` for
//! every k-space region), and updates auction bids.
//!
//! Usage: `cargo run --bin contracts_sync [region_id|all]`

use std::sync::Arc;

use mutamarket::contracts;
use mutamarket::db;
use mutamarket::esi::EsiClient;
use mutamarket::estimator::Estimator;
use mutamarket::mutation::reference::ReferenceData;
use mutamarket::sde::client::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let pool = db::connect().await?;
    db::migrate(&pool).await?;

    let reference = Arc::new(ReferenceData::from_tables(
        db::reference::load_reference(&pool).await?,
    ));
    let esi = EsiClient::from_env().with_failure_log(pool.clone());
    let estimator = Estimator::new();

    match contracts::sync_plex_market_history(&pool, &esi).await {
        Ok(days) => println!("PLEX market history: {days} days"),
        Err(error) => eprintln!("PLEX market history failed: {error}"),
    }

    let argument = std::env::args().nth(1);
    let regions = match argument.as_deref() {
        Some("all") => contracts::kspace_region_ids(&pool).await?,
        Some(region) => vec![region.parse()?],
        None => vec![contracts::FORGE_REGION_ID],
    };

    for region_id in regions {
        match contracts::sync_region(&pool, &reference, &esi, &estimator, region_id).await {
            Ok(stats) => println!(
                "region {region_id}: {} total, {} relevant, {} new, {} invalidated",
                stats.total, stats.relevant, stats.new, stats.invalidated,
            ),
            Err(error) => eprintln!("region {region_id} failed: {error}"),
        }
    }

    let updated = contracts::sync_auction_bids(&pool, &esi).await?;
    println!("auction bids updated: {updated}");

    Ok(())
}
