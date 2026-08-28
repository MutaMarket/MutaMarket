//! Personal contract ingestion, ported from the legacy
//! `GetCharacterContractsCommand` → `GetCharacterContractsJob` →
//! `CreateCharacterContractsAction` chain: fetch a character's contracts,
//! resolve acceptor categories via universe names, upsert them into the
//! dedicated character_contracts table, and classify their items (only
//! abyssal modules are stored as rows).
//!
//! Divergence, deliberate and local: acceptor corporations got stub rows
//! in legacy; the corporations table is not ported, so corporation
//! acceptors carry only their id and serialize as null. Character
//! acceptors get stub rows and alliance acceptors are fetched into the
//! alliances table, both like legacy.

use sqlx::PgPool;

use super::{ContractSyncError, plex_average};
use crate::auth::scopes;
use crate::auth::sso::SsoClient;
use crate::auth::tokens::{self, TokenError};
use crate::esi::{EsiCharacterContract, EsiClient, EsiContractItem, EsiError};
use crate::mutation::reference::ReferenceData;

/// Characters refreshed per scheduler tick (every five minutes), the
/// legacy `contracts.limit` config default.
const MAX_CHARACTERS_PER_RUN: i64 = 30;

/// The universe-names category for characters (acceptor stub rows are
/// only created for these; see the module divergence note).
const NAME_CATEGORY_CHARACTER: &str = "character";

/// The universe-names category for alliances; alliance acceptors get
/// their alliance row fetched like the legacy acceptors action.
const NAME_CATEGORY_ALLIANCE: &str = "alliance";

#[derive(Debug, Default, Clone, Copy)]
pub struct CharacterContractStats {
    pub total: usize,
    pub items_synced: usize,
    pub items_failed: usize,
}

#[derive(Debug)]
pub enum CharacterContractError {
    /// The character holds no token with the contracts scope.
    NoToken,
    Token(TokenError),
    Sync(ContractSyncError),
}

impl std::fmt::Display for CharacterContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharacterContractError::NoToken => write!(f, "no token with the contracts scope"),
            CharacterContractError::Token(error) => write!(f, "token: {error}"),
            CharacterContractError::Sync(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for CharacterContractError {}

impl From<TokenError> for CharacterContractError {
    fn from(error: TokenError) -> Self {
        CharacterContractError::Token(error)
    }
}

impl From<ContractSyncError> for CharacterContractError {
    fn from(error: ContractSyncError) -> Self {
        CharacterContractError::Sync(error)
    }
}

impl From<sqlx::Error> for CharacterContractError {
    fn from(error: sqlx::Error) -> Self {
        CharacterContractError::Sync(ContractSyncError::Db(error))
    }
}

impl From<EsiError> for CharacterContractError {
    fn from(error: EsiError) -> Self {
        CharacterContractError::Sync(ContractSyncError::Esi(error))
    }
}

/// The unified price of a character contract, the legacy
/// `CharacterContract::calculateUnifiedPrice`: auctions count their
/// highest bid only (the character contracts feed carries none, so this
/// stays 0 until one is known), item exchanges add the market value of
/// asked-for PLEX, and every other type prices at zero.
pub fn character_unified_price(
    contract_type: &str,
    price: Option<f64>,
    highest_bid: Option<f64>,
    plex_count: i64,
    plex_average: Option<f64>,
) -> f64 {
    match contract_type {
        "auction" => highest_bid.unwrap_or(0.0),
        "item_exchange" => {
            price.unwrap_or(0.0) + plex_average.unwrap_or(0.0) * plex_count as f64
        }
        _ => 0.0,
    }
}

/// Characters due for a contract refresh: they hold the contracts scope,
/// oldest fetch first — the legacy `GetCharacterContractsJobsAction`
/// selection. (MySQL puts ascending nulls first; Postgres needs it
/// spelled out, so never-fetched characters still go first.)
pub async fn pending_contract_characters(pool: &PgPool) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "select c.id from characters c
         where exists (
                   select 1 from esi_tokens t
                   where t.character_id = c.id and $1 = any(t.scopes)
               )
         order by c.contracts_fetched_at asc nulls first, c.id
         limit $2",
    )
    .bind(scopes::READ_CONTRACTS)
    .bind(MAX_CHARACTERS_PER_RUN)
    .fetch_all(pool)
    .await
}

/// Syncs one character's contracts and their pending items, the legacy
/// job + action chain.
pub async fn sync_character_contracts(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    character_id: i64,
) -> Result<CharacterContractStats, CharacterContractError> {
    let Some(token) =
        tokens::valid_access_token(pool, sso, character_id, scopes::READ_CONTRACTS).await?
    else {
        return Err(CharacterContractError::NoToken);
    };

    let mut contracts: Vec<EsiCharacterContract> = Vec::new();
    let mut page = 1;
    loop {
        let (mut batch, pages) = match esi
            .character_contracts(&token.access_token, character_id, page)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                // ESI rejecting the token deletes it, like the legacy
                // connector.
                if matches!(error, EsiError::Forbidden(_)) {
                    tokens::delete_token(pool, token.token_id).await?;
                }
                return Err(error.into());
            }
        };
        contracts.append(&mut batch);
        if page >= pages {
            break;
        }
        page += 1;
    }

    // Shared rows are written in contract-id order, like the legacy
    // deadlock-avoidance ordering.
    contracts.sort_by_key(|contract| contract.contract_id);

    // Acceptor categories via universe names; a failed lookup degrades to
    // no categories, like the legacy job using the possibly-failed
    // result's data.
    let acceptor_ids: Vec<i64> = {
        let mut ids: Vec<i64> = contracts
            .iter()
            // PHP truthiness: an acceptor id of 0 counts as absent.
            .filter_map(|contract| contract.acceptor_id.filter(|id| *id != 0))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let names = esi.universe_names(&acceptor_ids).await.unwrap_or_default();

    // Alliance acceptor rows, before the contract transaction so the ESI
    // sheet fetches never hold it open.
    let alliance_ids: Vec<i64> = names
        .iter()
        .filter(|name| name.category == NAME_CATEGORY_ALLIANCE)
        .map(|name| name.id)
        .collect();
    crate::alliances::ensure_alliances(pool, esi, &alliance_ids)
        .await
        .map_err(ContractSyncError::Db)?;

    let plex = plex_average(pool).await.map_err(ContractSyncError::Db)?;

    let mut tx = pool.begin().await?;

    for contract in &contracts {
        // Issuer (and character acceptor) stubs, like the legacy
        // Character::insertByIds.
        sqlx::query("insert into characters (id, name) values ($1, '') on conflict (id) do nothing")
            .bind(contract.issuer_id)
            .execute(&mut *tx)
            .await?;

        let acceptor_id = contract.acceptor_id.filter(|id| *id != 0);
        let acceptor_category = acceptor_id.and_then(|id| {
            names
                .iter()
                .find(|name| name.id == id)
                .map(|name| name.category.clone())
        });
        if acceptor_category.as_deref() == Some(NAME_CATEGORY_CHARACTER) {
            sqlx::query(
                "insert into characters (id, name) values ($1, '') on conflict (id) do nothing",
            )
            .bind(acceptor_id)
            .execute(&mut *tx)
            .await?;
        }

        // At upsert time no highest bid or plex count is known, so this
        // matches the legacy model-hook price: auctions land at 0,
        // exchanges at their plain price. Legacy quirk included: a
        // refetch of an already item-synced exchange clobbers the PLEX
        // component back out, because the fresh upsert model never knows
        // the stored plex count.
        let unified = character_unified_price(&contract.contract_type, contract.price, None, 0, plex);

        // highest_bid is deliberately not written: the legacy fillFromDTO
        // never sets it, so the upsert leaves the column alone.
        sqlx::query(
            "insert into character_contracts
             (id, issuer_id, issuer_corporation_id, for_corporation, type, title,
              date_issued, date_expired, price, buyout, acceptor_id, acceptor_type,
              assignee_id, availability, date_accepted, date_completed, status, volume,
              unified_price)
             values ($1, $2, $3, $4, $5, $6, $7::timestamptz, $8::timestamptz, $9, $10, $11,
                     $12, $13, $14, $15::timestamptz, $16::timestamptz, $17, $18, $19)
             on conflict (id) do update set
                 issuer_id = excluded.issuer_id,
                 issuer_corporation_id = excluded.issuer_corporation_id,
                 for_corporation = excluded.for_corporation,
                 type = excluded.type,
                 title = excluded.title,
                 date_issued = excluded.date_issued,
                 date_expired = excluded.date_expired,
                 price = excluded.price,
                 buyout = excluded.buyout,
                 acceptor_id = excluded.acceptor_id,
                 acceptor_type = excluded.acceptor_type,
                 assignee_id = excluded.assignee_id,
                 availability = excluded.availability,
                 date_accepted = excluded.date_accepted,
                 date_completed = excluded.date_completed,
                 status = excluded.status,
                 volume = excluded.volume,
                 unified_price = excluded.unified_price,
                 updated_at = now()",
        )
        .bind(contract.contract_id)
        .bind(contract.issuer_id)
        .bind(contract.issuer_corporation_id)
        .bind(contract.for_corporation.unwrap_or(false))
        .bind(&contract.contract_type)
        .bind(&contract.title)
        .bind(&contract.date_issued)
        .bind(&contract.date_expired)
        .bind(contract.price)
        .bind(contract.buyout)
        .bind(acceptor_id)
        .bind(acceptor_category.as_deref().or(Some(NAME_CATEGORY_CHARACTER)))
        // PHP truthiness again: assignee 0 becomes null.
        .bind(contract.assignee_id.filter(|id| *id != 0))
        .bind(availability(&contract.availability))
        .bind(&contract.date_accepted)
        .bind(&contract.date_completed)
        .bind(&contract.status)
        .bind(contract.volume)
        .bind(unified)
        .execute(&mut *tx)
        .await?;

        // The legacy updateContractStatus back-sync: a contract that left
        // outstanding updates its historic_contracts row (same ESI id),
        // folding the raw status like the ContractStatusCast. Legacy
        // quirk included: the update is unconditional, so every sync
        // re-touches updated_at on already-final rows.
        let folded = super::parse_contract_status(&contract.status);
        if folded != "outstanding" {
            sqlx::query(
                "update historic_contracts set status = $1, updated_at = now() where id = $2",
            )
            .bind(folded)
            .bind(contract.contract_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query("update characters set contracts_fetched_at = now(), updated_at = now() where id = $1")
        .bind(character_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Items are owed to every contract of this character not yet marked
    // synced — derived from domain state, so a crash between the upsert
    // and the item fetch only delays the items to the next cycle (the
    // legacy diffed new ids in memory instead).
    let pending: Vec<i64> = sqlx::query_scalar(
        "select id from character_contracts
         where items_synced_at is null
           and (issuer_id = $1 or acceptor_id = $1 or assignee_id = $1)
         order by id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    let mut items_synced = 0usize;
    let mut items_failed = 0usize;
    for contract_id in pending {
        match sync_contract_items(pool, reference, esi, sso, character_id, contract_id).await {
            Ok(()) => items_synced += 1,
            Err(error) => {
                // Per-contract failures stay local, like the legacy
                // queued item jobs.
                tracing::warn!(
                    "items for character contract {contract_id} (character {character_id}) failed: {error}",
                );
                items_failed += 1;
            }
        }
    }

    Ok(CharacterContractStats {
        total: contracts.len(),
        items_synced,
        items_failed,
    })
}

/// The legacy `ContractAvailability::fromEsi` mapping.
fn availability(esi_value: &str) -> &str {
    match esi_value {
        "public" | "personal" | "corporation" | "alliance" => esi_value,
        _ => "unknown",
    }
}

/// Fetches one character contract's items, updates the classification
/// stats, and stores the abyssal module rows — the legacy
/// `GetCharacterContractItemsJob` + `CreateCharacterContractItems`.
async fn sync_contract_items(
    pool: &PgPool,
    reference: &ReferenceData,
    esi: &EsiClient,
    sso: &SsoClient,
    character_id: i64,
    contract_id: i64,
) -> Result<(), CharacterContractError> {
    let Some(token) =
        tokens::valid_access_token(pool, sso, character_id, scopes::READ_CONTRACTS).await?
    else {
        return Err(CharacterContractError::NoToken);
    };

    let items = match esi
        .character_contract_items(&token.access_token, character_id, contract_id)
        .await
    {
        Ok(items) => items,
        // The contract is no longer accessible (expired, deleted): mark
        // it synced with no items so it is not retried forever. (The
        // legacy only ever tried once per contract.)
        Err(EsiError::NotFound) => {
            sqlx::query(
                "update character_contracts set items_synced_at = now(), updated_at = now()
                 where id = $1",
            )
            .bind(contract_id)
            .execute(pool)
            .await?;
            return Ok(());
        }
        Err(error) => {
            if matches!(error, EsiError::Forbidden(_)) {
                tokens::delete_token(pool, token.token_id).await?;
            }
            return Err(error.into());
        }
    };

    let asked_for: Vec<&EsiContractItem> = items.iter().filter(|item| !item.is_included).collect();
    let abyssal: Vec<&EsiContractItem> = items
        .iter()
        .filter(|item| reference.is_abyssal_type(item.type_id))
        .collect();

    let plex_count: i64 = asked_for
        .iter()
        .filter(|item| item.type_id == super::PLEX_TYPE_ID)
        .map(|item| item.quantity)
        .sum();

    let plex = plex_average(pool).await.map_err(ContractSyncError::Db)?;

    let mut tx = pool.begin().await?;

    // Stats plus the recomputed unified price, from the stored type,
    // price and highest bid.
    sqlx::query(
        "update character_contracts set
             asking_for_items = $1,
             plex_count = $2,
             abyssal_modules_count = $3,
             non_abyssal_modules_count = $4,
             unified_price = case
                 when type = 'auction' then coalesce(highest_bid, 0)
                 when type = 'item_exchange'
                     then coalesce(price, 0) + coalesce($5, 0) * $2
                 else 0
             end,
             items_synced_at = now(),
             updated_at = now()
         where id = $6",
    )
    .bind(!asked_for.is_empty())
    .bind(plex_count as i32)
    .bind(abyssal.len() as i32)
    .bind((items.len() - abyssal.len()) as i32)
    .bind(plex)
    .bind(contract_id)
    .execute(&mut *tx)
    .await?;

    // Only the abyssal modules become rows; insert-or-ignore rides over
    // the issuer/acceptor double-import, like the legacy unique key.
    for item in &abyssal {
        sqlx::query(
            "insert into character_contract_items (character_contract_id, type_id, record_id)
             values ($1, $2, $3)
             on conflict (character_contract_id, record_id) do nothing",
        )
        .bind(contract_id)
        .bind(item.type_id)
        .bind(item.record_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}
