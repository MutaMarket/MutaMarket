//! Downloading the SDE inputs: CCP's official JSONL zip (build-versioned)
//! and the community dynamic-item-attributes JSON.

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub const DEFAULT_BASE_URL: &str = "https://developers.eveonline.com/static-data/tranquility";
pub const DYNAMIC_ITEMS_URL: &str = "https://sde.hoboleaks.space/tq/dynamicitemattributes.json";

/// The SDE files the reference import needs.
pub const REQUIRED_FILES: [&str; 6] = [
    "types.jsonl",
    "dogmaAttributes.jsonl",
    "typeDogma.jsonl",
    "dogmaUnits.jsonl",
    "metaGroups.jsonl",
    "mapRegions.jsonl",
];

pub struct SdeClient {
    base_url: String,
    http: reqwest::Client,
}

impl Default for SdeClient {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL)
    }
}

impl SdeClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// The build number of the most recent SDE release on Tranquility, from
    /// the `latest.jsonl` record with `_key == "sde"`.
    pub async fn latest_build_number(&self) -> Result<i64, Error> {
        let body = self
            .http
            .get(format!("{}/latest.jsonl", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let record: serde_json::Value = match serde_json::from_str(line) {
                Ok(record) => record,
                Err(_) => continue,
            };

            if record["_key"] != "sde" {
                continue;
            }

            if let Some(build) = record["buildNumber"].as_i64() {
                return Ok(build);
            }
        }

        Err("could not resolve the latest SDE build number".into())
    }

    /// Downloads the JSONL data zip for a build to `dest`, streaming to disk.
    pub async fn download_data(&self, build_number: i64, dest: &Path) -> Result<(), Error> {
        let url = format!(
            "{}/eve-online-static-data-{}-jsonl.zip",
            self.base_url, build_number
        );

        let mut response = self.http.get(url).send().await?.error_for_status()?;
        let mut file = File::create(dest)?;

        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk)?;
        }

        Ok(())
    }

    /// Fetches the dynamic-item-attributes JSON (mutaplasmid definitions).
    pub async fn fetch_dynamic_items(&self) -> Result<serde_json::Value, Error> {
        Ok(self
            .http
            .get(DYNAMIC_ITEMS_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

/// Extracts the given file names (matched by base name) from a zip archive
/// into `dest_dir`, returning the extracted paths in the order requested.
pub fn extract_files(
    zip_path: &Path,
    file_names: &[&str],
    dest_dir: &Path,
) -> Result<Vec<PathBuf>, Error> {
    let mut archive = zip::ZipArchive::new(BufReader::new(File::open(zip_path)?))?;
    let mut extracted = vec![None; file_names.len()];

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().rsplit('/').next().unwrap_or_default().to_owned();

        let Some(position) = file_names.iter().position(|&wanted| wanted == name) else {
            continue;
        };

        let dest = dest_dir.join(&name);
        let mut file = File::create(&dest)?;
        io::copy(&mut entry, &mut file)?;
        extracted[position] = Some(dest);
    }

    extracted
        .into_iter()
        .enumerate()
        .map(|(position, path)| {
            path.ok_or_else(|| {
                format!("{} not found in the SDE archive", file_names[position]).into()
            })
        })
        .collect()
}
