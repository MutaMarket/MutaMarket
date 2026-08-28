//! `/sitemap.xml`, replacing the legacy `app:create-sitemap` schedule
//! (daily at 11:00): spatie's SitemapGenerator crawled the rendered site
//! from https://mutamarket.com, excluded everything under the forbidden
//! prefixes (og, collections, modules, characters, api, calculator —
//! index pages included) and wrote public/sitemap.xml.
//!
//! Mechanism, deliberate: the crawl over our fixed page inventory always
//! lands on the same URL set, so the rewrite renders that set on request
//! from Axum instead of running a daily crawler job — no scheduler job,
//! no cache, no file. The list mirrors the legacy generator's actual
//! output, crawl quirks included (auth-only pages the crawler saw links
//! to, and the all-modules sort links it followed).

use axum::http::header;
use axum::response::IntoResponse;

/// The site origin the legacy command hardcoded
/// (`SitemapGenerator::create('https://mutamarket.com')`).
const SITE_ORIGIN: &str = "https://mutamarket.com";

/// The legacy generator's URL paths, in its output order.
pub const SITEMAP_PATHS: [&str; 38] = [
    "/",
    "/calculator",
    "/documentation",
    "/donations",
    "/documentation/getting-started",
    "/documentation/browsing-the-market",
    "/documentation/module-details",
    "/documentation/appraisal",
    "/premium",
    "/characters",
    "/collections",
    "/documentation/rolling-guide",
    "/modules",
    "/documentation/selling-and-assets",
    "/documentation/offers",
    "/documentation/collections",
    "/documentation/historic-sales",
    "/documentation/workbench-and-tools",
    "/documentation/premium",
    "/documentation/about",
    "/documentation/support",
    "/documentation/donations-and-raffles",
    "/sell/modules",
    "/documentation/legal",
    "/settings",
    "/locations",
    "/historic-sales",
    "/personal/modules",
    "/personal/stats",
    "/offers",
    "/personal/contracts",
    "/omega-calculator",
    "/all-modules",
    "/login",
    "/moderator/contracts",
    "/statistics",
    "/all-modules/sort/value/asc",
    "/all-modules/sort/value/desc",
];

/// GET /sitemap.xml — the spatie output shape: plain `<loc>` entries
/// (the crawled URLs carried no priority, changefreq or lastmod).
pub async fn show() -> impl IntoResponse {
    let urls = SITEMAP_PATHS
        .iter()
        .map(|path| format!("    <url>\n        <loc>{SITE_ORIGIN}{path}</loc>\n    </url>\n"))
        .collect::<String>();
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" \
         xmlns:xhtml=\"http://www.w3.org/1999/xhtml\" \
         xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\" \
         xmlns:video=\"http://www.google.com/schemas/sitemap-video/1.1\" \
         xmlns:news=\"http://www.google.com/schemas/sitemap-news/0.9\">\n\
         {urls}</urlset>\n",
    );

    ([(header::CONTENT_TYPE, "application/xml")], body)
}
