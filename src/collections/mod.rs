//! Collections domain, ported from the legacy CollectionController and its
//! actions/policy: random-identifier slugs, visibility policy (private
//! collections are owner-only), and the collection-module CRUD.

use rand::Rng;
use sqlx::{PgPool, Row};

/// Valid `visibility` values (legacy CollectionVisibility enum).
pub const COLLECTION_VISIBILITIES: [&str; 3] = ["public", "private", "unlisted"];

/// Length of the random identifier, like Laravel's `Str::random()` default.
const IDENTIFIER_LENGTH: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Collection {
    pub id: i64,
    pub identifier: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub character_id: i64,
    /// The owning character's user, for the view/update policy.
    pub owner_user_id: Option<i64>,
}

impl Collection {
    /// `{slug(name)}-{identifier}`, the legacy slug accessor.
    pub fn slug(&self) -> String {
        format!("{}-{}", crate::modules::view::slugify(&self.name), self.identifier)
    }

    /// The legacy view policy: private collections are owner-only.
    pub fn viewable_by(&self, user_id: Option<i64>) -> bool {
        self.visibility != "private" || (self.owner_user_id.is_some() && self.owner_user_id == user_id)
    }

    pub fn owned_by(&self, user_id: i64) -> bool {
        self.owner_user_id == Some(user_id)
    }
}

/// The legacy `Str::lower(Str::random())`: 16 random alphanumeric
/// characters, lowercased.
pub fn random_identifier() -> String {
    /// Alphanumeric charset after lowercasing.
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

    let mut rng = rand::rng();
    (0..IDENTIFIER_LENGTH).map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char).collect()
}

/// The legacy HasSlug/route binding: the trailing dash segment.
pub fn identifier_from_slug(slug: &str) -> &str {
    slug.rsplit('-').next().unwrap_or(slug)
}

fn collection_from_row(row: &sqlx::postgres::PgRow) -> Collection {
    Collection {
        id: row.get("id"),
        identifier: row.get("identifier"),
        name: row.get("name"),
        description: row.get("description"),
        visibility: row.get("visibility"),
        character_id: row.get("character_id"),
        owner_user_id: row.get("owner_user_id"),
    }
}

/// Resolves a collection by its URL slug (trailing identifier segment).
pub async fn collection_by_slug(pool: &PgPool, slug: &str) -> sqlx::Result<Option<Collection>> {
    let row = sqlx::query(
        "select cl.id, cl.identifier, cl.name, cl.description, cl.visibility,
                cl.character_id, c.user_id as owner_user_id
         from collections cl join characters c on c.id = cl.character_id
         where cl.identifier = $1",
    )
    .bind(identifier_from_slug(slug))
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(collection_from_row))
}

pub async fn collection_by_id(pool: &PgPool, id: i64) -> sqlx::Result<Option<Collection>> {
    let row = sqlx::query(
        "select cl.id, cl.identifier, cl.name, cl.description, cl.visibility,
                cl.character_id, c.user_id as owner_user_id
         from collections cl join characters c on c.id = cl.character_id
         where cl.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(collection_from_row))
}

/// Creates a collection for the character, like StoreCollectionAction.
pub async fn create_collection(
    pool: &PgPool,
    character_id: i64,
    name: &str,
    description: Option<&str>,
    visibility: &str,
) -> sqlx::Result<Collection> {
    let row = sqlx::query(
        "insert into collections (identifier, name, description, visibility, character_id)
         values ($1, $2, $3, $4, $5)
         returning id, identifier, name, description, visibility, character_id,
                   (select user_id from characters where id = character_id) as owner_user_id",
    )
    .bind(random_identifier())
    .bind(name)
    .bind(description)
    .bind(visibility)
    .bind(character_id)
    .fetch_one(pool)
    .await?;

    Ok(collection_from_row(&row))
}

pub async fn update_collection(
    pool: &PgPool,
    collection_id: i64,
    name: &str,
    description: Option<&str>,
    visibility: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "update collections set name = $1, description = $2, visibility = $3, updated_at = now()
         where id = $4",
    )
    .bind(name)
    .bind(description)
    .bind(visibility)
    .bind(collection_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_collection(pool: &PgPool, collection_id: i64) -> sqlx::Result<()> {
    sqlx::query("delete from collections where id = $1").bind(collection_id).execute(pool).await?;
    Ok(())
}

/// Adds a module to a collection (no-op when already present, like the
/// legacy firstOrCreate).
pub async fn add_collection_module(
    pool: &PgPool,
    collection_id: i64,
    module_id: i64,
    note: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "insert into collection_modules (collection_id, module_id, note)
         values ($1, $2, $3) on conflict (collection_id, module_id) do nothing",
    )
    .bind(collection_id)
    .bind(module_id)
    .bind(note)
    .execute(pool)
    .await?;

    Ok(())
}

/// One row of the public collections index.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionListing {
    pub collection: Collection,
    pub character_name: String,
    pub character_has_premium: bool,
    pub modules_count: i64,
}

/// Collections per index page, like the legacy paginate(12).
pub const COLLECTIONS_PAGE_SIZE: i64 = 12;

/// The public collections with modules, searchable across name,
/// description and owner name. Divergence from legacy: ordered premium
/// owners first then by id (the legacy MySQL CASE mixing admin flags with
/// datetimes does not translate to Postgres).
pub async fn collections_index(
    pool: &PgPool,
    search: Option<&str>,
    page: i64,
) -> sqlx::Result<Vec<CollectionListing>> {
    let rows = sqlx::query(
        "select cl.id, cl.identifier, cl.name, cl.description, cl.visibility, cl.character_id,
                c.user_id as owner_user_id, c.name as character_name,
                (c.premium_paid_until is not null and c.premium_paid_until > now())
                    as character_has_premium,
                count(cm.id) as modules_count
         from collections cl
         join characters c on c.id = cl.character_id
         join collection_modules cm on cm.collection_id = cl.id
         where cl.visibility = 'public'
           and ($1::text is null or cl.name ilike '%' || $1 || '%'
                or cl.description ilike '%' || $1 || '%' or c.name ilike '%' || $1 || '%')
         group by cl.id, c.user_id, c.name, c.premium_paid_until
         order by (c.premium_paid_until is not null and c.premium_paid_until > now()) desc, cl.id
         limit $2 offset $3",
    )
    .bind(search)
    .bind(COLLECTIONS_PAGE_SIZE)
    .bind((page.max(1) - 1) * COLLECTIONS_PAGE_SIZE)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| CollectionListing {
            collection: collection_from_row(row),
            character_name: row.get("character_name"),
            character_has_premium: row.get("character_has_premium"),
            modules_count: row.get("modules_count"),
        })
        .collect())
}

/// The logged-in user's own collections, every visibility (the legacy
/// personal_collections section of the index).
pub async fn collections_index_for_user(
    pool: &PgPool,
    user_id: i64,
) -> sqlx::Result<Vec<CollectionListing>> {
    let rows = sqlx::query(
        "select cl.id, cl.identifier, cl.name, cl.description, cl.visibility, cl.character_id,
                c.user_id as owner_user_id, c.name as character_name,
                (c.premium_paid_until is not null and c.premium_paid_until > now())
                    as character_has_premium,
                count(cm.id) as modules_count
         from collections cl
         join characters c on c.id = cl.character_id
         left join collection_modules cm on cm.collection_id = cl.id
         where c.user_id = $1
         group by cl.id, c.user_id, c.name, c.premium_paid_until
         order by cl.id desc",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| CollectionListing {
            collection: collection_from_row(row),
            character_name: row.get("character_name"),
            character_has_premium: row.get("character_has_premium"),
            modules_count: row.get("modules_count"),
        })
        .collect())
}

/// Distinct module types per collection (most frequent first), for the
/// card icon strips.
pub async fn collection_type_ids(
    pool: &PgPool,
    collection_ids: &[i64],
) -> sqlx::Result<std::collections::HashMap<i64, Vec<i64>>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "select cm.collection_id, m.type_id
         from collection_modules cm
         join modules m on m.id = cm.module_id
         where cm.collection_id = any($1)
         group by cm.collection_id, m.type_id
         order by cm.collection_id, count(*) desc",
    )
    .bind(collection_ids)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (collection_id, type_id) in rows {
        map.entry(collection_id).or_default().push(type_id);
    }
    Ok(map)
}

/// The module ids of a collection, newest link first (legacy default order
/// is the primary key).
pub async fn collection_module_ids(pool: &PgPool, collection_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("select module_id from collection_modules where collection_id = $1 order by id")
        .bind(collection_id)
        .fetch_all(pool)
        .await
}

// --- Collection locations (bulk add/sync/remove per asset location) and
// --- auto-sync, ported from the legacy CollectionLocation actions,
// --- CollectionAutoSync actions and SyncCollectionWithLocationsAction.

/// Everything at or below one asset row, as item ids: the legacy
/// adjacency-list `ancestorsAndSelf` walk from a module upward, taken
/// downward from the location. The base row is pinned to the owner
/// ($1 = user id via characters, $2 = assets.id); the recursion follows
/// `location_id -> item_id` chains with no character filter, exactly
/// like the legacy CTE (which walks the whole assets table and only
/// filters the selected ancestor row).
const USER_LOCATION_SCOPE_CTE: &str = "
    with recursive scope as (
        select a.item_id from assets a
        join characters ch on ch.id = a.character_id
        where ch.user_id = $1 and a.id = $2
        union
        select a.item_id from assets a join scope s on a.location_id = s.item_id
    )";

/// The same walk pinned to one character ($1 = character id, $2 =
/// assets.id), for auto-sync (the legacy SyncCollectionWithLocationsAction
/// scopes to the collection's character only, not the whole account).
const CHARACTER_LOCATION_SCOPE_CTE: &str = "
    with recursive scope as (
        select a.item_id from assets a
        where a.character_id = $1 and a.id = $2
        union
        select a.item_id from assets a join scope s on a.location_id = s.item_id
    )";

/// The legacy StoreCollectionLocationAction: insert-or-ignore every module
/// whose asset sits at or below the location and belongs to one of the
/// user's characters.
pub async fn add_location_modules<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    user_id: i64,
    collection_id: i64,
    location_asset_id: i64,
) -> sqlx::Result<u64> {
    let result = sqlx::query(&format!(
        "{USER_LOCATION_SCOPE_CTE}
         insert into collection_modules (collection_id, module_id)
         select $3, m.id from modules m
         where m.id in (select item_id from scope)
           and exists (select 1 from assets a join characters ch on ch.id = a.character_id
                       where ch.user_id = $1 and a.item_id = m.id)
         on conflict (collection_id, module_id) do nothing",
    ))
    .bind(user_id)
    .bind(location_asset_id)
    .bind(collection_id)
    .execute(executor)
    .await?;

    Ok(result.rows_affected())
}

/// The legacy DeleteCollectionLocationAction. Legacy quirk kept: unlike
/// the store, the module's own asset row carries no character filter here
/// (only the location ancestor is pinned to the user), so any collection
/// module physically inside the location is removed.
pub async fn remove_location_modules(
    pool: &PgPool,
    user_id: i64,
    collection_id: i64,
    location_asset_id: i64,
) -> sqlx::Result<u64> {
    let result = sqlx::query(&format!(
        "{USER_LOCATION_SCOPE_CTE}
         delete from collection_modules cm
         where cm.collection_id = $3
           and cm.module_id in (select item_id from scope)",
    ))
    .bind(user_id)
    .bind(location_asset_id)
    .bind(collection_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// The legacy SyncCollectionLocationAction: clear the collection and
/// refill it from one location, in one transaction.
pub async fn sync_location_modules(
    pool: &PgPool,
    user_id: i64,
    collection_id: i64,
    location_asset_id: i64,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("delete from collection_modules where collection_id = $1")
        .bind(collection_id)
        .execute(&mut *tx)
        .await?;
    add_location_modules(&mut *tx, user_id, collection_id, location_asset_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The legacy EnableCollectionAutoSyncAction: flip auto_sync on, seed the
/// tracked locations, and run the initial sync, all in one transaction.
pub async fn enable_auto_sync(
    pool: &PgPool,
    collection_id: i64,
    character_id: i64,
    location_asset_ids: &[i64],
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("update collections set auto_sync = true, updated_at = now() where id = $1")
        .bind(collection_id)
        .execute(&mut *tx)
        .await?;
    if !location_asset_ids.is_empty() {
        sqlx::query(
            "insert into collection_locations (collection_id, asset_id)
             select $1, asset_id from unnest($2::bigint[]) as asset_id
             on conflict (collection_id, asset_id) do nothing",
        )
        .bind(collection_id)
        .bind(location_asset_ids)
        .execute(&mut *tx)
        .await?;
    }
    sync_with_locations_tx(&mut tx, collection_id, character_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The legacy DisableCollectionAutoSyncAction: clear the tracked
/// locations and flip auto_sync off; the current modules are kept.
pub async fn disable_auto_sync(pool: &PgPool, collection_id: i64) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("delete from collection_locations where collection_id = $1")
        .bind(collection_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "update collections set auto_sync = false, last_synced_at = null, updated_at = now()
         where id = $1",
    )
    .bind(collection_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// The legacy StoreCollectionAutoSyncLocationAction. The insert commits
/// on its own rather than sharing the sync's transaction: in legacy,
/// sharing one held the new row's locks for the whole re-sync (deadlocking
/// concurrent syncs) and nested the sync's transaction; the tracked row
/// also survives a failing sync (pinned by a legacy test).
pub async fn add_auto_sync_location(
    pool: &PgPool,
    collection_id: i64,
    character_id: i64,
    asset_id: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "insert into collection_locations (collection_id, asset_id) values ($1, $2)
         on conflict (collection_id, asset_id) do nothing",
    )
    .bind(collection_id)
    .bind(asset_id)
    .execute(pool)
    .await?;

    sync_with_locations(pool, collection_id, character_id).await?;
    Ok(())
}

/// The legacy DeleteCollectionAutoSyncLocationAction: untrack the
/// location and re-sync from the remaining ones, in one transaction.
pub async fn remove_auto_sync_location(
    pool: &PgPool,
    collection_id: i64,
    character_id: i64,
    asset_id: i64,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("delete from collection_locations where collection_id = $1 and asset_id = $2")
        .bind(collection_id)
        .bind(asset_id)
        .execute(&mut *tx)
        .await?;
    sync_with_locations_tx(&mut tx, collection_id, character_id).await?;
    tx.commit().await?;
    Ok(())
}

/// The legacy SyncCollectionWithLocationsAction: no-op unless the
/// collection is auto-sync, otherwise rebuild its modules from every
/// tracked location in one transaction.
pub async fn sync_with_locations(
    pool: &PgPool,
    collection_id: i64,
    character_id: i64,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sync_with_locations_tx(&mut tx, collection_id, character_id).await?;
    tx.commit().await?;
    Ok(())
}

async fn sync_with_locations_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    collection_id: i64,
    character_id: i64,
) -> sqlx::Result<()> {
    let auto_sync: Option<bool> =
        sqlx::query_scalar("select auto_sync from collections where id = $1")
            .bind(collection_id)
            .fetch_optional(&mut **tx)
            .await?;
    if auto_sync != Some(true) {
        return Ok(());
    }

    // The legacy cleanupStaleLocations. The asset_id foreign key already
    // cascades deletes (as it did in legacy), so this is the same
    // belt-and-braces sweep the legacy action carries.
    sqlx::query(
        "delete from collection_locations cl
         where cl.collection_id = $1
           and not exists (select 1 from assets a where a.id = cl.asset_id)",
    )
    .bind(collection_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query("delete from collection_modules where collection_id = $1")
        .bind(collection_id)
        .execute(&mut **tx)
        .await?;

    let asset_ids: Vec<i64> = sqlx::query_scalar(
        "select asset_id from collection_locations where collection_id = $1 order by id",
    )
    .bind(collection_id)
    .fetch_all(&mut **tx)
    .await?;

    for asset_id in asset_ids {
        sqlx::query(&format!(
            "{CHARACTER_LOCATION_SCOPE_CTE}
             insert into collection_modules (collection_id, module_id)
             select $3, m.id from modules m
             where m.id in (select item_id from scope)
               and exists (select 1 from assets a
                           where a.character_id = $1 and a.item_id = m.id)
             on conflict (collection_id, module_id) do nothing",
        ))
        .bind(character_id)
        .bind(asset_id)
        .bind(collection_id)
        .execute(&mut **tx)
        .await?;
    }

    sqlx::query("update collections set last_synced_at = now(), updated_at = now() where id = $1")
        .bind(collection_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

/// The legacy SyncAutoSyncCollectionsJob body: re-sync every auto-sync
/// collection owned by the character. Returns how many were synced.
pub async fn sync_auto_sync_collections(pool: &PgPool, character_id: i64) -> sqlx::Result<usize> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "select id from collections where character_id = $1 and auto_sync order by id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    for collection_id in &ids {
        sync_with_locations(pool, *collection_id, character_id).await?;
    }

    Ok(ids.len())
}
