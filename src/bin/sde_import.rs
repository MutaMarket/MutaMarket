//! Imports EVE reference data from the SDE into Postgres, replacing the
//! legacy `app:download-sde` / `app:create-static-data` / `db:seed` chain.
//!
//! Usage: `cargo run --bin sde_import`
//! The build-versioned SDE zip is cached in `storage/sde/`; delete the
//! directory to force a fresh download. The mutaplasmid definitions are
//! refetched on every reseed, being a single unversioned file. When the database already carries the latest SDE build
//! the import is skipped entirely (the docker bootstrap runs this on every
//! `up`); set `SDE_FORCE=1` to reseed anyway.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::mutation::reference::ReferenceTables;
use mutamarket::sde::client::{ClientResources, Error, REQUIRED_FILES, SdeClient, extract_files};
use mutamarket::sde::{build_reference_tables, data};

/// The frontend's bundled abyssal type list, the source of its URL slugs.
const ABYSSAL_TYPES_PATH: &str = "frontend/src/lib/abyssals.json";

/// Where the API serves `/img` from; the category dialog reads its type
/// icons out of `icons/`.
const ICON_DIR: &str = "assets/img/icons";

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let storage = Path::new("storage/sde");
    std::fs::create_dir_all(storage)?;

    let client = SdeClient::default();

    let build = client.latest_build_number().await?;
    println!("latest SDE build: {build}");

    let pool = db::connect().await?;
    db::migrate(&pool).await?;

    let force = std::env::var("SDE_FORCE").is_ok_and(|value| value == "1" || value == "true");
    if !force && db::seeded_sde_build(&pool).await? == Some(build.to_string()) {
        println!("SDE build {build} already seeded, skipping (SDE_FORCE=1 to reseed)");
        return Ok(());
    }

    let zip_path = storage.join(format!("eve-online-static-data-{build}-jsonl.zip"));
    if zip_path.exists() {
        println!("using cached {}", zip_path.display());
    } else {
        println!("downloading {}...", zip_path.display());
        client.download_data(build, &zip_path).await?;
    }

    let extracted = extract_files(&zip_path, &REQUIRED_FILES, storage)?;
    println!("extracted {} SDE files", extracted.len());

    // Unlike the SDE zip, whose file name carries the build number, this
    // one is a single moving file: reusing it pinned the mutaplasmid set
    // to whenever it was first written, and the abyssal types released
    // afterwards never became abyssal types here. Refetched on every
    // reseed, with the copy on disk as the fallback when the community
    // service is down.
    let dynamic_items_path = storage.join("dynamicitemattributes.json");
    println!("downloading dynamic item attributes...");
    let dynamic_items: serde_json::Value = match client.fetch_dynamic_items().await {
        Ok(fetched) => {
            std::fs::write(&dynamic_items_path, serde_json::to_vec(&fetched)?)?;
            fetched
        }
        Err(error) if dynamic_items_path.exists() => {
            println!(
                "could not fetch them ({error}), using cached {}",
                dynamic_items_path.display()
            );
            serde_json::from_reader(BufReader::new(File::open(&dynamic_items_path)?))?
        }
        Err(error) => return Err(error),
    };

    let sde = data::SdeData {
        types: data::parse_types(BufReader::new(File::open(&extracted[0])?))?,
        attributes: data::parse_dogma_attributes(BufReader::new(File::open(&extracted[1])?))?,
        type_dogma: data::parse_type_dogma(BufReader::new(File::open(&extracted[2])?))?,
        units: data::parse_dogma_units(BufReader::new(File::open(&extracted[3])?))?,
        meta_groups: data::parse_meta_groups(BufReader::new(File::open(&extracted[4])?))?,
        regions: data::parse_regions(BufReader::new(File::open(&extracted[5])?))?,
        dynamic_items: data::parse_dynamic_items(&dynamic_items),
        stations: data::build_stations(
            BufReader::new(File::open(&extracted[9])?),
            BufReader::new(File::open(&extracted[11])?),
            BufReader::new(File::open(&extracted[10])?),
            BufReader::new(File::open(&extracted[6])?),
            BufReader::new(File::open(&extracted[8])?),
            BufReader::new(File::open(&extracted[7])?),
        )?,
        market_groups: data::parse_market_groups(BufReader::new(File::open(&extracted[12])?))?,
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
    println!(
        "abyssal type statistics: {}",
        tables.abyssal_statistics.len(),
    );

    seed_reference(&pool, &tables).await?;

    // The estimator seeds run after the reference tables they match
    // against, like the legacy migration seeder + `db:seed` chain.
    mutamarket::estimator::seed::seed_estimator_attributes(&pool).await?;
    mutamarket::estimator::seed::seed_estimator_statistics(&pool).await?;

    db::record_sde_build(&pool, &build.to_string()).await?;
    println!("seeded Postgres (build {build})");

    // The generated inputs live in the source tree, which the shipped
    // image does not carry: there the reference tables are all the
    // import is for.
    if Path::new(ABYSSAL_TYPES_PATH)
        .parent()
        .is_some_and(Path::is_dir)
    {
        write_abyssal_types(&tables)?;
        fetch_missing_icons(&tables, &extracted[0], &extracted[1], &extracted[13]).await?;
    } else {
        println!("no source tree here, leaving the generated type list and icons alone");
    }

    Ok(())
}

/// The abyssal type list the frontend bundles for its URL slugs
/// (`abyssalSlug`), rewritten from the freshly seeded mutaplasmid output
/// types. Generated rather than maintained by hand so a type CCP adds
/// cannot reach the database while the frontend still calls it unknown;
/// `catalog.test.ts` then fails until the picker carries it too.
fn write_abyssal_types(tables: &ReferenceTables) -> Result<(), Error> {
    let names: HashMap<i64, &str> = tables
        .types
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();

    let ids: Vec<i64> = tables
        .mutaplasmids
        .iter()
        .map(|mutaplasmid| mutaplasmid.output_type_id)
        .collect::<std::collections::BTreeSet<i64>>()
        .into_iter()
        .collect();

    let mut out = String::from("[\n");
    for (index, id) in ids.iter().enumerate() {
        let Some(name) = names.get(id) else {
            return Err(format!("abyssal type {id} has no type row").into());
        };
        let comma = if index + 1 == ids.len() { "" } else { "," };
        let name = serde_json::to_string(name)?;
        out.push_str(&format!(
            "    {{ \"id\": {id}, \"name\": {name} }}{comma}\n"
        ));
    }
    out.push_str("]\n");

    std::fs::write(ABYSSAL_TYPES_PATH, out)?;
    println!("wrote {} abyssal types to {ABYSSAL_TYPES_PATH}", ids.len());

    Ok(())
}

/// Downloads the client icons the bundle has none of, the way the legacy
/// `app:create-icon-files` built the set: the SDE names each `res:/...`
/// icon, and the client's resource index says which file serves it. The
/// image server is not the source, since it composites the abyssal corner
/// badge onto its art, which the bundled set does not carry.
///
/// Both halves of the set are covered: one icon per abyssal type for the
/// category picker, and one per rollable attribute for the filter rows.
/// Files already in the bundle are left alone, which keeps the icons the
/// legacy command hand-placed where the SDE offers none or offers the
/// wrong art (the mutated mining drones, the siege and mining
/// attributes). Anything genuinely without an icon is reported for the
/// same hand-placement.
async fn fetch_missing_icons(
    tables: &ReferenceTables,
    types_path: &Path,
    attributes_path: &Path,
    icons_path: &Path,
) -> Result<(), Error> {
    let types = missing_icons(tables.mutaplasmids.iter().map(|row| row.output_type_id));
    let attributes = missing_icons(
        tables
            .mutaplasmid_attributes
            .iter()
            .map(|row| row.attribute_id),
    );

    if types.is_empty() && attributes.is_empty() {
        println!("every abyssal type and rollable attribute already has an icon");
        return Ok(());
    }

    let icon_files = read_icon_files(icons_path)?;
    let resources = ClientResources::load().await?;

    for (label, path, wanted) in [
        ("abyssal type", types_path, types),
        ("attribute", attributes_path, attributes),
    ] {
        let icon_ids = read_icon_ids(path, &wanted)?;

        for id in wanted {
            let Some(res_path) = icon_ids
                .get(&id)
                .and_then(|icon_id| icon_files.get(icon_id))
            else {
                println!("{label} {id} has no icon in the SDE, leaving it to the bundle");
                continue;
            };

            match resources.read(res_path).await? {
                Some(bytes) => {
                    std::fs::write(Path::new(ICON_DIR).join(format!("{id}.png")), &bytes)?;
                    println!("fetched {res_path} as the icon of {label} {id}");
                }
                None => println!("the client has no {res_path}, leaving {label} {id} alone"),
            }
        }
    }

    Ok(())
}

/// The ids among these with no icon file in the bundle, deduplicated and
/// in a stable order.
fn missing_icons(ids: impl Iterator<Item = i64>) -> Vec<i64> {
    ids.collect::<std::collections::BTreeSet<i64>>()
        .into_iter()
        .filter(|id| !Path::new(ICON_DIR).join(format!("{id}.png")).exists())
        .collect()
}

/// `iconID` per row key, for the rows asked about.
fn read_icon_ids(path: &Path, wanted: &[i64]) -> Result<HashMap<i64, i64>, Error> {
    let wanted: std::collections::HashSet<i64> = wanted.iter().copied().collect();
    let mut icon_ids = HashMap::new();

    for line in std::io::BufRead::lines(BufReader::new(File::open(path)?)) {
        let row: serde_json::Value = serde_json::from_str(&line?)?;
        let (Some(id), Some(icon_id)) = (row["_key"].as_i64(), row["iconID"].as_i64()) else {
            continue;
        };
        if wanted.contains(&id) {
            icon_ids.insert(id, icon_id);
        }
    }

    Ok(icon_ids)
}

/// The `res:/...` path of every icon id.
fn read_icon_files(path: &Path) -> Result<HashMap<i64, String>, Error> {
    let mut files = HashMap::new();

    for line in std::io::BufRead::lines(BufReader::new(File::open(path)?)) {
        let row: serde_json::Value = serde_json::from_str(&line?)?;
        let (Some(id), Some(file)) = (row["_key"].as_i64(), row["iconFile"].as_str()) else {
            continue;
        };
        files.insert(id, file.to_owned());
    }

    Ok(files)
}
