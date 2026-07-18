#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;

    let conf = get_configuration(Some("Cargo.toml")).expect("leptos configuration in Cargo.toml");
    let addr = conf.leptos_options.site_addr;

    let pool = mutamarket::db::connect().await.expect("database connection");
    mutamarket::db::migrate(&pool).await.expect("database migrations");

    let reference = mutamarket::db::reference::load_reference(&pool)
        .await
        .expect("reference tables load");

    let app = mutamarket::server::router(
        conf.leptos_options,
        pool,
        mutamarket::esi::EsiClient::from_env(),
        mutamarket::auth::sso::SsoClient::from_env(),
        std::sync::Arc::new(mutamarket::mutation::reference::ReferenceData::from_tables(reference)),
    );

    println!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind site address");
    axum::serve(listener, app.into_make_service()).await.expect("serve");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
