//! Alliance ingestion, ported from the legacy daily `app:get-alliances`
//! chain (`GetAlliancesJob` → `GetAllianceJob` → `CreateAllianceAction`):
//! list every alliance id on ESI, fetch each sheet, and upsert the
//! record with a stub character row for its creator.
//!
//! Divergence, deliberate: the legacy action first ran
//! `GetCorporationJob` for the executor corporation; corporations are
//! not ported, so the raw executor id is stored and no corporation rows
//! are created (see the alliances migration).

use futures_util::StreamExt;
use sqlx::PgPool;

use crate::esi::{EsiAlliance, EsiClient, EsiError};

/// Concurrent alliance-sheet fetches during the daily sweep. Small on
/// purpose: failures on these requests count against ESI's error-rate
/// budget, and four lanes already cut the thousands-of-alliances sweep
/// from hours to minutes (matching the spirit of `ESI_SYNC_LANES` in
/// the contracts sync).
const ALLIANCE_SYNC_LANES: usize = 4;

#[derive(Debug, Default, Clone, Copy)]
pub struct AllianceSyncStats {
    pub total: usize,
    pub upserted: usize,
    pub failed: usize,
}

#[derive(Debug)]
pub enum AllianceSyncError {
    Esi(EsiError),
    Db(sqlx::Error),
}

impl std::fmt::Display for AllianceSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllianceSyncError::Esi(error) => write!(f, "ESI: {error}"),
            AllianceSyncError::Db(error) => write!(f, "database: {error}"),
        }
    }
}

impl std::error::Error for AllianceSyncError {}

/// The full sweep: every alliance ESI lists. A failing list call fails
/// the run (the legacy "Failed to get alliances" early return); a
/// failing sheet only skips that alliance (the legacy per-job
/// "Failed to get alliance" log).
pub async fn sync_alliances(
    pool: &PgPool,
    esi: &EsiClient,
    mut progress: impl FnMut(String),
) -> Result<AllianceSyncStats, AllianceSyncError> {
    let ids = esi.alliance_ids().await.map_err(AllianceSyncError::Esi)?;

    let mut stats = AllianceSyncStats { total: ids.len(), ..Default::default() };
    let mut sheets = futures_util::stream::iter(ids)
        .map(|alliance_id| async move { (alliance_id, esi.alliance(alliance_id).await) })
        .buffer_unordered(ALLIANCE_SYNC_LANES);

    let mut done = 0usize;
    while let Some((alliance_id, result)) = sheets.next().await {
        done += 1;
        progress(format!("alliance {done}/{} (id {alliance_id})", stats.total));
        match result {
            Ok(details) => {
                upsert_alliance(pool, alliance_id, &details)
                    .await
                    .map_err(AllianceSyncError::Db)?;
                stats.upserted += 1;
            }
            Err(error) => {
                stats.failed += 1;
                tracing::warn!("alliance {alliance_id} failed: {error}");
            }
        }
    }

    Ok(stats)
}

/// The legacy `CreateContractAcceptorsAction` alliance path: fetch and
/// store only ids missing from the table (`getMissingAlliances` +
/// `GetAllianceJob::dispatchSync`). A failed sheet is logged and skipped
/// like the sweep's per-alliance failures.
pub async fn ensure_alliances(
    pool: &PgPool,
    esi: &EsiClient,
    alliance_ids: &[i64],
) -> sqlx::Result<()> {
    if alliance_ids.is_empty() {
        return Ok(());
    }
    let existing: Vec<i64> = sqlx::query_scalar("select id from alliances where id = any($1)")
        .bind(alliance_ids)
        .fetch_all(pool)
        .await?;

    for alliance_id in alliance_ids.iter().filter(|id| !existing.contains(id)) {
        match esi.alliance(*alliance_id).await {
            Ok(details) => upsert_alliance(pool, *alliance_id, &details).await?,
            Err(error) => tracing::warn!("alliance {alliance_id} failed: {error}"),
        }
    }
    Ok(())
}

/// The legacy `CreateAllianceAction::insertAlliance`: a stub character
/// row for the creator, then the updateOrCreate of the alliance record.
async fn upsert_alliance(
    pool: &PgPool,
    alliance_id: i64,
    details: &EsiAlliance,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query("insert into characters (id, name) values ($1, '') on conflict (id) do nothing")
        .bind(details.creator_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "insert into alliances
         (id, name, ticker, creator_id, date_founded, executor_corporation_id, faction_id)
         values ($1, $2, $3, $4, $5::timestamptz, $6, $7)
         on conflict (id) do update set
             name = excluded.name,
             ticker = excluded.ticker,
             creator_id = excluded.creator_id,
             date_founded = excluded.date_founded,
             executor_corporation_id = excluded.executor_corporation_id,
             faction_id = excluded.faction_id,
             updated_at = now()",
    )
    .bind(alliance_id)
    .bind(&details.name)
    .bind(&details.ticker)
    .bind(details.creator_id)
    .bind(&details.date_founded)
    .bind(details.executor_corporation_id)
    .bind(details.faction_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
