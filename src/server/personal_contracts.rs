//! The personal contracts page, the legacy `ContractController`:
//! `GET /api/personal/contracts` serves the page data (the Inertia props
//! of `ShowAllPersonalContractsPage`) and `POST /personal/contracts`
//! refreshes every character's contracts like the legacy
//! `dispatchSync(GetCharacterContractsJob)` loop.
//!
//! The page merges three sources, like the legacy controller: the user's
//! outstanding public contracts, their archived historic contracts, and
//! their ESI personal contracts — each serialized with the exact
//! `ContractResource` key set its model produces (`whenHas` keys follow
//! the table's columns, so public contracts carry no status and only
//! character contracts carry acceptor/availability keys).

use std::collections::HashMap;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde_json::json;
use sqlx::Row;

use super::AppState;
use super::support::require_api_session;
use crate::contracts::resource::{character_fragment, contract_base};
use crate::auth::session;

/// The issuer column list every source query selects.
const ISSUER_COLUMNS: &str = "ic.id as issuer_id, ic.name as issuer_name,
     ic.description as issuer_description, ic.corporation_id as issuer_corporation_id,
     (ic.premium_paid_until is not null and ic.premium_paid_until > now())
         as issuer_has_premium";

/// Loads the full module cards for the given contract items, keyed by
/// contract — the legacy `with('modules', withDefaultRelations)`, whose
/// loadout ends in `withUserNote`: the page is auth-only, so the cards
/// always carry the viewer's `note`.
async fn modules_by_contract(
    state: &AppState,
    items: Vec<(i64, i64)>,
    user_id: i64,
) -> sqlx::Result<HashMap<i64, Vec<serde_json::Value>>> {
    let mut ids: Vec<i64> = items.iter().map(|(_, item_id)| *item_id).collect();
    ids.sort_unstable();
    ids.dedup();

    let mut details =
        crate::modules::queries::details_for(&state.pool, &state.reference, ids).await?;
    crate::modules::queries::attach_user_notes(&state.pool, user_id, &mut details).await?;
    let by_id: HashMap<i64, serde_json::Value> = details
        .into_iter()
        .map(|module| (module.id, serde_json::to_value(&module).expect("module serializes")))
        .collect();

    let mut by_contract: HashMap<i64, Vec<serde_json::Value>> = HashMap::new();
    for (contract_id, item_id) in items {
        // Historic items carry no module foreign key; unknown modules are
        // simply absent, like the legacy inner-joined relation.
        if let Some(module) = by_id.get(&item_id) {
            by_contract.entry(contract_id).or_default().push(module.clone());
        }
    }
    Ok(by_contract)
}

/// The user's outstanding public contracts, the legacy `Contract` source:
/// no status/availability/acceptor columns, so those keys are absent.
async fn outstanding_contracts(
    state: &AppState,
    characters: &[i64],
    range: &(String, String),
    user_id: i64,
) -> sqlx::Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(&format!(
        "select c.id, c.type, c.unified_price as price, c.asking_for_items, c.plex_count,
                c.non_abyssal_modules_count, c.abyssal_modules_count,
                c.date_issued::text as date_issued, c.date_expired::text as date_expired,
                {ISSUER_COLUMNS}
         from contracts c
         join characters ic on ic.id = c.issuer_id
         where c.issuer_id = any($1)
           and c.abyssal_modules_count > 0
           and c.date_issued between $2::timestamptz and $3::timestamptz
         order by c.id",
    ))
    .bind(characters)
    .bind(&range.0)
    .bind(&range.1)
    .fetch_all(&state.pool)
    .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.get::<i64, _>("id")).collect();
    let items: Vec<(i64, i64)> = sqlx::query_as(
        "select contract_id, item_id from contract_items
         where contract_id = any($1) order by id",
    )
    .bind(&ids)
    .fetch_all(&state.pool)
    .await?;
    let mut modules = modules_by_contract(state, items, user_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let mut contract = contract_base(&row);
            contract["modules"] = json!(modules.remove(&id).unwrap_or_default());
            contract
        })
        .collect())
}

/// The user's archived contracts, the legacy `HistoricContract` source:
/// status present, `ignore_for_training` for admins only.
async fn historic_contracts(
    state: &AppState,
    characters: &[i64],
    range: &(String, String),
    is_admin: bool,
    user_id: i64,
) -> sqlx::Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(&format!(
        "select hc.id, hc.type, hc.unified_price as price, hc.asking_for_items, hc.plex_count,
                hc.non_abyssal_modules_count, hc.abyssal_modules_count, hc.status,
                hc.ignore_for_training,
                hc.date_issued::text as date_issued, hc.date_expired::text as date_expired,
                {ISSUER_COLUMNS}
         from historic_contracts hc
         join characters ic on ic.id = hc.issuer_id
         where hc.issuer_id = any($1)
           and hc.abyssal_modules_count > 0
           and hc.date_issued between $2::timestamptz and $3::timestamptz
         order by hc.id",
    ))
    .bind(characters)
    .bind(&range.0)
    .bind(&range.1)
    .fetch_all(&state.pool)
    .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.get::<i64, _>("id")).collect();
    let items: Vec<(i64, i64)> = sqlx::query_as(
        "select historic_contract_id, item_id from historic_contract_items
         where historic_contract_id = any($1) order by id",
    )
    .bind(&ids)
    .fetch_all(&state.pool)
    .await?;
    let mut modules = modules_by_contract(state, items, user_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let mut contract = contract_base(&row);
            contract["status"] = json!(row.get::<String, _>("status"));
            contract["modules"] = json!(modules.remove(&id).unwrap_or_default());
            if is_admin {
                contract["ignore_for_training"] = json!(row.get::<bool, _>("ignore_for_training"));
            }
            contract
        })
        .collect())
}

/// The ESI personal contracts, the legacy `CharacterContract` source:
/// raw statuses folded through `ContractStatus::parse`, `is_private`
/// from the availability, the acceptor by its universe-names category,
/// and the item `types` instead of module cards.
async fn character_contracts(
    state: &AppState,
    characters: &[i64],
    range: &(String, String),
) -> sqlx::Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(&format!(
        "select cc.id, cc.type, cc.unified_price as price, cc.asking_for_items, cc.plex_count,
                cc.non_abyssal_modules_count, cc.abyssal_modules_count, cc.status,
                cc.availability, cc.acceptor_id, cc.acceptor_type,
                cc.date_issued::text as date_issued, cc.date_expired::text as date_expired,
                cc.date_accepted::text as date_accepted,
                {ISSUER_COLUMNS},
                ac.id as acceptor_char_id, ac.name as acceptor_char_name,
                ac.description as acceptor_char_description,
                ac.corporation_id as acceptor_char_corporation_id,
                (ac.premium_paid_until is not null and ac.premium_paid_until > now())
                    as acceptor_char_has_premium,
                aa.id as acceptor_alliance_id, aa.name as acceptor_alliance_name
         from character_contracts cc
         join characters ic on ic.id = cc.issuer_id
         left join characters ac on ac.id = cc.acceptor_id and cc.acceptor_type = 'character'
         left join alliances aa on aa.id = cc.acceptor_id and cc.acceptor_type = 'alliance'
         where cc.abyssal_modules_count > 0
           and cc.date_issued between $2::timestamptz and $3::timestamptz
           and (cc.issuer_id = any($1) or cc.assignee_id = any($1) or cc.acceptor_id = any($1))
         order by cc.id",
    ))
    .bind(characters)
    .bind(&range.0)
    .bind(&range.1)
    .fetch_all(&state.pool)
    .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.get::<i64, _>("id")).collect();
    // One entry per item row, like the legacy hasManyDeep types().
    let type_rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "select cci.character_contract_id, t.id, t.name
         from character_contract_items cci
         join types t on t.id = cci.type_id
         where cci.character_contract_id = any($1)
         order by cci.id",
    )
    .bind(&ids)
    .fetch_all(&state.pool)
    .await?;
    let mut types: HashMap<i64, Vec<serde_json::Value>> = HashMap::new();
    for (contract_id, type_id, name) in type_rows {
        types
            .entry(contract_id)
            .or_default()
            .push(json!({ "id": type_id, "name": name }));
    }

    Ok(rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let acceptor_id: Option<i64> = row.get("acceptor_id");
            let acceptor_type: Option<String> = row.get("acceptor_type");
            // The legacy morphTo by acceptor_type: CharacterResource,
            // AllianceResource ({id, name}), or null for corporations
            // (their table is not ported; see the ingestion's
            // divergence note).
            let acceptor = match (acceptor_id, acceptor_type.as_deref()) {
                (Some(_), Some("character")) => {
                    character_fragment(&row, "acceptor_char").unwrap_or(serde_json::Value::Null)
                }
                (Some(_), Some("alliance")) => row
                    .get::<Option<i64>, _>("acceptor_alliance_id")
                    .map(|id| {
                        serde_json::json!({
                            "id": id,
                            "name": row.get::<String, _>("acceptor_alliance_name"),
                        })
                    })
                    .unwrap_or(serde_json::Value::Null),
                _ => serde_json::Value::Null,
            };
            let mut contract = contract_base(&row);
            contract["status"] =
                json!(crate::contracts::parse_contract_status(&row.get::<String, _>("status")));
            contract["types"] = json!(types.remove(&id).unwrap_or_default());
            contract["is_private"] = json!(row.get::<String, _>("availability") != "public");
            contract["acceptor"] = acceptor;
            contract["acceptor_type"] = json!(acceptor_type);
            contract["date_accepted"] = json!(row.get::<Option<String>, _>("date_accepted"));
            contract
        })
        .collect())
}

#[derive(serde::Deserialize, Default)]
pub struct DateRangeParams {
    date_start: Option<String>,
    date_end: Option<String>,
}

/// `GET /api/personal/contracts?date_start=&date_end=` — the page data.
/// The default window is the last month, like the legacy
/// `now()->subMonth()` fallback; an unparseable date surfaces as a 500,
/// like the legacy Carbon parse exception.
pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<DateRangeParams>,
) -> Response {
    let session = match require_api_session(&state.pool, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let range: (String, String) = match sqlx::query_as(
        "select coalesce($1::timestamptz, now() - interval '1 month')::text,
                coalesce($2::timestamptz, now())::text",
    )
    .bind(params.date_start.as_deref().filter(|text| !text.is_empty()))
    .bind(params.date_end.as_deref().filter(|text| !text.is_empty()))
    .fetch_one(&state.pool)
    .await
    {
        Ok(range) => range,
        Err(error) => return super::api::database_error(error),
    };

    let characters: Vec<i64> =
        match sqlx::query_scalar("select id from characters where user_id = $1 order by id")
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await
        {
            Ok(characters) => characters,
            Err(error) => return super::api::database_error(error),
        };
    let is_admin: bool = match sqlx::query_scalar("select is_admin from users where id = $1")
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(is_admin) => is_admin.unwrap_or(false),
        Err(error) => return super::api::database_error(error),
    };

    // The legacy spread order: outstanding, historic, character. The
    // three sources are independent, so they load concurrently.
    let (mut contracts, historic, personal) = match tokio::try_join!(
        outstanding_contracts(&state, &characters, &range, session.user_id),
        historic_contracts(&state, &characters, &range, is_admin, session.user_id),
        character_contracts(&state, &characters, &range),
    ) {
        Ok(sources) => sources,
        Err(error) => return super::api::database_error(error),
    };
    contracts.extend(historic);
    contracts.extend(personal);

    axum::Json(json!({
        "contracts": contracts,
        "date_start": range.0,
        "date_end": range.1,
    }))
    .into_response()
}

/// `POST /personal/contracts` — the legacy `ContractController::store`:
/// synchronously refreshes every character's contracts
/// (`dispatchSync(GetCharacterContractsJob)`) and redirects back. A
/// character whose fetch fails is skipped, like the legacy job's
/// log-and-return.
pub async fn store(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            tracing::warn!(%error, "personal contracts session lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let characters: Vec<i64> =
        match sqlx::query_scalar("select id from characters where user_id = $1 order by id")
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await
        {
            Ok(characters) => characters,
            Err(error) => {
                tracing::warn!(%error, "personal contracts character lookup failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    for character_id in characters {
        if let Err(error) = crate::contracts::character::sync_character_contracts(
            &state.pool,
            &state.reference,
            &state.esi,
            &state.sso,
            character_id,
        )
        .await
        {
            tracing::warn!(%error, character_id, "requested contract refresh failed");
        }
    }

    // back(): the previous page from the Referer header, falling back to
    // the contracts page itself.
    let back = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/personal/contracts");
    Redirect::to(back).into_response()
}
