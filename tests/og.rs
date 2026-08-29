//! The four OpenGraph card endpoints driven through the real router:
//! rendered PNGs with the legacy geometry, the `.png` suffix the legacy
//! accepted by MySQL coercion, the 404s `tests/routes.rs` pins, and the
//! production-only cache reuse of the legacy `fileExistsInProduction`.
//!
//! Needs the local database: `docker compose up -d postgres`.
//!
//! Exact pixels are deliberately not asserted — the renderer is resvg, not
//! Imagick, so the cards are ported by geometry and not by hash.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use sqlx::PgPool;
use tower::ServiceExt;

/// Synthetic ids owned by this suite only.
const OG_TYPE: i64 = 990_009_001;
const OG_SOURCE_TYPE: i64 = 990_009_002;
const OG_MUTAPLASMID: i64 = 990_009_003;
const OG_CHARACTER: i64 = 990_009_004;
const OG_MODULE: i64 = 990_009_005;
const OG_UNIT: i64 = 990_009_020;

/// Two real attributes and one virtual one, so the card's row count is
/// known and the virtual filter is exercised end to end.
const OG_ATTRIBUTE_A: i64 = 990_009_010;
const OG_ATTRIBUTE_B: i64 = 990_009_011;
const OG_ATTRIBUTE_VIRTUAL: i64 = 990_009_012;

/// The Tech II meta group, whose accent the header rule takes.
const META_GROUP_T2: i64 = 2;

/// The user row this suite owns, matched by name because the id is serial.
const OG_USER: &str = "OG Card Pilot";

/// The collection identifier this suite owns.
const OG_COLLECTION_IDENTIFIER: &str = "og-card-test";

/// The module card is 350 wide, and 50 per header/attribute row plus the
/// 2px rule and 10px padding on each side.
const MODULE_CARD_WIDTH: u32 = 350;
const MODULE_CARD_ROWS: u32 = 2;
const MODULE_CARD_HEIGHT: u32 = 50 + MODULE_CARD_ROWS * 50 + 2 + 20;

/// The legacy canvas of the type, character and collection cards.
const WIDE_CARD_WIDTH: u32 = 600;
const WIDE_CARD_HEIGHT: u32 = 315;

/// PNG signature, the first eight bytes of every response body.
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

#[tokio::test]
async fn open_graph_cards_render_cache_and_answer_both_url_forms() {
    let cache_dir = std::env::temp_dir().join(format!("mutamarket-og-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache_dir);

    // This is the only test in this binary, so the process env is ours.
    // SAFETY: no other thread reads these variables concurrently.
    unsafe {
        std::env::set_var("OG_CACHE_DIR", &cache_dir);
        // Port 9 refuses instantly, so the portrait fetch fails fast and
        // the cards render without one, like a legacy download failure.
        std::env::set_var("IMAGE_SERVER_BASE_URL", "http://127.0.0.1:9");
        std::env::set_var("APP_ENV", "local");
    }

    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let collection_id = seed(&pool).await;
    let app = mutamarket::server::test_router().await;

    // --- every card renders, in both URL forms -------------------------
    let cards = [
        (
            format!("/og/module/{OG_MODULE}"),
            MODULE_CARD_WIDTH,
            MODULE_CARD_HEIGHT,
        ),
        (
            format!("/og/type/{OG_TYPE}"),
            WIDE_CARD_WIDTH,
            WIDE_CARD_HEIGHT,
        ),
        (
            format!("/og/character/{OG_CHARACTER}"),
            WIDE_CARD_WIDTH,
            WIDE_CARD_HEIGHT,
        ),
        (
            format!("/og/collection/{collection_id}"),
            WIDE_CARD_WIDTH,
            WIDE_CARD_HEIGHT,
        ),
    ];

    for (path, width, height) in &cards {
        let plain = png(&app, path).await;
        assert_eq!(dimensions(&plain), (*width, *height), "{path} card size");
        assert!(
            plain.len() > 1_000,
            "{path} renders real content, not an empty canvas",
        );

        // The legacy route accepted `{id}.png` because MySQL coerced it to
        // the id; both forms must render the same card.
        let suffixed = png(&app, &format!("{path}.png")).await;
        assert_eq!(plain, suffixed, "{path} and {path}.png are the same card");
    }

    // --- unknown entities and unparsable ids 404 -----------------------
    for path in [
        "/og/module/999999999",
        "/og/module/999999999.png",
        "/og/type/999999999",
        "/og/character/999999999",
        "/og/collection/999999999",
        "/og/module/not-an-id",
        "/og/module/12.jpg",
    ] {
        assert_eq!(
            status(&app, path).await,
            StatusCode::NOT_FOUND,
            "{path} is not a card",
        );
    }

    // --- the cache is always written ----------------------------------
    let module_cache = cache_dir.join("modules").join(format!("{OG_MODULE}.png"));
    for path in [
        module_cache.clone(),
        cache_dir.join("types").join(format!("{OG_TYPE}.png")),
        cache_dir
            .join("characters")
            .join(format!("{OG_CHARACTER}.png")),
        cache_dir
            .join("collections")
            .join(format!("{collection_id}.png")),
    ] {
        assert!(path.is_file(), "{} was cached", path.display());
    }

    // --- but only read back outside the local environment --------------
    // Legacy `fileExistsInProduction`. A sentinel in place of the cached
    // card shows exactly which path served the response.
    const SENTINEL: &[u8] = b"cached-card-sentinel";
    std::fs::write(&module_cache, SENTINEL).expect("overwrite the cached card");

    let rerendered = png(&app, &format!("/og/module/{OG_MODULE}")).await;
    assert_ne!(
        rerendered, SENTINEL,
        "the local environment re-renders instead of serving the cache",
    );
    assert_eq!(
        std::fs::read(&module_cache).expect("cache file"),
        rerendered,
        "and it rewrites the cache while doing so",
    );

    std::fs::write(&module_cache, SENTINEL).expect("overwrite the cached card");
    // SAFETY: as above, this test owns the process env.
    unsafe {
        std::env::set_var("APP_ENV", "production");
    }

    let served = request(&app, &format!("/og/module/{OG_MODULE}")).await;
    assert_eq!(served, SENTINEL, "production serves the cached card as is");

    // --- clearing the cache brings the renderer back -------------------
    mutamarket::og::clear_cache().expect("cache cleared");
    assert!(!module_cache.exists(), "the cached card is gone");
    assert!(
        cache_dir.is_dir(),
        "the cache root is recreated, like legacy"
    );

    let after_clear = png(&app, &format!("/og/module/{OG_MODULE}")).await;
    assert_eq!(
        dimensions(&after_clear),
        (MODULE_CARD_WIDTH, MODULE_CARD_HEIGHT),
        "a cleared cache re-renders the card",
    );

    // --- the character portrait is a JPEG ------------------------------
    // The image server answers PNG everywhere except character portraits;
    // a mislabelled data URI silently drops the portrait from the card, so
    // a served JPEG has to reach the raster.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock image server");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        let portraits = axum::Router::new().route(
            "/characters/{character_id}/portrait",
            axum::routing::get(|| async {
                ([(header::CONTENT_TYPE, "image/jpeg")], jpeg_portrait())
            }),
        );
        axum::serve(listener, portraits).await.expect("mock server");
    });

    // SAFETY: as above.
    unsafe {
        std::env::set_var("IMAGE_SERVER_BASE_URL", format!("http://{address}"));
    }
    let _ = std::fs::remove_dir_all(&cache_dir);

    let with_portrait = png(&app, &format!("/og/character/{OG_CHARACTER}")).await;
    let without_portrait = {
        // SAFETY: as above.
        unsafe {
            std::env::set_var("IMAGE_SERVER_BASE_URL", "http://127.0.0.1:9");
        }
        let _ = std::fs::remove_dir_all(&cache_dir);
        png(&app, &format!("/og/character/{OG_CHARACTER}")).await
    };
    assert_eq!(
        dimensions(&with_portrait),
        (WIDE_CARD_WIDTH, WIDE_CARD_HEIGHT),
        "the portrait card keeps the legacy canvas",
    );
    assert_ne!(
        with_portrait, without_portrait,
        "the fetched JPEG portrait reaches the card",
    );

    // SAFETY: as above.
    unsafe {
        std::env::set_var("APP_ENV", "local");
    }
    let _ = std::fs::remove_dir_all(&cache_dir);
}

/// A red 8x8 JPEG, the shape the image server returns for a portrait.
fn jpeg_portrait() -> Vec<u8> {
    let mut pixels = image::RgbImage::new(8, 8);
    for pixel in pixels.pixels_mut() {
        *pixel = image::Rgb([200, 30, 30]);
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(pixels)
        .write_to(&mut bytes, image::ImageFormat::Jpeg)
        .expect("encode jpeg");
    bytes.into_inner()
}

async fn request(app: &Router, path: &str) -> Vec<u8> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible");

    assert_eq!(response.status(), StatusCode::OK, "{path} renders");
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("image/png"),
        "{path} is served as a PNG",
    );

    response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec()
}

/// A response body that must be a real PNG.
async fn png(app: &Router, path: &str) -> Vec<u8> {
    let body = request(app, path).await;

    assert_eq!(
        body[..8],
        PNG_MAGIC,
        "{path} body starts with the PNG magic"
    );
    assert_eq!(&body[12..16], b"IHDR", "{path} body opens with the header");

    body
}

async fn status(app: &Router, path: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("infallible")
        .status()
}

/// Width and height out of the PNG's IHDR chunk.
fn dimensions(png: &[u8]) -> (u32, u32) {
    let width = u32::from_be_bytes(png[16..20].try_into().expect("width"));
    let height = u32::from_be_bytes(png[20..24].try_into().expect("height"));

    (width, height)
}

async fn seed(pool: &PgPool) -> i64 {
    // Idempotent across runs: collections of an aborted previous run would
    // block the character delete.
    sqlx::query("delete from modules where id = $1")
        .bind(OG_MODULE)
        .execute(pool)
        .await
        .expect("clean modules");
    sqlx::query("delete from collections where character_id = $1")
        .bind(OG_CHARACTER)
        .execute(pool)
        .await
        .expect("clean collections");
    sqlx::query("delete from characters where id = $1")
        .bind(OG_CHARACTER)
        .execute(pool)
        .await
        .expect("clean characters");
    sqlx::query("delete from users where name = $1")
        .bind(OG_USER)
        .execute(pool)
        .await
        .expect("clean users");

    sqlx::query(
        "insert into units (id, name, display_name) values ($1, 'ogUnit', 'OG')
         on conflict (id) do nothing",
    )
    .bind(OG_UNIT)
    .execute(pool)
    .await
    .expect("seed unit");

    sqlx::query(
        "insert into meta_groups (id, name) values ($1, 'Tech II') on conflict (id) do nothing",
    )
    .bind(META_GROUP_T2)
    .execute(pool)
    .await
    .expect("seed meta group");

    for (id, name) in [
        (OG_TYPE, "OG Abyssal Stasis Webifier"),
        (OG_SOURCE_TYPE, "OG Gravid Stasis Webifier"),
    ] {
        sqlx::query(
            "insert into types (id, name, published, meta_group_id) values ($1, $2, true, $3)
             on conflict (id) do update set name = excluded.name, published = true,
                 meta_group_id = excluded.meta_group_id",
        )
        .bind(id)
        .bind(name)
        .bind(META_GROUP_T2)
        .execute(pool)
        .await
        .expect("seed type");
    }

    for (id, name, display_name, derived) in [
        (OG_ATTRIBUTE_A, "ogSpeedFactor", "Velocity Bonus", false),
        (OG_ATTRIBUTE_B, "ogCapacitorNeed", "Activation Cost", false),
        (OG_ATTRIBUTE_VIRTUAL, "ogVirtual", "Bar Only", false),
    ] {
        sqlx::query(
            "insert into attributes (id, name, display_name, derived, unit_id)
             values ($1, $2, $3, $4, $5)
             on conflict (id) do update set name = excluded.name,
                 display_name = excluded.display_name, derived = excluded.derived,
                 unit_id = excluded.unit_id",
        )
        .bind(id)
        .bind(name)
        .bind(display_name)
        .bind(derived)
        .bind(OG_UNIT)
        .execute(pool)
        .await
        .expect("seed attribute");
    }

    sqlx::query(
        "insert into mutaplasmids (id, name, output_type_id) values ($1, 'OG Gravid Mutaplasmid', $2)
         on conflict (id) do update set name = excluded.name,
             output_type_id = excluded.output_type_id",
    )
    .bind(OG_MUTAPLASMID)
    .bind(OG_TYPE)
    .execute(pool)
    .await
    .expect("seed mutaplasmid");

    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
        .bind(OG_USER)
        .fetch_one(pool)
        .await
        .expect("create user");

    sqlx::query("insert into characters (id, name, user_id, description) values ($1, $2, $3, $4)")
        .bind(OG_CHARACTER)
        .bind("OG Card Pilot")
        .bind(user_id)
        .bind("Rolls webs, sells webs, repeat")
        .execute(pool)
        .await
        .expect("create character");

    sqlx::query(
        "insert into modules (id, type_id, source_type_id, mutaplasmid_id, creator_id,
                              average_fraction)
         values ($1, $2, $3, $4, $5, 0.62)",
    )
    .bind(OG_MODULE)
    .bind(OG_TYPE)
    .bind(OG_SOURCE_TYPE)
    .bind(OG_MUTAPLASMID)
    .bind(OG_CHARACTER)
    .execute(pool)
    .await
    .expect("create module");

    // A positive roll, a negative one with a brown bar, and a virtual row
    // the card must drop.
    for (attribute_id, value, base_value, fraction, bar, is_virtual) in [
        (OG_ATTRIBUTE_A, 62.5, 55.0, 0.74_f64, 0_i16, false),
        (OG_ATTRIBUTE_B, 41.0, 34.0, -0.31_f64, -1_i16, false),
        (OG_ATTRIBUTE_VIRTUAL, 1.0, 1.0, 0.0_f64, 0_i16, true),
    ] {
        sqlx::query(
            "insert into mutated_attributes
                 (module_id, attribute_id, type_id, value, base_value, fraction,
                  fraction_type, fraction_absolute, bar, is_virtual)
             values ($1, $2, $3, $4, $5, $6, $6, $6, $7, $8)",
        )
        .bind(OG_MODULE)
        .bind(attribute_id)
        .bind(OG_TYPE)
        .bind(value)
        .bind(base_value)
        .bind(fraction)
        .bind(bar)
        .bind(is_virtual)
        .execute(pool)
        .await
        .expect("seed mutated attribute");
    }

    sqlx::query_scalar(
        "insert into collections (identifier, name, visibility, character_id, description)
         values ($1, 'OG Webs', 'public', $2, 'The good ones')
         on conflict (identifier) do update set name = excluded.name,
             character_id = excluded.character_id, description = excluded.description
         returning id",
    )
    .bind(OG_COLLECTION_IDENTIFIER)
    .bind(OG_CHARACTER)
    .fetch_one(pool)
    .await
    .expect("create collection")
}
