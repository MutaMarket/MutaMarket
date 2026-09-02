//! Contract tests for the documented public API.
//!
//! The spec is generated from the handler annotations and the response
//! DTOs, so paths and schemas cannot drift from the code by construction.
//! What these tests add is the part generation cannot prove: that the
//! documented set is exactly the public set, and that the example error
//! messages are the ones the API really sends.
//!
//! Needs the local database: `docker compose up -d postgres`.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn send(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(path);
    let request = match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .expect("valid request");

    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The generated document, as the endpoint serves it.
async fn spec(app: &Router) -> Value {
    let (status, body) = send(app, Method::GET, "/api/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    body
}

/// The generated document is a usable OpenAPI 3 description.
async fn the_spec_is_served_and_well_formed(app: &Router) {
    let spec = spec(app).await;
    assert!(
        spec["openapi"]
            .as_str()
            .expect("an openapi version")
            .starts_with('3'),
    );
    assert_eq!(spec["info"]["title"], "MutaMarket API");
    assert_eq!(spec["servers"][0]["url"], "https://mutamarket.com/api");
}

/// Every path the spec documents is routed, and nothing public is missing.
async fn every_documented_path_is_routed(app: &Router) {
    let spec = spec(app).await;
    let paths = spec["paths"].as_object().expect("paths");

    // The documented set, so adding a public endpoint without documenting
    // it (or the reverse) fails here.
    let mut documented: Vec<&str> = paths.keys().map(String::as_str).collect();
    documented.sort_unstable();
    assert_eq!(
        documented,
        [
            "/abyssal-type-statistics",
            "/estimator-statistics",
            "/modules",
            "/modules/{query}",
        ],
    );

    for (path, item) in paths {
        for method in item.as_object().expect("path item").keys() {
            // A concrete path for the templated one; the id need not exist,
            // only the route.
            let concrete = path.replace("{query}", "1");
            let request_method = match method.as_str() {
                "get" => Method::GET,
                "post" => Method::POST,
                other => panic!("undocumented method {other} for {path}"),
            };
            let body = (request_method == Method::POST)
                .then(|| serde_json::json!({ "type_id": 1, "item_id": 1 }));
            let (status, _) = send(app, request_method, &format!("/api{concrete}"), body).await;
            assert_ne!(
                status,
                StatusCode::METHOD_NOT_ALLOWED,
                "{method} {path} is documented but not routed",
            );
        }
    }
}

/// The documented error messages are the ones the API actually sends.
async fn the_documented_errors_are_the_real_ones(app: &Router) {
    let spec = spec(app).await;

    // The bare list rejects, because it needs a type option.
    let documented = spec["paths"]["/modules"]["get"]["responses"]["404"]["content"]
        ["application/json"]["example"]["message"]
        .clone();
    let (status, body) = send(app, Method::GET, "/api/modules", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], documented);
    assert_eq!(body["message"], "Please provide a valid type.");

    // A query naming no type is the same refusal.
    let (status, body) = send(app, Method::GET, "/api/modules/nonsense/segments", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], documented);

    // An unknown item id, taken from the templated path's examples.
    let unknown =
        spec["paths"]["/modules/{query}"]["get"]["responses"]["404"]["content"]["application/json"]
            ["example"]["message"]
            .clone();
    let (status, body) = send(app, Method::GET, "/api/modules/999999999999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], unknown);
    assert_eq!(
        body["message"],
        "No module with this item id is known to MutaMarket.",
    );

    // The import's validation error. The 422 body is documented by schema,
    // so assert the real response carries what ValidationError declares --
    // and that `message` is the legacy one, which is the field order the
    // handler has to pick deliberately.
    assert_eq!(
        spec["paths"]["/modules"]["post"]["responses"]["422"]["content"]["application/json"]["schema"]
            ["$ref"],
        "#/components/schemas/ValidationError",
    );
    let (status, body) = send(
        app,
        Method::POST,
        "/api/modules",
        Some(serde_json::json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body["message"], "The message field is required when item id is not present.",
        "legacy reports the first failing field in field order, not alphabetically",
    );
    for field in ["message", "item_id", "type_id"] {
        assert!(
            body["errors"][field].is_array(),
            "the validation error names {field}",
        );
    }

    // A message with no item link is a 400, not a validation error.
    let documented = spec["paths"]["/modules"]["post"]["responses"]["400"]["content"]
        ["application/json"]["example"]["message"]
        .clone();
    let (status, body) = send(
        app,
        Method::POST,
        "/api/modules",
        Some(serde_json::json!({ "message": "no link in here" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], documented);
}

/// The reference endpoints answer with the array shape the spec declares,
/// and their rows carry every field the spec marks required.
async fn the_reference_endpoints_match_their_schemas(app: &Router) {
    let spec = spec(app).await;

    for (path, schema_name) in [
        ("/api/estimator-statistics", "EstimatorStatistic"),
        ("/api/abyssal-type-statistics", "AbyssalTypeStatistic"),
    ] {
        let (status, body) = send(app, Method::GET, path, None).await;
        assert_eq!(status, StatusCode::OK);
        let rows = body
            .as_array()
            .unwrap_or_else(|| panic!("{path} answers a bare array"));

        let schema = &spec["components"]["schemas"][schema_name];
        let required: Vec<&str> = schema["required"]
            .as_array()
            .expect("required")
            .iter()
            .map(|value| value.as_str().expect("a field name"))
            .collect();
        let known: Vec<&str> = schema["properties"]
            .as_object()
            .expect("properties")
            .keys()
            .map(String::as_str)
            .collect();

        for row in rows.iter().take(5) {
            let row = row.as_object().expect("an object");
            for field in &required {
                assert!(
                    row.contains_key(*field),
                    "{path}: {schema_name} requires {field}, which the response omits",
                );
            }
            for field in row.keys() {
                assert!(
                    known.contains(&field.as_str()),
                    "{path} returns {field}, which {schema_name} does not document",
                );
            }
        }
    }
}

/// One test, run in sequence: the phases share one router and one
/// database, and the suite runs alongside others.
#[tokio::test]
async fn the_public_api_matches_its_documentation() {
    let app = mutamarket::server::test_router().await;

    the_spec_is_served_and_well_formed(&app).await;
    every_documented_path_is_routed(&app).await;
    the_documented_errors_are_the_real_ones(&app).await;
    the_reference_endpoints_match_their_schemas(&app).await;
}
