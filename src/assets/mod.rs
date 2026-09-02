//! Character asset ingestion, ported from the legacy
//! `GetCharacterAssetsCommand` → `DispatchAssetsImportsAction` →
//! `GetAssetsJob` → `CreateAssetsAction` chain: fetch a character's (and
//! optionally their corporation's) assets from ESI, keep only the abyssal
//! modules and the container chain around them, ingest the modules through
//! the shared import path, and track every run in the `asset_imports`
//! state machine so crashes are observable and recoverable.

pub mod public;

use std::collections::{HashMap, HashSet};

use sqlx::{PgPool, Row};

use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiAsset, EsiClient, EsiError};
use crate::estimator::Estimator;
use crate::modules::ingest::import_module;
use crate::modules::view::{AssetLocationView, CharacterRef, StationRef, module_slug, slugify};
use crate::mutation::reference::ReferenceData;
use crate::structures;

/// A character's assets are refreshed at most this often, the legacy
/// `assets.update_interval_hours`.
const UPDATE_INTERVAL_HOURS: i32 = 6;

/// Characters imported per scheduler tick (every five minutes), the legacy
/// `assets.max_characters_to_import`.
const MAX_CHARACTERS_PER_RUN: i64 = 10;

/// A pending/processing import untouched for this long is considered
/// crashed and marked failed, the legacy `assets.import_timeout`.
const IMPORT_TIMEOUT_MINUTES: i32 = 30;

/// The ESI asset-names endpoints take at most this many ids per request,
/// the legacy vendor chunking.
const NAME_ID_CHUNK: usize = 1000;

/// Location flags under which an asset sits directly inside a structure,
/// the legacy `LocationFlag::isStructureLocation`.
const STRUCTURE_LOCATION_FLAGS: [&str; 3] = ["Hangar", "ShipHangar", "OfficeFolder"];

/// An asset in a structure hangar reports this `location_type`.
const LOCATION_TYPE_ITEM: &str = "item";

/// The legacy `AssetImportStatus` values.
pub mod status {
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
}

/// The legacy `AssetImportStep` values, advanced as the fetch progresses.
pub mod step {
    pub const FETCHING_ASSETS: &str = "fetching_assets";
    pub const FETCHING_ASSET_NAMES: &str = "fetching_asset_names";
    pub const FETCHING_CORPORATION_ASSETS: &str = "fetching_corporation_assets";
    pub const FETCHING_CORPORATION_ASSET_NAMES: &str = "fetching_corporation_asset_names";
    pub const SEARCHING_ABYSSAL_MODULES: &str = "searching_abyssal_modules";
    pub const IMPORTING_ABYSSAL_MODULES: &str = "importing_abyssal_modules";
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AssetSyncStats {
    /// Rows kept (abyssal modules plus their container chain).
    pub assets: usize,
    /// Raw corporation assets seen (before filtering), like the legacy
    /// `assets_corporation_count`.
    pub corporation_assets: usize,
    pub abyssal_modules: usize,
    pub modules_imported: usize,
    pub modules_failed: usize,
}

#[derive(Debug)]
pub enum AssetSyncError {
    /// The character holds no token with the assets scope.
    NoToken,
    Token(TokenError),
    Esi(EsiError),
    Db(sqlx::Error),
}

impl std::fmt::Display for AssetSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetSyncError::NoToken => write!(f, "no token with the assets scope"),
            AssetSyncError::Token(error) => write!(f, "token: {error}"),
            AssetSyncError::Esi(error) => write!(f, "ESI: {error}"),
            AssetSyncError::Db(error) => write!(f, "database: {error}"),
        }
    }
}

impl std::error::Error for AssetSyncError {}

impl From<TokenError> for AssetSyncError {
    fn from(error: TokenError) -> Self {
        AssetSyncError::Token(error)
    }
}

impl From<EsiError> for AssetSyncError {
    fn from(error: EsiError) -> Self {
        AssetSyncError::Esi(error)
    }
}

impl From<sqlx::Error> for AssetSyncError {
    fn from(error: sqlx::Error) -> Self {
        AssetSyncError::Db(error)
    }
}

/// Characters due for an asset import: they hold the assets scope and have
/// no import newer than the update interval, oldest last import first —
/// the legacy `DispatchAssetsImportsAction` selection. (Characters without
/// any import sort first; MySQL puts ascending nulls first and Postgres
/// needs it spelled out.)
pub async fn pending_asset_characters(pool: &PgPool) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select c.id from characters c
         where exists (
                   select 1 from esi_tokens t
                   where t.character_id = c.id and $1 = any(t.scopes)
               )
           and not exists (
                   select 1 from asset_imports ai
                   where ai.character_id = c.id
                     and ai.created_at > now() - make_interval(hours => $2)
               )
         order by (
             select ai.created_at from asset_imports ai
             where ai.id = c.latest_asset_import_id
         ) asc nulls first, c.id
         limit $3",
    )
    .bind(scopes::READ_ASSETS)
    .bind(UPDATE_INTERVAL_HOURS)
    .bind(MAX_CHARACTERS_PER_RUN)
    .fetch_all(pool)
    .await
}

/// Marks imports stuck in pending/processing beyond the timeout as failed,
/// the legacy `FailStaleAssetImportsCommand` sweeper.
pub async fn fail_stale_asset_imports(pool: &PgPool) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "update asset_imports set status = $1, updated_at = now()
         where status in ($2, $3) and updated_at < now() - make_interval(mins => $4)",
    )
    .bind(status::FAILED)
    .bind(status::PENDING)
    .bind(status::PROCESSING)
    .bind(IMPORT_TIMEOUT_MINUTES)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Runs one character's asset import, the legacy `AssetImport::dispatch` +
/// `GetAssetsJob`: creates the import row, fetches and stores the assets,
/// ingests the abyssal modules, and completes or fails the row.
pub async fn sync_character_assets(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    estimator: &Estimator,
    character_id: i64,
) -> Result<AssetSyncStats, AssetSyncError> {
    let import_id: i64 = sqlx::query_scalar(
        "insert into asset_imports (character_id, status, step) values ($1, $2, $3) returning id",
    )
    .bind(character_id)
    .bind(status::PENDING)
    .bind(step::FETCHING_ASSETS)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "update characters set latest_asset_import_id = $1, updated_at = now() where id = $2",
    )
    .bind(import_id)
    .bind(character_id)
    .execute(pool)
    .await?;

    match run_import(
        pool,
        reference,
        esi,
        sso,
        estimator,
        character_id,
        import_id,
    )
    .await
    {
        Ok(stats) => {
            set_import(pool, import_id, status::COMPLETED, None).await?;
            Ok(stats)
        }
        Err(error) => {
            // Any stage failure fails the whole import, like the legacy
            // state machine; the character is retried after the interval.
            set_import(pool, import_id, status::FAILED, None).await?;
            Err(error)
        }
    }
}

async fn set_import(
    pool: &PgPool,
    import_id: i64,
    status: &str,
    step: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "update asset_imports set status = $1, step = coalesce($2, step), updated_at = now()
         where id = $3",
    )
    .bind(status)
    .bind(step)
    .bind(import_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// A valid token for the scope; ESI rejections of it must delete the row.
async fn scope_token(
    pool: &PgPool,
    sso: &SsoClient,
    character_id: i64,
    scope: &str,
) -> Result<Option<tokens::AccessToken>, AssetSyncError> {
    Ok(tokens::valid_access_token(pool, sso, character_id, scope).await?)
}

/// Deletes the token when ESI answered 401/403 with it, like the legacy
/// connector, then converts the error.
async fn fail_authed(
    pool: &PgPool,
    token: &tokens::AccessToken,
    error: EsiError,
) -> AssetSyncError {
    if matches!(error, EsiError::Forbidden(_))
        && let Err(db_error) = tokens::delete_token(pool, token.token_id).await
    {
        return AssetSyncError::Db(db_error);
    }
    AssetSyncError::Esi(error)
}

/// The legacy `MarketGroup::SHIPS` root of the nameable-type filter.
const MARKET_GROUP_SHIPS: i64 = 4;
/// The legacy `MarketGroup::CONTAINERS` root of the nameable-type filter.
const MARKET_GROUP_CONTAINERS: i64 = 379;

/// Published types under the Ships/Containers market groups — the only
/// items ESI can name, like the legacy `getNameableTypeIds`. Requesting
/// anything else trips the whole names batch into a 404.
async fn nameable_type_ids(pool: &PgPool) -> sqlx::Result<HashSet<i64>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "with recursive groups as (
             select id from market_groups where id = any($1)
             union all
             select mg.id from market_groups mg join groups g on mg.parent_id = g.id
         )
         select t.id from types t join groups on t.market_group_id = groups.id
         where t.published",
    )
    .bind(vec![MARKET_GROUP_SHIPS, MARKET_GROUP_CONTAINERS])
    .fetch_all(pool)
    .await?;

    Ok(ids.into_iter().collect())
}

/// Fetches asset names, bisecting rejected batches: ESI answers 404 for
/// the whole request when any id cannot be named (offices, wrapper items).
/// Legacy avoids those by filtering to ship/container market groups; the
/// market-group tree is outside our minimal SDE import, so unnameable ids
/// are isolated by splitting instead — the same names come back.
async fn fetch_names_bisecting(
    esi: &EsiClient,
    access_token: &str,
    owner_path: &str,
    ids: &[i64],
    names: &mut HashMap<i64, String>,
) -> Result<(), crate::esi::EsiError> {
    let mut queue: Vec<Vec<i64>> = ids.chunks(NAME_ID_CHUNK).map(<[i64]>::to_vec).collect();

    while let Some(batch) = queue.pop() {
        match esi.asset_names(access_token, owner_path, &batch).await {
            Ok(resolved) => {
                names.extend(resolved.into_iter().map(|name| (name.item_id, name.name)));
            }
            Err(crate::esi::EsiError::NotFound)
            | Err(crate::esi::EsiError::UnexpectedStatus(reqwest::StatusCode::NOT_FOUND)) => {
                if batch.len() <= 1 {
                    tracing::debug!(owner = owner_path, item = ?batch.first(), "asset not nameable, skipped");
                    continue;
                }
                let half = batch.len().div_ceil(2);
                queue.push(batch[..half].to_vec());
                queue.push(batch[half..].to_vec());
            }
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

async fn run_import(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    estimator: &Estimator,
    character_id: i64,
    import_id: i64,
) -> Result<AssetSyncStats, AssetSyncError> {
    let Some(token) = scope_token(pool, sso, character_id, scopes::READ_ASSETS).await? else {
        return Err(AssetSyncError::NoToken);
    };

    set_import(
        pool,
        import_id,
        status::PROCESSING,
        Some(step::FETCHING_ASSETS),
    )
    .await?;

    let mut character_assets: Vec<EsiAsset> = Vec::new();
    let mut page = 1;
    loop {
        let (mut batch, pages) = match esi
            .character_assets(&token.access_token, character_id, page)
            .await
        {
            Ok(result) => result,
            Err(error) => return Err(fail_authed(pool, &token, error).await),
        };
        character_assets.append(&mut batch);
        if page >= pages {
            break;
        }
        page += 1;
    }

    // Corporation assets ride along when the corporation scope was
    // granted, like the legacy resolveCorporationAssets gate.
    set_import(
        pool,
        import_id,
        status::PROCESSING,
        Some(step::FETCHING_CORPORATION_ASSETS),
    )
    .await?;

    let corporation_id: Option<i64> =
        sqlx::query_scalar("select corporation_id from characters where id = $1")
            .bind(character_id)
            .fetch_one(pool)
            .await?;

    let corporation_token =
        scope_token(pool, sso, character_id, scopes::READ_CORPORATION_ASSETS).await?;

    let mut corporation_assets: Vec<EsiAsset> = Vec::new();
    if let (Some(corporation_token), Some(corporation_id)) = (&corporation_token, corporation_id) {
        let mut page = 1;
        loop {
            let (mut batch, pages) = match esi
                .corporation_assets(&corporation_token.access_token, corporation_id, page)
                .await
            {
                Ok(result) => result,
                Err(error) => return Err(fail_authed(pool, corporation_token, error).await),
            };
            corporation_assets.append(&mut batch);
            if page >= pages {
                break;
            }
            page += 1;
        }
    }

    set_import(
        pool,
        import_id,
        status::PROCESSING,
        Some(step::SEARCHING_ABYSSAL_MODULES),
    )
    .await?;

    let corporation_item_ids: HashSet<i64> = corporation_assets
        .iter()
        .map(|asset| asset.item_id)
        .collect();

    // All fetched assets by item id, for walking the container chain.
    let by_item: HashMap<i64, &EsiAsset> = character_assets
        .iter()
        .chain(corporation_assets.iter())
        .map(|asset| (asset.item_id, asset))
        .collect();

    let modules: Vec<&EsiAsset> = by_item
        .values()
        .copied()
        .filter(|asset| reference.is_abyssal_type(asset.type_id))
        .collect();

    // The legacy keeps every published ship/container-market-group
    // singleton via market-group ancestry the minimal SDE import does not
    // carry. Kept here instead: the ancestor chain of every abyssal
    // module — exactly the containers the location and sell pages display
    // (both filter to containers holding abyssal descendants).
    let mut kept: HashMap<i64, &EsiAsset> = HashMap::new();
    for module in &modules {
        kept.insert(module.item_id, module);
        let mut cursor = module.location_id;
        while let Some(parent) = by_item.get(&cursor) {
            if kept.insert(parent.item_id, parent).is_some() {
                break;
            }
            cursor = parent.location_id;
        }
    }

    // Names for the container chain (ships and containers are singletons;
    // stacked items cannot be named).
    set_import(
        pool,
        import_id,
        status::PROCESSING,
        Some(step::FETCHING_ASSET_NAMES),
    )
    .await?;

    let module_ids: HashSet<i64> = modules.iter().map(|asset| asset.item_id).collect();
    // Pre-filter to nameable types like legacy; the bisecting fetch below
    // stays as the safety net for anything that slips through.
    let nameable_types = nameable_type_ids(pool).await?;
    let nameable = |asset: &&&EsiAsset| {
        asset.is_singleton
            && !module_ids.contains(&asset.item_id)
            && nameable_types.contains(&asset.type_id)
    };

    let character_nameable: Vec<i64> = kept
        .values()
        .filter(nameable)
        .filter(|asset| !corporation_item_ids.contains(&asset.item_id))
        .map(|asset| asset.item_id)
        .collect();
    let corporation_nameable: Vec<i64> = kept
        .values()
        .filter(nameable)
        .filter(|asset| corporation_item_ids.contains(&asset.item_id))
        .map(|asset| asset.item_id)
        .collect();

    let mut names: HashMap<i64, String> = HashMap::new();
    if let Err(error) = fetch_names_bisecting(
        esi,
        &token.access_token,
        &format!("characters/{character_id}"),
        &character_nameable,
        &mut names,
    )
    .await
    {
        return Err(fail_authed(pool, &token, error).await);
    }
    if let (Some(corporation_token), Some(corporation_id)) = (&corporation_token, corporation_id) {
        set_import(
            pool,
            import_id,
            status::PROCESSING,
            Some(step::FETCHING_CORPORATION_ASSET_NAMES),
        )
        .await?;
        if let Err(error) = fetch_names_bisecting(
            esi,
            &corporation_token.access_token,
            &format!("corporations/{corporation_id}"),
            &corporation_nameable,
            &mut names,
        )
        .await
        {
            return Err(fail_authed(pool, corporation_token, error).await);
        }
    }

    sqlx::query(
        "update asset_imports set
             assets_count = $1, assets_corporation_count = $2, abyssal_modules_count = $3,
             updated_at = now()
         where id = $4",
    )
    .bind(kept.len() as i32)
    .bind(corporation_assets.len() as i32)
    .bind(modules.len() as i32)
    .bind(import_id)
    .execute(pool)
    .await?;

    set_import(
        pool,
        import_id,
        status::PROCESSING,
        Some(step::IMPORTING_ABYSSAL_MODULES),
    )
    .await?;

    // Character and corporation inventories are separate trees, like the
    // legacy per-owner asset jobs; corporation ordinals overlay.
    let mut indexes = type_indexes(&character_assets);
    indexes.extend(type_indexes(&corporation_assets));

    store_assets(
        pool,
        character_id,
        corporation_id,
        &kept,
        &module_ids,
        &corporation_item_ids,
        &names,
        &indexes,
    )
    .await?;

    // Ingest the abyssal modules through the shared import path (already
    // known modules are skipped there). Failures stay per module, like
    // the legacy batch jobs; they are counted, not fatal.
    let mut imported = 0usize;
    let mut failed = 0usize;
    for module in &modules {
        match import_module(
            pool,
            reference,
            esi,
            estimator,
            module.type_id,
            module.item_id,
        )
        .await
        {
            Ok(()) => imported += 1,
            Err(error) => {
                tracing::warn!(
                    "asset module {} (type {}) failed to import: {error}",
                    module.item_id,
                    module.type_id,
                );
                failed += 1;
            }
        }
        sqlx::query(
            "update asset_imports set
                 abyssal_modules_imported_count = $1, abyssal_modules_failed_count = $2,
                 updated_at = now()
             where id = $3",
        )
        .bind(imported as i32)
        .bind(failed as i32)
        .bind(import_id)
        .execute(pool)
        .await?;
    }

    // The legacy GetAssetsJob dispatches SyncAutoSyncCollectionsJob from
    // the module batch's finally() once every module imported; the
    // sequential equivalent runs it right after the ingestion loop.
    // Legacy quirk kept: an import that found no abyssal modules returns
    // before the batch is dispatched, so auto-sync collections are only
    // re-synced when modules were found. A failure is logged, not fatal
    // (the legacy job is queued separately from the import).
    if !modules.is_empty()
        && let Err(error) = crate::collections::sync_auto_sync_collections(pool, character_id).await
    {
        tracing::warn!("auto-sync collections for character {character_id} failed: {error}");
    }

    // Structures the assets sit in but which are not assets themselves get
    // their names resolved, the legacy dispatchGetStructureJobs.
    let structure_ids: HashSet<i64> = kept
        .values()
        .filter(|asset| {
            asset.location_type == LOCATION_TYPE_ITEM
                && STRUCTURE_LOCATION_FLAGS.contains(&asset.location_flag.as_str())
                && !kept.contains_key(&asset.location_id)
        })
        .map(|asset| asset.location_id)
        .collect();
    for structure_id in structure_ids {
        if let Err(error) =
            structures::sync_structure(pool, esi, sso, character_id, structure_id).await
        {
            tracing::warn!("structure {structure_id} resolution failed: {error}");
        }
    }

    Ok(AssetSyncStats {
        assets: kept.len(),
        corporation_assets: corporation_assets.len(),
        abyssal_modules: modules.len(),
        modules_imported: imported,
        modules_failed: failed,
    })
}

/// The per-container ordinal of each asset among same-type siblings, the
/// legacy `Tree::traverseRecursive` type index: siblings are all assets
/// sharing a `location_id` ordered by item id, and an asset's index counts
/// the same-type siblings before it. Computed over the FULL inventory
/// (before the abyssal filter), so "the 3rd Caracal in this hangar" stays
/// correct even when its siblings are not stored.
fn type_indexes(assets: &[EsiAsset]) -> HashMap<i64, i64> {
    let mut groups: HashMap<i64, Vec<&EsiAsset>> = HashMap::new();
    for asset in assets {
        groups.entry(asset.location_id).or_default().push(asset);
    }

    let mut indexes = HashMap::with_capacity(assets.len());
    for siblings in groups.values_mut() {
        siblings.sort_by_key(|asset| asset.item_id);
        let mut per_type: HashMap<i64, i64> = HashMap::new();
        for asset in siblings.iter() {
            let counter = per_type.entry(asset.type_id).or_insert(0);
            indexes.insert(asset.item_id, *counter);
            *counter += 1;
        }
    }

    indexes
}

/// Upserts the kept assets and removes the character's stale rows — the
/// legacy `CreateAssetsAction` upsert-plus-diff-delete, never a full wipe,
/// so a crash leaves the previous state intact.
#[allow(clippy::too_many_arguments)]
async fn store_assets(
    pool: &PgPool,
    character_id: i64,
    corporation_id: Option<i64>,
    kept: &HashMap<i64, &EsiAsset>,
    module_ids: &HashSet<i64>,
    corporation_item_ids: &HashSet<i64>,
    names: &HashMap<i64, String>,
    indexes: &HashMap<i64, i64>,
) -> Result<(), AssetSyncError> {
    // Deterministic write order (item id) for stable reruns.
    let mut ordered: Vec<&&EsiAsset> = kept.values().collect();
    ordered.sort_by_key(|asset| asset.item_id);

    let mut tx = pool.begin().await?;

    for asset in ordered.iter() {
        sqlx::query(
            "insert into assets
             (character_id, corporation_id, item_id, type_id, name, location_id, location_flag,
              location_type, quantity, index, is_abyssal)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             on conflict (character_id, item_id) do update set
                 corporation_id = excluded.corporation_id,
                 type_id = excluded.type_id,
                 name = excluded.name,
                 location_id = excluded.location_id,
                 location_flag = excluded.location_flag,
                 location_type = excluded.location_type,
                 quantity = excluded.quantity,
                 index = excluded.index,
                 is_abyssal = excluded.is_abyssal,
                 updated_at = now()",
        )
        .bind(character_id)
        .bind(
            corporation_item_ids
                .contains(&asset.item_id)
                .then_some(corporation_id)
                .flatten(),
        )
        .bind(asset.item_id)
        .bind(asset.type_id)
        .bind(names.get(&asset.item_id))
        .bind(asset.location_id)
        .bind(&asset.location_flag)
        .bind(&asset.location_type)
        .bind(asset.quantity)
        .bind(indexes.get(&asset.item_id).copied().unwrap_or(0))
        .bind(module_ids.contains(&asset.item_id))
        .execute(&mut *tx)
        .await?;
    }

    let fresh_ids: Vec<i64> = ordered.iter().map(|asset| asset.item_id).collect();
    sqlx::query("delete from assets where character_id = $1 and item_id <> all($2)")
        .bind(character_id)
        .bind(&fresh_ids)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

/// One asset row of a character's inventory as loaded for the location
/// views below.
struct CharacterAssetRow {
    asset_id: i64,
    item_id: i64,
    type_id: i64,
    type_name: Option<String>,
    name: Option<String>,
    location_id: Option<i64>,
    corporation_id: Option<i64>,
    is_abyssal: bool,
    public_asset_id: Option<i64>,
}

async fn character_asset_rows(
    pool: &PgPool,
    character_id: i64,
) -> sqlx::Result<Vec<CharacterAssetRow>> {
    let rows = sqlx::query(
        "select a.id as asset_id, a.item_id, a.type_id, t.name as type_name, a.name,
                a.location_id, a.corporation_id, a.is_abyssal,
                (select min(pa.id) from public_assets pa
                 where pa.character_id = a.character_id and pa.asset_id = a.id)
                    as public_asset_id
         from assets a
         left join types t on t.id = a.type_id
         where a.character_id = $1
         order by a.item_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| CharacterAssetRow {
            asset_id: row.get("asset_id"),
            item_id: row.get("item_id"),
            type_id: row.get("type_id"),
            type_name: row.get("type_name"),
            name: row.get("name"),
            location_id: row.get("location_id"),
            corporation_id: row.get("corporation_id"),
            is_abyssal: row.get("is_abyssal"),
            public_asset_id: row.get("public_asset_id"),
        })
        .collect())
}

/// Per-item count of abyssal modules at or below each asset (the legacy
/// withCount('descendants', abyssal) rollup), walked over the fetched
/// inventory.
fn abyssal_descendant_counts(assets: &[CharacterAssetRow]) -> HashMap<i64, i64> {
    let by_item: HashMap<i64, &CharacterAssetRow> =
        assets.iter().map(|asset| (asset.item_id, asset)).collect();
    let mut counts: HashMap<i64, i64> = HashMap::new();
    for module in assets.iter().filter(|asset| asset.is_abyssal) {
        let mut cursor = module.location_id;
        let mut visited: HashSet<i64> = HashSet::new();
        while let Some(parent) = cursor.and_then(|location_id| by_item.get(&location_id)) {
            if !visited.insert(parent.item_id) {
                break;
            }
            *counts.entry(parent.item_id).or_default() += 1;
            cursor = parent.location_id;
        }
    }
    counts
}

/// The EVE id rooting each asset's chain (the location of its topmost
/// known ancestor): a station or structure id, resolved by the caller.
fn root_location_id(
    assets: &HashMap<i64, &CharacterAssetRow>,
    start: &CharacterAssetRow,
) -> Option<i64> {
    let mut current = start;
    let mut visited: HashSet<i64> = HashSet::new();
    while let Some(parent) = current
        .location_id
        .and_then(|location_id| assets.get(&location_id))
    {
        if !visited.insert(parent.item_id) {
            break;
        }
        current = parent;
    }
    current.location_id
}

/// Names the given station/structure ids, structure names winning like
/// the legacy ancestor loop ($structure ?? $station).
async fn station_refs(pool: &PgPool, ids: &[i64]) -> sqlx::Result<HashMap<i64, StationRef>> {
    let stations: Vec<(i64, String, Option<i64>)> =
        sqlx::query_as("select id, name, type_id from stations where id = any($1)")
            .bind(ids)
            .fetch_all(pool)
            .await?;
    let structures: Vec<(i64, Option<String>, Option<i64>)> =
        sqlx::query_as("select id, name, type_id from structures where id = any($1)")
            .bind(ids)
            .fetch_all(pool)
            .await?;

    let mut refs = HashMap::new();
    for (id, name, type_id) in stations {
        refs.insert(
            id,
            StationRef {
                slug: format!("{}-{id}", slugify(&name)),
                id,
                name,
                type_id,
            },
        );
    }
    for (id, name, type_id) in structures {
        if let Some(name) = name {
            refs.insert(
                id,
                StationRef {
                    slug: format!("{}-{id}", slugify(&name)),
                    id,
                    name,
                    type_id,
                },
            );
        }
    }
    Ok(refs)
}

fn character_location_view(
    asset: &CharacterAssetRow,
    modules_count: i64,
    station: Option<StationRef>,
) -> crate::modules::view::CharacterLocationView {
    // The legacy slug: the asset name, or the type name for unnamed rows.
    let name_slug = asset
        .name
        .as_deref()
        .map(slugify)
        .filter(|slug| !slug.is_empty())
        .or_else(|| {
            asset
                .type_name
                .as_deref()
                .map(slugify)
                .filter(|slug| !slug.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    crate::modules::view::CharacterLocationView {
        asset_id: asset.asset_id,
        item_id: asset.item_id,
        name: asset.name.clone(),
        type_id: asset.type_id,
        type_name: asset.type_name.clone(),
        location_id: asset.location_id,
        station,
        modules_count,
        public_asset_id: asset.public_asset_id,
        corporation_id: asset.corporation_id,
        slug: format!("{name_slug}-{}", asset.item_id),
    }
}

/// The collection page's two location loadouts from a single inventory
/// read: the tracked rows for the page's `collection_locations` and the
/// full holding list for the location picker (previously two functions,
/// each loading the character's full asset inventory).
pub async fn collection_location_views(
    pool: &PgPool,
    character_id: i64,
    tracked_asset_ids: &[i64],
) -> sqlx::Result<(
    Vec<crate::modules::view::CharacterLocationView>,
    Vec<crate::modules::view::CharacterLocationView>,
)> {
    let assets = character_asset_rows(pool, character_id).await?;
    let tracked = tracked_location_views(pool, &assets, tracked_asset_ids).await?;
    let locations = character_location_views(pool, &assets).await?;
    Ok((tracked, locations))
}

/// The legacy `LocationService::getCharacterLocations` as the collection
/// page calls it (corporation locations included): the character's
/// non-abyssal assets holding abyssal modules somewhere below, with the
/// rolled-up counts and the rooting station/structure.
async fn character_location_views(
    pool: &PgPool,
    assets: &[CharacterAssetRow],
) -> sqlx::Result<Vec<crate::modules::view::CharacterLocationView>> {
    let counts = abyssal_descendant_counts(assets);
    let by_item: HashMap<i64, &CharacterAssetRow> =
        assets.iter().map(|asset| (asset.item_id, asset)).collect();

    let holding: Vec<&CharacterAssetRow> = assets
        .iter()
        .filter(|asset| !asset.is_abyssal && counts.get(&asset.item_id).copied().unwrap_or(0) > 0)
        .collect();
    let root_ids: Vec<i64> = holding
        .iter()
        .filter_map(|asset| root_location_id(&by_item, asset))
        .collect();
    let stations = station_refs(pool, &root_ids).await?;

    Ok(holding
        .into_iter()
        .map(|asset| {
            let station =
                root_location_id(&by_item, asset).and_then(|root| stations.get(&root).cloned());
            character_location_view(asset, counts[&asset.item_id], station)
        })
        .collect())
}

/// Location rows for specific tracked asset rows (the legacy
/// `tracked_locations`, mapping collectionLocations to their assets),
/// input order kept. Counts stay 0, like the legacy resource when
/// descendants_count is not loaded.
async fn tracked_location_views(
    pool: &PgPool,
    assets: &[CharacterAssetRow],
    asset_ids: &[i64],
) -> sqlx::Result<Vec<crate::modules::view::CharacterLocationView>> {
    let by_item: HashMap<i64, &CharacterAssetRow> =
        assets.iter().map(|asset| (asset.item_id, asset)).collect();
    let by_asset_id: HashMap<i64, &CharacterAssetRow> =
        assets.iter().map(|asset| (asset.asset_id, asset)).collect();

    let picked: Vec<&CharacterAssetRow> = asset_ids
        .iter()
        .filter_map(|asset_id| by_asset_id.get(asset_id).copied())
        .collect();
    let root_ids: Vec<i64> = picked
        .iter()
        .filter_map(|asset| root_location_id(&by_item, asset))
        .collect();
    let stations = station_refs(pool, &root_ids).await?;

    Ok(picked
        .into_iter()
        .map(|asset| {
            let station =
                root_location_id(&by_item, asset).and_then(|root| stations.get(&root).cloned());
            character_location_view(asset, 0, station)
        })
        .collect())
}

/// Where each of the given modules sits for this user, the legacy
/// `AssetResource` resolution: walk the asset's parent chain to the top,
/// the outermost station/structure wins, the direct parent (or the
/// station itself for loose hangar items) names the row.
pub async fn module_locations(
    pool: &PgPool,
    user_id: i64,
    module_ids: &[i64],
) -> sqlx::Result<std::collections::HashMap<i64, AssetLocationView>> {
    let rows = sqlx::query(
        "with recursive owned as (
             select a.*, a.item_id as leaf_item_id, 0 as depth
             from assets a join characters c on c.id = a.character_id
             where c.user_id = $1 and a.item_id = any($2) and a.is_abyssal
         ),
         chain as (
             select * from owned
             union all
             select p.*, chain.leaf_item_id, chain.depth + 1
             from assets p join chain
               on p.item_id = chain.location_id and p.character_id = chain.character_id
         )
         select chain.*, st.name as station_name, st.type_id as station_type_id,
                str.name as structure_name, str.type_id as structure_type_id,
                t.id as asset_type_id,
                oc.name as owner_name, oc.description as owner_description,
                oc.corporation_id as owner_corporation_id,
                (oc.premium_paid_until is not null and oc.premium_paid_until > now())
                    as owner_has_premium
         from chain
         left join stations st on st.id = chain.location_id
         left join structures str on str.id = chain.location_id
         left join types t on t.id = chain.type_id
         join characters oc on oc.id = chain.character_id
         order by chain.leaf_item_id, chain.depth",
    )
    .bind(user_id)
    .bind(module_ids)
    .fetch_all(pool)
    .await?;

    let mut locations = std::collections::HashMap::new();

    let mut index = 0;
    while index < rows.len() {
        let leaf_item: i64 = rows[index].get("leaf_item_id");
        let mut end = index;
        while end < rows.len() && rows[end].get::<i64, _>("leaf_item_id") == leaf_item {
            end += 1;
        }
        let group = &rows[index..end];
        index = end;

        let leaf = &group[0];
        let parent = group.get(1);

        // The outermost station/structure along the chain wins, exactly
        // like the legacy ancestor loop (later ancestors overwrite).
        let mut station: Option<(i64, String, Option<i64>)> = None;
        let mut structure: Option<(i64, String, Option<i64>)> = None;
        for row in group {
            let location_id: Option<i64> = row.get("location_id");
            if let (Some(id), Some(name)) =
                (location_id, row.get::<Option<String>, _>("station_name"))
            {
                station = Some((id, name, row.get("station_type_id")));
            }
            if let (Some(id), Some(name)) =
                (location_id, row.get::<Option<String>, _>("structure_name"))
            {
                structure = Some((id, name, row.get("structure_type_id")));
            }
        }

        let location = structure.clone().or_else(|| station.clone());
        let parent_name = parent
            .and_then(|row| row.get::<Option<String>, _>("name"))
            .or_else(|| structure.as_ref().map(|(_, name, _)| name.clone()))
            .or_else(|| station.as_ref().map(|(_, name, _)| name.clone()))
            .unwrap_or_else(|| "Unknown Location".to_owned());
        let parent_type_id = parent
            .map(|row| row.get::<i64, _>("type_id"))
            .or(structure.as_ref().and_then(|(_, _, type_id)| *type_id))
            .or(station.as_ref().and_then(|(_, _, type_id)| *type_id));

        let leaf_location_id: i64 = leaf.get::<Option<i64>, _>("location_id").unwrap_or(0);
        let owner_id: i64 = leaf.get("character_id");
        let owner_name: String = leaf.get("owner_name");
        locations.insert(
            leaf_item,
            AssetLocationView {
                parent_slug: format!("{}-{leaf_location_id}", slugify(&parent_name)),
                parent_name,
                parent_type_id,
                station: location.map(|(id, name, type_id)| StationRef {
                    id,
                    slug: format!("{}-{id}", slugify(&name)),
                    name,
                    type_id,
                }),
                location_id: leaf_location_id,
                location_type: leaf.get("location_type"),
                location_flag: leaf.get("location_flag"),
                location_index: leaf.get("index"),
                corporation_id: leaf.get("corporation_id"),
                owner: CharacterRef {
                    id: owner_id,
                    slug: module_slug(&owner_name, owner_id),
                    name: owner_name,
                    description: leaf.get("owner_description"),
                    has_premium: leaf.get("owner_has_premium"),
                    corporation_id: leaf.get("owner_corporation_id"),
                },
            },
        );
    }

    Ok(locations)
}

#[cfg(test)]
mod index_tests {
    use super::type_indexes;
    use crate::esi::EsiAsset;

    fn asset(item_id: i64, type_id: i64, location_id: i64) -> EsiAsset {
        EsiAsset {
            item_id,
            type_id,
            location_id,
            location_flag: "Hangar".to_owned(),
            location_type: "item".to_owned(),
            quantity: 1,
            is_singleton: true,
        }
    }

    #[test]
    fn indexes_count_same_type_siblings_per_container_in_item_id_order() {
        // Station 60000004 hangar: two Caracals (621) around a Drake (24698),
        // plus a nested module chain inside the second Caracal.
        let assets = vec![
            asset(1003, 621, 60000004),
            asset(1001, 621, 60000004),
            asset(1002, 24698, 60000004),
            // Two same-type abyssal modules inside ship 1003.
            asset(2002, 47408, 1003),
            asset(2001, 47408, 1003),
            asset(2003, 47749, 1003),
        ];

        let indexes = type_indexes(&assets);

        // Caracals: 1001 first (item-id order), the Drake does not count.
        assert_eq!(indexes[&1001], 0);
        assert_eq!(indexes[&1003], 1);
        assert_eq!(indexes[&1002], 0);
        // Fitted modules: same type counts, the afterburner restarts at 0.
        assert_eq!(indexes[&2001], 0);
        assert_eq!(indexes[&2002], 1);
        assert_eq!(indexes[&2003], 0);
    }
}
