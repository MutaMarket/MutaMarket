//! Character domain: naming the stub rows created during ingestion, ported
//! from the legacy `GetCharacterNamesCommand` + `UpdateCharacterNamesAction`.
//!
//! Contract issuers and module creators are inserted as empty-named stubs;
//! this fills their names from ESI's bulk names endpoint. ESI rejects the
//! whole batch when any single id is unresolvable (biomassed characters),
//! so failed batches are bisected until the poison ids are isolated and
//! stamped as fetched-without-name, exactly like the legacy command.

use sqlx::{PgPool, Row};

use crate::esi::{EsiClient, EsiError};

/// The legacy HasSlug route binding: the trailing dash segment must be a
/// nonzero integer id.
pub fn character_id_from_slug(slug: &str) -> Option<i64> {
    slug.rsplit('-').next().and_then(|segment| segment.parse().ok()).filter(|&id| id != 0)
}

/// A character with the fields the CharacterResource emits.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CharacterView {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub has_premium: bool,
    pub corporation_id: Option<i64>,
    /// Public ownership count; present on the index (whenCounted).
    pub modules_count: Option<i64>,
    /// The owning user's id, for owner-only sections (not emitted).
    #[serde(skip)]
    pub user_id: Option<i64>,
}

fn character_from_row(row: &sqlx::postgres::PgRow) -> CharacterView {
    let id: i64 = row.get("id");
    let name: String = row.get("name");

    CharacterView {
        id,
        slug: crate::modules::view::module_slug(&name, id),
        name,
        description: row.get("description"),
        has_premium: row.get("has_premium"),
        corporation_id: row.get("corporation_id"),
        modules_count: row.try_get("modules_count").ok(),
        user_id: row.get("user_id"),
    }
}

pub async fn character_by_id(pool: &PgPool, id: i64) -> sqlx::Result<Option<CharacterView>> {
    let row = sqlx::query(
        "select id, name, description, corporation_id, user_id, null::bigint as modules_count,
                (premium_paid_until is not null and premium_paid_until > now()) as has_premium
         from characters where id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(character_from_row))
}

/// Characters per index page, like the legacy paginate(32).
pub const CHARACTERS_PAGE_SIZE: i64 = 32;

/// The characters index: only characters with public ownerships, optional
/// name search, premium members first (most recent premium expiry first,
/// like the legacy CASE ordering).
pub async fn characters_index(
    pool: &PgPool,
    search: Option<&str>,
    page: i64,
) -> sqlx::Result<Vec<CharacterView>> {
    let rows = sqlx::query(
        "select c.id, c.name, c.description, c.corporation_id, c.user_id,
                (c.premium_paid_until is not null and c.premium_paid_until > now()) as has_premium,
                count(o.id) as modules_count
         from characters c
         join public_module_ownerships o on o.character_id = c.id
         where ($1::text is null or c.name ilike '%' || $1 || '%')
         group by c.id
         order by (c.premium_paid_until is not null) desc,
                  c.premium_paid_until desc nulls last, c.id
         limit $2 offset $3",
    )
    .bind(search)
    .bind(CHARACTERS_PAGE_SIZE)
    .bind((page.max(1) - 1) * CHARACTERS_PAGE_SIZE)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(character_from_row).collect())
}

/// Module ids publicly owned by the character, newest first — the show
/// page's default set (`whereVisibleByCharacter`).
pub async fn publicly_owned_module_ids(
    pool: &PgPool,
    character_id: i64,
    limit: i64,
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select o.module_id from public_module_ownerships o
         where o.character_id = $1 order by o.module_id desc limit $2",
    )
    .bind(character_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Module ids created by the character, the show page's `created` mode.
pub async fn created_module_ids(
    pool: &PgPool,
    character_id: i64,
    limit: i64,
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select id from modules where creator_id = $1 order by id desc limit $2",
    )
    .bind(character_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Ids per ESI names request, like the legacy command's chunk size.
const NAME_CHUNK_SIZE: usize = 256;

/// Errors of a name sync run. ESI failures inside a chunk are handled by
/// bisection; only database errors abort the run.
#[derive(Debug)]
pub enum NameSyncError {
    Db(sqlx::Error),
}

impl std::fmt::Display for NameSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameSyncError::Db(error) => write!(f, "database error: {error}"),
        }
    }
}

impl From<sqlx::Error> for NameSyncError {
    fn from(error: sqlx::Error) -> Self {
        NameSyncError::Db(error)
    }
}

/// Names every character still missing a fetch stamp. Returns how many
/// characters were named.
pub async fn sync_character_names(pool: &PgPool, esi: &EsiClient) -> Result<usize, NameSyncError> {
    // Random order like the legacy command, so a poison id cannot starve
    // the same tail of the queue every run.
    let ids: Vec<i64> = sqlx::query_scalar(
        "select id from characters where name_fetched_at is null order by random()",
    )
    .fetch_all(pool)
    .await?;

    let mut named = 0;
    for chunk in ids.chunks(NAME_CHUNK_SIZE) {
        named += fetch_chunk(pool, esi, chunk).await?;
    }

    Ok(named)
}

/// Fetches one chunk, bisecting on batch rejection like the legacy
/// `handleFailedResponse`: a rejected single id is stamped as fetched so it
/// is never retried.
async fn fetch_chunk(pool: &PgPool, esi: &EsiClient, chunk: &[i64]) -> Result<usize, NameSyncError> {
    // Iterative bisection (a recursive async fn would need boxing).
    let mut queue: Vec<&[i64]> = vec![chunk];
    let mut named = 0;

    while let Some(batch) = queue.pop() {
        match esi.names(batch).await {
            Ok(names) => {
                let characters: Vec<&crate::esi::EsiName> =
                    names.iter().filter(|name| name.category == "character").collect();

                sqlx::query(
                    "update characters set name = data.name, name_fetched_at = now()
                     from (select * from unnest($1::bigint[], $2::text[])) as data (id, name)
                     where characters.id = data.id",
                )
                .bind(characters.iter().map(|name| name.id).collect::<Vec<_>>())
                .bind(characters.iter().map(|name| name.name.clone()).collect::<Vec<_>>())
                .execute(pool)
                .await?;

                named += characters.len();
            }
            Err(EsiError::NotFound) if batch.len() <= 1 => {
                // The id itself is unresolvable; give up on it for good.
                sqlx::query("update characters set name_fetched_at = now() where id = any($1)")
                    .bind(batch)
                    .execute(pool)
                    .await?;
            }
            Err(EsiError::NotFound) => {
                let half = batch.len().div_ceil(2);
                queue.push(&batch[..half]);
                queue.push(&batch[half..]);
            }
            Err(error) => {
                // Transient ESI trouble: leave the batch unstamped so the
                // next run retries it, like the legacy catch-and-log.
                eprintln!("character names for {} ids failed: {error}", batch.len());
            }
        }
    }

    Ok(named)
}
