pub mod admin;
pub mod api;
pub mod appraise;
pub mod auth;
pub mod calculator;
pub mod collection_locations;
pub mod display;
pub mod docs;
pub mod estimate;
pub mod limits;
pub mod linked;
pub mod locations;
pub mod moderator;
pub mod nav;
pub mod notes;
pub mod offers;
pub mod openapi;
pub mod personal;
pub mod personal_contracts;
pub mod premium;
pub mod pricing;
pub mod raffles;
pub mod sell;
pub mod settings;
pub mod sidebar;
pub mod sitemap;
pub mod social;
pub mod statistics;
mod support;
pub mod ui;
pub mod workbench;
pub mod ws;

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
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
    /// Request counters, shared with the flush job through the scheduler.
    pub activity: Arc<crate::activity::ActivityRecorder>,
    /// Per-client windows of the ESI fan-out routes (`server::limits`).
    pub limits: Arc<limits::RateLimits>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

/// Anything not owned by the API answers a JSON 404; pages live in the
/// SvelteKit frontend behind the shared proxy.
async fn json_not_found() -> axum::response::Response {
    api::error(StatusCode::NOT_FOUND, "Not Found")
}

/// Attributes any ESI failure raised while handling a request to the
/// route that handled it, the way the scheduler attributes a job's.
async fn esi_caller_layer(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| request.uri().path().to_owned());
    let label = format!("{} {route}", request.method());

    crate::esi::failures::ESI_CALLER
        .scope(
            crate::esi::failures::EsiCaller::http(label),
            next.run(request),
        )
        .await
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
            activity: Arc::default(),
            reference: reference.clone(),
            esi: esi.clone(),
            estimator: estimator.clone(),
            sso: sso.clone(),
            discord: linked.discord.clone(),
        })
    });

    // One recorder per process: the router counts into the same buffer
    // the scheduler's flush job drains.
    let activity = scheduler.activity();
    let state = AppState {
        pool,
        esi,
        sso,
        linked,
        estimator,
        reference,
        scheduler,
        activity,
        limits: Arc::new(limits::RateLimits::default()),
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
        .route("/sitemap.xml", get(sitemap::show))
        .nest_service("/img", tower_http::services::ServeDir::new("assets/img"))
        .nest("/api", api_router())
        .fallback(json_not_found)
        .layer(axum::middleware::from_fn(reject_cross_site_mutations))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            limits::enforce,
        ))
        .layer(axum::middleware::from_fn(esi_caller_layer))
        .layer(axum::middleware::from_fn(crate::i18n::layer))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::activity::middleware::record,
        ))
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

    let esi = EsiClient::from_env().with_failure_log(pool.clone());
    router(
        pool,
        esi,
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
        .route(
            "/twitch",
            get(linked::twitch_login).put(settings::update_twitch),
        )
        .route("/twitch/callback", get(linked::twitch_callback))
        .route(
            "/discord",
            get(linked::discord_login).put(settings::update_discord),
        )
        .route("/discord/callback", get(linked::discord_callback))
        .route(
            "/patreon",
            get(linked::patreon_login).put(settings::update_patreon),
        )
        .route("/patreon/callback", get(linked::patreon_callback))
}

fn authed_router() -> Router<AppState> {
    Router::new()
        .route("/personal/modules", post(personal::store))
        .route("/characters/{character}", put(social::update_character))
        .route(
            "/characters/{character}/scope-warnings",
            put(social::update_scope_warnings),
        )
        .route("/public-assets", post(personal::publish_asset))
        .route("/public-assets/{asset}", delete(personal::unpublish_asset))
        .route("/estimate/{module}", post(estimate::update))
        .route("/settings", put(settings::update))
        .route("/settings/accent", put(settings::update_accent))
        .route("/premium/gift", post(premium::gift))
        .route("/offers", post(offers::store))
        .route("/offers/{offer}", delete(offers::destroy))
        .route("/messages", post(offers::store_message))
        .route("/collections", post(social::store_collection))
        .route(
            "/collections/modules",
            post(social::store_collection_with_modules),
        )
        .route(
            "/collections/{collection}",
            put(social::update_collection).delete(social::destroy_collection),
        )
        .route("/collection-modules", post(social::store_collection_module))
        .route(
            "/collection-modules/all",
            delete(social::destroy_all_collection_modules),
        )
        .route(
            "/collection-modules/{collection_module}",
            put(social::update_collection_module).delete(social::destroy_collection_module),
        )
        .route(
            "/collection-locations",
            post(collection_locations::store)
                .put(collection_locations::put)
                .delete(collection_locations::destroy),
        )
        .route("/location-collections", post(locations::store_collection))
        .route(
            "/collections/{collection}/auto-sync",
            post(collection_locations::enable).delete(collection_locations::disable),
        )
        .route(
            "/collections/{collection}/auto-sync/locations",
            post(collection_locations::store_location),
        )
        .route(
            "/collections/{collection}/auto-sync/locations/{asset}",
            delete(collection_locations::destroy_location),
        )
        .route("/bookmarks", post(sidebar::store))
        .route(
            "/bookmarks/{bookmark}",
            put(sidebar::update).delete(sidebar::destroy),
        )
        .route("/ui/contract", post(ui::open_contract))
        .route("/personal/contracts", post(personal_contracts::store))
        .route("/workbench/{*modules}", post(workbench::accept))
        .route("/workbench-modules", post(workbench::store))
        .route("/workbench-modules/all", delete(workbench::destroy_all))
        .route(
            "/workbench-modules/{workbench_module}",
            put(workbench::update).delete(workbench::destroy),
        )
        .route("/workbench-collections", post(workbench::to_collection))
        .route("/logout", post(auth::logout))
        .route(
            "/auth/character/{character}",
            put(auth::switch_character).delete(auth::remove_character),
        )
        .route("/module-pricing", post(pricing::store))
        .route("/notes", post(notes::store))
        .route("/collection-notes", post(notes::store_collection))
        .route(
            "/raffle/{raffle_item}",
            put(raffles::put).delete(raffles::destroy),
        )
        .route("/blocked-users", post(offers::store_blocked_user))
        .route(
            "/blocked-users/{user}",
            delete(offers::destroy_blocked_user),
        )
        .route("/raffles", post(admin::create_raffle_items))
        .route(
            "/moderator/contracts/{historic_contract}",
            post(moderator::store),
        )
}

/// `GET /api/health` — for uptime checks and the container health
/// probe: 200 once the database answers, 503 otherwise.
async fn health(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => axum::Json(serde_json::json!({ "status": "ok" })).into_response(),
        Err(error) => {
            tracing::warn!(%error, "health check database probe failed");
            api::error(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable.")
        }
    }
}

fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route(
            "/modules",
            get(api::modules_index_root).post(api::store_module),
        )
        .route("/modules/{*query}", get(api::modules_show_or_index))
        .route("/openapi.json", get(api::openapi))
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
        .route("/premium/page", get(api::premium_page))
        .route("/historic-sales-cards", get(api::historic_sales_cards_root))
        .route(
            "/historic-sales-cards/{*query}",
            get(api::historic_sales_cards),
        )
        .route("/module-stats", get(api::module_stats))
        .route("/filter-panel/{type}", get(api::filter_panel))
        .route("/characters", get(social::characters_index))
        .route("/characters/{character}", get(social::character_show))
        .route("/collections", get(social::collections_index))
        .route("/collections/{collection}", get(social::collection_show))
        .route("/statistics/overview", get(statistics::overview))
        .route("/statistics/top", get(statistics::top_root))
        .route("/statistics/top/{*query}", get(statistics::top))
        .route("/personal/stats", get(statistics::personal))
        .route("/settings", get(settings::index))
        .route("/locations", get(locations::index))
        .route("/locations/{location}", get(locations::show_root))
        .route("/locations/{location}/{*query}", get(locations::show))
        .route("/personal/page", get(personal::page))
        .route("/personal/contracts", get(personal_contracts::page))
        .route("/moderator/contracts", get(moderator::page_root))
        .route("/moderator/contracts/{*query}", get(moderator::page))
        .route("/personal/modules", get(personal::modules))
        .route("/calculator", get(calculator::index_root))
        .route("/calculator/{*query}", get(calculator::index))
        .route(
            "/collections/module/{module}",
            get(social::collections_for_module),
        )
        .route("/sidebar", get(sidebar::payload))
        .route("/workbench", get(workbench::index))
        .route("/workbench-page/{*modules}", get(workbench::shared))
        .route("/offers", get(offers::index))
        .route("/offers/sent", get(offers::sent))
        .route("/offers/{offer}", get(offers::show))
        .route("/sell/page", get(sell::page))
        .route("/sell/modules", get(sell::modules))
        .route("/sell/locations", get(sell::locations))
        .route(
            "/admin/advertisements",
            get(admin::advertisements).post(admin::create_advertisement),
        )
        .route(
            "/admin/advertisements/{advertisement}",
            put(admin::update_advertisement).delete(admin::destroy_advertisement),
        )
        .route(
            "/admin/advertisements/{advertisement}/toggle",
            axum::routing::patch(admin::toggle_advertisement),
        )
        .route(
            "/admin/gear-items",
            get(admin::gear_items).post(admin::create_gear_item),
        )
        .route(
            "/admin/gear-items/{gear_item}",
            put(admin::update_gear_item).delete(admin::destroy_gear_item),
        )
        .route(
            "/admin/gear-items/{gear_item}/toggle",
            axum::routing::patch(admin::toggle_gear_item),
        )
        .route("/admin/raffles", get(admin::raffles))
        .route("/admin/live", get(admin::live))
        .route("/admin/activity", get(admin::activity))
        .route("/admin/esi-failures", get(admin::esi_failures))
        .route("/admin/esi-failures/{failure}", get(admin::esi_failure))
        .route("/admin/scheduler", get(admin::scheduler_status))
        .route("/admin/system", get(admin::system))
        .route("/admin/metrics", get(admin::metrics_history))
        .route("/admin/telemetry", get(admin::telemetry))
        .route("/admin/scheduler/{job}/run", post(admin::scheduler_run))
        .route("/admin/scheduler/{job}", put(admin::scheduler_update))
        .route(
            "/historic-contracts/{id}",
            put(admin::historic_contract_update),
        )
        .route("/admin/service-character", get(admin::service_character))
}

/// Every mutation is cookie-authenticated, and the session cookie's
/// `SameSite=Lax` keeps it off cross-site requests in current browsers.
/// This is the second lock: a non-GET request the browser marks as
/// cross-site, or whose Origin names another host, is refused before any
/// handler runs (the legacy app had Laravel's CSRF token here).
async fn reject_cross_site_mutations(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::Method;
    let read_only = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if !read_only && support::is_cross_site(request.headers()) {
        return support::error_json(axum::http::StatusCode::FORBIDDEN, "Forbidden.");
    }
    next.run(request).await
}
