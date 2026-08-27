//! `/api/calculator` — the mutation calculator, the legacy
//! `CalculatorController`: for the queried abyssal type (and optional
//! meta filters), every (mutaplasmid, source type) combination with the
//! probability of rolling into the queried attribute bounds and the
//! expected cost per successful roll. The probability math lives in
//! `mutation::probability` (with the derived-attribute correction).

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::AppState;
use crate::mutation::probability::{RequestedBound, combination_probability};

pub async fn index_root(State(state): State<AppState>) -> Response {
    respond(&state, "").await
}

pub async fn index(
    State(state): State<AppState>,
    axum::extract::Path(query): axum::extract::Path<String>,
) -> Response {
    respond(&state, &query).await
}

async fn respond(state: &AppState, query: &str) -> Response {
    let search = match crate::modules::search::parse(&state.pool, &state.reference, query).await {
        Ok(search) => search,
        Err(crate::modules::search::SearchError::Db(error)) => {
            return super::api::database_error(error);
        }
        // A broken query degrades to the empty calculator, like the
        // legacy QueryService's silent fallbacks.
        Err(_) => return axum::Json(serde_json::Value::Null).into_response(),
    };

    // Without a type the page shows its select-a-category state (the
    // legacy page received a null probability prop).
    let Some(type_filter) = &search.type_filter else {
        return axum::Json(serde_json::Value::Null).into_response();
    };
    let output_type_id = type_filter.id;

    // Every combination producing the type, the legacy
    // MutaplasmidInputType query with the published/meta gates (meta
    // level is the dogma attribute, like the module search filter). The
    // legacy get() is unordered; ours orders for a stable response.
    type ComboRow = (i64, String, i64, String);
    let combos: Result<Vec<ComboRow>, _> = sqlx::query_as(
        "select mit.mutaplasmid_id, mp.name, mit.type_id, t.name
         from mutaplasmid_input_types mit
         join mutaplasmids mp on mp.id = mit.mutaplasmid_id
         join types t on t.id = mit.type_id
         where mp.output_type_id = $1
           and t.published
           and ($2::bigint is null or t.meta_group_id = $2)
           and ($3::float8 is null or exists (
               select 1 from type_attributes ta
               where ta.type_id = t.id and ta.attribute_id = $4 and ta.value = $3))
         order by mit.mutaplasmid_id, mit.type_id",
    )
    .bind(output_type_id)
    .bind(search.meta_group_id)
    .bind(search.meta_level)
    .bind(crate::modules::META_LEVEL_ATTRIBUTE_ID)
    .fetch_all(&state.pool)
    .await;
    let combos = match combos {
        Ok(combos) => combos,
        Err(error) => return super::api::database_error(error),
    };

    // Latest recorded market average per involved type (the legacy
    // marketHistory relation; ours picks the newest day).
    let mut price_ids: Vec<i64> = combos
        .iter()
        .flat_map(|(mutaplasmid_id, _, type_id, ..)| [*mutaplasmid_id, *type_id])
        .collect();
    price_ids.sort_unstable();
    price_ids.dedup();
    let prices: Result<Vec<(i64, f64)>, _> = sqlx::query_as(
        "select distinct on (type_id) type_id, average
         from market_histories
         where type_id = any($1)
         order by type_id, date desc",
    )
    .bind(&price_ids)
    .fetch_all(&state.pool)
    .await;
    let prices: std::collections::HashMap<i64, f64> = match prices {
        Ok(prices) => prices.into_iter().collect(),
        Err(error) => return super::api::database_error(error),
    };

    let requested: Vec<RequestedBound> = search
        .attributes
        .iter()
        .map(|filter| RequestedBound {
            attribute_id: filter.attribute_id,
            min: filter.min,
            max: filter.max,
        })
        .collect();

    let rows: Vec<serde_json::Value> = combos
        .into_iter()
        .map(|(mutaplasmid_id, mutaplasmid_name, type_id, type_name)| {
            let probability =
                combination_probability(&state.reference, type_id, mutaplasmid_id, &requested)
                    .unwrap_or(0.0);

            // The legacy cost model: buy the source and the mutaplasmid
            // until one roll succeeds. Unknown mutaplasmid price
            // coalesces to 0, unknown source price nulls the cost.
            let cost_mutaplasmid = prices.get(&mutaplasmid_id).copied().unwrap_or(0.0);
            let cost_type = prices.get(&type_id).copied();
            let cost = match cost_type {
                Some(cost_type) if probability > 0.0 => {
                    Some((cost_type + cost_mutaplasmid) / probability)
                }
                _ => None,
            };

            json!({
                "mutaplasmid": { "id": mutaplasmid_id, "name": mutaplasmid_name },
                "type": { "id": type_id, "name": type_name },
                "probability": probability,
                "cost": cost,
                "cost_mutaplasmid": cost_mutaplasmid,
                "cost_type": cost_type,
            })
        })
        .collect();

    axum::Json(rows).into_response()
}
