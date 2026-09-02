//! The OpenGraph card renderer, a port of the legacy
//! `App\Http\Controllers\OpenGraphController` together with
//! `App\Services\OpenGraph\{OpenGraphService,Renderer,Components\*}`.
//!
//! The legacy drew the cards imperatively with Imagick: a tree of
//! components each compositing itself onto one `Imagick` canvas. The tree
//! is declarative in everything but the drawing calls, so here it is
//! emitted as an SVG document and rasterized with resvg instead of
//! reimplementing a drawing API. Text shaping, patterns and image scaling
//! come from the renderer; the geometry, colours, wording and ordering are
//! ported literally from the legacy components.
//!
//! Rendered cards are cached on disk with the legacy layout, and — like the
//! legacy — the cache is only ever *read back* outside the local
//! environment, so development always re-renders and picks up card changes.

use std::path::{Path, PathBuf};
use std::time::Duration;

use resvg::tiny_skia;
use resvg::usvg;
use sqlx::PgPool;
use sqlx::Row as _;

use crate::modules::queries::module_detail;
use crate::mutation::reference::ReferenceData;

mod cards;
mod fonts;
mod service;
mod svg;

pub use service::{CardAttribute, card_attributes};

/// The content type of every card, and of the "download image" action the
/// module toolbar points at these routes.
pub const CONTENT_TYPE: &str = "image/png";

/// Where rendered cards are cached, mirroring the legacy
/// `storage/app/public/og/{modules,types,characters,collections}`.
const CACHE_ROOT: &str = "storage/app/public/og";

/// Overrides [`CACHE_ROOT`], so a test run never writes into the working
/// copy's cache.
const CACHE_DIR_ENV: &str = "OG_CACHE_DIR";

/// The EVE image server, which the character and collection cards fetch
/// portraits from. Overridable so tests do not depend on the network.
const IMAGE_SERVER_ENV: &str = "IMAGE_SERVER_BASE_URL";
const IMAGE_SERVER_DEFAULT: &str = "https://images.evetech.net";

/// A portrait fetch is best-effort, so it gets a short budget rather than
/// holding a link unfurler's request open.
const PORTRAIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Portrait size the legacy character card requests, for a 130px render.
const CHARACTER_PORTRAIT_SIZE: u32 = 256;

/// Portrait size the legacy collection card requests, for a 44px render.
const COLLECTION_PORTRAIT_SIZE: u32 = 128;

/// The four cache directories, which are also the four routes.
const MODULES: &str = "modules";
const TYPES: &str = "types";
const CHARACTERS: &str = "characters";
const COLLECTIONS: &str = "collections";

/// The rendered PNG of a module card, or `None` when the module is unknown.
pub async fn module_card(
    pool: &PgPool,
    reference: &ReferenceData,
    id: i64,
) -> sqlx::Result<Option<Vec<u8>>> {
    let path = cache_path(MODULES, id);
    if let Some(cached) = cached(&path) {
        return Ok(Some(cached));
    }

    let Some(module) = module_detail(pool, reference, id).await? else {
        return Ok(None);
    };

    let card = cards::ModuleCard {
        type_id: module.r#type.id,
        // The legacy models type these relations as non-null. They are
        // optional here, and an empty line renders where the legacy would
        // have thrown rather than answering a link unfurler with a 500.
        source_type_name: module
            .source_type
            .as_ref()
            .map(|source| source.name.clone())
            .unwrap_or_default(),
        mutaplasmid_name: module
            .mutaplasmid
            .as_ref()
            .map(|mutaplasmid| mutaplasmid.name.clone())
            .unwrap_or_default(),
        meta_group_id: module
            .source_type
            .as_ref()
            .and_then(|source| source.meta_group_id),
        attributes: card_attributes(&module.mutated_attributes),
    };

    Ok(render(&cards::module(&card), &path))
}

/// The rendered PNG of a type card, or `None` when the type is unknown.
pub async fn type_card(pool: &PgPool, id: i64) -> sqlx::Result<Option<Vec<u8>>> {
    let path = cache_path(TYPES, id);
    if let Some(cached) = cached(&path) {
        return Ok(Some(cached));
    }

    let Some(name) = sqlx::query_scalar::<_, String>("select name from types where id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };

    let card = cards::TypeCard { id, name };

    Ok(render(&cards::type_card(&card), &path))
}

/// The rendered PNG of a character card, or `None` when the character is
/// unknown.
pub async fn character_card(pool: &PgPool, id: i64) -> sqlx::Result<Option<Vec<u8>>> {
    let path = cache_path(CHARACTERS, id);
    if let Some(cached) = cached(&path) {
        return Ok(Some(cached));
    }

    let Some(row) = sqlx::query("select name, description from characters where id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };

    let card = cards::CharacterCard {
        name: row.get("name"),
        description: row.get("description"),
        portrait: portrait(id, CHARACTER_PORTRAIT_SIZE).await,
    };

    Ok(render(&cards::character_card(&card), &path))
}

/// The rendered PNG of a collection card, or `None` when the collection is
/// unknown.
pub async fn collection_card(pool: &PgPool, id: i64) -> sqlx::Result<Option<Vec<u8>>> {
    let path = cache_path(COLLECTIONS, id);
    if let Some(cached) = cached(&path) {
        return Ok(Some(cached));
    }

    let Some(row) = sqlx::query(
        "select c.name, c.description, c.character_id,
                coalesce(ch.name, '') as creator_name,
                (select count(*) from collection_modules cm
                 where cm.collection_id = c.id) as module_count
         from collections c
         left join characters ch on ch.id = c.character_id
         where c.id = $1 and c.visibility <> 'private'",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };

    let card = cards::CollectionCard {
        name: row.get("name"),
        description: row.get("description"),
        creator_name: row.get("creator_name"),
        module_count: row.get("module_count"),
        portrait: portrait(row.get("character_id"), COLLECTION_PORTRAIT_SIZE).await,
    };

    Ok(render(&cards::collection_card(&card), &path))
}

/// Rasterize a card and write it to the cache, like the legacy
/// `Renderer::renderToFile` followed by `response()->file()`. The write is
/// unconditional — only the read back is environment-dependent.
fn render(document: &str, path: &Path) -> Option<Vec<u8>> {
    let png = rasterize(document)?;
    store(path, &png);

    Some(png)
}

fn rasterize(document: &str) -> Option<Vec<u8>> {
    let options = usvg::Options {
        fontdb: fonts::fonts().database(),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_str(document, &options).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    pixmap.encode_png().ok()
}

/// The EVE portrait of a character, or `None` when it cannot be fetched.
/// The legacy `downloadPortrait` returned null on a failed or empty
/// download so the card renders without the portrait rather than
/// compositing a zero byte file; the same holds here.
async fn portrait(character_id: i64, size: u32) -> Option<Vec<u8>> {
    let base = std::env::var(IMAGE_SERVER_ENV)
        .unwrap_or_else(|_| IMAGE_SERVER_DEFAULT.to_owned())
        .trim_end_matches('/')
        .to_owned();
    let url = format!("{base}/characters/{character_id}/portrait?size={size}");

    let client = reqwest::Client::builder()
        .timeout(PORTRAIT_TIMEOUT)
        .build()
        .ok()?;

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    (!bytes.is_empty()).then(|| bytes.to_vec())
}

fn cache_root() -> PathBuf {
    std::env::var(CACHE_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(CACHE_ROOT))
}

fn cache_path(kind: &str, id: i64) -> PathBuf {
    cache_root().join(kind).join(format!("{id}.png"))
}

/// Legacy `fileExistsInProduction`: a cached card is served only outside
/// the local environment. Development re-renders on every request, which is
/// what makes card changes visible without clearing anything.
fn cached(path: &Path) -> Option<Vec<u8>> {
    if crate::environment::is_local() {
        return None;
    }

    std::fs::read(path).ok()
}

fn store(path: &Path, png: &[u8]) {
    if let Some(directory) = path.parent() {
        let _ = std::fs::create_dir_all(directory);
    }

    let _ = std::fs::write(path, png);
}

/// Legacy `app:clear-og-cache`: empty the cache directory so a card
/// design change reaches the links that were already shared. The
/// directory itself stays: in the containers it is a volume mount point,
/// which cannot be removed.
pub fn clear_cache() -> std::io::Result<()> {
    let root = cache_root();
    std::fs::create_dir_all(&root)?;
    clear_dir(&root)
}

fn clear_dir(root: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(entry.path())?;
        } else {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CACHE_ROOT, cache_path, clear_dir};

    #[test]
    fn clearing_empties_the_directory_but_keeps_it() {
        let root = std::env::temp_dir().join(format!("og-clear-{}", std::process::id()));
        std::fs::create_dir_all(root.join("modules")).expect("nested dir");
        std::fs::write(root.join("modules/1.png"), b"png").expect("card");
        std::fs::write(root.join("stray"), b"x").expect("file");

        clear_dir(&root).expect("clears");

        assert!(root.is_dir());
        assert_eq!(std::fs::read_dir(&root).expect("readable").count(), 0);
        std::fs::remove_dir(&root).expect("cleanup");
    }

    #[test]
    fn cache_paths_follow_the_legacy_layout() {
        // OG_CACHE_DIR is unset in a plain unit-test run, so this is the
        // legacy-shaped default.
        if std::env::var(super::CACHE_DIR_ENV).is_ok() {
            return;
        }

        assert_eq!(
            cache_path("modules", 42).to_string_lossy(),
            format!("{CACHE_ROOT}/modules/42.png"),
        );
        assert_eq!(
            cache_path("collections", 7).to_string_lossy(),
            format!("{CACHE_ROOT}/collections/7.png"),
        );
    }
}
