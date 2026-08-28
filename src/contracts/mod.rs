//! Public contract ingestion, ported from the legacy contract jobs and
//! actions: fetch a region's public contracts, keep the relevant ones
//! (auctions and item exchanges) with their unified price, fetch items for
//! new contracts to classify them and link their abyssal modules, delete
//! contracts that vanished from the feed, and track auction bids.

pub mod character;

use futures_util::StreamExt;
use sqlx::{PgPool, Row};

use crate::esi::{EsiClient, EsiContractItem, EsiError, EsiPublicContract};
use crate::estimator::Estimator;
use crate::modules::ingest::import_module;
use crate::mutation::reference::ReferenceData;

/// PLEX, the legacy `SupportType::PLEX`: asked-for PLEX counts into the
/// unified price of item exchanges.
pub const PLEX_TYPE_ID: i64 = 44992;

/// The Forge (Jita's region), where the PLEX reference price comes from.
pub const FORGE_REGION_ID: i64 = 10000002;

/// K-space region ids (`UniverseType::EVE` in the legacy id ranges); only
/// those carry public contracts worth scanning.
pub const KSPACE_REGION_RANGE: std::ops::RangeInclusive<i64> = 10_000_000..=10_999_999;

/// Contract types the app cares about, like the legacy relevant-types
/// filter.
const RELEVANT_CONTRACT_TYPES: [&str; 2] = ["auction", "item_exchange"];

/// Concurrent ESI lanes for a region sync (page fetches and per-contract
/// item syncs). Well inside ESI's error-rate budget, and low enough to
/// leave database pool connections (10) for the request handlers.
const ESI_SYNC_LANES: usize = 6;

#[derive(Debug, Default, Clone, Copy)]
pub struct SyncStats {
    pub total: usize,
    pub relevant: usize,
    pub new: usize,
    pub invalidated: usize,
}

#[derive(Debug)]
pub enum ContractSyncError {
    Esi(EsiError),
    Db(sqlx::Error),
}

impl std::fmt::Display for ContractSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractSyncError::Esi(error) => write!(f, "ESI: {error}"),
            ContractSyncError::Db(error) => write!(f, "database: {error}"),
        }
    }
}

impl std::error::Error for ContractSyncError {}

impl From<EsiError> for ContractSyncError {
    fn from(error: EsiError) -> Self {
        ContractSyncError::Esi(error)
    }
}

impl From<sqlx::Error> for ContractSyncError {
    fn from(error: sqlx::Error) -> Self {
        ContractSyncError::Db(error)
    }
}

/// The legacy `ContractStatus::parse`: raw ESI statuses folded into the
/// four site statuses (character contracts store the raw string and fold
/// it on read, through the legacy `ContractStatusCast`).
pub fn parse_contract_status(status: &str) -> &'static str {
    match status {
        "finished" | "finished_issuer" | "finished_contractor" | "accepted" | "completed" => {
            "completed"
        }
        "deleted" | "failed" | "cancelled" | "reversed" | "rejected" | "expired" => "failed",
        "outstanding" => "outstanding",
        _ => "unknown",
    }
}

/// The auction/item-exchange price normalization of the legacy
/// `Contract::calculateUnifiedPrice`: auctions count their highest bid,
/// item exchanges add the market value of asked-for PLEX.
pub fn unified_price(
    contract_type: &str,
    price: Option<f64>,
    highest_bid: Option<f64>,
    plex_count: i64,
    plex_average: Option<f64>,
) -> f64 {
    match contract_type {
        "auction" => highest_bid.unwrap_or_else(|| price.unwrap_or(0.0)),
        "item_exchange" => {
            price.unwrap_or(0.0) + plex_average.unwrap_or(0.0) * plex_count as f64
        }
        // The legacy fallthrough; relevant contracts never hit it.
        _ => 69.0,
    }
}

/// The latest PLEX daily average, like `MarketHistory::plex()->latest()`.
pub async fn plex_average(pool: &PgPool) -> sqlx::Result<Option<f64>> {
    sqlx::query_scalar(
        "select average from market_histories where type_id = $1 order by date desc limit 1",
    )
    .bind(PLEX_TYPE_ID)
    .fetch_optional(pool)
    .await
}

/// ESI's items-endpoint error for a contract a player accepted; maps to
/// the `completed` final status (legacy `ContractItemsError`).
const CONTRACT_ACCEPTED_ERROR: &str = "Contract accepted by player";
/// ESI's items-endpoint error for a deleted/expired contract; maps to
/// the `failed` final status.
const CONTRACT_NOT_FOUND_ERROR: &str = "Contract not found";

/// Whether a vanished contract can serve as estimator training data, the
/// legacy `InvalidateContractJob::qualifiesForTrainingData`: exactly one
/// abyssal module, and any second item must be PLEX payment.
fn qualifies_for_training(abyssal: i32, non_abyssal: i32, plex: i32) -> bool {
    if abyssal > 1 || non_abyssal > 1 {
        return false;
    }
    !(non_abyssal > 0 && plex == 0)
}

/// The final status of a vanished contract, probed from the items
/// endpoint's error body like the legacy `GetContractStatusAction`.
/// Transport errors and unrecognized messages read `unknown`.
async fn contract_final_status(esi: &EsiClient, contract_id: i64) -> &'static str {
    match esi.public_contract_items_error(contract_id).await {
        Ok(Some(message)) if message.contains(CONTRACT_ACCEPTED_ERROR) => "completed",
        Ok(Some(message)) if message.contains(CONTRACT_NOT_FOUND_ERROR) => "failed",
        _ => "unknown",
    }
}

/// Archives a contract that vanished from the public feed into
/// `historic_contracts` (when it held abyssal modules) and deletes it,
/// the legacy `InvalidateContractJob` + `DeletePublicContractAction`.
pub async fn invalidate_contract(
    pool: &PgPool,
    esi: &EsiClient,
    contract_id: i64,
) -> Result<(), ContractSyncError> {
    let counts: Option<(i32, i32, i32)> = sqlx::query_as(
        "select abyssal_modules_count, non_abyssal_modules_count, plex_count
         from contracts where id = $1",
    )
    .bind(contract_id)
    .fetch_optional(pool)
    .await?;
    let Some((abyssal, non_abyssal, plex)) = counts else {
        return Ok(());
    };

    if abyssal == 0 {
        sqlx::query("delete from contracts where id = $1")
            .bind(contract_id)
            .execute(pool)
            .await?;
        return Ok(());
    }

    let status = if qualifies_for_training(abyssal, non_abyssal, plex) {
        contract_final_status(esi, contract_id).await
    } else {
        "unknown"
    };

    let mut tx = pool.begin().await?;
    sqlx::query(
        "insert into historic_contracts
             (id, status, region_id, start_location_id, issuer_id,
              issuer_corporation_id, for_corporation, type, title,
              date_issued, date_expired, price, buyout, highest_bid,
              unified_price, asking_for_items, abyssal_modules_count,
              non_abyssal_modules_count, plex_count)
         select id, $2, region_id, start_location_id, issuer_id,
                issuer_corporation_id, for_corporation, type, title,
                date_issued, date_expired, price, buyout, highest_bid,
                unified_price, asking_for_items, abyssal_modules_count,
                non_abyssal_modules_count, plex_count
         from contracts where id = $1
         on conflict (id) do nothing",
    )
    .bind(contract_id)
    .bind(status)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "insert into historic_contract_items
             (historic_contract_id, record_id, type_id, item_id)
         select contract_id, record_id, type_id, item_id
         from contract_items where contract_id = $1
         on conflict (historic_contract_id, record_id) do nothing",
    )
    .bind(contract_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("delete from contracts where id = $1")
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(())
}

/// A low-meta module sold above this price is probably a mislabeled
/// multi-item deal, not a real roll price — the legacy
/// `SearchTrainingModulesCommand::PRICE_THRESHOLD`.
const TRAINING_PRICE_THRESHOLD: f64 = 500_000_000.0;

/// Refreshes `training_modules` from the archived contracts, the legacy
/// `SearchTrainingModulesCommand`: drops modules of admin-ignored
/// contracts, then upserts every module sold alone in a completed item
/// exchange — except suspicious rolls (low-meta source, low-tier
/// mutaplasmid, above the price threshold, non-capital), which read as
/// data errors. Returns (deleted, upserted).
pub async fn sync_training_modules(pool: &PgPool) -> sqlx::Result<(u64, u64)> {
    let deleted = sqlx::query(
        "delete from training_modules tm using historic_contracts hc
         where hc.id = tm.historic_contract_id and hc.ignore_for_training",
    )
    .execute(pool)
    .await?
    .rows_affected();

    // A module sold several times appears in several qualifying
    // contracts; the newest archive entry wins (the legacy chunked
    // updateOrCreate let the last processed row overwrite).
    let upserted = sqlx::query(
        "insert into training_modules (module_id, historic_contract_id, issued_at)
         select distinct on (hci.item_id) hci.item_id, hc.id, hc.date_issued
         from historic_contract_items hci
         join historic_contracts hc on hc.id = hci.historic_contract_id
         join modules m on m.id = hci.item_id
         join types t on t.id = m.type_id
         left join types st on st.id = m.source_type_id
         left join mutaplasmids mp on mp.id = m.mutaplasmid_id
         where not hc.ignore_for_training
           and hc.abyssal_modules_count = 1
           and hc.non_abyssal_modules_count = 0
           and hc.status = 'completed'
           and hc.type = 'item_exchange'
           and not (
               st.meta_group_id in (1, 2)
               and coalesce(mp.name, '') not like '%Radical%'
               and coalesce(mp.name, '') not like '%Exigent%'
               and coalesce(mp.name, '') not like '%Unstable%'
               and coalesce(hc.unified_price, 0) >= $1
               and t.name not like '%Capital%'
               and t.name not like '%Siege%'
               and t.name not like '%Fighter%'
               and t.name not like '%10000MN%'
               and t.name not like '%50000MN%'
           )
         order by hci.item_id, hci.id desc
         on conflict (module_id) do update set
             historic_contract_id = excluded.historic_contract_id,
             issued_at = excluded.issued_at,
             updated_at = now()",
    )
    .bind(TRAINING_PRICE_THRESHOLD)
    .execute(pool)
    .await?
    .rows_affected();

    Ok((deleted, upserted))
}

/// Refreshes the PLEX market history from The Forge, keeping every day
/// (the unified price and the statistics page read the accumulated
/// series; see the divergence note on [`sync_market_histories`]).
pub async fn sync_plex_market_history(
    pool: &PgPool,
    esi: &EsiClient,
) -> Result<usize, ContractSyncError> {
    let days = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await?;

    let mut tx = pool.begin().await?;
    for day in &days {
        upsert_market_day(&mut tx, PLEX_TYPE_ID, FORGE_REGION_ID, day).await?;
    }
    tx.commit().await?;

    Ok(days.len())
}

async fn upsert_market_day(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    type_id: i64,
    region_id: i64,
    day: &crate::esi::EsiMarketDay,
) -> sqlx::Result<()> {
    sqlx::query(
        "insert into market_histories
         (type_id, region_id, date, average, highest, lowest, order_count, volume)
         values ($1, $2, $3::date, $4, $5, $6, $7, $8)
         on conflict (type_id, region_id, date) do update set
             average = excluded.average,
             highest = excluded.highest,
             lowest = excluded.lowest,
             order_count = excluded.order_count,
             volume = excluded.volume",
    )
    .bind(type_id)
    .bind(region_id)
    .bind(&day.date)
    .bind(day.average)
    .bind(day.highest)
    .bind(day.lowest)
    .bind(day.order_count)
    .bind(day.volume)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The type ids the daily market-history sweep covers, in the legacy
/// `GetMarketHistoriesCommand` dispatch order: every mutaplasmid (their
/// ids are type ids), every published source module type (types with
/// mutaplasmid input rows), then the support types (PLEX — the whole
/// legacy `SupportType` enum).
pub async fn market_history_type_ids(pool: &PgPool) -> sqlx::Result<Vec<i64>> {
    let mut ids: Vec<i64> = sqlx::query_scalar("select id from mutaplasmids order by id")
        .fetch_all(pool)
        .await?;
    let sources: Vec<i64> = sqlx::query_scalar(
        "select distinct t.id from types t
         join mutaplasmid_input_types mit on mit.type_id = t.id
         where t.published order by t.id",
    )
    .fetch_all(pool)
    .await?;
    ids.extend(sources);
    ids.push(PLEX_TYPE_ID);
    Ok(ids)
}

/// Stores the newest market-history day for one type in a region, the
/// legacy `GetMarketHistoryJob` + `ProcessMarketHistory`: the full
/// history is fetched but only the latest day is written. `Ok(false)`
/// when ESI had no data (the legacy "no data" log-and-return).
pub async fn sync_market_history_latest(
    pool: &PgPool,
    esi: &EsiClient,
    region_id: i64,
    type_id: i64,
) -> Result<bool, ContractSyncError> {
    let days = esi.market_history(region_id, type_id).await?;
    let Some(latest) = days.iter().max_by(|a, b| a.date.cmp(&b.date)) else {
        return Ok(false);
    };

    let mut tx = pool.begin().await?;
    upsert_market_day(&mut tx, type_id, region_id, latest).await?;
    tx.commit().await?;
    Ok(true)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarketHistoryStats {
    pub types: usize,
    pub days: usize,
    pub empty: usize,
    pub failed: usize,
}

/// The daily market-history sweep over [`market_history_type_ids`], the
/// legacy `GetMarketHistoriesCommand` fan-out. Per-type failures are
/// logged and counted, like the legacy job's log-and-return.
///
/// Divergence, deliberate: legacy `ProcessMarketHistory` overwrote a
/// single row per (type, region) with the latest day; our table keys on
/// (type, region, date), so the sweep accumulates one row per day.
/// Every consumer picks the newest row per type, and PLEX keeps its
/// full-history refresh (the series the statistics page charts).
pub async fn sync_market_histories(
    pool: &PgPool,
    esi: &EsiClient,
    progress: impl FnMut(String),
) -> sqlx::Result<MarketHistoryStats> {
    let type_ids = market_history_type_ids(pool).await?;
    Ok(sync_market_history_set(pool, esi, &type_ids, progress).await)
}

/// The sweep over an explicit type set; [`sync_market_histories`] with
/// the production set.
pub async fn sync_market_history_set(
    pool: &PgPool,
    esi: &EsiClient,
    type_ids: &[i64],
    mut progress: impl FnMut(String),
) -> MarketHistoryStats {
    let mut stats = MarketHistoryStats { types: type_ids.len(), ..Default::default() };
    for (index, type_id) in type_ids.iter().copied().enumerate() {
        progress(format!("type {}/{} (id {type_id}): {} days so far", index + 1, stats.types, stats.days));
        let outcome = if type_id == PLEX_TYPE_ID {
            sync_plex_market_history(pool, esi).await.map(|days| {
                stats.days += days;
                days > 0
            })
        } else {
            sync_market_history_latest(pool, esi, FORGE_REGION_ID, type_id).await.inspect(
                |&stored| {
                    if stored {
                        stats.days += 1;
                    }
                },
            )
        };
        match outcome {
            Ok(true) => {}
            Ok(false) => stats.empty += 1,
            Err(error) => {
                stats.failed += 1;
                tracing::warn!("market history for type {type_id} failed: {error}");
            }
        }
    }

    stats
}

/// Every region worth scanning for contracts, like `Region::kspace()`.
pub async fn kspace_region_ids(pool: &PgPool) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("select id from regions where id between $1 and $2 order by id")
        .bind(*KSPACE_REGION_RANGE.start())
        .bind(*KSPACE_REGION_RANGE.end())
        .fetch_all(pool)
        .await
}

/// Syncs one region's public contracts, the legacy
/// `GetPublicContractsJob` + `CreatePublicContractsAction` flow.
pub async fn sync_region(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    estimator: &Estimator,
    region_id: i64,
) -> Result<SyncStats, ContractSyncError> {
    // The first page carries the page count; the rest fetch in parallel
    // lanes.
    let (mut contracts, pages) = esi.public_contracts(region_id, 1).await?;
    if pages > 1 {
        let batches: Vec<Result<(Vec<EsiPublicContract>, u32), EsiError>> =
            futures_util::stream::iter(2..=pages)
                .map(|page| esi.public_contracts(region_id, page))
                .buffer_unordered(ESI_SYNC_LANES)
                .collect()
                .await;
        for batch in batches {
            contracts.append(&mut batch?.0);
        }
    }

    let relevant: Vec<&EsiPublicContract> = contracts
        .iter()
        .filter(|contract| RELEVANT_CONTRACT_TYPES.contains(&contract.contract_type.as_str()))
        .collect();

    let known_ids: Vec<i64> = sqlx::query_scalar("select id from contracts where region_id = $1")
        .bind(region_id)
        .fetch_all(pool)
        .await?;

    let relevant_ids: std::collections::HashSet<i64> =
        relevant.iter().map(|contract| contract.contract_id).collect();
    let new_ids: std::collections::HashSet<i64> = relevant_ids
        .iter()
        .copied()
        .filter(|id| !known_ids.contains(id))
        .collect();
    let invalidated: Vec<i64> = known_ids
        .iter()
        .copied()
        .filter(|id| !relevant_ids.contains(id))
        .collect();

    let plex = plex_average(pool).await?;

    let mut tx = pool.begin().await?;

    for contract in &relevant {
        // Issuers need at least a stub character row, like the legacy
        // Character::insertByIds.
        sqlx::query("insert into characters (id, name) values ($1, '') on conflict (id) do nothing")
            .bind(contract.issuer_id)
            .execute(&mut *tx)
            .await?;

        let price = unified_price(&contract.contract_type, contract.price, None, 0, plex);

        // On refresh the unified price is recomputed from the persisted
        // bid and PLEX state, like the legacy fill-and-recalculate.
        sqlx::query(
            "insert into contracts
             (id, region_id, start_location_id, issuer_id, issuer_corporation_id,
              for_corporation, type, title, date_issued, date_expired, price, buyout,
              unified_price)
             values ($1, $2, $3, $4, $5, $6, $7, $8, $9::timestamptz, $10::timestamptz, $11, $12, $13)
             on conflict (id) do update set
                 region_id = excluded.region_id,
                 start_location_id = excluded.start_location_id,
                 issuer_id = excluded.issuer_id,
                 issuer_corporation_id = excluded.issuer_corporation_id,
                 for_corporation = excluded.for_corporation,
                 type = excluded.type,
                 title = excluded.title,
                 date_issued = excluded.date_issued,
                 date_expired = excluded.date_expired,
                 price = excluded.price,
                 buyout = excluded.buyout,
                 unified_price = case
                     when excluded.type = 'auction'
                         then coalesce(contracts.highest_bid, excluded.price, 0)
                     else coalesce(excluded.price, 0)
                         + coalesce($14, 0) * contracts.plex_count
                 end,
                 updated_at = now()",
        )
        .bind(contract.contract_id)
        .bind(region_id)
        .bind(contract.start_location_id)
        .bind(contract.issuer_id)
        .bind(contract.issuer_corporation_id)
        .bind(contract.for_corporation.unwrap_or(false))
        .bind(&contract.contract_type)
        .bind(&contract.title)
        .bind(&contract.date_issued)
        .bind(&contract.date_expired)
        .bind(contract.price)
        .bind(contract.buyout)
        .bind(price)
        .bind(plex)
        .execute(&mut *tx)
        .await?;
    }

    // Contracts gone from the feed are finished or cancelled; each gets
    // archived (with an ESI status probe) and deleted after the commit,
    // like the legacy per-contract InvalidateContractJob.

    sqlx::query(
        "insert into contract_imports (region_id, contracts_total_count, contracts_invalidated_count)
         values ($1, $2, $3)",
    )
    .bind(region_id)
    .bind(contracts.len() as i32)
    .bind(invalidated.len() as i32)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    futures_util::stream::iter(invalidated.clone())
        .for_each_concurrent(ESI_SYNC_LANES, |contract_id| async move {
            if let Err(error) = invalidate_contract(pool, esi, contract_id).await {
                tracing::warn!("invalidating contract {contract_id} failed: {error}");
            }
        })
        .await;

    // Items are owed to every contract not yet marked synced — not just
    // this cycle's new ids — so a crash between the contract upsert and the
    // item fetch only delays the items until the next cycle.
    let pending: Vec<i64> = sqlx::query_scalar(
        "select id from contracts where region_id = $1 and items_synced_at is null",
    )
    .bind(region_id)
    .fetch_all(pool)
    .await?;

    // Item failures stay per contract, like the legacy queued jobs: one
    // broken contract must not abort the whole region. Contracts sync in
    // parallel lanes; each lane still pages its own contract serially.
    futures_util::stream::iter(pending)
        .for_each_concurrent(ESI_SYNC_LANES, |contract_id| async move {
            if let Err(error) =
                sync_contract_items(pool, reference, esi, estimator, contract_id).await
            {
                tracing::warn!("items for contract {contract_id} failed: {error}");
            }
        })
        .await;

    Ok(SyncStats {
        total: contracts.len(),
        relevant: relevant.len(),
        new: new_ids.len(),
        invalidated: invalidated.len(),
    })
}

/// Fetches a contract's items, classifies them and links abyssal modules,
/// the legacy `GetPublicContractItemsJob` + `CreateContractItemsAction` +
/// `GetPublicContractModuleJob` chain.
pub async fn sync_contract_items(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    estimator: &Estimator,
    contract_id: i64,
) -> Result<(), ContractSyncError> {
    let mut items: Vec<EsiContractItem> = Vec::new();
    let mut page = 1;
    let fetched = loop {
        match esi.public_contract_items(contract_id, page).await {
            Ok((mut batch, pages)) => {
                items.append(&mut batch);
                if page >= pages {
                    break true;
                }
                page += 1;
            }
            // The contract vanished before its items could be fetched.
            Err(EsiError::NotFound) => break false,
            Err(error) => return Err(error.into()),
        }
    };

    if !fetched {
        sqlx::query("delete from contracts where id = $1")
            .bind(contract_id)
            .execute(pool)
            .await?;
        return Ok(());
    }

    let included: Vec<&EsiContractItem> = items.iter().filter(|item| item.is_included).collect();
    let asked_for: Vec<&EsiContractItem> = items.iter().filter(|item| !item.is_included).collect();

    let potential_abyssal: Vec<&EsiContractItem> = included
        .iter()
        .copied()
        .filter(|item| reference.is_abyssal_type(item.type_id))
        .collect();
    let abyssal: Vec<&EsiContractItem> = potential_abyssal
        .iter()
        .copied()
        .filter(|item| item.item_id.is_some())
        .collect();

    let plex_count: i64 = asked_for
        .iter()
        .filter(|item| item.type_id == PLEX_TYPE_ID)
        .map(|item| item.quantity)
        .sum();

    let contract_row =
        sqlx::query("select type, price, highest_bid from contracts where id = $1")
            .bind(contract_id)
            .fetch_optional(pool)
            .await?;
    let Some(contract_row) = contract_row else {
        return Ok(());
    };

    let plex = plex_average(pool).await?;
    let price = unified_price(
        &contract_row.get::<String, _>("type"),
        contract_row.get("price"),
        contract_row.get("highest_bid"),
        plex_count,
        plex,
    );

    sqlx::query(
        "update contracts set
             asking_for_items = $1,
             plex_count = $2,
             abyssal_modules_count = $3,
             non_abyssal_modules_count = $4,
             unified_price = $5,
             updated_at = now()
         where id = $6",
    )
    .bind(!asked_for.is_empty())
    .bind(plex_count as i32)
    .bind(abyssal.len() as i32)
    .bind((included.len() + asked_for.len() - abyssal.len()) as i32)
    .bind(price)
    .bind(contract_id)
    .execute(pool)
    .await?;

    // A contract item row is only written after its module import and link
    // succeeded, so its presence marks that item as done: retries after a
    // crash or a failed import skip it.
    let imported: std::collections::HashSet<i64> =
        sqlx::query_scalar("select record_id from contract_items where contract_id = $1")
            .bind(contract_id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();

    let mut failures = 0usize;
    for item in &abyssal {
        let item_id = item.item_id.expect("filtered on item_id");
        if imported.contains(&item.record_id) {
            continue;
        }

        if let Err(error) =
            import_module(pool, reference, esi, estimator, item.type_id, item_id).await
        {
            tracing::warn!("failed to fetch module {item_id} for contract {contract_id}: {error}");
            failures += 1;
            continue;
        }

        // Guarded against the contract vanishing concurrently (another
        // sync process may have invalidated it since the fetch).
        sqlx::query(
            "update modules set latest_contract_id = $1, updated_at = now()
             where id = $2 and exists (select 1 from contracts where id = $1)",
        )
            .bind(contract_id)
            .bind(item_id)
            .execute(pool)
            .await?;

        sqlx::query(
            "insert into contract_items (contract_id, record_id, type_id, item_id)
             select $1, $2, $3, $4
             where exists (select 1 from contracts where id = $1)
             on conflict (contract_id, record_id) do nothing",
        )
        .bind(contract_id)
        .bind(item.record_id)
        .bind(item.type_id)
        .bind(item_id)
        .execute(pool)
        .await?;

        // The ownership row of the issuing character, the legacy
        // after_public_contract_item trigger (character pages list their
        // sales through public_module_ownerships).
        sqlx::query(
            "insert into public_module_ownerships (character_id, module_id, contract_id)
             select ct.issuer_id, $2, ct.id
             from contracts ct
             where ct.id = $1
               and exists (select 1 from modules where id = $2)
               and exists (select 1 from characters where id = ct.issuer_id)
             on conflict (character_id, module_id) do update
             set contract_id = excluded.contract_id, updated_at = now()",
        )
        .bind(contract_id)
        .bind(item_id)
        .execute(pool)
        .await?;
    }

    // The contract only counts as item-synced once every abyssal module
    // import landed; any failure leaves it pending so the next cycle
    // retries the remainder (every write above is idempotent).
    if failures == 0 {
        sqlx::query("update contracts set items_synced_at = now(), updated_at = now() where id = $1")
            .bind(contract_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

/// Updates the highest bid of running auctions carrying abyssal modules and
/// recomputes their unified price, the legacy bids job chain.
pub async fn sync_auction_bids(pool: &PgPool, esi: &EsiClient) -> Result<usize, ContractSyncError> {
    let auction_ids: Vec<i64> = sqlx::query_scalar(
        "select id from contracts where type = 'auction' and abyssal_modules_count > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut updated = 0;
    for contract_id in auction_ids {
        let bids = match esi.public_contract_bids(contract_id).await {
            Ok(bids) => bids,
            Err(EsiError::NotFound) => continue,
            Err(error) => return Err(error.into()),
        };

        let highest = bids.iter().map(|bid| bid.amount).fold(f64::NEG_INFINITY, f64::max);
        if !highest.is_finite() {
            continue;
        }

        sqlx::query(
            "update contracts set
                 highest_bid = $1,
                 unified_price = $1,
                 updated_at = now()
             where id = $2 and (highest_bid is null or highest_bid < $1)",
        )
        .bind(highest)
        .bind(contract_id)
        .execute(pool)
        .await?;
        updated += 1;
    }

    Ok(updated)
}
