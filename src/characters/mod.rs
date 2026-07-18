//! Character domain: naming the stub rows created during ingestion, ported
//! from the legacy `GetCharacterNamesCommand` + `UpdateCharacterNamesAction`.
//!
//! Contract issuers and module creators are inserted as empty-named stubs;
//! this fills their names from ESI's bulk names endpoint. ESI rejects the
//! whole batch when any single id is unresolvable (biomassed characters),
//! so failed batches are bisected until the poison ids are isolated and
//! stamped as fetched-without-name, exactly like the legacy command.

use sqlx::PgPool;

use crate::esi::{EsiClient, EsiError};

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
