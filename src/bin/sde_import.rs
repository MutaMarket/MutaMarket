//! Imports EVE reference data from the SDE into Postgres, replacing the
//! legacy `app:download-sde` / `app:create-static-data` / `db:seed` chain.
//!
//! Usage: `cargo run --bin sde_import`
//! Downloads are cached in `storage/sde/`; delete the directory to force a
//! fresh download.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::sde::client::{Error, REQUIRED_FILES, SdeClient, extract_files};
use mutamarket::sde::{build_reference_tables, data};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let storage = Path::new("storage/sde");
    std::fs::create_dir_all(storage)?;

    let client = SdeClient::default();

    let build = client.latest_build_number().await?;
    println!("latest SDE build: {build}");

    let zip_path = storage.join(format!("eve-online-static-data-{build}-jsonl.zip"));
    if zip_path.exists() {
        println!("using cached {}", zip_path.display());
    } else {
        println!("downloading {}...", zip_path.display());
        client.download_data(build, &zip_path).await?;
    }

    let extracted = extract_files(&zip_path, &REQUIRED_FILES, storage)?;
    println!("extracted {} SDE files", extracted.len());

    let dynamic_items_path = storage.join("dynamicitemattributes.json");
    let dynamic_items: serde_json::Value = if dynamic_items_path.exists() {
        println!("using cached {}", dynamic_items_path.display());
        serde_json::from_reader(BufReader::new(File::open(&dynamic_items_path)?))?
    } else {
        println!("downloading dynamic item attributes...");
        let fetched = client.fetch_dynamic_items().await?;
        std::fs::write(&dynamic_items_path, serde_json::to_vec(&fetched)?)?;
        fetched
    };

    let sde = data::SdeData {
        types: data::parse_types(BufReader::new(File::open(&extracted[0])?))?,
        attributes: data::parse_dogma_attributes(BufReader::new(File::open(&extracted[1])?))?,
        type_dogma: data::parse_type_dogma(BufReader::new(File::open(&extracted[2])?))?,
        dynamic_items: data::parse_dynamic_items(&dynamic_items),
    };

    let tables = build_reference_tables(sde);

    println!(
        "built reference tables: {} attributes, {} types, {} type attributes, {} mutaplasmids, {} mutaplasmid attributes, {} input types, {} statistics",
        tables.attributes.len(),
        tables.types.len(),
        tables.type_attributes.len(),
        tables.mutaplasmids.len(),
        tables.mutaplasmid_attributes.len(),
        tables.input_types.len(),
        tables.statistics.len(),
    );

    let pool = db::connect().await?;
    db::migrate(&pool).await?;
    seed_reference(&pool, &tables).await?;

    println!("seeded Postgres (build {build})");

    Ok(())
}
