//! Public (published) assets, ported from the legacy
//! `CreatePublicAssetAction` / `DeletePublicAssetAction` /
//! `UpdatePublicAssetsAction` and the `after_public_asset` trigger.
//!
//! Publishing an asset makes it and its whole descendant subtree public;
//! every abyssal module in that subtree becomes a `public_module_ownership`
//! row, which is what surfaces the module on the owner's character page and
//! populates the `public_asset` key on the module resource. Every asset
//! import re-derives the published subtrees from the fresh inventory, so
//! modules moved into a published container appear on the sell page and
//! modules moved out of one leave it without the owner toggling anything.

use sqlx::{PgConnection, PgPool};

/// The publish outcome, mainly for the caller's redirect/notification.
#[derive(Debug)]
pub enum PublishError {
    /// The asset does not exist or is not owned by the user.
    NotOwned,
    Db(sqlx::Error),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::NotOwned => write!(f, "asset not owned by user"),
            PublishError::Db(error) => write!(f, "database error: {error}"),
        }
    }
}

impl From<sqlx::Error> for PublishError {
    fn from(error: sqlx::Error) -> Self {
        PublishError::Db(error)
    }
}

/// Makes an owned asset and its descendant subtree public. The asset must
/// belong to a character of the given user. Idempotent: re-publishing
/// upserts the same rows.
pub async fn publish_asset(pool: &PgPool, user_id: i64, asset_id: i64) -> Result<(), PublishError> {
    // The asset must belong to one of the user's characters.
    let owner: Option<i64> = sqlx::query_scalar(
        "select a.character_id from assets a
         join characters c on c.id = a.character_id
         where a.id = $1 and c.user_id = $2",
    )
    .bind(asset_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some(character_id) = owner else {
        return Err(PublishError::NotOwned);
    };

    let mut tx = pool.begin().await?;

    // The published asset itself is the root public asset; the subtree
    // below it is derived like on every import.
    sqlx::query(
        "insert into public_assets (character_id, asset_id, public_parent_id, module_id)
         select a.character_id, a.id, null,
                case when a.is_abyssal then a.item_id else null end
         from assets a where a.id = $1
         on conflict (character_id, asset_id) do update
         set public_parent_id = null, module_id = excluded.module_id, updated_at = now()",
    )
    .bind(asset_id)
    .execute(&mut *tx)
    .await?;

    refresh_published_subtrees(&mut tx, character_id).await?;

    tx.commit().await?;
    Ok(())
}

/// Every asset in the subtree of each of the character's published roots
/// (child.location_id = parent.item_id, same character), tagged with the
/// root and its distance from it so an asset under nested published
/// containers can be attributed to the nearest one. A macro so the
/// statements below can splice it into their literal SQL.
macro_rules! published_subtrees {
    () => {
        "with recursive roots as (
             select pa.id as root_id, pa.asset_id
             from public_assets pa
             where pa.character_id = $1 and pa.public_parent_id is null
         ),
         subtree as (
             select r.root_id, a.id as asset_id, a.item_id, 0 as depth
             from roots r join assets a on a.id = r.asset_id
             union all
             select s.root_id, child.id, child.item_id, s.depth + 1
             from subtree s
             join assets child on child.location_id = s.item_id and child.character_id = $1
         )"
    };
}

/// Re-derives the character's published subtrees from the current asset
/// rows: assets now inside a published root get a public row under it
/// (their abyssal modules an ownership row), rows whose asset is no longer
/// inside its root go, and the roots themselves are left alone. The legacy
/// `UpdatePublicAssetsAction` ran this after every asset import; it only
/// looked one level deep, so a module inside a container inside a published
/// ship was unpublished by the next import. Here the whole subtree counts,
/// matching `publish_asset` and the select-modules dialog's module count.
///
/// Ownership rows need the module row to exist (the legacy trigger's
/// `INSERT IGNORE`), so callers run this after the import ingested the
/// modules. Each statement only touches rows that actually change.
pub async fn refresh_published_subtrees(
    conn: &mut PgConnection,
    character_id: i64,
) -> sqlx::Result<()> {
    // Roots stay roots (a container published inside another published
    // container keeps its own toggle); everything else in a subtree hangs
    // off its nearest root.
    sqlx::query(concat!(
        published_subtrees!(),
        " insert into public_assets (character_id, asset_id, public_parent_id, module_id)
         select distinct on (s.asset_id)
                $1, s.asset_id, s.root_id,
                case when a.is_abyssal then a.item_id else null end
         from subtree s
         join assets a on a.id = s.asset_id
         where s.depth > 0
           and s.asset_id not in (select asset_id from roots)
         order by s.asset_id, s.depth, s.root_id
         on conflict (character_id, asset_id) do update
         set public_parent_id = excluded.public_parent_id,
             module_id = excluded.module_id, updated_at = now()
         where (public_assets.public_parent_id, public_assets.module_id)
               is distinct from (excluded.public_parent_id, excluded.module_id)",
    ))
    .bind(character_id)
    .execute(&mut *conn)
    .await?;

    // Ownership rows cascade away with their public asset.
    sqlx::query(concat!(
        published_subtrees!(),
        " delete from public_assets child
         where child.character_id = $1 and child.public_parent_id is not null
           and not exists (select 1 from subtree s
                           where s.root_id = child.public_parent_id
                             and s.asset_id = child.asset_id)",
    ))
    .bind(character_id)
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id, public_asset_id)
         select pa.character_id, pa.module_id, pa.id
         from public_assets pa
         where pa.character_id = $1 and pa.module_id is not null
           and exists (select 1 from modules m where m.id = pa.module_id)
         on conflict (character_id, module_id) do update
         set public_asset_id = excluded.public_asset_id, updated_at = now()
         where public_module_ownerships.public_asset_id is distinct from excluded.public_asset_id",
    )
    .bind(character_id)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Unpublishes a public asset by id (owner only); the cascade removes its
/// descendants and their ownership rows.
pub async fn unpublish_asset(
    pool: &PgPool,
    user_id: i64,
    public_asset_id: i64,
) -> Result<(), PublishError> {
    let owner: Option<i64> = sqlx::query_scalar(
        "select pa.id from public_assets pa
         join characters c on c.id = pa.character_id
         where pa.id = $1 and c.user_id = $2",
    )
    .bind(public_asset_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if owner.is_none() {
        return Err(PublishError::NotOwned);
    }

    // Ownership rows reference the public asset with ON DELETE CASCADE, so
    // deleting the root (and its cascading descendants) clears them.
    sqlx::query("delete from public_assets where id = $1 or public_parent_id = $1")
        .bind(public_asset_id)
        .execute(pool)
        .await?;

    Ok(())
}
