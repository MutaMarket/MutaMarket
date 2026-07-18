//! Character asset ingestion, ported from the legacy
//! `GetCharacterAssetsCommand` → `DispatchAssetsImportsAction` →
//! `GetAssetsJob` → `CreateAssetsAction` chain: fetch a character's (and
//! optionally their corporation's) assets from ESI, keep only the abyssal
//! modules and the container chain around them, ingest the modules through
//! the shared import path, and track every run in the `asset_imports`
//! state machine so crashes are observable and recoverable.

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiAsset, EsiClient, EsiError};
use crate::estimator::EstimatorClient;
use crate::modules::ingest::import_module;
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
    estimator: &EstimatorClient,
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

    sqlx::query("update characters set latest_asset_import_id = $1, updated_at = now() where id = $2")
        .bind(import_id)
        .bind(character_id)
        .execute(pool)
        .await?;

    match run_import(pool, reference, esi, sso, estimator, character_id, import_id).await {
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

async fn run_import(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    estimator: &EstimatorClient,
    character_id: i64,
    import_id: i64,
) -> Result<AssetSyncStats, AssetSyncError> {
    let Some(token) = scope_token(pool, sso, character_id, scopes::READ_ASSETS).await? else {
        return Err(AssetSyncError::NoToken);
    };

    set_import(pool, import_id, status::PROCESSING, Some(step::FETCHING_ASSETS)).await?;

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
    set_import(pool, import_id, status::PROCESSING, Some(step::FETCHING_CORPORATION_ASSETS)).await?;

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

    set_import(pool, import_id, status::PROCESSING, Some(step::SEARCHING_ABYSSAL_MODULES)).await?;

    let corporation_item_ids: HashSet<i64> =
        corporation_assets.iter().map(|asset| asset.item_id).collect();

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
    set_import(pool, import_id, status::PROCESSING, Some(step::FETCHING_ASSET_NAMES)).await?;

    let module_ids: HashSet<i64> = modules.iter().map(|asset| asset.item_id).collect();
    let nameable =
        |asset: &&&EsiAsset| asset.is_singleton && !module_ids.contains(&asset.item_id);

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
    for chunk in character_nameable.chunks(NAME_ID_CHUNK) {
        let batch = match esi
            .asset_names(&token.access_token, &format!("characters/{character_id}"), chunk)
            .await
        {
            Ok(batch) => batch,
            Err(error) => return Err(fail_authed(pool, &token, error).await),
        };
        names.extend(batch.into_iter().map(|name| (name.item_id, name.name)));
    }
    if let (Some(corporation_token), Some(corporation_id)) = (&corporation_token, corporation_id) {
        set_import(
            pool,
            import_id,
            status::PROCESSING,
            Some(step::FETCHING_CORPORATION_ASSET_NAMES),
        )
        .await?;
        for chunk in corporation_nameable.chunks(NAME_ID_CHUNK) {
            let batch = match esi
                .asset_names(
                    &corporation_token.access_token,
                    &format!("corporations/{corporation_id}"),
                    chunk,
                )
                .await
            {
                Ok(batch) => batch,
                Err(error) => return Err(fail_authed(pool, corporation_token, error).await),
            };
            names.extend(batch.into_iter().map(|name| (name.item_id, name.name)));
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

    set_import(pool, import_id, status::PROCESSING, Some(step::IMPORTING_ABYSSAL_MODULES)).await?;

    store_assets(
        pool,
        character_id,
        corporation_id,
        &kept,
        &module_ids,
        &corporation_item_ids,
        &names,
    )
    .await?;

    // Ingest the abyssal modules through the shared import path (already
    // known modules are skipped there). Failures stay per module, like
    // the legacy batch jobs; they are counted, not fatal.
    let mut imported = 0usize;
    let mut failed = 0usize;
    for module in &modules {
        match import_module(pool, reference, esi, estimator, module.type_id, module.item_id).await {
            Ok(()) => imported += 1,
            Err(error) => {
                eprintln!(
                    "asset module {} (type {}) failed to import: {error}",
                    module.item_id, module.type_id,
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
            eprintln!("structure {structure_id} resolution failed: {error}");
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
) -> Result<(), AssetSyncError> {
    // Tree traversal order for the index column: containers first, then
    // their contents, children in item-id order for determinism.
    let mut children: HashMap<i64, Vec<&EsiAsset>> = HashMap::new();
    let mut roots: Vec<&EsiAsset> = Vec::new();
    for asset in kept.values() {
        if kept.contains_key(&asset.location_id) {
            children.entry(asset.location_id).or_default().push(asset);
        } else {
            roots.push(asset);
        }
    }
    roots.sort_by_key(|asset| asset.item_id);
    for list in children.values_mut() {
        list.sort_by_key(|asset| asset.item_id);
    }

    let mut ordered: Vec<&EsiAsset> = Vec::with_capacity(kept.len());
    let mut stack: Vec<&EsiAsset> = roots.into_iter().rev().collect();
    while let Some(asset) = stack.pop() {
        ordered.push(asset);
        if let Some(list) = children.get(&asset.item_id) {
            stack.extend(list.iter().rev());
        }
    }

    let mut tx = pool.begin().await?;

    for (index, asset) in ordered.iter().enumerate() {
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
        .bind(corporation_item_ids.contains(&asset.item_id).then_some(corporation_id).flatten())
        .bind(asset.item_id)
        .bind(asset.type_id)
        .bind(names.get(&asset.item_id))
        .bind(asset.location_id)
        .bind(&asset.location_flag)
        .bind(&asset.location_type)
        .bind(asset.quantity)
        .bind(index as i64)
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
