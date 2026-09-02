//! Structure resolution, ported from the legacy `GetPublicStructuresCommand`
//! → `GetPublicStructuresJob` → `GetStructureJob` chain: sweep the public
//! structure list into id stubs and resolve names per character through
//! the structures scope, recording which characters can see which
//! structures.
//!
//! Scope note: structure reads use `esi-universe.read_structures.v1`,
//! the same scope the legacy app requested (see `crate::auth::scopes`;
//! an earlier claim of a CCP rename was wrong).

use sqlx::{PgPool, Row};

use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiClient, EsiError};

/// A named structure is refreshed after this long, the legacy
/// `updated_at->diffInDays() < 7` guard in `GetStructureJob`.
const REFRESH_AFTER_DAYS: i32 = 7;

#[derive(Debug, Default, Clone, Copy)]
pub struct StructureSweepStats {
    pub total: usize,
    pub resolved: usize,
    pub unresolved: usize,
    pub skipped: usize,
}

#[derive(Debug)]
pub enum StructureSyncError {
    Token(TokenError),
    Esi(EsiError),
    Db(sqlx::Error),
}

impl std::fmt::Display for StructureSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructureSyncError::Token(error) => write!(f, "token: {error}"),
            StructureSyncError::Esi(error) => write!(f, "ESI: {error}"),
            StructureSyncError::Db(error) => write!(f, "database: {error}"),
        }
    }
}

impl std::error::Error for StructureSyncError {}

impl From<TokenError> for StructureSyncError {
    fn from(error: TokenError) -> Self {
        StructureSyncError::Token(error)
    }
}

impl From<EsiError> for StructureSyncError {
    fn from(error: EsiError) -> Self {
        StructureSyncError::Esi(error)
    }
}

impl From<sqlx::Error> for StructureSyncError {
    fn from(error: sqlx::Error) -> Self {
        StructureSyncError::Db(error)
    }
}

/// The outcome of one structure's resolution attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureOutcome {
    Resolved,
    /// ESI denied or failed the detail call; the character is recorded as
    /// unable to resolve the structure.
    Unresolved,
    /// The skip guard or a missing token short-circuited the attempt.
    Skipped,
}

/// Sweeps the public structure list into stubs and resolves each through
/// the given character, the legacy `GetPublicStructuresJob` fan-out.
pub async fn sync_public_structures(
    pool: &PgPool,
    esi: &EsiClient,
    sso: &SsoClient,
    character_id: i64,
) -> Result<StructureSweepStats, StructureSyncError> {
    let mut structure_ids: Vec<i64> = Vec::new();
    let mut page = 1;
    loop {
        let (mut batch, pages) = esi.public_structures(page).await?;
        structure_ids.append(&mut batch);
        if page >= pages {
            break;
        }
        page += 1;
    }

    let mut stats = StructureSweepStats {
        total: structure_ids.len(),
        ..Default::default()
    };

    for structure_id in structure_ids {
        match sync_structure(pool, esi, sso, character_id, structure_id).await? {
            StructureOutcome::Resolved => stats.resolved += 1,
            StructureOutcome::Unresolved => stats.unresolved += 1,
            StructureOutcome::Skipped => stats.skipped += 1,
        }
    }

    Ok(stats)
}

/// Ensures the structure row exists and tries to resolve its sheet with
/// the given character's token, the legacy `GetStructureJob`.
pub async fn sync_structure(
    pool: &PgPool,
    esi: &EsiClient,
    sso: &SsoClient,
    character_id: i64,
    structure_id: i64,
) -> Result<StructureOutcome, StructureSyncError> {
    sqlx::query("insert into structures (id) values ($1) on conflict (id) do nothing")
        .bind(structure_id)
        .execute(pool)
        .await?;

    // The legacy skip guard, PHP operator precedence included: skip when
    // (named AND fresher than a week AND this character already failed on
    // it) OR the character lacks the scope. A structure this character
    // resolved fine is refetched every sweep — only known failures are
    // spared inside the freshness window.
    let guard = sqlx::query(
        "select s.name is not null as named,
                s.updated_at > now() - make_interval(days => $1) as fresh,
                exists (
                    select 1 from character_structure cs
                    where cs.character_id = $2 and cs.structure_id = s.id
                      and not cs.could_resolve
                ) as known_failure
         from structures s where s.id = $3",
    )
    .bind(REFRESH_AFTER_DAYS)
    .bind(character_id)
    .bind(structure_id)
    .fetch_one(pool)
    .await?;

    if guard.get::<bool, _>("named")
        && guard.get::<bool, _>("fresh")
        && guard.get::<bool, _>("known_failure")
    {
        return Ok(StructureOutcome::Skipped);
    }

    let Some(token) =
        tokens::valid_access_token(pool, sso, character_id, scopes::READ_STRUCTURES).await?
    else {
        return Ok(StructureOutcome::Skipped);
    };

    match esi.structure(&token.access_token, structure_id).await {
        Ok(structure) => {
            sqlx::query(
                "update structures set
                     name = $1, owner_id = $2, type_id = $3, solarsystem_id = $4,
                     last_fetched_at = now(), updated_at = now()
                 where id = $5",
            )
            .bind(&structure.name)
            .bind(structure.owner_id)
            .bind(structure.type_id)
            .bind(structure.solar_system_id)
            .bind(structure_id)
            .execute(pool)
            .await?;

            record_resolution(pool, character_id, structure_id, true).await?;
            Ok(StructureOutcome::Resolved)
        }
        Err(error) => {
            // 403 (no docking access) is the routine case and stays
            // silent, like the legacy job that only logs 404s and 5xx.
            match &error {
                EsiError::Forbidden(status) => {
                    // The legacy connector deletes the token on any
                    // 401/403 — including a merely inaccessible
                    // structure. Ported faithfully: the remaining sweep
                    // then skips until the character logs in again (the
                    // legacy mitigation was an hourly admin-scope alert).
                    tokens::delete_token(pool, token.token_id).await?;
                    let _ = status;
                }
                EsiError::NotFound | EsiError::UnexpectedStatus(_) | EsiError::Decode(_) => {
                    tracing::warn!(
                        "structure {structure_id} fetch failed for character {character_id}: {error}",
                    );
                }
                EsiError::Http(_) => return Err(error.into()),
            }

            record_resolution(pool, character_id, structure_id, false).await?;
            Ok(StructureOutcome::Unresolved)
        }
    }
}

async fn record_resolution(
    pool: &PgPool,
    character_id: i64,
    structure_id: i64,
    could_resolve: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "insert into character_structure (character_id, structure_id, could_resolve)
         values ($1, $2, $3)
         on conflict (character_id, structure_id) do update set
             could_resolve = excluded.could_resolve,
             updated_at = now()",
    )
    .bind(character_id)
    .bind(structure_id)
    .bind(could_resolve)
    .execute(pool)
    .await?;

    Ok(())
}
