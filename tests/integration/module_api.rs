//! Behavior tests for the module JSON API against real data: seed the
//! fixture reference, ingest a known module, and exercise show and index
//! through the full router.
//!
//! Needs the local database: `docker compose up -d postgres`.

use crate::common;

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use tower::ServiceExt;

/// No test here exercises a live AI server through this path: types
/// without a trained statistic never call it, and a leftover trained
/// statistic just gets a fast connection refusal (estimate skipped).
fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

async fn get_json(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
    get_json_as(app, path, None).await
}

async fn get_json_as(
    app: &Router,
    path: &str,
    session: Option<&str>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder().uri(path);
    if let Some(session) = session {
        builder = builder.header("cookie", format!("mm_session={session}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::empty()).expect("valid request"))
        .await
        .expect("infallible");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);

    (status, json)
}

#[tokio::test]
async fn module_api_serves_ingested_modules() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let mut tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    // The variance-bounds assertions below need the per-type roll
    // extremes, which seed_reference would otherwise truncate away.
    tables.abyssal_statistics = mutamarket::sde::statistics::compute_abyssal_statistics(&tables);
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    // The estimated_value assertions below rely on the type having no
    // trained estimator model; other suites may have seeded a statistics
    // row for it.
    sqlx::query("delete from estimator_statistics where type_id = 47408")
        .execute(&pool)
        .await
        .expect("clean estimator statistic");

    // Ingest the first module of the first fixture file: a 50MN Abyssal
    // Microwarpdrive (type 47408).
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let module = &fixture.modules[0];

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

    // Idempotency: a prior run of this suite may have left a contract
    // linked (the index section below attaches one). The parity assertions
    // rely on the module being contract-less, so unlink it here.
    sqlx::query("update modules set latest_contract_id = null where id = $1")
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("unlink prior contract");
    // Same for public assets other suites may have published: the
    // loaded-but-empty assertions below expect a null public_asset.
    sqlx::query("delete from public_assets where module_id = $1")
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("unpublish prior assets");

    let app = mutamarket::server::test_router().await;

    // Show by bare item id.
    let (status, body) = get_json(&app, &format!("/api/modules/{}", module.module_id)).await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];
    assert_eq!(data["id"], serde_json::json!(module.module_id));
    assert_eq!(data["type"]["id"], serde_json::json!(fixture.type_id));
    assert_eq!(
        data["source_type"]["id"],
        serde_json::json!(module.source_type_id)
    );
    assert_eq!(
        data["mutaplasmid"]["id"],
        serde_json::json!(module.mutaplasmid_id)
    );
    assert_eq!(
        data["mutated_attributes"].as_array().map(Vec::len),
        Some(module.expected.attributes.len()),
    );

    let slug = data["slug"].as_str().expect("slug present").to_owned();
    assert!(slug.ends_with(&module.module_id.to_string()));

    // A rolled attribute matches its expected computed values.
    let expected_first = &module.expected.attributes[0];
    let attribute = data["mutated_attributes"]
        .as_array()
        .expect("attributes array")
        .iter()
        .find(|attribute| attribute["id"] == serde_json::json!(expected_first.attribute_id))
        .expect("expected attribute present");
    assert!(common::matches(
        expected_first.fraction,
        attribute["fraction"].as_f64().expect("fraction"),
    ));

    // Exact key parity with the legacy ModuleResource for guests with the
    // default relations: every key legacy emits, nothing missing.
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

    assert_eq!(
        sorted_keys(data),
        [
            "average_fraction",
            "contract",
            "creator",
            "estimated_value",
            "estimated_value_updated_at",
            "id",
            "mutaplasmid",
            "mutated_attributes",
            "public_asset",
            "slug",
            "source_type",
            "type",
        ],
        "module key set diverges from the legacy resource",
    );
    assert_eq!(sorted_keys(&data["type"]), ["id", "name"]);
    assert_eq!(sorted_keys(&data["mutaplasmid"]), ["id", "name"]);
    assert_eq!(
        sorted_keys(&data["source_type"]),
        ["id", "meta_group", "meta_group_id", "name", "published"],
    );
    assert_eq!(
        sorted_keys(&data["creator"]),
        [
            "corporation_id",
            "description",
            "has_premium",
            "id",
            "name",
            "slug"
        ],
    );
    assert!(
        data["source_type"]["meta_group"].is_string(),
        "meta group name resolves: {}",
        data["source_type"]["meta_group"],
    );

    // Feature keys owned by unported milestones are present and null, like
    // legacy's loaded-but-empty relations.
    assert!(data["contract"].is_null());
    assert!(data["public_asset"].is_null());

    // Ingestion runs the estimate like the legacy ProcessModule tail: with
    // no trained model for the type the value stays null but the
    // timestamp advances.
    assert!(data["estimated_value"].is_null());
    assert!(
        data["estimated_value_updated_at"].is_string(),
        "ingestion stamps the estimate attempt: {}",
        data["estimated_value_updated_at"],
    );

    // Attribute rows carry the exact legacy MutatedAttributeResource keys
    // (plus our server-computed type_band).
    assert_eq!(
        sorted_keys(attribute),
        [
            "bar",
            "base_value",
            "display_name",
            "fraction",
            "fraction_absolute",
            "fraction_type",
            "id",
            "is_derived",
            "is_virtual",
            "name",
            "type_band",
            "unit",
            "value",
        ],
        "attribute key set diverges from the legacy resource",
    );
    let unit_attribute = data["mutated_attributes"]
        .as_array()
        .expect("attributes array")
        .iter()
        .find(|attribute| attribute["unit"].is_object())
        .expect("an attribute with a unit");
    assert_eq!(
        sorted_keys(&unit_attribute["unit"]),
        ["display_name", "id", "name"]
    );

    // Show by slug.
    let (status, body) = get_json(&app, &format!("/api/modules/{slug}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], serde_json::json!(module.module_id));

    // The show-page payload: the module plus the type's estimator
    // statistic (null while no statistic row exists).
    sqlx::query("delete from estimator_statistics where type_id = $1")
        .bind(fixture.type_id)
        .execute(&pool)
        .await
        .expect("clean statistic");
    let (status, body) = get_json(&app, &format!("/api/module-page/{slug}")).await;
    assert_eq!(status, StatusCode::OK);
    crate::common::assert_default_module_keys(&body["module"], false, &[]);
    assert_eq!(
        sorted_keys(&body),
        [
            "abyssal_type_statistics",
            "estimator_statistic",
            "historic_contracts",
            "module",
            "source_type_comparisons",
        ],
    );

    // The variance-search bounds source: one row per rollable attribute
    // of the type.
    let type_statistics = body["abyssal_type_statistics"]
        .as_array()
        .expect("stats array");
    assert!(
        !type_statistics.is_empty(),
        "the fixture type has roll statistics"
    );
    assert_eq!(
        sorted_keys(&type_statistics[0]),
        [
            "attribute_id",
            "best",
            "high_is_good",
            "is_virtual",
            "worst"
        ],
    );
    assert_eq!(body["module"]["id"], serde_json::json!(module.module_id));
    assert!(body["estimator_statistic"].is_null());

    // The source-type table data: every published input type of the
    // module's mutaplasmid, in the legacy default order.
    let comparisons = body["source_type_comparisons"]
        .as_array()
        .expect("comparisons array");
    assert!(
        !comparisons.is_empty(),
        "the fixture mutaplasmid has input types"
    );
    let module_attributes = body["module"]["mutated_attributes"]
        .as_array()
        .expect("attributes");
    for comparison in comparisons {
        assert_eq!(
            sorted_keys(comparison),
            ["attributes", "average_price", "type"]
        );
        assert_eq!(
            sorted_keys(&comparison["type"]),
            ["id", "meta_group_id", "meta_level", "name"],
        );
        let attributes = comparison["attributes"]
            .as_array()
            .expect("attribute values");
        assert_eq!(attributes.len(), module_attributes.len());
        for (value, module_attribute) in attributes.iter().zip(module_attributes) {
            assert_eq!(sorted_keys(value), ["id", "value"]);
            assert_eq!(
                value["id"], module_attribute["id"],
                "column order mirrors the module"
            );
        }
    }

    // The module's own source type is one of the rows, and its values are
    // the module's base values (that is what mutation math rolls from).
    let own = comparisons
        .iter()
        .find(|comparison| comparison["type"]["id"] == serde_json::json!(module.source_type_id))
        .expect("own source type listed");
    for (value, module_attribute) in own["attributes"]
        .as_array()
        .expect("values")
        .iter()
        .zip(module_attributes)
    {
        if module_attribute["is_virtual"] == serde_json::json!(false)
            && module_attribute["is_derived"] == serde_json::json!(false)
        {
            assert!(
                (value["value"].as_f64().expect("value")
                    - module_attribute["base_value"].as_f64().expect("base"))
                .abs()
                    < 1e-9,
                "own source type carries the base value",
            );
        }
    }

    // The contract-history rows: archived contracts holding this module,
    // newest first, with the legacy ContractResource key set (no
    // ignore_for_training for guests).
    // Clean every archived contract touching this module (other suites
    // seed some, e.g. the legacy importer test), then our two rows.
    sqlx::query(
        "delete from historic_contracts where id in
             (select historic_contract_id from historic_contract_items where item_id = $1)",
    )
    .bind(module.module_id)
    .execute(&pool)
    .await
    .expect("clean linked historic contracts");
    sqlx::query("delete from historic_contracts where id = any($1)")
        .bind(vec![800_301i64, 800_302])
        .execute(&pool)
        .await
        .expect("clean historic contracts");
    sqlx::query(
        "insert into characters (id, name) values (90999998, 'History Issuer')
         on conflict (id) do nothing",
    )
    .execute(&pool)
    .await
    .expect("seed issuer");
    for (contract_id, contract_status, price) in [
        (800_301i64, "completed", 250_000_000.0),
        (800_302, "failed", 300_000_000.0),
    ] {
        sqlx::query(
            "insert into historic_contracts
                 (id, status, region_id, issuer_id, type, unified_price,
                  date_issued, date_expired, abyssal_modules_count)
             values ($1, $2, 10000002, 90999998, 'item_exchange', $3,
                     now() - interval '3 days', now() + interval '4 days', 1)",
        )
        .bind(contract_id)
        .bind(contract_status)
        .bind(price)
        .execute(&pool)
        .await
        .expect("seed historic contract");
        sqlx::query(
            "insert into historic_contract_items
                 (historic_contract_id, record_id, type_id, item_id)
             values ($1, 1, $2, $3)",
        )
        .bind(contract_id)
        .bind(fixture.type_id)
        .bind(module.module_id)
        .execute(&pool)
        .await
        .expect("seed historic item");
    }
    let (_, body) = get_json(&app, &format!("/api/module-page/{slug}")).await;
    let historic = body["historic_contracts"]
        .as_array()
        .expect("historic array");
    assert_eq!(
        historic
            .iter()
            .map(|contract| contract["id"].as_i64())
            .collect::<Vec<_>>(),
        [Some(800_302), Some(800_301)],
        "newest contract first",
    );
    let first = &historic[0];
    assert_eq!(
        sorted_keys(first),
        [
            "abyssal_modules_count",
            "asking_for_items",
            "date_expired",
            "date_issued",
            "id",
            "issuer",
            "non_abyssal_modules_count",
            "plex_count",
            "price",
            "status",
            "type",
        ],
        "historic contract key set diverges from the legacy resource",
    );
    assert_eq!(
        sorted_keys(&first["issuer"]),
        [
            "corporation_id",
            "description",
            "has_premium",
            "id",
            "name",
            "slug"
        ],
    );
    assert_eq!(first["status"], serde_json::json!("failed"));
    assert_eq!(first["price"], serde_json::json!(300_000_000.0));
    sqlx::query("delete from historic_contracts where id = any($1)")
        .bind(vec![800_301i64, 800_302])
        .execute(&pool)
        .await
        .expect("clean historic contracts");

    // Default order: meta-group rank (T1, T2, Storyline, Faction,
    // Deadspace, Officer), then meta level, then name.
    let rank = |comparison: &serde_json::Value| {
        let group = comparison["type"]["meta_group_id"].as_i64();
        let level = comparison["type"]["meta_level"].as_i64().unwrap_or(0);
        let group_rank = match group {
            Some(1) => 1,
            Some(2) => 2,
            Some(3) => 3,
            Some(4) => 4,
            Some(6) => 5,
            Some(5) => 6,
            Some(other) => other,
            None => i64::MAX,
        };
        (
            group_rank,
            level,
            comparison["type"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        )
    };
    let mut expected_order: Vec<_> = comparisons.iter().map(rank).collect();
    expected_order.sort();
    assert_eq!(
        comparisons.iter().map(rank).collect::<Vec<_>>(),
        expected_order,
        "server emits the legacy default order",
    );

    sqlx::query(
        "insert into estimator_statistics
             (type_id, name, data_count, r2, mae, nmae, last_trained_at, data_statistics)
         values ($1, 'Test Type', 120, 0.87, 12000000, 9.5, now(),
                 '{\"50MN Microwarpdrive II\": 80, \"Core X-Type MWD\": 40}'::jsonb)",
    )
    .bind(fixture.type_id)
    .execute(&pool)
    .await
    .expect("seed statistic");
    let (_, body) = get_json(&app, &format!("/api/module-page/{slug}")).await;
    let statistic = &body["estimator_statistic"];
    assert_eq!(
        sorted_keys(statistic),
        [
            "data_count",
            "data_statistics",
            "last_trained_at",
            "mae",
            "nmae",
            "r2"
        ],
    );
    assert_eq!(statistic["r2"], serde_json::json!(0.87));
    assert_eq!(statistic["data_count"], serde_json::json!(120));
    assert_eq!(
        statistic["data_statistics"]["50MN Microwarpdrive II"],
        serde_json::json!(80),
    );
    assert!(statistic["last_trained_at"].is_string());

    // Leave no synthetic statistic behind: the estimator suite asserts
    // this type's seeded row.
    sqlx::query("delete from estimator_statistics where type_id = $1")
        .bind(fixture.type_id)
        .execute(&pool)
        .await
        .expect("clean statistic");

    let (status, body) = get_json(&app, "/api/module-page/does-not-exist-999999999999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        serde_json::json!("No module with this item id is known to MutaMarket."),
    );

    // The similar-sold data: a second module of the same type sold alone
    // in a completed exchange becomes a training module and shows up for
    // premium accounts only.
    let sibling = &fixture.modules[1];
    process_module(
        &pool,
        &reference,
        &estimator_stub(),
        fixture.type_id,
        sibling.module_id,
        &DogmaItem {
            created_by: sibling.creator_id,
            source_type_id: sibling.source_type_id,
            mutator_type_id: sibling.mutaplasmid_id,
            dogma_attributes: common::fixture_dogma(sibling),
        },
    )
    .await
    .expect("process sibling module");

    const SOLD_CONTRACT: i64 = 800_303;
    const SOLD_PRICE: f64 = 425_000_000.0;
    sqlx::query("delete from historic_contracts where id = $1")
        .bind(SOLD_CONTRACT)
        .execute(&pool)
        .await
        .expect("clean sold contract");
    sqlx::query("delete from training_modules where module_id = any($1)")
        .bind(vec![module.module_id, sibling.module_id])
        .execute(&pool)
        .await
        .expect("clean training modules");
    sqlx::query(
        "insert into historic_contracts
             (id, status, region_id, issuer_id, type, unified_price,
              date_issued, abyssal_modules_count, non_abyssal_modules_count)
         values ($1, 'completed', 10000002, 90999998, 'item_exchange', $2,
                 now() - interval '9 days', 1, 0)",
    )
    .bind(SOLD_CONTRACT)
    .bind(SOLD_PRICE)
    .execute(&pool)
    .await
    .expect("seed sold contract");
    sqlx::query(
        "insert into historic_contract_items
             (historic_contract_id, record_id, type_id, item_id)
         values ($1, 1, $2, $3)",
    )
    .bind(SOLD_CONTRACT)
    .bind(fixture.type_id)
    .bind(sibling.module_id)
    .execute(&pool)
    .await
    .expect("seed sold item");

    let (_, upserted) = mutamarket::contracts::sync_training_modules(&pool)
        .await
        .expect("training sweep");
    assert!(upserted >= 1, "the sold sibling qualifies: {upserted}");
    let trained: Option<i64> = sqlx::query_scalar(
        "select historic_contract_id from training_modules where module_id = $1",
    )
    .bind(sibling.module_id)
    .fetch_optional(&pool)
    .await
    .expect("training module row");
    assert_eq!(trained, Some(SOLD_CONTRACT));

    // Guests (and non-premium users) get the empty list.
    let (status, body) = get_json(&app, &format!("/api/module-page/{slug}/similar")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::json!({ "similar_modules": [] }));

    // A premium account sees the sold sibling with its sale attached.
    let existing: Option<i64> =
        sqlx::query_scalar("select id from users where name = 'Premium Similar'")
            .fetch_optional(&pool)
            .await
            .expect("user lookup");
    let premium_user: i64 = match existing {
        Some(id) => id,
        None => {
            sqlx::query_scalar("insert into users (name) values ('Premium Similar') returning id")
                .fetch_one(&pool)
                .await
                .expect("seed premium user")
        }
    };
    sqlx::query(
        "insert into characters (id, name, user_id, premium_paid_until)
         values (90999996, 'Premium Character', $1, now() + interval '30 days')
         on conflict (id) do update
         set user_id = excluded.user_id, premium_paid_until = excluded.premium_paid_until",
    )
    .bind(premium_user)
    .execute(&pool)
    .await
    .expect("seed premium character");
    let session = mutamarket::auth::session::create_session(&pool, premium_user, None)
        .await
        .expect("create session");
    let (status, body) = get_json_as(
        &app,
        &format!("/api/module-page/{slug}/similar"),
        Some(&session),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let similar = body["similar_modules"].as_array().expect("similar array");
    assert_eq!(
        similar.len(),
        1,
        "the sibling is the only sold module of the type"
    );
    let entry = &similar[0];
    assert_eq!(entry["id"], serde_json::json!(sibling.module_id));
    crate::common::assert_default_module_keys(entry, true, &["training_module"]);
    assert!(
        sorted_keys(entry).contains(&"training_module"),
        "the sale rides on the module resource",
    );
    assert_eq!(
        sorted_keys(&entry["training_module"]),
        ["contract_id", "sold_at", "sold_for"],
    );
    assert_eq!(
        entry["training_module"]["sold_for"],
        serde_json::json!(SOLD_PRICE)
    );
    assert_eq!(
        entry["training_module"]["contract_id"],
        serde_json::json!(SOLD_CONTRACT)
    );

    // Admin-ignored contracts drop out on the next sweep.
    sqlx::query("update historic_contracts set ignore_for_training = true where id = $1")
        .bind(SOLD_CONTRACT)
        .execute(&pool)
        .await
        .expect("ignore contract");
    let (deleted, _) = mutamarket::contracts::sync_training_modules(&pool)
        .await
        .expect("training sweep");
    assert!(
        deleted >= 1,
        "ignored contracts lose their training module: {deleted}"
    );
    sqlx::query("delete from historic_contracts where id = $1")
        .bind(SOLD_CONTRACT)
        .execute(&pool)
        .await
        .expect("clean sold contract");
    sqlx::query("delete from sessions where user_id = $1")
        .bind(premium_user)
        .execute(&pool)
        .await
        .expect("clean session");

    // Unknown module id.
    let (status, body) = get_json(&app, "/api/modules/does-not-exist-999999999999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["message"].is_string());

    // The index lists for-sale modules; give ours a live contract (after
    // the parity assertions above saw the loaded-but-empty null).
    common::attach_contract(
        &pool,
        module.module_id,
        800_201,
        "item_exchange",
        275_000_000.0,
        1,
        0,
        0,
    )
    .await;

    // Type-scoped index by id and by slug contains the module.
    for type_query in [
        format!("/api/modules/type/{}", fixture.type_id),
        "/api/modules/type/50mn-abyssal-microwarpdrive".to_owned(),
    ] {
        let (status, body) = get_json(&app, &type_query).await;
        assert_eq!(status, StatusCode::OK, "{type_query}");
        let ids: Vec<i64> = body["data"]
            .as_array()
            .expect("data array")
            .iter()
            .filter_map(|module| module["id"].as_i64())
            .collect();
        assert!(ids.contains(&module.module_id), "{type_query}: {ids:?}");
    }

    // The index without a type option rejects, like the legacy API.
    let (status, body) = get_json(&app, "/api/modules").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body["message"],
        serde_json::json!("Please provide a valid type.")
    );

    // Estimator statistics serve a JSON array (row shape is pinned in the
    // estimator suite).
    let (status, body) = get_json(&app, "/api/estimator-statistics").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_array());
}
