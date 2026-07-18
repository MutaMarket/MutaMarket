//! Public contract ingestion, ported from the legacy contract jobs and
//! actions: fetch a region's public contracts, keep the relevant ones
//! (auctions and item exchanges) with their unified price, fetch items for
//! new contracts to classify them and link their abyssal modules, delete
//! contracts that vanished from the feed, and track auction bids.

use sqlx::{PgPool, Row};

use crate::esi::{EsiClient, EsiContractItem, EsiError, EsiPublicContract};
use crate::estimator::EstimatorClient;
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

/// Refreshes the PLEX market history from The Forge (the legacy market
/// histories job, reduced to what the unified price needs).
pub async fn sync_plex_market_history(
    pool: &PgPool,
    esi: &EsiClient,
) -> Result<usize, ContractSyncError> {
    let days = esi.market_history(FORGE_REGION_ID, PLEX_TYPE_ID).await?;

    let mut tx = pool.begin().await?;
    for day in &days {
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
        .bind(PLEX_TYPE_ID)
        .bind(FORGE_REGION_ID)
        .bind(&day.date)
        .bind(day.average)
        .bind(day.highest)
        .bind(day.lowest)
        .bind(day.order_count)
        .bind(day.volume)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(days.len())
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
    estimator: &EstimatorClient,
    region_id: i64,
) -> Result<SyncStats, ContractSyncError> {
    let mut contracts: Vec<EsiPublicContract> = Vec::new();
    let mut page = 1;
    loop {
        let (mut batch, pages) = esi.public_contracts(region_id, page).await?;
        contracts.append(&mut batch);
        if page >= pages {
            break;
        }
        page += 1;
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

    // Contracts gone from the feed are finished or cancelled; drop them.
    // Moving them into historic contracts (the training data source)
    // arrives with the estimator milestone.
    if !invalidated.is_empty() {
        sqlx::query("delete from contracts where id = any($1)")
            .bind(&invalidated)
            .execute(&mut *tx)
            .await?;
    }

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
    // broken contract must not abort the whole region.
    for contract_id in pending {
        if let Err(error) = sync_contract_items(pool, reference, esi, estimator, contract_id).await {
            eprintln!("items for contract {contract_id} failed: {error}");
        }
    }

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
    estimator: &EstimatorClient,
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
            eprintln!("failed to fetch module {item_id} for contract {contract_id}: {error}");
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
