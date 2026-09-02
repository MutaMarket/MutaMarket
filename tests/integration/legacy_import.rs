//! Behavior test for the one-time legacy MySQL import: a scratch MySQL
//! database gets legacy-shaped tables with rows covering every type
//! coercion (unsigned ids, tinyint bools, decimals, datetimes, the scope
//! pivot, JSON columns) plus reference-skip and two-pass cases, and the
//! import is asserted row by row in Postgres.
//!
//! Needs the local Postgres (`docker compose up -d postgres`) AND a local
//! MySQL server at 127.0.0.1:3306 (root, no password, or set
//! TEST_LEGACY_DATABASE_URL). Unlike Postgres, MySQL is only needed for
//! the legacy bootstrap, so this test skips with a notice instead of
//! failing when MySQL is not reachable.

use crate::common;

use std::path::Path;

use mutamarket::contracts::sync_training_modules;
use mutamarket::db;
use mutamarket::db::reference::seed_reference;
use mutamarket::legacy::{run_import, validate_sample};
use mutamarket::mutation::calculator::calculate;
use mutamarket::mutation::reference::{ReferenceData, ReferenceTables};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, PgPool, Row};

const SERVER_URL: &str = "mysql://root@127.0.0.1:3306/mysql";
const TEST_DB: &str = "mutamarket_legacy_test";

async fn exec(pool: &MySqlPool, sql: &str) {
    sqlx::query(sql)
        .execute(pool)
        .await
        .unwrap_or_else(|error| {
            panic!("mysql statement failed: {error}\n{sql}");
        });
}

/// The legacy-shaped tables, reduced to the columns the importer reads.
async fn create_legacy_schema(mysql: &MySqlPool) {
    for ddl in [
        "create table users (
             id bigint unsigned primary key, name varchar(255), is_admin tinyint(1) not null,
             discord_id bigint unsigned, discord_name varchar(255), discord_avatar varchar(255),
             discord_channel_id bigint unsigned, twitch_id bigint unsigned,
             twitch_name varchar(255), twitch_avatar varchar(255), twitch_email varchar(255),
             patreon_id bigint unsigned, patreon_name varchar(255), patreon_avatar varchar(255),
             patreon_email varchar(255), patreon_nickname varchar(255),
             is_patreon_member tinyint(1) not null default 0,
             created_at datetime, updated_at datetime)",
        "create table characters (
             id bigint unsigned primary key, name varchar(255), corporation_id bigint unsigned,
             alliance_id bigint unsigned, user_id bigint unsigned,
             character_owner_hash varchar(255), description text,
             premium_paid_until datetime,
             premium_paid_total decimal(50,2) not null default 0,
             premium_payment_rest decimal(50,2) not null default 0,
             name_fetched_at datetime,
             contracts_fetched_at datetime, latest_asset_import_id bigint unsigned,
             created_at datetime, updated_at datetime)",
        "create table donations (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             journal_id bigint unsigned, amount decimal(50,2) not null, date datetime not null,
             confirmation_sent tinyint(1) not null default 0,
             created_at datetime, updated_at datetime)",
        "create table esi_tokens (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             access_token text, refresh_token text, token_type varchar(255),
             character_owner_hash varchar(255), expires_at datetime, created_at datetime)",
        "create table esi_scopes (id bigint unsigned primary key, name varchar(255))",
        "create table esi_token_scope (
             id bigint unsigned primary key, esi_token_id bigint unsigned,
             esi_scope_id bigint unsigned)",
        "create table modules (
             id bigint unsigned primary key, type_id bigint unsigned not null,
             source_type_id bigint unsigned not null, mutaplasmid_id bigint unsigned not null,
             creator_id bigint unsigned, estimated_value decimal(50,2),
             estimated_value_updated_at datetime, average_fraction double,
             created_at datetime, updated_at datetime)",
        "create table mutated_attributes (
             id bigint unsigned primary key, module_id bigint unsigned not null,
             attribute_id bigint unsigned not null, type_id bigint unsigned not null,
             value double not null, base_value double not null, fraction double not null,
             fraction_type double not null, fraction_absolute double not null,
             bar tinyint not null, is_virtual tinyint(1) not null)",
        "create table historic_contracts (
             id bigint unsigned primary key, status varchar(255) not null,
             region_id bigint unsigned not null, start_location_id bigint unsigned,
             issuer_id bigint unsigned not null, issuer_corporation_id bigint unsigned,
             for_corporation tinyint(1) not null, type varchar(255) not null,
             title varchar(255), date_issued datetime, date_expired datetime,
             price double, buyout double, highest_bid double, unified_price double,
             asking_for_items tinyint(1) not null, abyssal_modules_count bigint not null,
             non_abyssal_modules_count bigint not null, plex_count bigint not null,
             ignore_for_training tinyint(1) not null,
             created_at datetime, updated_at datetime)",
        "create table historic_contract_items (
             id bigint unsigned primary key, historic_contract_id bigint unsigned not null,
             record_id bigint unsigned not null, type_id bigint unsigned not null,
             item_id bigint unsigned not null)",
        "create table market_histories (
             id bigint unsigned primary key, type_id bigint unsigned not null,
             region_id bigint unsigned not null, date date not null,
             average decimal(50,2) not null, highest decimal(50,2) not null,
             lowest decimal(50,2) not null, order_count decimal(50,2) not null,
             volume decimal(50,2) not null)",
        "create table estimator_statistics (
             id bigint unsigned primary key, type_id bigint unsigned not null,
             name varchar(255) not null, data_count bigint unsigned not null,
             r2 double, mae double, nmae double, last_trained_at datetime,
             data_statistics json not null, created_at datetime, updated_at datetime)",
        "create table collections (
             id bigint unsigned primary key, identifier varchar(255) not null,
             name varchar(255) not null, description text, visibility varchar(255) not null,
             character_id bigint unsigned not null, created_at datetime, updated_at datetime)",
        "create table collection_modules (
             id bigint unsigned primary key, collection_id bigint unsigned not null,
             module_id bigint unsigned not null, note text,
             created_at datetime, updated_at datetime)",
        "create table asset_imports (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             status varchar(255) not null, step varchar(255) not null,
             assets_count bigint unsigned not null,
             assets_corporation_count bigint unsigned not null,
             abyssal_modules_count bigint unsigned not null,
             abyssal_modules_imported_count bigint unsigned not null,
             abyssal_modules_failed_count bigint unsigned not null,
             created_at datetime, updated_at datetime)",
        "create table assets (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             corporation_id bigint unsigned, item_id bigint unsigned not null,
             type_id bigint unsigned not null, name varchar(255),
             location_id bigint unsigned not null, location_flag varchar(255) not null,
             location_type varchar(255) not null, quantity bigint not null,
             `index` int not null, is_abyssal tinyint(1) not null,
             created_at datetime, updated_at datetime)",
        "create table public_assets (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             asset_id bigint unsigned not null, public_parent_id bigint unsigned,
             module_id bigint unsigned, created_at datetime, updated_at datetime)",
        "create table public_module_ownerships (
             id bigint unsigned primary key, character_id bigint unsigned not null,
             module_id bigint unsigned not null, public_asset_id bigint unsigned,
             contract_id bigint unsigned, created_at datetime, updated_at datetime)",
    ] {
        exec(mysql, ddl).await;
    }
}

async fn pg_count(pg: &PgPool, sql: &str) -> i64 {
    sqlx::query_scalar(sql)
        .fetch_one(pg)
        .await
        .expect("count query")
}

#[tokio::test]
async fn legacy_import_replaces_the_domain_data() {
    let server_url =
        std::env::var("TEST_LEGACY_DATABASE_URL").unwrap_or_else(|_| SERVER_URL.to_owned());
    let server = match MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&server_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!(
                "skipping legacy_import test: MySQL not reachable at {server_url} ({error}); \
                 only the legacy bootstrap needs MySQL",
            );
            return;
        }
    };
    exec(&server, &format!("drop database if exists {TEST_DB}")).await;
    exec(&server, &format!("create database {TEST_DB}")).await;
    let mysql_url = server_url
        .rsplit_once('/')
        .map(|(base, _)| format!("{base}/{TEST_DB}"));
    let mysql = MySqlPoolOptions::new()
        .max_connections(2)
        .connect(&mysql_url.expect("db url"))
        .await
        .expect("connect scratch database");
    create_legacy_schema(&mysql).await;

    let pool = db::test_pool()
        .await
        .expect("Postgres not reachable - start it with `docker compose up -d postgres`");
    db::migrate(&pool).await.expect("migrations run");
    let tables =
        ReferenceTables::load_from_dir(Path::new("tests/fixtures/reference")).expect("dumps parse");
    seed_reference(&pool, &tables)
        .await
        .expect("seed reference tables");
    let reference = ReferenceData::from_tables(tables);

    // The imported module reuses a parsing fixture so its attribute rows
    // can be computed with the real mutation math (validation must see a
    // faithful copy recompute exactly).
    let fixtures = common::load_module_fixtures();
    let fixture = &fixtures[0];
    let module = &fixture.modules[0];
    let context = reference
        .context(module.mutaplasmid_id, module.source_type_id)
        .expect("fixture combination");
    let results = calculate(&context, &common::fixture_dogma(module));

    // Users: an admin with linked services and NULL leftovers, plus a
    // plain user.
    exec(&mysql, "insert into users
            (id, name, is_admin, discord_id, discord_name, is_patreon_member, created_at, updated_at)
         values (1, 'Tim', 1, 190000000000000001, 'tim#1', 1, '2024-05-01 12:00:00', '2026-02-23 09:30:00'),
                (2, 'Plain', 0, null, null, 0, '2025-01-01 00:00:00', '2025-01-01 00:00:00')").await;

    // Characters: the creator (user 1, premium with a paid history), a
    // stub with a dangling user id (must import with user_id nulled),
    // and an issuer.
    exec(
        &mysql,
        &format!(
            "insert into characters
            (id, name, corporation_id, user_id, character_owner_hash, premium_paid_until,
             premium_paid_total, premium_payment_rest, created_at, updated_at)
         values ({creator}, 'Creator', 1000100, 1, 'hash-1', '2030-01-01 00:00:00',
                 350000000.00, 50000000.00, '2024-05-01 12:00:00', '2026-02-23 09:30:00'),
                (95000001, 'Dangling User', null, 57, null, null,
                 0, 0, '2024-05-01 12:00:00', '2024-05-01 12:00:00'),
                (95000002, 'Issuer', 1000100, null, null, null,
                 0, 0, '2024-05-01 12:00:00', '2024-05-01 12:00:00')",
            creator = module.creator_id,
        ),
    )
    .await;

    // Donations: a wallet-journal one, a manual one without a journal
    // id, and one for a character the snapshot lacks (skipped).
    exec(
        &mysql,
        &format!(
            "insert into donations
            (id, character_id, journal_id, amount, date, confirmation_sent,
             created_at, updated_at)
         values (61, {creator}, 22000000001, 150000000.00, '2025-06-01 18:00:00', 1,
                 '2025-06-01 18:05:00', '2025-06-01 18:05:00'),
                (62, {creator}, null, 200000000.00, '2025-07-01 12:00:00', 1,
                 '2025-07-01 12:00:00', '2025-07-01 12:00:00'),
                (63, 999999999, 33000000001, 5000000.00, '2025-07-02 12:00:00', 0,
                 '2025-07-02 12:00:00', '2025-07-02 12:00:00')",
            creator = module.creator_id,
        ),
    )
    .await;

    // Tokens: one with two scopes through the pivot; one for a character
    // the snapshot does not contain (skipped by the reference filter).
    exec(
        &mysql,
        &format!(
            "insert into esi_tokens
            (id, character_id, access_token, refresh_token, token_type,
             character_owner_hash, expires_at, created_at)
         values (11, {creator}, 'access-jwt', 'refresh-blob', 'Bearer', 'hash-1',
                 '2026-02-23 10:00:00', '2024-05-01 12:00:00'),
                (12, 999999999, 'x', 'y', 'Bearer', 'z', '2026-02-23 10:00:00',
                 '2024-05-01 12:00:00')",
            creator = module.creator_id,
        ),
    )
    .await;
    exec(
        &mysql,
        "insert into esi_scopes (id, name) values
            (1, 'esi-assets.read_assets.v1'), (2, 'esi-contracts.read_character_contracts.v1')",
    )
    .await;
    exec(
        &mysql,
        "insert into esi_token_scope (id, esi_token_id, esi_scope_id)
         values (1, 11, 1), (2, 11, 2)",
    )
    .await;

    // Modules: the fixture roll, and one whose type no longer exists in
    // the current SDE reference (skipped, and its attributes with it).
    exec(
        &mysql,
        &format!(
            "insert into modules
            (id, type_id, source_type_id, mutaplasmid_id, creator_id, estimated_value,
             estimated_value_updated_at, average_fraction, created_at, updated_at)
         values ({id}, {type_id}, {source}, {muta}, {creator}, 275000000.50,
                 '2026-02-20 08:00:00', 0.25, '2025-06-01 00:00:00', '2026-02-20 08:00:00'),
                (910000001, 999999901, {source}, {muta}, {creator}, null, null, null,
                 '2025-06-01 00:00:00', '2025-06-01 00:00:00')",
            id = module.module_id,
            type_id = fixture.type_id,
            source = module.source_type_id,
            muta = module.mutaplasmid_id,
            creator = module.creator_id,
        ),
    )
    .await;

    let mut attribute_rows = Vec::new();
    for (index, result) in results.iter().enumerate() {
        attribute_rows.push(format!(
            "({id}, {module_id}, {attribute_id}, {type_id}, {value}, {base}, {fraction}, \
              {fraction_type}, {fraction_absolute}, {bar}, {virtual_})",
            id = index + 1,
            module_id = module.module_id,
            attribute_id = result.attribute_id,
            type_id = fixture.type_id,
            value = result.value,
            base = result.base_value,
            fraction = result.fraction,
            fraction_type = result.fraction_type,
            fraction_absolute = result.fraction_absolute,
            bar = result.bar.as_int(),
            virtual_ = i32::from(result.is_virtual),
        ));
    }
    // Two rows for the skipped module: they must be skipped with it.
    attribute_rows.push(format!(
        "(9001, 910000001, {attribute}, 999999901, 1, 1, 0, 0, 0, 0, 0)",
        attribute = results[0].attribute_id,
    ));
    attribute_rows.push(format!(
        "(9002, 910000001, {attribute}, 999999901, 2, 2, 0, 0, 0, 0, 0)",
        attribute = results[1].attribute_id,
    ));
    exec(
        &mysql,
        &format!(
            "insert into mutated_attributes
            (id, module_id, attribute_id, type_id, value, base_value, fraction,
             fraction_type, fraction_absolute, bar, is_virtual)
         values {}",
            attribute_rows.join(", "),
        ),
    )
    .await;

    // Historic contracts: a completed single-module exchange (training
    // material) and one with an unknown issuer (skipped).
    exec(
        &mysql,
        "insert into historic_contracts
            (id, status, region_id, start_location_id, issuer_id, issuer_corporation_id,
             for_corporation, type, title, date_issued, date_expired, price, buyout,
             highest_bid, unified_price, asking_for_items, abyssal_modules_count,
             non_abyssal_modules_count, plex_count, ignore_for_training,
             created_at, updated_at)
         values (700001, 'completed', 10000002, 60003760, 95000002, 1000100, 0,
                 'item_exchange', 'juicy roll', '2026-01-10 10:00:00', '2026-01-24 10:00:00',
                 600000000, null, null, 600000000, 0, 1, 0, 0, 0,
                 '2026-01-25 00:00:00', '2026-01-25 00:00:00'),
                (700002, 'failed', 10000002, null, 999999902, null, 0, 'auction',
                 null, null, null, null, null, null, null, 0, 1, 0, 0, 0,
                 '2026-01-25 00:00:00', '2026-01-25 00:00:00')",
    )
    .await;
    exec(
        &mysql,
        &format!(
            "insert into historic_contract_items
            (id, historic_contract_id, record_id, type_id, item_id)
         values (1, 700001, 1, {type_id}, {module_id}),
                (2, 700002, 1, {type_id}, 910000001)",
            type_id = fixture.type_id,
            module_id = module.module_id,
        ),
    )
    .await;

    exec(
        &mysql,
        "insert into market_histories
            (id, type_id, region_id, date, average, highest, lowest, order_count, volume)
         values (1, 44992, 10000002, '2026-02-22', 5000000.25, 5100000.00, 4900000.00,
                 120.00, 6000.00)",
    )
    .await;

    exec(
        &mysql,
        &format!(
            "insert into estimator_statistics
            (id, type_id, name, data_count, r2, mae, nmae, last_trained_at,
             data_statistics, created_at, updated_at)
         values (1, {type_id}, '50MN Abyssal Microwarpdrive', 120, 0.87, 12000000, 9.5,
                 '2026-02-01 00:00:00', '{{\"50MN Microwarpdrive II\": 80}}',
                 '2026-02-01 00:00:00', '2026-02-01 00:00:00')",
            type_id = fixture.type_id,
        ),
    )
    .await;

    // Collections: one entry per state — a kept module link and one to
    // the skipped module (dropped with it).
    exec(
        &mysql,
        &format!(
            "insert into collections
            (id, identifier, name, description, visibility, character_id,
             created_at, updated_at)
         values (31, 'abc123', 'My rolls', null, 'public', {creator},
                 '2025-06-01 00:00:00', '2025-06-01 00:00:00')",
            creator = module.creator_id,
        ),
    )
    .await;
    exec(
        &mysql,
        &format!(
            "insert into collection_modules
            (id, collection_id, module_id, note, created_at, updated_at)
         values (41, 31, {module_id}, 'keeper', '2025-06-01 00:00:00', '2025-06-01 00:00:00'),
                (42, 31, 910000001, null, '2025-06-01 00:00:00', '2025-06-01 00:00:00')",
            module_id = module.module_id,
        ),
    )
    .await;

    // Assets: an abyssal item plus the import run and the two-pass
    // pointers (character.latest_asset_import_id, public parent chain).
    exec(
        &mysql,
        &format!(
            "insert into asset_imports
            (id, character_id, status, step, assets_count, assets_corporation_count,
             abyssal_modules_count, abyssal_modules_imported_count,
             abyssal_modules_failed_count, created_at, updated_at)
         values (51, {creator}, 'finished', 'done', 100, 0, 3, 3, 0,
                 '2026-02-23 09:00:00', '2026-02-23 09:05:00')",
            creator = module.creator_id,
        ),
    )
    .await;
    exec(
        &mysql,
        &format!(
            "update characters set latest_asset_import_id = 51 where id = {creator}",
            creator = module.creator_id,
        ),
    )
    .await;
    exec(
        &mysql,
        &format!(
            "insert into assets
            (id, character_id, corporation_id, item_id, type_id, name, location_id,
             location_flag, location_type, quantity, `index`, is_abyssal,
             created_at, updated_at)
         values (61, {creator}, null, {module_id}, {type_id}, null, 60003760,
                 'Hangar', 'station', 1, 4, 1, '2026-02-23 09:00:00', '2026-02-23 09:00:00'),
                (62, {creator}, null, 555000001, 35832, 'Home', 30000142,
                 'Hangar', 'solar_system', 1, 0, 0, '2026-02-23 09:00:00', '2026-02-23 09:00:00')",
            creator = module.creator_id,
            module_id = module.module_id,
            type_id = fixture.type_id,
        ),
    )
    .await;
    exec(
        &mysql,
        &format!(
            "insert into public_assets
            (id, character_id, asset_id, public_parent_id, module_id, created_at, updated_at)
         values (71, {creator}, 62, null, null, '2026-02-23 09:00:00', '2026-02-23 09:00:00'),
                (72, {creator}, 61, 71, {module_id}, '2026-02-23 09:00:00', '2026-02-23 09:00:00')",
            creator = module.creator_id,
            module_id = module.module_id,
        ),
    )
    .await;

    exec(&mysql, &format!(
        "insert into public_module_ownerships
            (id, character_id, module_id, public_asset_id, contract_id, created_at, updated_at)
         values (81, {creator}, {module_id}, 72, 555000999, '2026-02-23 09:00:00', '2026-02-23 09:00:00'),
                (82, {creator}, 910000001, null, null, '2026-02-23 09:00:00', '2026-02-23 09:00:00')",
        creator = module.creator_id,
        module_id = module.module_id,
    )).await;

    // First import.
    let report = run_import(&mysql, &pool).await.expect("import runs");
    let by_name = |name: &str| {
        report
            .tables
            .iter()
            .find(|table| table.table == name)
            .unwrap_or_else(|| panic!("table {name} in report"))
    };
    assert_eq!(
        (by_name("users").imported, by_name("users").skipped),
        (2, 0)
    );
    assert_eq!(
        (
            by_name("characters").imported,
            by_name("characters").skipped
        ),
        (3, 0)
    );
    assert_eq!(
        (
            by_name("esi_tokens").imported,
            by_name("esi_tokens").skipped
        ),
        (1, 1)
    );
    assert_eq!(
        (by_name("modules").imported, by_name("modules").skipped),
        (1, 1)
    );
    assert_eq!(
        (
            by_name("mutated_attributes").imported,
            by_name("mutated_attributes").skipped
        ),
        (results.len() as u64, 2),
    );
    assert_eq!(
        (
            by_name("historic_contracts").imported,
            by_name("historic_contracts").skipped
        ),
        (1, 1),
    );
    assert_eq!(
        (
            by_name("historic_contract_items").imported,
            by_name("historic_contract_items").skipped,
        ),
        (1, 1),
    );
    assert_eq!(by_name("market_histories").imported, 1);
    assert_eq!(by_name("estimator_statistics").imported, 1);
    assert_eq!(
        (
            by_name("collections").imported,
            by_name("collections").skipped
        ),
        (1, 0)
    );
    assert_eq!(
        (
            by_name("collection_modules").imported,
            by_name("collection_modules").skipped
        ),
        (1, 1),
    );
    assert_eq!(by_name("asset_imports").imported, 1);
    assert_eq!(by_name("characters.latest_asset_import_id").imported, 1);
    assert_eq!(by_name("assets").imported, 2);
    assert_eq!(by_name("public_assets").imported, 2);
    assert_eq!(by_name("public_assets.public_parent_id").imported, 1);
    assert_eq!(
        (
            by_name("public_module_ownerships").imported,
            by_name("public_module_ownerships").skipped,
        ),
        (1, 1),
        "the ownership of the skipped module is dropped with it",
    );
    let ownership: (Option<i64>, Option<i64>) = sqlx::query_as(
        "select public_asset_id, contract_id from public_module_ownerships where id = 81",
    )
    .fetch_one(&pool)
    .await
    .expect("ownership row");
    assert_eq!(
        ownership,
        (Some(72), None),
        "the stale contract link is dropped"
    );

    // Type coercions and the two-pass pointers landed.
    let user_row = sqlx::query(
        "select name, is_admin, discord_name, is_patreon_member from users where id = 1",
    )
    .fetch_one(&pool)
    .await
    .expect("user row");
    assert_eq!(user_row.get::<String, _>("name"), "Tim");
    assert!(user_row.get::<bool, _>("is_admin"));
    assert_eq!(
        user_row.get::<Option<String>, _>("discord_name"),
        Some("tim#1".to_owned())
    );
    assert!(user_row.get::<bool, _>("is_patreon_member"));

    let character = sqlx::query(
        "select user_id, latest_asset_import_id,
                premium_paid_until > now() as premium,
                premium_paid_total, premium_payment_rest,
                created_at::text as created_at
         from characters where id = $1",
    )
    .bind(module.creator_id)
    .fetch_one(&pool)
    .await
    .expect("creator row");
    assert_eq!(character.get::<Option<i64>, _>("user_id"), Some(1));
    assert_eq!(
        character.get::<Option<i64>, _>("latest_asset_import_id"),
        Some(51)
    );
    assert_eq!(character.get::<Option<bool>, _>("premium"), Some(true));
    assert_eq!(character.get::<f64, _>("premium_paid_total"), 350_000_000.0);
    assert_eq!(
        character.get::<f64, _>("premium_payment_rest"),
        50_000_000.0
    );

    // Donations: both creator rows landed (the manual one keeps its null
    // journal id), the unknown-character one was skipped.
    let donations: Vec<(i64, Option<i64>, f64, bool)> = sqlx::query_as(
        "select id, journal_id, amount, confirmation_sent from donations order by id",
    )
    .fetch_all(&pool)
    .await
    .expect("donation rows");
    assert_eq!(
        donations,
        vec![
            (61, Some(22_000_000_001), 150_000_000.0, true),
            (62, None, 200_000_000.0, true),
        ],
    );
    assert!(
        character
            .get::<String, _>("created_at")
            .starts_with("2024-05-01 12:00:00"),
        "datetimes import as UTC: {}",
        character.get::<String, _>("created_at"),
    );
    let dangling: Option<i64> =
        sqlx::query_scalar("select user_id from characters where id = 95000001")
            .fetch_one(&pool)
            .await
            .expect("dangling character");
    assert_eq!(dangling, None, "unknown user links are nulled");

    let scopes: Vec<String> =
        sqlx::query_scalar("select unnest(scopes) from esi_tokens where id = 11 order by 1")
            .fetch_all(&pool)
            .await
            .expect("scopes");
    assert_eq!(
        scopes,
        [
            "esi-assets.read_assets.v1",
            "esi-contracts.read_character_contracts.v1"
        ],
    );

    let imported_module =
        sqlx::query("select estimated_value, average_fraction from modules where id = $1")
            .bind(module.module_id)
            .fetch_one(&pool)
            .await
            .expect("module row");
    assert_eq!(
        imported_module.get::<Option<f64>, _>("estimated_value"),
        Some(275_000_000.5)
    );
    assert_eq!(
        imported_module.get::<Option<f64>, _>("average_fraction"),
        Some(0.25)
    );
    assert_eq!(pg_count(&pool, "select count(*) from modules").await, 1);

    let statistic: serde_json::Value =
        sqlx::query_scalar("select data_statistics from estimator_statistics where id = 1")
            .fetch_one(&pool)
            .await
            .expect("statistic json");
    assert_eq!(statistic["50MN Microwarpdrive II"], serde_json::json!(80));

    let parent: Option<i64> =
        sqlx::query_scalar("select public_parent_id from public_assets where id = 72")
            .fetch_one(&pool)
            .await
            .expect("child public asset");
    assert_eq!(
        parent,
        Some(71),
        "the parent pointer lands in the second pass"
    );
    let asset_index: i64 = sqlx::query_scalar("select \"index\" from assets where id = 61")
        .fetch_one(&pool)
        .await
        .expect("asset row");
    assert_eq!(asset_index, 4);

    // Sequences are bumped past the imported ids.
    let next_collection: i64 = sqlx::query_scalar(
        "insert into collections (identifier, name, visibility, character_id)
         values ('seq-check', 'Sequence check', 'private', $1) returning id",
    )
    .bind(module.creator_id)
    .fetch_one(&pool)
    .await
    .expect("native insert");
    assert!(
        next_collection > 31,
        "sequence continues after legacy ids: {next_collection}"
    );

    // The training sweep derives from the imported archive.
    let (_, upserted) = sync_training_modules(&pool).await.expect("training sweep");
    assert!(upserted >= 1);
    let trained: Option<i64> = sqlx::query_scalar(
        "select historic_contract_id from training_modules where module_id = $1",
    )
    .bind(module.module_id)
    .fetch_optional(&pool)
    .await
    .expect("training row");
    assert_eq!(trained, Some(700001));

    // The faithful copy recomputes exactly through our mutation math.
    let validation = validate_sample(&pool, &reference, 10)
        .await
        .expect("validation");
    assert_eq!(validation.sampled, 1);
    assert_eq!(
        validation.matching, 1,
        "imported attributes recompute exactly"
    );

    // Idempotency: a second run wipes and reloads to the same counts.
    let second = run_import(&mysql, &pool).await.expect("second import");
    let imported = |report: &mutamarket::legacy::ImportReport| {
        report
            .tables
            .iter()
            .map(|table| (table.table, table.imported))
            .collect::<Vec<_>>()
    };
    assert_eq!(imported(&report), imported(&second));
    assert_eq!(pg_count(&pool, "select count(*) from modules").await, 1);

    exec(&server, &format!("drop database if exists {TEST_DB}")).await;
}
