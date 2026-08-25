//! `GET /api/documentation[/{page}]` — the documentation page payload with
//! the sidebar sections and previous/next neighbours, carrying the legacy
//! controller's outcomes as statuses: 404 for an unknown slug, 503 when
//! the docs cannot load.

use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::view::docs::DocumentationOutcome;

/// The index shows the first page, like the legacy controller default.
pub async fn index() -> Response {
    respond(None)
}

pub async fn show(Path(page): Path<String>) -> Response {
    respond(Some(page))
}

fn respond(page: Option<String>) -> Response {
    match crate::docs::documentation_outcome(page) {
        DocumentationOutcome::Page(data) => Json(*data).into_response(),
        DocumentationOutcome::NotFound => super::api::error(
            StatusCode::NOT_FOUND,
            "This documentation page does not exist.",
        ),
        DocumentationOutcome::Unavailable => super::api::error(
            StatusCode::SERVICE_UNAVAILABLE,
            "The documentation is temporarily unavailable.",
        ),
    }
}
