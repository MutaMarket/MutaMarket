//! Behavior tests for module notes and collection notes: the bulk
//! upsert/delete semantics (including the PHP `empty()` quirks), the
//! Laravel-shaped validation, the collection-notes authorization quirk,
//! and the `note` / `collection_note` keys on module payloads.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use serde_json::json;
use std::path::Path;
use tower::ServiceExt;

/// Characters owned by this suite alone, so parallel suites never share
/// state.
const OWNER_CHARACTER: i64 = 920_101;
const OTHER_CHARACTER: i64 = 920_102;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

async fn send(
    app: &Router,
    method: &str,
    path: &str,
    session: Option<&str>,
    body: Option<serde_json::Value>,
) -> (StatusCode, String, String) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(session) = session {
        request = request.header(header::COOKIE, format!("mm_session={session}"));
    }
    let request = match body {
        Some(body) => request
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .expect("valid request");

    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();

    (
        status,
        location,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn sorted_keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

/// Asserts a Laravel 422 with exactly one error message on one field.
fn assert_validation(status: StatusCode, body: &str, field: &str, message: &str) {
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected 422, body: {body}"
    );
    let errors: serde_json::Value = serde_json::from_str(body).expect("json");
    assert_eq!(errors["message"], json!("The given data was invalid."));
    assert_eq!(errors["errors"][field], json!([message]), "field {field}");
}

async fn note_content(pool: &sqlx::PgPool, user_id: i64, module_id: i64) -> Option<String> {
    sqlx::query_scalar("select content from notes where user_id = $1 and module_id = $2")
        .bind(user_id)
        .bind(module_id)
        .fetch_optional(pool)
        .await
        .expect("note lookup")
}

#[tokio::test]
async fn notes_and_collection_notes() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    mutamarket::db::reference::seed_reference(&pool, &tables)
        .await
        .expect("seed");
    let reference = ReferenceData::from_tables(tables);

    // Two modules to annotate.
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let mut module_ids = Vec::new();
    for module in &fixture.modules[..2] {
        process_module(
            &pool,
            &reference,
            &estimator_stub(),
            fixture.type_id,
            module.module_id,
            &DogmaItem {
                created_by: module.creator_id,
                source_type_id: module.source_type_id,
                mutator_type_id: module.mutaplasmid_id,
                dogma_attributes: common::fixture_dogma(module),
            },
        )
        .await
        .expect("process module");
        module_ids.push(module.module_id);
    }
    let (module_a, module_b) = (module_ids[0], module_ids[1]);

    // Two users with one character each; idempotent across runs.
    sqlx::query("delete from collections where character_id = any($1)")
        .bind(vec![OWNER_CHARACTER, OTHER_CHARACTER])
        .execute(&pool)
        .await
        .expect("cleanup collections");
    sqlx::query("delete from characters where id = any($1)")
        .bind(vec![OWNER_CHARACTER, OTHER_CHARACTER])
        .execute(&pool)
        .await
        .expect("cleanup characters");
    sqlx::query("delete from users where name = any($1)")
        .bind(vec!["Note Owner", "Note Other"])
        .execute(&pool)
        .await
        .expect("cleanup users");

    let mut users = Vec::new();
    for (name, character_id) in [
        ("Note Owner", OWNER_CHARACTER),
        ("Note Other", OTHER_CHARACTER),
    ] {
        let user_id: i64 = sqlx::query_scalar("insert into users (name) values ($1) returning id")
            .bind(name)
            .fetch_one(&pool)
            .await
            .expect("user");
        sqlx::query("insert into characters (id, name, user_id) values ($1, $2, $3)")
            .bind(character_id)
            .bind(name)
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("character");
        let session = mutamarket::auth::session::create_session(&pool, user_id, Some(character_id))
            .await
            .expect("session");
        users.push((user_id, session));
    }
    let (owner_id, owner) = (users[0].0, users[0].1.clone());
    let (other_id, other) = (users[1].0, users[1].1.clone());

    let app = mutamarket::server::test_router().await;

    // Guests are redirected to login.
    for path in ["/notes", "/collection-notes"] {
        let (status, location, _) =
            send(&app, "POST", path, None, Some(json!({"notes": []}))).await;
        assert!(
            status.is_redirection(),
            "guest POST {path} redirects, got {status}"
        );
        assert_eq!(location, "/login");
    }

    // Laravel-shaped validation, exact default messages.
    let (status, _, body) = send(&app, "POST", "/notes", Some(&owner), Some(json!({}))).await;
    assert_validation(status, &body, "notes", "The notes field is required.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": []})),
    )
    .await;
    assert_validation(status, &body, "notes", "The notes field is required.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": "x"})),
    )
    .await;
    assert_validation(status, &body, "notes", "The notes field must be an array.");
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [{"content": "orphan"}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "notes.0.module_id",
        "The notes.0.module id field is required.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [{"module_id": "abc", "content": "x"}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "notes.0.module_id",
        "The notes.0.module id field must be an integer.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [
            {"module_id": module_a, "content": "ok"},
            {"module_id": 999_999_999, "content": "x"},
        ]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "notes.1.module_id",
        "The selected notes.1.module id is invalid.",
    );
    let (status, _, body) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [{"module_id": module_a, "content": 5}]})),
    )
    .await;
    assert_validation(
        status,
        &body,
        "notes.0.content",
        "The notes.0.content field must be a string.",
    );

    // Bulk store: both modules get a note; success redirects back.
    let (status, _, _) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [
            {"module_id": module_a, "content": "first note"},
            {"module_id": module_b, "content": "second note"},
        ]})),
    )
    .await;
    assert!(status.is_redirection(), "store redirects, got {status}");
    assert_eq!(
        note_content(&pool, owner_id, module_a).await.as_deref(),
        Some("first note")
    );
    assert_eq!(
        note_content(&pool, owner_id, module_b).await.as_deref(),
        Some("second note")
    );

    // Upsert on (user, module): same module id updates in place.
    let (status, _, _) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [{"module_id": module_a, "content": "rewritten"}]})),
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(
        note_content(&pool, owner_id, module_a).await.as_deref(),
        Some("rewritten")
    );
    let count: i64 =
        sqlx::query_scalar("select count(*) from notes where user_id = $1 and module_id = $2")
            .bind(owner_id)
            .bind(module_a)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1, "upsert must not duplicate");

    // Another user's notes are separate.
    let (status, _, _) = send(
        &app,
        "POST",
        "/notes",
        Some(&other),
        Some(json!({"notes": [{"module_id": module_a, "content": "other note"}]})),
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(
        note_content(&pool, owner_id, module_a).await.as_deref(),
        Some("rewritten")
    );
    assert_eq!(
        note_content(&pool, other_id, module_a).await.as_deref(),
        Some("other note")
    );

    // The module page carries the signed-in user's note and the asset
    // saying where the module sits if they own it; guests get neither
    // key at all (the legacy unloaded relations).
    let (status, _, body) = send(
        &app,
        "GET",
        &format!("/api/module-page/{module_a}"),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(sorted_keys(&page["module"]["note"]), ["content", "id"]);
    assert_eq!(page["module"]["note"]["content"], json!("rewritten"));
    // Nobody in this test owns the module, so `asset` is present-and-null
    // exactly like a loaded-but-empty legacy relation.
    assert!(page["module"]["asset"].is_null(), "asset key present, null");
    crate::common::assert_default_module_keys(&page["module"], true, &[]);

    // Authed without a note on the module: the key is present and null.
    let (_, _, body) = send(
        &app,
        "GET",
        &format!("/api/module-page/{module_b}"),
        Some(&other),
        None,
    )
    .await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(page["module"]["note"].is_null());
    assert!(
        page["module"]
            .as_object()
            .expect("object")
            .contains_key("note")
    );

    let (_, _, body) = send(
        &app,
        "GET",
        &format!("/api/module-page/{module_a}"),
        None,
        None,
    )
    .await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(
        !page["module"]
            .as_object()
            .expect("object")
            .contains_key("note"),
        "guests must not get a note key",
    );

    // Deletion quirks, PHP empty(): null, "" and the literal "0" all
    // delete the note instead of storing it.
    for empty in [json!(null), json!(""), json!("0")] {
        let (status, _, _) = send(
            &app,
            "POST",
            "/notes",
            Some(&owner),
            Some(json!({"notes": [{"module_id": module_a, "content": "to be deleted"}]})),
        )
        .await;
        assert!(status.is_redirection());
        let (status, _, _) = send(
            &app,
            "POST",
            "/notes",
            Some(&owner),
            Some(json!({"notes": [{"module_id": module_a, "content": empty}]})),
        )
        .await;
        assert!(status.is_redirection());
        assert_eq!(
            note_content(&pool, owner_id, module_a).await,
            None,
            "content {empty:?} must delete the note (PHP empty() semantics)",
        );
    }
    // A missing content key deletes as well.
    let (status, _, _) = send(
        &app,
        "POST",
        "/notes",
        Some(&owner),
        Some(json!({"notes": [{"module_id": module_b}]})),
    )
    .await;
    assert!(status.is_redirection());
    assert_eq!(note_content(&pool, owner_id, module_b).await, None);

    // ---- Collection notes ----

    // The owner creates a collection holding module_a.
    let (status, location, _) = send(
        &app,
        "POST",
        "/collections",
        Some(&owner),
        Some(json!({"name": "Noted Rolls", "visibility": "public"})),
    )
    .await;
    assert!(
        status.is_redirection(),
        "collection create redirects, got {status}"
    );
    let slug = location.trim_start_matches("/collections/").to_owned();
    let identifier = slug.rsplit('-').next().expect("identifier").to_owned();
    let collection_id: i64 = sqlx::query_scalar("select id from collections where identifier = $1")
        .bind(&identifier)
        .fetch_one(&pool)
        .await
        .expect("collection id");
    sqlx::query(
        "insert into collection_modules (collection_id, module_id) values ($1, $2)
         on conflict do nothing",
    )
    .bind(collection_id)
    .bind(module_a)
    .execute(&pool)
    .await
    .expect("collect module");

    // A missing or unknown collection 404s before validation (the
    // legacy findOrFail inside authorize()).
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-notes",
        Some(&owner),
        Some(json!({"notes": [{"module_id": module_a, "content": "x"}]})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-notes",
        Some(&owner),
        Some(json!({"collection_id": 999_999_999, "notes": []})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Only the owner writes a collection's notes (the legacy
    // NotePolicy::create quirk let anyone; see server::notes).
    let (status, _, body) = send(
        &app,
        "POST",
        "/collection-notes",
        Some(&other),
        Some(json!({
            "collection_id": collection_id,
            "notes": [{"module_id": module_a, "content": "left by a stranger"}],
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, r#"{"message":"Forbidden."}"#);
    let stranger_rows: i64 = sqlx::query_scalar(
        "select count(*) from collection_notes where collection_id = $1 and module_id = $2",
    )
    .bind(collection_id)
    .bind(module_a)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(stranger_rows, 0, "nothing is stored for a non-owner");

    // Upsert in place on (collection, module).
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-notes",
        Some(&owner),
        Some(json!({
            "collection_id": collection_id,
            "notes": [{"module_id": module_a, "content": "curated"}],
        })),
    )
    .await;
    assert!(status.is_redirection());
    let count: i64 = sqlx::query_scalar(
        "select count(*) from collection_notes where collection_id = $1 and module_id = $2",
    )
    .bind(collection_id)
    .bind(module_a)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 1);

    // The collection page carries collection_note (every viewer) with
    // the embedded legacy collection resource, plus the viewer's own
    // note key.
    let (status, _, body) = send(
        &app,
        "GET",
        &format!("/api/collections/{slug}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    let module = page["modules"]
        .as_array()
        .expect("modules array")
        .iter()
        .find(|module| module["id"] == json!(module_a))
        .expect("collected module present");
    assert_eq!(
        sorted_keys(&module["collection_note"]),
        ["collection", "content", "id"]
    );
    assert_eq!(module["collection_note"]["content"], json!("curated"));
    assert_eq!(
        sorted_keys(&module["collection_note"]["collection"]),
        [
            "auto_sync",
            "created_at",
            "description",
            "id",
            "identifier",
            "last_synced_at",
            "name",
            "slug",
            "updated_at",
            "visibility",
        ],
        "embedded collection diverges from the legacy CollectionResource",
    );
    assert_eq!(
        module["collection_note"]["collection"]["id"],
        json!(collection_id)
    );
    assert_eq!(module["collection_note"]["collection"]["slug"], json!(slug));
    assert!(module.as_object().expect("object").contains_key("note"));

    // The embedded collection carries the real auto-sync columns, not
    // hardcoded defaults: flip them and the payload follows.
    assert_eq!(
        module["collection_note"]["collection"]["auto_sync"],
        json!(false)
    );
    assert_eq!(
        module["collection_note"]["collection"]["last_synced_at"],
        json!(null)
    );
    sqlx::query(
        "update collections set auto_sync = true,
             last_synced_at = '2026-08-28T10:00:00Z'::timestamptz
         where id = $1",
    )
    .bind(collection_id)
    .execute(&pool)
    .await
    .expect("enable auto-sync");
    let (_, _, body) = send(
        &app,
        "GET",
        &format!("/api/collections/{slug}"),
        Some(&other),
        None,
    )
    .await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    let module = page["modules"]
        .as_array()
        .expect("modules array")
        .iter()
        .find(|module| module["id"] == json!(module_a))
        .expect("collected module present");
    assert_eq!(
        module["collection_note"]["collection"]["auto_sync"],
        json!(true)
    );
    assert_eq!(
        module["collection_note"]["collection"]["last_synced_at"],
        json!("2026-08-28T10:00:00Z"),
    );
    sqlx::query("update collections set auto_sync = false, last_synced_at = null where id = $1")
        .bind(collection_id)
        .execute(&pool)
        .await
        .expect("restore auto-sync defaults");

    // Guests viewing the collection still see the collection note but no
    // personal note key.
    let (_, _, body) = send(&app, "GET", &format!("/api/collections/{slug}"), None, None).await;
    let page: serde_json::Value = serde_json::from_str(&body).expect("json");
    let module = &page["modules"][0];
    assert_eq!(module["collection_note"]["content"], json!("curated"));
    assert!(!module.as_object().expect("object").contains_key("note"));

    // Empty content deletes the collection note (same PHP-empty rules).
    let (status, _, _) = send(
        &app,
        "POST",
        "/collection-notes",
        Some(&owner),
        Some(json!({
            "collection_id": collection_id,
            "notes": [{"module_id": module_a, "content": "0"}],
        })),
    )
    .await;
    assert!(status.is_redirection());
    let count: i64 = sqlx::query_scalar(
        "select count(*) from collection_notes where collection_id = $1 and module_id = $2",
    )
    .bind(collection_id)
    .bind(module_a)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 0, "'0' deletes (PHP empty() semantics)");
}
