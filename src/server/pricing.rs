//! The module pricing route: `POST /module-pricing` (legacy
//! `ModulePricingController::store`).
//!
//! Divergence, as documented in `server::offers`: the legacy answered
//! `back()->notify(...)` flash toasts; the fetch-driven frontend gets the
//! bare referer redirect on success and JSON statuses with the legacy
//! texts on failure. The legacy success toast literally said "Notes
//! updated successfully." — a copy-paste quirk that has no place to land
//! here, noted for parity's sake.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use crate::auth::session;
use crate::modules::pricing::PricingEntry;

async fn session_or_login(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<session::Session, Response> {
    match session::session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "pricing session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

fn validation_error(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(serde_json::json!({
            "message": "The given data was invalid.",
            "errors": { field: [message] },
        })),
    )
        .into_response()
}

fn back(headers: &HeaderMap) -> Redirect {
    let target = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");
    Redirect::to(target)
}

fn db_error(error: sqlx::Error) -> Response {
    tracing::warn!(%error, "pricing database error");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// A Laravel `integer`-rule value: a JSON integer, or an integer string.
fn as_integer(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

/// A Laravel `numeric`-rule value: a JSON number, or a numeric string.
fn as_numeric(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(string) => string.parse().ok(),
        _ => None,
    }
}

/// `POST /module-pricing` — bulk upsert of the user's asking prices,
/// validated like `StoreBulkModulePricingsRequest` with Laravel's
/// default messages.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();

    let pricing = &payload["module_pricing"];
    if pricing.is_null() || pricing.as_array().is_some_and(Vec::is_empty) {
        return validation_error("module_pricing", "The module pricing field is required.");
    }
    let Some(items) = pricing.as_array() else {
        return validation_error("module_pricing", "The module pricing field must be an array.");
    };

    let mut entries = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let module_id_value = &item["module_id"];
        if module_id_value.is_null() {
            return validation_error(
                &format!("module_pricing.{index}.module_id"),
                &format!("The module pricing.{index}.module id field is required."),
            );
        }
        let Some(module_id) = as_integer(module_id_value) else {
            return validation_error(
                &format!("module_pricing.{index}.module_id"),
                &format!("The module pricing.{index}.module id field must be an integer."),
            );
        };

        let price_value = &item["price"];
        if price_value.is_null() {
            return validation_error(
                &format!("module_pricing.{index}.price"),
                &format!("The module pricing.{index}.price field is required."),
            );
        }
        let Some(price) = as_numeric(price_value) else {
            return validation_error(
                &format!("module_pricing.{index}.price"),
                &format!("The module pricing.{index}.price field must be a number."),
            );
        };

        entries.push(PricingEntry { module_id, price });
    }

    // The exists:modules,id rule, batched.
    let ids: Vec<i64> = entries.iter().map(|entry| entry.module_id).collect();
    let known: Vec<i64> = match sqlx::query_scalar("select id from modules where id = any($1)")
        .bind(&ids)
        .fetch_all(&state.pool)
        .await
    {
        Ok(known) => known,
        Err(error) => return db_error(error),
    };
    if let Some(index) = ids.iter().position(|id| !known.contains(id)) {
        return validation_error(
            &format!("module_pricing.{index}.module_id"),
            &format!("The selected module pricing.{index}.module id is invalid."),
        );
    }

    match crate::modules::pricing::store_module_pricings(&state.pool, session.user_id, &entries)
        .await
    {
        Ok(()) => back(&headers).into_response(),
        Err(error) => db_error(error),
    }
}
