#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::*;

    let conf = get_configuration(None).expect("leptos configuration in Cargo.toml");
    let addr = conf.leptos_options.site_addr;
    let app = mutamarket::server::router(conf.leptos_options);

    println!("listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind site address");
    axum::serve(listener, app.into_make_service()).await.expect("serve");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
