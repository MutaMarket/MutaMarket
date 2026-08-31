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

    let reference = mutamarket::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    let esi = mutamarket::esi::EsiClient::from_env().with_failure_log(pool.clone());
    let estimator = mutamarket::estimator::Estimator::new();
    let sso = mutamarket::auth::sso::SsoClient::from_env();
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
        mutamarket::auth::linked::LinkedClients::from_env(),
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
