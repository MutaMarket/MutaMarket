//! Corporation ingestion, ported from the legacy `GetCorporationJob` →
//! `CreateCorporationAction` pair: fetch a corporation's public sheet
//! and upsert the record with stub character rows for its CEO and
//! creator.

use sqlx::PgPool;

use crate::esi::{EsiClient, EsiCorporation};

/// The legacy `GetCorporationJob` semantics: fetch the sheet and upsert
/// even when the row already exists (the alliance-executor path always
/// refreshes). An ESI failure is logged and skipped (the legacy
/// "Failed to get corporation" early return); database errors propagate.
pub async fn fetch_corporation(
    pool: &PgPool,
    esi: &EsiClient,
    corporation_id: i64,
) -> sqlx::Result<()> {
    match esi.corporation(corporation_id).await {
        Ok(details) => upsert_corporation(pool, corporation_id, &details).await,
        Err(error) => {
            tracing::warn!("corporation {corporation_id} failed: {error}");
            Ok(())
        }
    }
}

/// The legacy `CreateContractAcceptorsAction` corporation path: fetch
/// and store only ids missing from the table (`getMissingCorporations`
/// + `GetCorporationJob::dispatchSync`).
pub async fn ensure_corporations(
    pool: &PgPool,
    esi: &EsiClient,
    corporation_ids: &[i64],
) -> sqlx::Result<()> {
    if corporation_ids.is_empty() {
        return Ok(());
    }
    let existing: Vec<i64> = sqlx::query_scalar("select id from corporations where id = any($1)")
        .bind(corporation_ids)
        .fetch_all(pool)
        .await?;

    for corporation_id in corporation_ids.iter().filter(|id| !existing.contains(id)) {
        fetch_corporation(pool, esi, *corporation_id).await?;
    }
    Ok(())
}

/// The legacy `CreateCorporationAction`: stub character rows for the CEO
/// and creator, then the updateOrCreate of the corporation record.
/// Quirks ported faithfully: `ceo_id` is stored only while
/// `member_count` is truthy (the action's `$corporation_details->member_count
/// ? $corporation_details->ceo_id : null`, which also keeps a closed
/// corporation's NPC CEO out of the characters FK), and `alliance_id`
/// is never written (the action omits it, so the column stays null).
async fn upsert_corporation(
    pool: &PgPool,
    corporation_id: i64,
    details: &EsiCorporation,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    for character_id in [details.ceo_id, details.creator_id] {
        sqlx::query("insert into characters (id, name) values ($1, '') on conflict (id) do nothing")
            .bind(character_id)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        "insert into corporations
         (id, name, ticker, member_count, ceo_id, creator_id, date_founded, description,
          faction_id, home_station_id, shares, tax_rate, url, war_eligible)
         values ($1, $2, $3, $4, $5, $6, $7::timestamptz, $8, $9, $10, $11, $12, $13, $14)
         on conflict (id) do update set
             name = excluded.name,
             ticker = excluded.ticker,
             member_count = excluded.member_count,
             ceo_id = excluded.ceo_id,
             creator_id = excluded.creator_id,
             date_founded = excluded.date_founded,
             description = excluded.description,
             faction_id = excluded.faction_id,
             home_station_id = excluded.home_station_id,
             shares = excluded.shares,
             tax_rate = excluded.tax_rate,
             url = excluded.url,
             war_eligible = excluded.war_eligible,
             updated_at = now()",
    )
    .bind(corporation_id)
    .bind(&details.name)
    .bind(&details.ticker)
    .bind(details.member_count)
    .bind((details.member_count != 0).then_some(details.ceo_id))
    .bind(details.creator_id)
    .bind(&details.date_founded)
    .bind(&details.description)
    .bind(details.faction_id)
    .bind(details.home_station_id)
    .bind(details.shares)
    .bind(details.tax_rate)
    .bind(&details.url)
    .bind(details.war_eligible)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
