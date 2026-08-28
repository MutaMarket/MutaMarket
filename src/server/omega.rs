//! `/api/omega-calculator` — the legacy `OmegaCalculatorController`:
//! the page's props are just the two store sale percentages from
//! `config('services.markeedragon.sale')` / `config('services.evestore.sale')`,
//! env-driven and passed through as raw strings (null when unset).
//! Everything else on the page — PLEX package prices, NES omega
//! packages, the stacking math — is hard-coded client-side in the
//! legacy Vue component and ports to `frontend/src/lib/omega.ts`; no
//! live PLEX market data is involved.

use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Env var carrying the current MarkeeDragon sale percentage, the
/// legacy `services.markeedragon.sale`. Legacy quirk, ported: the page
/// receives both values but never uses them in its calculations.
pub const MARKEEDRAGON_SALE_ENV: &str = "MARKEEDRAGON_SALE";

/// Env var carrying the current EVE Store sale percentage, the legacy
/// `services.evestore.sale`.
pub const EVESTORE_SALE_ENV: &str = "EVESTORE_SALE";

fn sale(var: &str) -> Option<String> {
    std::env::var(var).ok()
}

/// `GET /api/omega-calculator`.
pub async fn index() -> Response {
    axum::Json(json!({
        "sales": {
            "markeedragon": sale(MARKEEDRAGON_SALE_ENV),
            "evestore": sale(EVESTORE_SALE_ENV),
        },
    }))
    .into_response()
}
