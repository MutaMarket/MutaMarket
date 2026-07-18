pub mod api;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::{delete, get, patch, post, put};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use sqlx::PgPool;

use crate::app::{App, shell};
use crate::esi::EsiClient;
use crate::mutation::reference::ReferenceData;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: PgPool,
    pub esi: EsiClient,
    /// Reference data is effectively static between SDE updates, so it is
    /// held in memory for the request handlers.
    pub reference: Arc<ReferenceData>,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(state: &AppState) -> Self {
        state.leptos_options.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// The route exists in the legacy application but has not been ported yet.
async fn not_implemented() -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

/// Stand-in for every authenticated route until the session layer lands:
/// without a session there is no way to be logged in, so the correct
/// response is always the guest redirect. Becomes a real session check later.
async fn guest_redirect() -> Redirect {
    Redirect::to("/login")
}

pub fn router(
    leptos_options: LeptosOptions,
    pool: PgPool,
    esi: EsiClient,
    reference: Arc<ReferenceData>,
) -> Router {
    let routes = generate_route_list(App);
    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool,
        esi,
        reference,
    };

    Router::new()
        .route(
            "/about",
            get(|| async { Redirect::permanent("/documentation/about") }),
        )
        .route("/help", get(|| async { Redirect::permanent("/documentation") }))
        .merge(oauth_router())
        .merge(authed_router())
        .route("/modules", post(not_implemented))
        .route("/display", put(not_implemented))
        .nest("/api", api_router())
        .leptos_routes(&state, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .with_state(state)
}

/// Router used by integration tests: same as production, configured from
/// the crate's Cargo.toml metadata and the development database. The ESI
/// base URL comes from `ESI_BASE_URL` when tests need a mock.
pub async fn test_router() -> Router {
    let conf = get_configuration(Some("Cargo.toml")).expect("leptos configuration in Cargo.toml");
    let pool = crate::db::connect()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    crate::db::migrate(&pool).await.expect("migrations run");

    let reference = crate::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    router(
        conf.leptos_options,
        pool,
        EsiClient::from_env(),
        Arc::new(ReferenceData::from_tables(reference)),
    )
}

fn oauth_router() -> Router<AppState> {
    Router::new()
        .route("/eve", get(not_implemented))
        .route("/eve/corporation", get(not_implemented))
        .route("/eve/admin", get(not_implemented))
        .route("/eve/callback", get(not_implemented))
        .route("/twitch", get(not_implemented).put(guest_redirect))
        .route("/twitch/callback", get(not_implemented))
        .route("/discord", get(not_implemented).put(guest_redirect))
        .route("/discord/callback", get(not_implemented))
        .route("/patreon", get(not_implemented).put(guest_redirect))
        .route("/patreon/callback", get(not_implemented))
}

fn authed_router() -> Router<AppState> {
    Router::new()
        .route("/sell/modules", get(guest_redirect))
        .route("/sell/modules/{*query}", get(guest_redirect))
        .route("/personal/modules", get(guest_redirect).post(guest_redirect))
        .route("/personal/modules/{*query}", get(guest_redirect))
        .route("/locations", get(guest_redirect))
        .route("/locations/{location}", get(guest_redirect))
        .route("/locations/{location}/{*query}", get(guest_redirect))
        .route("/characters/{character}", put(guest_redirect))
        .route("/public-assets", post(guest_redirect))
        .route("/public-assets/{asset}", delete(guest_redirect))
        .route("/historic-sales", get(guest_redirect))
        .route("/historic-sales/{*query}", get(guest_redirect))
        .route("/estimate/{module}", post(guest_redirect))
        .route(
            "/settings",
            get(guest_redirect).post(guest_redirect).put(guest_redirect),
        )
        .route("/offers", get(guest_redirect).post(guest_redirect))
        .route("/offers/{offer}", get(guest_redirect).delete(guest_redirect))
        .route("/messages", post(guest_redirect))
        .route("/collections", post(guest_redirect))
        .route("/collections/modules", post(guest_redirect))
        .route(
            "/collections/{collection}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/collection-modules", post(guest_redirect))
        .route("/collection-modules/all", delete(guest_redirect))
        .route(
            "/collection-modules/{collection_module}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route(
            "/collection-locations",
            post(guest_redirect).put(guest_redirect).delete(guest_redirect),
        )
        .route("/location-collections", post(guest_redirect))
        .route(
            "/collections/{collection}/auto-sync",
            post(guest_redirect).delete(guest_redirect),
        )
        .route(
            "/collections/{collection}/auto-sync/locations",
            post(guest_redirect),
        )
        .route(
            "/collections/{collection}/auto-sync/locations/{asset}",
            delete(guest_redirect),
        )
        .route("/bookmarks", post(guest_redirect))
        .route(
            "/bookmarks/{bookmark}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/ui/contract", post(guest_redirect))
        .route("/personal/contracts", get(guest_redirect).post(guest_redirect))
        .route("/workbench/{*modules}", post(guest_redirect))
        .route("/workbench-modules", post(guest_redirect))
        .route("/workbench-modules/all", delete(guest_redirect))
        .route(
            "/workbench-modules/{workbench_module}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/workbench-collections", post(guest_redirect))
        .route("/personal/stats", get(guest_redirect))
        .route("/logout", post(guest_redirect))
        .route(
            "/auth/character/{character}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/module-pricing", post(guest_redirect))
        .route("/notes", post(guest_redirect))
        .route("/collection-notes", post(guest_redirect))
        .route(
            "/raffle/{raffle_item}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/blocked-users", post(guest_redirect))
        .route(
            "/historic-contracts/{historic_contract}",
            put(guest_redirect),
        )
        .route("/raffles", get(guest_redirect).post(guest_redirect))
        .route("/advertisements", get(guest_redirect).post(guest_redirect))
        .route(
            "/advertisements/{advertisement}",
            post(guest_redirect).delete(guest_redirect),
        )
        .route(
            "/advertisements/{advertisement}/toggle",
            patch(guest_redirect),
        )
        .route(
            "/moderator/contracts/{historic_contract}",
            post(guest_redirect),
        )
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route(
            "/modules",
            get(api::modules_index_root).post(api::store_module),
        )
        .route("/modules/{*query}", get(api::modules_show_or_index))
        .route("/estimator-statistics", get(api::estimator_statistics))
        .route("/abyssal-type-statistics", get(not_implemented))
}
