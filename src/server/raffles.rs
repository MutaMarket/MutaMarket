//! `PUT|DELETE /raffle/{raffle_item}` — the legacy `RaffleController`:
//! the winner claiming or declining a drawn prize from the site-wide
//! dialog. Both aborts are the legacy bare 403s; claiming lands on the
//! settings page (where the code shows), declining goes `back()`.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use super::support::{back, session_or_login};
use crate::raffles::{STATUS_ACTIVE, STATUS_CLAIMED, STATUS_PENDING};

/// The legacy guard pair: the item must exist (route model binding 404),
/// belong to the session user and still be active.
async fn guarded_item(
    state: &AppState,
    headers: &HeaderMap,
    raffle_item: i64,
    context: &str,
) -> Result<i64, Response> {
    let session = session_or_login(state, headers, context).await?;

    let row: Option<(Option<i64>, i32)> =
        sqlx::query_as("select winner_id, status from raffle_items where id = $1")
            .bind(raffle_item)
            .fetch_optional(&state.pool)
            .await
            .map_err(|error| super::support::db_error(error, context))?;

    let Some((winner_id, status)) = row else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };
    if winner_id != Some(session.user_id) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    if status != STATUS_ACTIVE {
        return Err(StatusCode::FORBIDDEN.into_response());
    }

    Ok(raffle_item)
}

/// `PUT /raffle/{raffle_item}` — claim: the code moves to the winner's
/// settings page (the legacy to_route('settings') with the "Prize
/// claimed!" flash; the frontend shows the toast).
pub async fn put(
    State(state): State<AppState>,
    Path(raffle_item): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let item = match guarded_item(&state, &headers, raffle_item, "raffle claim").await {
        Ok(item) => item,
        Err(response) => return response,
    };

    if let Err(error) =
        sqlx::query("update raffle_items set status = $1, updated_at = now() where id = $2")
            .bind(STATUS_CLAIMED)
            .bind(item)
            .execute(&state.pool)
            .await
    {
        return super::support::db_error(error, "raffle claim");
    }

    Redirect::to("/settings").into_response()
}

/// `DELETE /raffle/{raffle_item}` — decline: the item returns to the
/// pool (the legacy back() with the "Prize declined!" flash).
pub async fn destroy(
    State(state): State<AppState>,
    Path(raffle_item): Path<i64>,
    headers: HeaderMap,
) -> Response {
    let item = match guarded_item(&state, &headers, raffle_item, "raffle decline").await {
        Ok(item) => item,
        Err(response) => return response,
    };

    if let Err(error) = sqlx::query(
        "update raffle_items
         set status = $1, winner_id = null, expires_at = null, updated_at = now()
         where id = $2",
    )
    .bind(STATUS_PENDING)
    .bind(item)
    .execute(&state.pool)
    .await
    {
        return super::support::db_error(error, "raffle decline");
    }

    back(&headers).into_response()
}
