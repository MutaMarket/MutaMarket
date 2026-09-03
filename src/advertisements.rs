//! Launcher-ad sync: discovers CCP's AdGlare zone id from the
//! eveonline.com bundle, pulls the launcher ad rotation, keeps the
//! store campaigns, downloads their creatives into our own static dir
//! and mirrors them into the `advertisements` rotation with the landing
//! page swapped for the Markee Dragon affiliate store (no legacy
//! counterpart - the legacy sidebar had a hand-made Markee Dragon card).

use std::path::Path;

use sqlx::PgPool;

/// Where the zone id is discovered: the site inlines the full AdGlare
/// engine URL in its main JS chunk.
pub const DISCOVERY_SITE: &str = "https://www.eveonline.com/";

/// The engine host the discovered zone is queried on. AdGlare reads the
/// first query parameter name as the zone id; `ag_custom_term` picks
/// the creative language.
const ENGINE_PREFIX: &str = "engine2.extccp.com/?";

/// The launcher zone as last seen, the fallback when discovery fails.
pub const FALLBACK_FEED_URL: &str = "https://engine2.extccp.com/?930188625&ag_custom_term=en";

/// Environment override for the feed URL (tests point it at a mock and
/// skip discovery entirely).
pub const FEED_URL_ENV: &str = "LAUNCHER_ADS_URL";

/// Only campaigns landing on the EVE store are mirrored.
const STORE_HOST: &str = "store.eveonline.com";

/// Where mirrored ads send buyers instead: the Markee Dragon store, with
/// the deployment's own affiliate link when `MARKEEDRAGON_STORE_URL` is
/// set (the legacy MarkeeDragonStoreAd.vue carried the site's id).
pub const STORE_URL_ENV: &str = "MARKEEDRAGON_STORE_URL";

const DEFAULT_STORE_URL: &str = "https://store.markeedragon.com/";

pub fn store_link() -> String {
    std::env::var(STORE_URL_ENV)
        .ok()
        .filter(|url| !url.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_STORE_URL.to_owned())
}

/// Marks the rows this sync owns; hand-made ads are never touched.
pub const SYNC_MARKER: &str = "launcher-store-sync";

/// Marks the generic store advert the sync puts into the rotation while
/// the feed carries no store campaign, that is while no sale is on.
pub const FALLBACK_MARKER: &str = "launcher-store-fallback";

/// The generic creative ("PLEX, Omega, SP and more"), shipped with the
/// API in `assets/img` at the sidebar's 250x300.
pub const FALLBACK_IMAGE_URL: &str = "/img/store-generic.png";

/// The rotation name of the generic advert.
const FALLBACK_NAME: &str = "EVE store";

/// Downloaded creatives land here, inside the ServeDir the router
/// already exposes as `/img` (proxy-paths.ts routes it to Axum).
pub const ADS_IMAGE_DIR: &str = "assets/img/ads";

/// The public path the stored creatives are served under.
const ADS_PUBLIC_PREFIX: &str = "/img/ads";

/// Finds the AdGlare engine URL inside the site's JS chunks: fetch the
/// page, walk its `/static/js/*.js` assets and scan for the engine
/// host. Returns the full feed URL.
pub async fn discover_feed_url(site: &str) -> Option<String> {
    let html = reqwest::get(site).await.ok()?.text().await.ok()?;
    let mut assets: Vec<String> = Vec::new();
    for chunk in html.split("\"/static/js/").skip(1) {
        if let Some(end) = chunk.find('"') {
            let path = &chunk[..end];
            if path.ends_with(".js") {
                assets.push(format!("/static/js/{path}"));
            }
        }
    }
    // The engine URL sits in the main chunk; check those first.
    assets.sort_by_key(|asset| !asset.contains("/main."));

    let base = site.trim_end_matches('/');
    for asset in assets {
        let Ok(response) = reqwest::get(format!("{base}{asset}")).await else {
            continue;
        };
        let Ok(body) = response.text().await else {
            continue;
        };
        if let Some(position) = body.find(ENGINE_PREFIX) {
            let digits: String = body[position + ENGINE_PREFIX.len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if !digits.is_empty() {
                return Some(format!("https://{ENGINE_PREFIX}{digits}&ag_custom_term=en"));
            }
        }
    }
    None
}

/// The feed URL the job uses: the env override, else discovery from the
/// site, else the last known zone.
pub async fn resolve_feed_url() -> String {
    if let Ok(url) = std::env::var(FEED_URL_ENV) {
        return url;
    }
    match discover_feed_url(DISCOVERY_SITE).await {
        Some(url) => url,
        None => FALLBACK_FEED_URL.to_owned(),
    }
}

#[derive(serde::Deserialize)]
struct Feed {
    response: FeedResponse,
}

#[derive(serde::Deserialize)]
struct FeedResponse {
    campaigns: Vec<Campaign>,
}

#[derive(serde::Deserialize)]
struct Campaign {
    #[serde(rename = "crID")]
    creative_id: String,
    creative_data: CreativeData,
}

#[derive(serde::Deserialize)]
struct CreativeData {
    image_url: String,
    landing_url: String,
}

/// What one sync run did.
pub struct SyncReport {
    pub upserted: i64,
    pub removed: i64,
    pub downloaded: i64,
    /// Whether the generic store advert is in the rotation after this run.
    pub fallback: bool,
}

/// One full sync: fetch the feed, download missing store creatives into
/// `image_dir`, upsert their rotation rows (serving our own copies) and
/// drop rows plus files for creatives that left the feed. A feed without
/// any store campaign puts the generic store advert into the rotation
/// instead; the next campaign takes it out again.
pub async fn sync_launcher_store_ads(
    pool: &PgPool,
    feed_url: &str,
    image_dir: &Path,
) -> Result<SyncReport, String> {
    let response = reqwest::get(feed_url)
        .await
        .map_err(|error| format!("feed fetch: {error}"))?;
    let feed: Feed = response
        .json()
        .await
        .map_err(|error| format!("feed parse: {error}"))?;

    let store_campaigns: Vec<&Campaign> = feed
        .response
        .campaigns
        .iter()
        .filter(|campaign| {
            campaign
                .creative_data
                .landing_url
                .split('/')
                .nth(2)
                .is_some_and(|host| host == STORE_HOST)
        })
        .collect();

    std::fs::create_dir_all(image_dir).map_err(|error| format!("image dir: {error}"))?;

    let mut upserted = 0i64;
    let mut downloaded = 0i64;
    let mut served_urls: Vec<String> = Vec::new();
    for campaign in &store_campaigns {
        // The feed's id becomes a filename inside the served directory:
        // only a plain token is trusted.
        let creative_id: String = campaign
            .creative_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if creative_id.is_empty() {
            continue;
        }
        let extension = campaign
            .creative_data
            .image_url
            .rsplit('.')
            .next()
            .filter(|extension| ["png", "jpg", "jpeg", "webp"].contains(extension))
            .unwrap_or("png");
        let filename = format!("{creative_id}.{extension}");
        let file_path = image_dir.join(&filename);
        let served_url = format!("{ADS_PUBLIC_PREFIX}/{filename}");

        // Serve our own copy; a missing file (fresh container) is
        // re-downloaded even when the row already exists.
        if !file_path.exists() {
            let bytes = reqwest::get(&campaign.creative_data.image_url)
                .await
                .map_err(|error| format!("creative fetch: {error}"))?
                .bytes()
                .await
                .map_err(|error| format!("creative read: {error}"))?;
            std::fs::write(&file_path, &bytes)
                .map_err(|error| format!("creative write: {error}"))?;
            downloaded += 1;
        }

        let result = sqlx::query(
            "insert into advertisements (name, description, image_url, link, size, active)
             select $1, $2, $3, $4, 'sidebar', true
             where not exists (
                 select 1 from advertisements where description = $2 and image_url = $3
             )",
        )
        .bind(format!("EVE store promo {}", campaign.creative_id))
        .bind(SYNC_MARKER)
        .bind(&served_url)
        .bind(store_link())
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
        upserted += result.rows_affected() as i64;
        served_urls.push(served_url);
    }

    // Creatives that left the rotation: drop the rows and their files.
    let departed: Vec<String> = sqlx::query_scalar(
        "delete from advertisements
         where description = $1 and image_url <> all($2)
         returning image_url",
    )
    .bind(SYNC_MARKER)
    .bind(&served_urls)
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    for url in &departed {
        if let Some(filename) = url.strip_prefix(&format!("{ADS_PUBLIC_PREFIX}/")) {
            let _ = std::fs::remove_file(image_dir.join(filename));
        }
    }

    let fallback = served_urls.is_empty();
    if fallback {
        sqlx::query(
            "insert into advertisements (name, description, image_url, link, size, active)
             select $1, $2, $3, $4, 'sidebar', true
             where not exists (select 1 from advertisements where description = $2)",
        )
        .bind(FALLBACK_NAME)
        .bind(FALLBACK_MARKER)
        .bind(FALLBACK_IMAGE_URL)
        .bind(store_link())
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
    } else {
        sqlx::query("delete from advertisements where description = $1")
            .bind(FALLBACK_MARKER)
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }

    Ok(SyncReport {
        upserted,
        removed: departed.len() as i64,
        downloaded,
        fallback,
    })
}
