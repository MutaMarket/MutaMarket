//! Behavior test for publishing owned assets: making an asset public
//! creates the ownership rows that surface its abyssal modules on the
//! owner's character page; unpublishing removes them.

mod common;

use std::path::Path;

use mutamarket::assets::public::{publish_asset, unpublish_asset};
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::modules::ingest::{DogmaItem, process_module};
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};

const CHARACTER_ID: i64 = 930_001;

fn estimator_stub() -> mutamarket::estimator::Estimator {
    mutamarket::estimator::Estimator::new()
}

#[tokio::test]
async fn publishing_an_asset_surfaces_its_modules_on_the_character_page() {
    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");

    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables).await.expect("seed reference");
    let reference = ReferenceData::from_tables(tables);

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

    // A user owning a character that holds the module as an abyssal asset.
    // Idempotent across runs.
    sqlx::query("delete from characters where id = $1").bind(CHARACTER_ID).execute(&pool).await.ok();
    sqlx::query("delete from users where name = 'Asset Publisher'").execute(&pool).await.ok();
    let user_id: i64 = sqlx::query_scalar("insert into users (name) values ('Asset Publisher') returning id")
        .fetch_one(&pool)
        .await
        .expect("user");
    sqlx::query("insert into characters (id, name, user_id) values ($1, 'Asset Publisher', $2)")
        .bind(CHARACTER_ID)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("character");
    let asset_id: i64 = sqlx::query_scalar(
        "insert into assets
             (character_id, item_id, type_id, location_id, location_flag, location_type,
              quantity, index, is_abyssal)
         values ($1, $2, $3, 60003760, 'Hangar', 'station', 1, 0, true)
         returning id",
    )
    .bind(CHARACTER_ID)
    .bind(module.module_id)
    .bind(fixture.type_id)
    .fetch_one(&pool)
    .await
    .expect("asset");

    // Before publishing, the character has no public ownerships and does
    // not appear on the characters index.
    let index = mutamarket::characters::characters_index(&pool, None, 1).await.expect("index");
    assert!(!index.iter().any(|c| c.id == CHARACTER_ID), "not listed before publish");

    // Publish: an ownership row appears and the character is now listed.
    publish_asset(&pool, user_id, asset_id).await.expect("publish");
    let owned: i64 = sqlx::query_scalar(
        "select count(*) from public_module_ownerships where character_id = $1 and module_id = $2",
    )
    .bind(CHARACTER_ID)
    .bind(module.module_id)
    .fetch_one(&pool)
    .await
    .expect("ownership count");
    assert_eq!(owned, 1, "publishing creates the module ownership");

    let index = mutamarket::characters::characters_index(&pool, None, 1).await.expect("index");
    let listed = index.iter().find(|c| c.id == CHARACTER_ID).expect("character now listed");
    assert_eq!(listed.modules_count, Some(1));

    let module_ids =
        mutamarket::characters::publicly_owned_module_ids(&pool, CHARACTER_ID, 40).await.expect("ids");
    assert!(module_ids.contains(&module.module_id), "module surfaces on the character page");

    // A published module with no contract is also visible in the for-sale
    // browse (legacy `visible` = contract OR public asset).
    let search = mutamarket::modules::search::parse(
        &pool,
        &reference,
        &format!("type/{}", fixture.type_id),
    )
    .await
    .expect("parse");
    let visible = mutamarket::modules::search::module_ids(
        &pool,
        &search,
        mutamarket::modules::search::Visibility::ForSale,
        50,
    )
    .await
    .expect("visible ids");
    assert!(
        visible.contains(&module.module_id),
        "published module appears in the for-sale browse without a contract",
    );

    // Publishing again is idempotent (no duplicate ownership).
    publish_asset(&pool, user_id, asset_id).await.expect("re-publish");
    let owned: i64 = sqlx::query_scalar(
        "select count(*) from public_module_ownerships where character_id = $1 and module_id = $2",
    )
    .bind(CHARACTER_ID)
    .bind(module.module_id)
    .fetch_one(&pool)
    .await
    .expect("ownership count");
    assert_eq!(owned, 1, "re-publishing does not duplicate");

    // Ownership by a foreign user is refused.
    let other_id: i64 = sqlx::query_scalar("insert into users (name) values ('Someone Else') returning id")
        .fetch_one(&pool)
        .await
        .expect("other user");
    assert!(publish_asset(&pool, other_id, asset_id).await.is_err(), "foreign publish refused");

    // Unpublish removes the ownership and delists the character.
    let public_asset_id: i64 =
        sqlx::query_scalar("select id from public_assets where asset_id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("public asset id");
    unpublish_asset(&pool, user_id, public_asset_id).await.expect("unpublish");
    let owned: i64 =
        sqlx::query_scalar("select count(*) from public_module_ownerships where character_id = $1")
            .bind(CHARACTER_ID)
            .fetch_one(&pool)
            .await
            .expect("ownership count");
    assert_eq!(owned, 0, "unpublishing removes the ownership");

    sqlx::query("delete from users where id = any($1)")
        .bind(vec![user_id, other_id])
        .execute(&pool)
        .await
        .ok();
}
