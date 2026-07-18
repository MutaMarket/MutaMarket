//! The display preference endpoint, ported from the legacy
//! `DisplayController`: `PUT /display` validates the three settings and
//! stores them as year-long cookies, then redirects back.

use axum::Json;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use serde_json::json;

use crate::modules::view::{ATTRIBUTE_BAR_MODES, DISPLAY_VALUES, DisplaySettings};

/// Cookie lifetime, like the legacy controller's one year.
const TTL_SECONDS: i64 = 60 * 60 * 24 * 365;

pub const DISPLAY_COOKIE: &str = "display";
pub const ATTRIBUTE_BAR_MODE_COOKIE: &str = "attribute_bar_mode";
pub const SHOW_ATTRIBUTE_SCORES_COOKIE: &str = "show_attribute_scores";

#[derive(Deserialize, Default)]
struct DisplayPayload {
    display: Option<String>,
    attribute_bar_mode: Option<String>,
    show_attribute_scores: Option<serde_json::Value>,
}

/// `PUT /display`
pub async fn update(headers: HeaderMap, body: Bytes) -> Response {
    let payload: DisplayPayload = serde_json::from_slice(&body).unwrap_or_default();

    let display = payload
        .display
        .filter(|value| DISPLAY_VALUES.contains(&value.as_str()));
    let attribute_bar_mode = payload
        .attribute_bar_mode
        .filter(|value| ATTRIBUTE_BAR_MODES.contains(&value.as_str()));
    let show_attribute_scores = payload.show_attribute_scores.as_ref().and_then(boolean);

    let (Some(display), Some(attribute_bar_mode), Some(show_attribute_scores)) =
        (display, attribute_bar_mode, show_attribute_scores)
    else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "message": "The given data was invalid." })),
        )
            .into_response();
    };

    let back = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");

    let mut response = Redirect::to(back).into_response();
    for (name, value) in [
        (DISPLAY_COOKIE, display),
        (ATTRIBUTE_BAR_MODE_COOKIE, attribute_bar_mode),
        (
            SHOW_ATTRIBUTE_SCORES_COOKIE,
            if show_attribute_scores { "1" } else { "0" }.to_owned(),
        ),
    ] {
        let cookie = format!("{name}={value}; Path=/; SameSite=Lax; Max-Age={TTL_SECONDS}");
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }

    response
}

/// The display settings of the request's cookies, with legacy defaults.
pub fn settings_from_headers(headers: &HeaderMap) -> DisplaySettings {
    let cookie = |name| crate::auth::session::cookie_value(headers, name);
    let defaults = DisplaySettings::default();

    DisplaySettings {
        display: cookie(DISPLAY_COOKIE)
            .filter(|value| DISPLAY_VALUES.contains(&value.as_str()))
            .unwrap_or(defaults.display),
        attribute_bar_mode: cookie(ATTRIBUTE_BAR_MODE_COOKIE)
            .filter(|value| ATTRIBUTE_BAR_MODES.contains(&value.as_str()))
            .unwrap_or(defaults.attribute_bar_mode),
        show_attribute_scores: cookie(SHOW_ATTRIBUTE_SCORES_COOKIE)
            .map(|value| value == "1" || value == "true")
            .unwrap_or(defaults.show_attribute_scores),
    }
}

/// Laravel's boolean validation accepts true/false, 1/0 and "1"/"0".
fn boolean(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(boolean) => Some(*boolean),
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        },
        serde_json::Value::String(string) => match string.as_str() {
            "1" | "true" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
