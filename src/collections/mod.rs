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
            modules_count: row.get("modules_count"),
        })
        .collect())
}

/// The module ids of a collection, newest link first (legacy default order
/// is the primary key).
pub async fn collection_module_ids(pool: &PgPool, collection_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("select module_id from collection_modules where collection_id = $1 order by id")
        .bind(collection_id)
        .fetch_all(pool)
        .await
}
