#[tokio::main]
async fn main() {
    // Structured logs; tune with RUST_LOG (e.g. RUST_LOG=mutamarket=debug).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let addr = mutamarket::server::bind_addr();

    let pool = mutamarket::db::connect()
        .await
        .expect("database connection");
    mutamarket::db::migrate(&pool)
        .await
        .expect("database migrations");

    // Cards cached by the previous build keep its design until the weekly
    // og-cache job, so a deploy wipes them up front; they re-render on
    // demand.
    match mutamarket::og::clear_cache() {
        Ok(()) => tracing::info!("OpenGraph card cache cleared"),
        Err(error) => tracing::warn!(%error, "OpenGraph card cache not cleared"),
    }

    let reference = mutamarket::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    let esi = mutamarket::esi::EsiClient::from_env().with_failure_log(pool.clone());
    let estimator = mutamarket::estimator::Estimator::new();
    // Decoding every forest takes tens of seconds; the server listens
    // meanwhile and a type asked for early loads on demand.
    tokio::spawn({
        let estimator = estimator.clone();
        let pool = pool.clone();
        async move {
            match estimator.load_models(&pool).await {
                Ok(load) => tracing::info!(
                    "estimator: {} models loaded, {} resident",
                    load.loaded,
                    load.resident
                ),
                Err(error) => tracing::error!(%error, "estimator: models failed to load"),
            }
        }
    });
    let sso = mutamarket::auth::sso::SsoClient::from_env();
    let linked = mutamarket::auth::linked::LinkedClients::from_env();
    let reference = std::sync::Arc::new(
        mutamarket::mutation::reference::ReferenceData::from_tables(reference),
    );

    let scheduler = mutamarket::scheduler::Scheduler::load(
        mutamarket::scheduler::JobDeps {
            pool: pool.clone(),
            activity: std::sync::Arc::default(),
            reference: reference.clone(),
            esi: esi.clone(),
            estimator: estimator.clone(),
            sso: sso.clone(),
            discord: linked.discord.clone(),
        },
        mutamarket::scheduler::enabled_by_env(),
    )
    .await
    .expect("scheduler state loads");
    if scheduler.enabled {
        mutamarket::scheduler::start(scheduler.clone());
        tracing::info!("scheduler enabled");
    }

    let app = mutamarket::server::router(
        pool,
        esi,
        sso,
        linked,
        estimator,
        reference,
        Some(scheduler),
    );

    tracing::info!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind address");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve");
}
