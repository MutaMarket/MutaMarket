//! Launcher-ad sync: pulls the EVE launcher's ad rotation from CCP's
//! AdGlare zone endpoint, keeps the store campaigns, and mirrors them
//! into our `advertisements` rotation with the landing page swapped for
//! the Markee Dragon affiliate store (no legacy counterpart - the
//! legacy sidebar had a hand-made Markee Dragon card instead).

use sqlx::PgPool;

/// The AdGlare zone endpoint of the EVE launcher's ad slot. The bare
/// number is the zone id (AdGlare reads the first query parameter name
/// as the zone); `ag_custom_term` picks the creative language.
pub const LAUNCHER_FEED_URL: &str = "https://engine2.extccp.com/?930188625&ag_custom_term=en";

/// Environment override for the feed URL (tests point it at a mock).
pub const FEED_URL_ENV: &str = "LAUNCHER_ADS_URL";

/// Only campaigns landing on the EVE store are mirrored.
const STORE_HOST: &str = "store.eveonline.com";

/// Where mirrored ads send buyers instead: the Markee Dragon store with
/// the legacy affiliate id (the legacy MarkeeDragonStoreAd.vue link).
pub const MARKEE_DRAGON_LINK: &str =
    "https://store.markeedragon.com/affiliate.php?id=1034&redirect=index.php?cat=4";

/// Marks the rows this sync owns; hand-made ads are never touched.
pub const SYNC_MARKER: &str = "launcher-store-sync";

pub fn feed_url() -> String {
    std::env::var(FEED_URL_ENV).unwrap_or_else(|_| LAUNCHER_FEED_URL.to_owned())
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
}

/// Fetches the zone feed and mirrors the store campaigns: one
/// advertisement per creative (keyed by image URL), linking to the
/// affiliate store; creatives that left the rotation are removed.
pub async fn sync_launcher_store_ads(pool: &PgPool, url: &str) -> Result<SyncReport, String> {
    let response = reqwest::get(url).await.map_err(|error| format!("feed fetch: {error}"))?;
    let feed: Feed =
        response.json().await.map_err(|error| format!("feed parse: {error}"))?;

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

    let mut upserted = 0i64;
    for campaign in &store_campaigns {
        // No unique constraint on image_url: dedupe by the marker +
        // creative pair so reruns are idempotent.
        let result = sqlx::query(
            "insert into advertisements (name, description, image_url, link, size, active)
             select $1, $2, $3, $4, 'sidebar', true
             where not exists (
                 select 1 from advertisements where description = $2 and image_url = $3
             )",
        )
        .bind(format!("EVE store promo {}", campaign.creative_id))
        .bind(SYNC_MARKER)
        .bind(&campaign.creative_data.image_url)
        .bind(MARKEE_DRAGON_LINK)
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
        upserted += result.rows_affected() as i64;
    }

    let image_urls: Vec<String> = store_campaigns
        .iter()
        .map(|campaign| campaign.creative_data.image_url.clone())
        .collect();
    let removed = sqlx::query(
        "delete from advertisements
         where description = $1 and image_url <> all($2)",
    )
    .bind(SYNC_MARKER)
    .bind(&image_urls)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?
    .rows_affected() as i64;

    Ok(SyncReport { upserted, removed })
}
