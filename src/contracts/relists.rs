//! Failure inference from relists: the item in a contract the seller
//! puts back on the market never changed hands, so the earlier contract
//! failed. ESI's items-endpoint probe only answers while a contract is
//! fresh and is never asked about multi-item contracts, which leaves a
//! large `unknown` backlog for the moderator review; this resolves the
//! part of it the market itself already answered.
//!
//! Measured against the probe's own verdicts on the first production
//! run (2026-09-13): of the archived contracts a same-seller relist
//! followed, 142'809 were recorded `failed` and 814 `completed`, so the
//! rule agrees with ESI on 99.4% of the contracts it can be checked
//! against. It resolved 15'250 of the 122'959 that were `unknown`.
//!
//! Those 814 keep their status: an accepted contract whose seller later
//! bought the module back looks exactly like a relist from here, and
//! the probe saw the acceptance directly. They keep the evidence column
//! too, so the conflict stays findable.

use sqlx::PgPool;

#[derive(Debug, Default, Clone, Copy)]
pub struct RelistStats {
    /// Contracts the relist proved failed: `unknown` until now.
    pub resolved: u64,
    /// Contracts ESI reported `completed` although the seller relisted
    /// the module. Status untouched, evidence recorded.
    pub conflicting: u64,
    /// Contracts already recorded `failed`; the relist confirms them and
    /// backfills the evidence.
    pub confirmed: u64,
}

/// Records the relist evidence on every archived contract whose module
/// the same seller listed again, and flips the `unknown` ones to
/// `failed`.
///
/// Only the *immediate* successor of a module counts: in a
/// seller A -> seller B -> seller A chain, A's first contract did sell
/// (to B, who sold it back), so a same-seller listing further down the
/// line proves nothing on its own.
///
/// The whole contract history is swept each run (a couple of seconds at
/// production size) and every write is idempotent: a second run over
/// unchanged data reports zeroes. A moderator review is not exempted:
/// the only status this overwrites is `unknown`, which is the reviewer
/// saying they could not tell either.
pub async fn infer_failed_relists(pool: &PgPool) -> sqlx::Result<RelistStats> {
    let previous_statuses: Vec<String> = sqlx::query_scalar(
        "with timeline as (
             select hci.item_id as module_id, hc.id as contract_id, hc.issuer_id,
                    hc.date_issued, hc.status
             from historic_contract_items hci
             join historic_contracts hc on hc.id = hci.historic_contract_id
             union all
             select ci.item_id, c.id, c.issuer_id, c.date_issued, 'outstanding'
             from contract_items ci
             join contracts c on c.id = ci.contract_id
         ),
         successor as (
             select contract_id, status, issuer_id,
                    lead(issuer_id) over w as next_issuer_id,
                    lead(contract_id) over w as next_contract_id
             from timeline
             window w as (partition by module_id order by date_issued, contract_id)
         ),
         relisted as (
             -- A multi-module contract can be relisted module by module;
             -- the earliest of those listings is the evidence.
             select distinct on (contract_id) contract_id, status, next_contract_id
             from successor
             where next_issuer_id = issuer_id
             order by contract_id, next_contract_id
         )
         update historic_contracts hc
         set status = case when hc.status = 'unknown' then 'failed' else hc.status end,
             relisted_by_contract_id = r.next_contract_id,
             updated_at = now()
         from relisted r
         where hc.id = r.contract_id
           and (hc.status = 'unknown'
                or hc.relisted_by_contract_id is distinct from r.next_contract_id)
         returning r.status",
    )
    .fetch_all(pool)
    .await?;

    let mut stats = RelistStats::default();
    for status in previous_statuses {
        match status.as_str() {
            "unknown" => stats.resolved += 1,
            "completed" => stats.conflicting += 1,
            _ => stats.confirmed += 1,
        }
    }
    Ok(stats)
}
