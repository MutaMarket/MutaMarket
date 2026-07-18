#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;

    // Local configuration from .env, if present; real environment wins.
    dotenvy::dotenv().ok();

    let conf = get_configuration(Some("Cargo.toml")).expect("leptos configuration in Cargo.toml");
    let addr = conf.leptos_options.site_addr;

    let pool = mutamarket::db::connect().await.expect("database connection");
    mutamarket::db::migrate(&pool).await.expect("database migrations");

    let reference = mutamarket::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    let esi = mutamarket::esi::EsiClient::from_env();
    let estimator = mutamarket::estimator::EstimatorClient::from_env();
    let sso = mutamarket::auth::sso::SsoClient::from_env();
    let reference =
        std::sync::Arc::new(mutamarket::mutation::reference::ReferenceData::from_tables(reference));

    if mutamarket::scheduler::enabled_by_env() {
        mutamarket::scheduler::start(
            pool.clone(),
            reference.clone(),
            esi.clone(),
            estimator.clone(),
            sso.clone(),
        );
        println!("scheduler enabled");
    }

    let app = mutamarket::server::router(
        conf.leptos_options,
        pool,
        esi,
        sso,
        mutamarket::auth::linked::LinkedClients::from_env(),
        estimator,
        reference,
    );

    println!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind site address");
    axum::serve(listener, app.into_make_service()).await.expect("serve");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
