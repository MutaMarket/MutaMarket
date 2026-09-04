//! The moderator contract review, the legacy
//! `ModeratorContractController`: `GET /api/moderator/contracts[/{query}]`
//! picks a random unreviewed historic contract matching the filter query,
//! `POST /moderator/contracts/{historicContract}` records the review.
//!
//! Legacy quirk ported faithfully: there is no moderator role — the page
//! route carries no middleware at all (the data is public) and the review
//! action sits behind the plain auth gate, so any logged-in user may
//! review. Authorship lands per user in `contract_review_history`.
//!
//! Divergence, like the offer routes: the legacy answered the review
//! action's failure paths with redirects carrying flash toasts; the
//! fetch-driven frontend gets the same texts as JSON statuses instead
//! (409 already reviewed, 422 validation).

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde_json::json;
use sqlx::{Postgres, QueryBuilder, Row};

use super::AppState;
use super::support::{error_json, validation_error};
use crate::auth::session;
use crate::modules::search::{self, Search, SearchError};

/// The statuses a review may assign, the legacy `ContractStatus` enum
/// behind `Rule::enum`.
const REVIEW_STATUSES: [&str; 4] = ["outstanding", "completed", "failed", "unknown"];

/// A random reviewable contract id: an unknown-status single-abyssal
/// item exchange whose module matches the filter query — the legacy
/// index query including its `inRandomOrder()`.
async fn random_reviewable_contract(
    pool: &sqlx::PgPool,
    search: &Search,
) -> sqlx::Result<Option<i64>> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        // The legacy single-item rule: exactly one abyssal module and
        // nothing else, so the sale price belongs to that module.
        "select hc.id from historic_contracts hc
         where hc.type = 'item_exchange' and hc.status = 'unknown'
           and hc.abyssal_modules_count = 1 and hc.non_abyssal_modules_count = 0
           and exists (select 1 from historic_contract_items hci
                       join modules m on m.id = hci.item_id
                       where hci.historic_contract_id = hc.id",
    );
    search::push_common_filters(&mut builder, search);
    if let Some(minimum) = search.needs_training {
        // The legacy whereNeedsTraining: the module type's estimator
        // holds fewer than `minimum` training samples.
        builder.push(
            " and exists (select 1 from estimator_statistics es
               where es.type_id = m.type_id and es.data_count < ",
        );
        builder.push_bind(minimum);
        builder.push(")");
    }
    builder.push(") order by random() limit 1");

    builder.build_query_scalar().fetch_optional(pool).await
}

/// One historic contract with its issuer and full module cards, the
/// exact `ContractResource` key set of the review page (the
/// `ignore_for_training` key rides along for admins only; a signed-in
/// viewer's module cards carry their `note`, the legacy
/// `withDefaultRelations` loadout ending in `withUserNote`).
async fn historic_contract_json(
    state: &AppState,
    contract_id: i64,
    for_admin: bool,
    user_id: Option<i64>,
) -> sqlx::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        "select hc.id, hc.type, hc.unified_price as price, hc.asking_for_items, hc.plex_count,
                hc.non_abyssal_modules_count, hc.abyssal_modules_count, hc.status,
                hc.ignore_for_training,
                hc.date_issued::text as date_issued, hc.date_expired::text as date_expired,
                ic.id as issuer_id, ic.name as issuer_name,
                ic.description as issuer_description,
                ic.corporation_id as issuer_corporation_id,
                (ic.premium_paid_until is not null and ic.premium_paid_until > now())
                    as issuer_has_premium
         from historic_contracts hc
         join characters ic on ic.id = hc.issuer_id
         where hc.id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(row) = row else { return Ok(None) };

    let item_ids: Vec<i64> = sqlx::query_scalar(
        "select item_id from historic_contract_items
         where historic_contract_id = $1 order by id",
    )
    .bind(contract_id)
    .fetch_all(&state.pool)
    .await?;
    let details = crate::modules::queries::with_default_relations(
        &state.pool,
        &state.reference,
        item_ids,
        user_id,
    )
    .await?;
    let modules: Vec<serde_json::Value> = details
        .iter()
        .map(|module| serde_json::to_value(module).expect("module serializes"))
        .collect();

    let mut contract = crate::contracts::resource::contract_base(&row);
    contract["status"] = json!(row.get::<String, _>("status"));
    contract["modules"] = json!(modules);
    if for_admin {
        contract["ignore_for_training"] = json!(row.get::<bool, _>("ignore_for_training"));
    }
    Ok(Some(contract))
}

pub async fn page_root(State(state): State<AppState>, headers: HeaderMap) -> Response {
    page_response(&state, &headers, "").await
}

/// `GET /api/moderator/contracts/{query}` — the review page data: a
/// random reviewable contract (or null once none are left) and the
/// server-resolved filter echo.
pub async fn page(
    State(state): State<AppState>,
    Path(query): Path<String>,
    headers: HeaderMap,
) -> Response {
    page_response(&state, &headers, &query).await
}

async fn page_response(state: &AppState, headers: &HeaderMap, query: &str) -> Response {
    let search = match search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(SearchError::TypeNotFound) => {
            return super::api::error(StatusCode::NOT_FOUND, "Please provide a valid type.");
        }
        Err(SearchError::Invalid(message)) => {
            return super::api::error(StatusCode::BAD_REQUEST, &message);
        }
        Err(SearchError::Db(error)) => return super::api::database_error(error),
    };

    // The training flag is admin-only, like the resource; the page
    // itself needs no session, but a signed-in viewer gets their notes.
    let (for_admin, user_id) = match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => {
            match sqlx::query_scalar::<_, bool>("select is_admin from users where id = $1")
                .bind(session.user_id)
                .fetch_optional(&state.pool)
                .await
            {
                Ok(is_admin) => (is_admin.unwrap_or(false), Some(session.user_id)),
                Err(error) => return super::api::database_error(error),
            }
        }
        Ok(None) => (false, None),
        Err(error) => return super::api::database_error(error),
    };

    let contract = match random_reviewable_contract(&state.pool, &search).await {
        Ok(Some(contract_id)) => {
            match historic_contract_json(state, contract_id, for_admin, user_id).await {
                Ok(contract) => contract,
                Err(error) => return super::api::database_error(error),
            }
        }
        Ok(None) => None,
        Err(error) => return super::api::database_error(error),
    };

    axum::Json(json!({
        "contract": contract,
        "search": {
            "type": search
                .type_filter
                .as_ref()
                .map(|filter| json!({ "id": filter.id, "name": filter.name })),
            "needs_training": search.needs_training,
        },
    }))
    .into_response()
}

/// `POST /moderator/contracts/{historicContract}` — the legacy
/// `ModeratorContractController::store`: only unknown contracts may be
/// reviewed, the status update lands with an audit row, and success
/// redirects back.
pub async fn store(
    State(state): State<AppState>,
    Path(contract_id): Path<i64>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session::session_from_headers(&state.pool, &headers).await {
        Ok(Some(session)) => session,
        Ok(None) => return Redirect::to("/login").into_response(),
        Err(error) => {
            tracing::warn!(%error, "contract review session lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Route model binding: an unknown contract is a 404, and Laravel
    // resolves the binding before the FormRequest validates, so the 404
    // precedes any 422.
    let previous: Option<String> =
        match sqlx::query_scalar("select status from historic_contracts where id = $1")
            .bind(contract_id)
            .fetch_optional(&state.pool)
            .await
        {
            Ok(previous) => previous,
            Err(error) => {
                tracing::warn!(%error, "contract review lookup failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
    let Some(previous) = previous else {
        return error_json(StatusCode::NOT_FOUND, "Not found.");
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        status: Option<String>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(status) = payload.status else {
        return validation_error("status", "The status field is required.");
    };
    if !REVIEW_STATUSES.contains(&status.as_str()) {
        return validation_error("status", "The selected status is invalid.");
    }

    if previous != "unknown" {
        return error_json(
            StatusCode::CONFLICT,
            "The contract has already been reviewed.",
        );
    }

    let update =
        sqlx::query("update historic_contracts set status = $2, updated_at = now() where id = $1")
            .bind(contract_id)
            .bind(&status)
            .execute(&state.pool)
            .await;
    if let Err(error) = update {
        tracing::warn!(%error, "contract review update failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let audit = sqlx::query(
        "insert into contract_review_history
             (historic_contract_id, user_id, previous_status, new_status)
         values ($1, $2, $3, $4)",
    )
    .bind(contract_id)
    .bind(session.user_id)
    .bind(&previous)
    .bind(&status)
    .execute(&state.pool)
    .await;
    if let Err(error) = audit {
        tracing::warn!(%error, "contract review audit insert failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // back(): the previous page, falling back to the review page.
    super::support::back_or(&headers, "/moderator/contracts").into_response()
}
