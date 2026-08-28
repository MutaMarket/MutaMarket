//! The sidebar routes: bookmarks (the legacy `BookmarkController` and
//! `BookmarksData` shared prop) plus the in-app advertisement and
//! recommended-gear rotations (`Advertisements`/`GearItems` shared
//! props, the `visible()` scopes).

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde_json::json;

use super::AppState;
use crate::auth::session::{Session, session_from_headers};

async fn session_or_login(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Session, Response> {
    match session_from_headers(&state.pool, headers).await {
        Ok(Some(session)) => Ok(session),
        Ok(None) => Err(Redirect::to("/login").into_response()),
        Err(error) => {
            tracing::warn!(%error, "bookmark session lookup failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

fn back(headers: &HeaderMap) -> Redirect {
    let target = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("/");
    Redirect::to(target)
}

fn db_error(error: sqlx::Error) -> Response {
    tracing::warn!(%error, "sidebar database error");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

fn validation_error(field: &str, message: &str) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        axum::Json(json!({
            "message": "The given data was invalid.",
            "errors": { field: [message] },
        })),
    )
        .into_response()
}

/// `GET /api/sidebar` — everything the sidebar renders in one payload:
/// the user's bookmarks (null for guests), the visible ad and gear
/// rotations, and the donation lists (the legacy shared `donations`
/// prop, rendered by the sidebar's top-donors card and the /donations
/// page).
pub async fn payload(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let session = match session_from_headers(&state.pool, &headers).await {
        Ok(session) => session,
        Err(error) => return db_error(error),
    };

    let bookmarks = match &session {
        Some(session) => {
            type Row = (i64, String, String, Option<i64>);
            let rows: Result<Vec<Row>, _> = sqlx::query_as(
                "select id, name, query, type_id from bookmarks
                 where user_id = $1 order by id",
            )
            .bind(session.user_id)
            .fetch_all(&state.pool)
            .await;
            match rows {
                Ok(rows) => Some(
                    rows.into_iter()
                        .map(|(id, name, query, type_id)| {
                            json!({ "id": id, "name": name, "query": query, "type_id": type_id })
                        })
                        .collect::<Vec<_>>(),
                ),
                Err(error) => return db_error(error),
            }
        }
        None => None,
    };

    // The legacy Advertisement::visible() scope, priority first.
    type AdRow = (i64, String, Option<String>, Option<String>, Option<String>, String);
    let advertisements: Result<Vec<AdRow>, _> = sqlx::query_as(
        "select id, name, description, image_url, link, size from advertisements
         where active
           and (starts_at is null or starts_at <= now())
           and (expires_at is null or expires_at > now())
         order by priority desc, id desc",
    )
    .fetch_all(&state.pool)
    .await;
    let advertisements = match advertisements {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, name, description, image_url, link, size)| {
                json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "image_url": image_url,
                    "link": link,
                    "size": size,
                })
            })
            .collect::<Vec<_>>(),
        Err(error) => return db_error(error),
    };

    type GearRow = (i64, String, Option<String>, Option<String>, String);
    let gear: Result<Vec<GearRow>, _> = sqlx::query_as(
        "select id, name, description, image_url, link from gear_items
         where active order by priority desc, id desc",
    )
    .fetch_all(&state.pool)
    .await;
    let gear_items = match gear {
        Ok(rows) => rows
            .into_iter()
            .map(|(id, name, description, image_url, link)| {
                json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "image_url": image_url,
                    "link": link,
                })
            })
            .collect::<Vec<_>>(),
        Err(error) => return db_error(error),
    };

    let donations = match crate::donations::donation_lists(&state.pool).await {
        Ok(donations) => donations,
        Err(error) => return db_error(error),
    };

    axum::Json(json!({
        "bookmarks": bookmarks,
        "advertisements": advertisements,
        "gear_items": gear_items,
        "donations": donations,
    }))
    .into_response()
}

/// `POST /bookmarks` — the legacy `BookmarkController::store`.
pub async fn store(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        query: Option<String>,
        name: Option<String>,
        type_id: Option<i64>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(query) = payload.query.filter(|query| !query.is_empty()) else {
        return validation_error("query", "The query field is required.");
    };
    let Some(name) = payload.name.filter(|name| !name.is_empty()) else {
        return validation_error("name", "The name field is required.");
    };

    if let Some(type_id) = payload.type_id {
        let exists: bool =
            match sqlx::query_scalar("select exists(select 1 from types where id = $1)")
                .bind(type_id)
                .fetch_one(&state.pool)
                .await
            {
                Ok(exists) => exists,
                Err(error) => return db_error(error),
            };
        if !exists {
            return validation_error("type_id", "The selected type id is invalid.");
        }
    }

    let result = sqlx::query(
        "insert into bookmarks (user_id, type_id, name, query) values ($1, $2, $3, $4)",
    )
    .bind(session.user_id)
    .bind(payload.type_id)
    .bind(&name)
    .bind(&query)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => back(&headers).into_response(),
        Err(error) => db_error(error),
    }
}

/// `PUT /bookmarks/{bookmark}` — renames, owner only.
pub async fn update(
    State(state): State<AppState>,
    axum::extract::Path(bookmark): axum::extract::Path<i64>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    #[derive(serde::Deserialize, Default)]
    struct Payload {
        name: Option<String>,
    }
    let payload: Payload = serde_json::from_slice(&body).unwrap_or_default();
    let Some(name) = payload.name.filter(|name| !name.is_empty()) else {
        return validation_error("name", "The name field is required.");
    };

    let result = sqlx::query(
        "update bookmarks set name = $1, updated_at = now() where id = $2 and user_id = $3",
    )
    .bind(&name)
    .bind(bookmark)
    .bind(session.user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(updated) if updated.rows_affected() > 0 => back(&headers).into_response(),
        Ok(_) => super::api::error(StatusCode::FORBIDDEN, "Forbidden."),
        Err(error) => db_error(error),
    }
}

/// `DELETE /bookmarks/{bookmark}` — owner only.
pub async fn destroy(
    State(state): State<AppState>,
    axum::extract::Path(bookmark): axum::extract::Path<i64>,
    headers: HeaderMap,
) -> Response {
    let session = match session_or_login(&state, &headers).await {
        Ok(session) => session,
        Err(response) => return response,
    };

    let result = sqlx::query("delete from bookmarks where id = $1 and user_id = $2")
        .bind(bookmark)
        .bind(session.user_id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(deleted) if deleted.rows_affected() > 0 => back(&headers).into_response(),
        Ok(_) => super::api::error(StatusCode::FORBIDDEN, "Forbidden."),
        Err(error) => db_error(error),
    }
}
