//! Public (published) assets, ported from the legacy
//! `CreatePublicAssetAction` / `DeletePublicAssetAction` and the
//! `after_public_asset` trigger.
//!
//! Publishing an asset makes it and its whole descendant subtree public;
//! every abyssal module in that subtree becomes a `public_module_ownership`
//! row, which is what surfaces the module on the owner's character page and
//! populates the `public_asset` key on the module resource.

use sqlx::{PgPool, Row};

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

    // The published asset itself is the root public asset.
    let root_id: i64 = sqlx::query_scalar(
        "insert into public_assets (character_id, asset_id, public_parent_id, module_id)
         select a.character_id, a.id, null,
                case when a.is_abyssal then a.item_id else null end
         from assets a where a.id = $1
         on conflict (character_id, asset_id) do update
         set updated_at = now(), module_id = excluded.module_id
         returning id",
    )
    .bind(asset_id)
    .fetch_one(&mut *tx)
    .await?;

    // The descendant subtree: assets whose container chain leads back to
    // the root (child.location_id = parent.item_id, same character).
    let descendants = sqlx::query(
        "with recursive subtree as (
             select a.id, a.item_id, a.is_abyssal from assets a where a.id = $1
             union all
             select child.id, child.item_id, child.is_abyssal
             from assets child join subtree on child.location_id = subtree.item_id
             where child.character_id = $2
         )
         select id, item_id, is_abyssal from subtree where id <> $1",
    )
    .bind(asset_id)
    .bind(character_id)
    .fetch_all(&mut *tx)
    .await?;

    for row in &descendants {
        let descendant_id: i64 = row.get("id");
        let item_id: i64 = row.get("item_id");
        let is_abyssal: bool = row.get("is_abyssal");
        let module_id = is_abyssal.then_some(item_id);

        sqlx::query(
            "insert into public_assets (character_id, asset_id, public_parent_id, module_id)
             values ($1, $2, $3, $4)
             on conflict (character_id, asset_id) do update
             set public_parent_id = excluded.public_parent_id,
                 module_id = excluded.module_id, updated_at = now()",
        )
        .bind(character_id)
        .bind(descendant_id)
        .bind(root_id)
        .bind(module_id)
        .execute(&mut *tx)
        .await?;
    }

    // Ownership rows for every abyssal module in the subtree (root included),
    // the legacy after_public_asset trigger.
    sqlx::query(
        "insert into public_module_ownerships (character_id, module_id, public_asset_id)
         select pa.character_id, pa.module_id, pa.id
         from public_assets pa
         where pa.character_id = $1 and pa.module_id is not null
           and (pa.id = $2 or pa.public_parent_id = $2)
         on conflict (character_id, module_id) do update
         set public_asset_id = excluded.public_asset_id, updated_at = now()",
    )
    .bind(character_id)
    .bind(root_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
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
