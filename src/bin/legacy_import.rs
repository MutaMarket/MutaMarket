//! One-time import of the legacy Laravel/MySQL production database into
//! Postgres.
//!
//! Usage:
//!   LEGACY_IMPORT_CONFIRM=1 cargo run --bin legacy_import
//!
//! `LEGACY_DATABASE_URL` points at the legacy MySQL (default
//! `mysql://root@127.0.0.1:3306/mutamarket`), `DATABASE_URL` at the
//! target Postgres. DESTRUCTIVE: the domain tables (users, characters,
//! modules, contracts, assets, collections, ...) are wiped and replaced
//! with the legacy snapshot; without `LEGACY_IMPORT_CONFIRM=1` the tool
//! only prints what it would do. Reference/SDE tables stay untouched.
//! Afterwards the region sweep rebuilds the live market from ESI and the
//! training sweep re-derives training modules (the latter runs here).

use mutamarket::contracts::sync_training_modules;
use mutamarket::db;
use mutamarket::legacy::{VALIDATION_SAMPLE, run_import, table_specs, validate_sample};
use mutamarket::mutation::reference::ReferenceData;
use sqlx::mysql::MySqlPoolOptions;

/// Falls back to the local Laravel dev database.
const DEFAULT_LEGACY_URL: &str = "mysql://root@127.0.0.1:3306/mutamarket";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let legacy_url =
        std::env::var("LEGACY_DATABASE_URL").unwrap_or_else(|_| DEFAULT_LEGACY_URL.to_owned());

    let confirmed =
        std::env::var("LEGACY_IMPORT_CONFIRM").is_ok_and(|v| v == "1" || v == "true");
    if !confirmed {
        println!("legacy_import replaces the Postgres domain data with the legacy");
        println!("MySQL snapshot at {legacy_url} (override with LEGACY_DATABASE_URL).");
        println!("It would wipe and reload these tables:");
        for spec in table_specs() {
            println!("  - {}", spec.name);
        }
        println!("Run again with LEGACY_IMPORT_CONFIRM=1 to actually do it.");
        return Ok(());
    }

    let mysql = MySqlPoolOptions::new().max_connections(2).connect(&legacy_url).await?;
    let pg = db::connect().await?;
    db::migrate(&pg).await?;

    println!("importing the legacy snapshot from {legacy_url}");
    let started = std::time::Instant::now();
    let report = run_import(&mysql, &pg).await?;
    let total: u64 = report.tables.iter().map(|table| table.imported).sum();
    println!("imported {total} rows in {:.1?}", started.elapsed());

    // Training modules re-derive from the imported historic contracts
    // with the same rules the scheduler job runs.
    let (deleted, upserted) = sync_training_modules(&pg).await?;
    println!("training sweep: {upserted} modules qualified, {deleted} dropped");

    // Sampled validation: recompute imported modules through our own
    // mutation math. Drift is expected for old rolls whose mutaplasmid
    // data moved in later SDE builds; large drift means a mapping bug.
    let tables = db::reference::load_reference(&pg).await?;
    let reference = ReferenceData::from_tables(tables);
    let validation = validate_sample(&pg, &reference, VALIDATION_SAMPLE).await?;
    println!(
        "validation: {}/{} sampled modules recompute exactly ({} drifted, {} uncomputable)",
        validation.matching, validation.sampled, validation.drifted, validation.uncomputable,
    );

    println!("done - run the region-contracts job (or wait for the scheduler)");
    println!("to rebuild the live market from ESI");

    Ok(())
}
