pub mod admin;
pub mod appraise;
pub mod sell;
pub mod api;
pub mod auth;
pub mod display;
pub mod docs;
pub mod estimate;
pub mod linked;
pub mod nav;
pub mod personal;
pub mod social;
pub mod ws;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::{delete, get, patch, post, put};
use sqlx::PgPool;

use crate::auth::linked::LinkedClients;
use crate::auth::sso::SsoClient;
use crate::esi::EsiClient;
use crate::estimator::Estimator;
use crate::mutation::reference::ReferenceData;
use crate::scheduler::{JobDeps, Scheduler, SchedulerHandle};

/// The default bind address of the JSON API server; the SvelteKit dev
/// proxy and the production reverse proxy point at it.
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// The listen address from `BIND_ADDR`, with the local default.
pub fn bind_addr() -> String {
    std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub esi: EsiClient,
    pub sso: SsoClient,
    pub linked: LinkedClients,
    pub estimator: Estimator,
    /// Reference data is effectively static between SDE updates, so it is
    /// held in memory for the request handlers.
    pub reference: Arc<ReferenceData>,
    /// The background job registry the admin endpoints observe and control.
    pub scheduler: SchedulerHandle,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// Stand-in for every authenticated route until the session layer lands:
/// without a session there is no way to be logged in, so the correct
/// response is always the guest redirect. Becomes a real session check later.
async fn guest_redirect() -> Redirect {
    Redirect::to("/login")
}

/// Anything not owned by the API answers a JSON 404; pages live in the
/// SvelteKit frontend behind the shared proxy.
async fn json_not_found() -> axum::response::Response {
    api::error(StatusCode::NOT_FOUND, "Not Found")
}

/// `scheduler: None` builds a loop-less registry from the same
/// dependencies (the test setup); production passes the loaded handle.
pub fn router(
    pool: PgPool,
    esi: EsiClient,
    sso: SsoClient,
    linked: LinkedClients,
    estimator: Estimator,
    reference: Arc<ReferenceData>,
    scheduler: Option<SchedulerHandle>,
) -> Router {
    admin::mark_started();

    let scheduler = scheduler.unwrap_or_else(|| {
        Scheduler::disabled(JobDeps {
            pool: pool.clone(),
            reference: reference.clone(),
            esi: esi.clone(),
            estimator: estimator.clone(),
            sso: sso.clone(),
        })
    });

    let state = AppState {
        pool,
        esi,
        sso,
        linked,
        estimator,
        reference,
        scheduler,
    };

    Router::new()
        .merge(oauth_router())
        .merge(authed_router())
        .route("/modules", post(appraise::store))
        .route("/display", put(display::update))
        .route("/ws", get(ws::websocket))
        .route("/og/module/{module}", get(social::og_module))
        .route("/og/type/{type}", get(social::og_type))
        .route("/og/character/{character}", get(social::og_character))
        .route("/og/collection/{collection}", get(social::og_collection))
        .nest_service("/img", tower_http::services::ServeDir::new("public/img"))
        .nest("/api", api_router())
        .fallback(json_not_found)
        .with_state(state)
}

/// Router used by integration tests: same as production, on the dedicated
/// test database. The ESI base URL comes from `ESI_BASE_URL` when tests
/// need a mock.
pub async fn test_router() -> Router {
    let pool = crate::db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    crate::db::migrate(&pool).await.expect("migrations run");

    let reference = crate::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    router(
        pool,
        EsiClient::from_env(),
        SsoClient::from_env(),
        LinkedClients::from_env(),
        Estimator::new(),
        Arc::new(ReferenceData::from_tables(reference)),
        None,
    )
}

fn oauth_router() -> Router<AppState> {
    Router::new()
        .route("/eve", get(auth::eve_login))
        .route("/eve/corporation", get(auth::eve_login_corporation))
        .route("/eve/admin", get(auth::eve_login_admin))
        .route("/eve/callback", get(auth::eve_callback))
        .route("/twitch", get(linked::twitch_login).put(guest_redirect))
        .route("/twitch/callback", get(linked::twitch_callback))
        .route("/discord", get(linked::discord_login).put(guest_redirect))
        .route("/discord/callback", get(linked::discord_callback))
        .route("/patreon", get(linked::patreon_login).put(guest_redirect))
        .route("/patreon/callback", get(linked::patreon_callback))
}

fn authed_router() -> Router<AppState> {
    Router::new()
        .route("/personal/modules", post(personal::store))
        .route("/characters/{character}", put(social::update_character))
        .route("/public-assets", post(personal::publish_asset))
        .route("/public-assets/{asset}", delete(personal::unpublish_asset))
        .route("/estimate/{module}", post(estimate::update))
        .route("/settings", post(guest_redirect).put(guest_redirect))
        .route("/offers", post(guest_redirect))
        .route("/offers/{offer}", delete(guest_redirect))
        .route("/messages", post(guest_redirect))
        .route("/collections", post(social::store_collection))
        .route("/collections/modules", post(social::store_collection_with_modules))
        .route(
            "/collections/{collection}",
            put(social::update_collection).delete(social::destroy_collection),
        )
        .route("/collection-modules", post(social::store_collection_module))
        .route("/collection-modules/all", delete(social::destroy_all_collection_modules))
        .route(
            "/collection-modules/{collection_module}",
            put(social::update_collection_module).delete(social::destroy_collection_module),
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
        .route("/personal/contracts", post(guest_redirect))
        .route("/workbench/{*modules}", post(guest_redirect))
        .route("/workbench-modules", post(guest_redirect))
        .route("/workbench-modules/all", delete(guest_redirect))
        .route(
            "/workbench-modules/{workbench_module}",
            put(guest_redirect).delete(guest_redirect),
        )
        .route("/workbench-collections", post(guest_redirect))
        .route("/logout", post(auth::logout))
        .route(
            "/auth/character/{character}",
            put(auth::switch_character).delete(auth::remove_character),
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
        .route("/raffles", post(guest_redirect))
        .route("/advertisements", post(guest_redirect))
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
        .route(
            "/abyssal-type-statistics",
            get(api::abyssal_type_statistics),
        )
        .route("/nav-state", get(nav::show))
        .route("/documentation", get(docs::index))
        .route("/documentation/{page}", get(docs::show))
        .route("/module-page/{module}", get(api::module_page))
        .route("/module-page/{module}/similar", get(api::module_similar))
        .route("/module-cards", get(api::module_cards_root))
        .route("/module-cards/{*query}", get(api::module_cards))
        .route("/module-stats", get(api::module_stats))
        .route("/filter-panel/{type}", get(api::filter_panel))
        .route("/characters", get(social::characters_index))
        .route("/characters/{character}", get(social::character_show))
        .route("/collections", get(social::collections_index))
        .route("/collections/{collection}", get(social::collection_show))
        .route("/personal/page", get(personal::page))
        .route("/personal/modules", get(personal::modules))
        .route("/sell/page", get(sell::page))
        .route("/sell/modules", get(sell::modules))
        .route("/sell/locations", get(sell::locations))
        .route("/admin/scheduler", get(admin::scheduler_status))
        .route("/admin/system", get(admin::system))
        .route("/admin/telemetry", get(admin::telemetry))
        .route("/admin/scheduler/{job}/run", post(admin::scheduler_run))
        .route("/admin/scheduler/{job}", put(admin::scheduler_update))
        .route("/historic-contracts/{id}", put(admin::historic_contract_update))
}
