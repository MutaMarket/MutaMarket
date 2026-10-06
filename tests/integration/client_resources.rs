//! The game client's resource-file index, read the way the SDE import
//! fetches type icons: build listing, gzip-served index, resource file.

use std::io::Write;

use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use mutamarket::sde::client::ClientResources;

const BUILD: &str = "3579973";
const INDEX_FILE: &str = "1d/1d34143a37d4b739_20c9e0d73ca2179c3b68d1aae27cb91f";
const ICON_FILE: &str = "fe/fe511ebd45d943b9_f9a34d4c18631371c1c60f79e6a043d0";
const ICON_BYTES: &[u8] = b"\x89PNG remote armor repair";

fn gzip(text: &str) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text.as_bytes()).expect("gzip index");
    encoder.finish().expect("finish gzip")
}

/// Serves both client hosts from one listener, the index gzip-encoded as
/// CCP serves it.
async fn mock_client_hosts() -> String {
    let index = gzip(&format!(
        "res:/intromovie.txt,a9/a9d1721dd5cc6d54_e6bbb2df307e5a9527159a4c971034b5,e6bb,9719,3312\n\
         res:/ui/texture/icons/remote_armor_repair.png,{ICON_FILE},f9a3,5206,5279\n"
    ));

    let router = Router::new()
        .route(
            "/eveclient_TQ.json",
            get(|| async { axum::Json(serde_json::json!({ "build": BUILD })) }),
        )
        .route(
            &format!("/eveonline_{BUILD}.txt"),
            get(|| async { format!("app:/resfileindex.txt,{INDEX_FILE},20c9,5284846,5284846\n") }),
        )
        .route(
            &format!("/{INDEX_FILE}"),
            get(move || async move {
                ([(header::CONTENT_ENCODING, "gzip")], index.clone()).into_response()
            }),
        )
        .route(&format!("/{ICON_FILE}"), get(|| async { ICON_BYTES }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock client hosts");
    let address = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve mock client hosts");
    });
    format!("http://{address}")
}

#[tokio::test]
async fn a_gzip_served_index_resolves_icon_paths() {
    let url = mock_client_hosts().await;
    let resources = ClientResources::load_from(&url, &url)
        .await
        .expect("load resource index");

    let icon = resources
        .read("res:/UI/Texture/Icons/remote_armor_repair.png")
        .await
        .expect("read icon");
    assert_eq!(icon.as_deref(), Some(ICON_BYTES));

    let missing = resources
        .read("res:/ui/texture/icons/1_64_11.png")
        .await
        .expect("read missing icon");
    assert_eq!(missing, None);
}
