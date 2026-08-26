//! One-time import of the legacy Laravel/MySQL production database into
//! Postgres (`cargo run --bin legacy_import`).
//!
//! This is a bootstrap tool, not a production path: it wipes the domain
//! tables and replaces them with the legacy snapshot, table by table in
//! foreign-key order. Reference/SDE tables are never touched — those
//! come from the native `sde_import` — and the live contract market is
//! wiped rather than imported (the snapshot's contracts are stale; the
//! region sweep rebuilds the market from ESI afterwards).
//!
//! Mechanics: each table is pumped in keyset-paginated batches. The
//! MySQL side serializes every row to a `JSON_OBJECT` (which settles the
//! MySQL-side type quirks: unsigned bigints are cast signed, decimals
//! cast to double, datetimes formatted as UTC ISO strings), and the
//! Postgres side unpacks the batch with `jsonb_to_recordset`, casting
//! and filtering rows whose references are missing on our side (types
//! dropped from the current SDE, for example). Skipped rows are counted
//! and reported — never silently dropped.

use sqlx::{MySqlPool, PgPool};

use crate::mutation::calculator::{DogmaAttribute, calculate};
use crate::mutation::reference::ReferenceData;

/// Rows per keyset page. JSON batches stay a few MB even for the widest
/// tables.
const BATCH_SIZE: i64 = 10_000;

/// How many modules the post-import validation recomputes through the
/// mutation math.
pub const VALIDATION_SAMPLE: i64 = 200;

#[derive(Debug, Clone)]
pub struct TableReport {
    pub table: &'static str,
    pub imported: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub tables: Vec<TableReport>,
}

/// One pumped step: the MySQL query MUST select `(id, json)` — a signed
/// id for pagination and the row as a JSON object string — and take two
/// placeholders (last seen id, batch size). The Postgres statement takes
/// the JSON batch as `$1` and is responsible for casts, reference
/// filtering and conflict handling.
pub struct TableSpec {
    pub name: &'static str,
    pub mysql: &'static str,
    pub postgres: &'static str,
}

/// The domain tables the import owns, wiped before loading. Everything
/// here either comes from the legacy snapshot or is rebuilt by our own
/// jobs afterwards (contracts by the region sweep, training modules by
/// the training sweep). Referencing tables not listed are cleared by the
/// cascade.
const WIPED_TABLES: &str = "users, sessions, characters, esi_tokens,
     modules, mutated_attributes,
     contracts, contract_items, contract_imports,
     character_contracts, character_contract_items, character_structure,
     historic_contracts, historic_contract_items, training_modules,
     market_histories, estimator_statistics,
     collections, collection_modules,
     assets, asset_imports, public_assets, public_module_ownerships";

/// Tables whose `id` is a Postgres sequence that must be bumped past the
/// imported ids so later native inserts do not collide.
const SEQUENCED_TABLES: &[&str] = &[
    "users",
    "esi_tokens",
    "mutated_attributes",
    "historic_contract_items",
    "market_histories",
    "estimator_statistics",
    "collections",
    "collection_modules",
    "assets",
    "asset_imports",
    "public_assets",
];

pub fn table_specs() -> Vec<TableSpec> {
    vec![
        TableSpec {
            name: "users",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'name', name,
                        'is_admin', cast(is_admin as signed),
                        'discord_id', cast(discord_id as signed),
                        'discord_name', discord_name,
                        'discord_avatar', discord_avatar,
                        'discord_channel_id', cast(discord_channel_id as signed),
                        'twitch_id', cast(twitch_id as signed),
                        'twitch_name', twitch_name,
                        'twitch_avatar', twitch_avatar,
                        'twitch_email', twitch_email,
                        'patreon_id', cast(patreon_id as signed),
                        'patreon_name', patreon_name,
                        'patreon_avatar', patreon_avatar,
                        'patreon_email', patreon_email,
                        'patreon_nickname', patreon_nickname,
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from users where id > ? order by id limit ?",
            postgres: "insert into users
                        (id, name, is_admin, discord_id, discord_name, discord_avatar,
                         discord_channel_id, twitch_id, twitch_name, twitch_avatar,
                         twitch_email, patreon_id, patreon_name, patreon_avatar,
                         patreon_email, patreon_nickname, created_at, updated_at)
                       select t.id, coalesce(t.name, ''), t.is_admin <> 0,
                              t.discord_id, t.discord_name, t.discord_avatar,
                              t.discord_channel_id, t.twitch_id, t.twitch_name,
                              t.twitch_avatar, t.twitch_email, t.patreon_id,
                              t.patreon_name, t.patreon_avatar, t.patreon_email,
                              t.patreon_nickname,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, name text, is_admin int, discord_id bigint,
                             discord_name text, discord_avatar text,
                             discord_channel_id bigint, twitch_id bigint,
                             twitch_name text, twitch_avatar text, twitch_email text,
                             patreon_id bigint, patreon_name text, patreon_avatar text,
                             patreon_email text, patreon_nickname text,
                             created_at text, updated_at text)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "characters",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'name', name,
                        'corporation_id', cast(corporation_id as signed),
                        'alliance_id', cast(alliance_id as signed),
                        'user_id', cast(user_id as signed),
                        'character_owner_hash', character_owner_hash,
                        'description', description,
                        'premium_paid_until', date_format(premium_paid_until, '%Y-%m-%dT%H:%i:%SZ'),
                        'name_fetched_at', date_format(name_fetched_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'contracts_fetched_at', date_format(contracts_fetched_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from characters where id > ? order by id limit ?",
            postgres: "insert into characters
                        (id, name, corporation_id, alliance_id, user_id,
                         character_owner_hash, description, premium_paid_until,
                         name_fetched_at, contracts_fetched_at, created_at, updated_at)
                       select t.id, coalesce(t.name, ''), t.corporation_id, t.alliance_id,
                              (select u.id from users u where u.id = t.user_id),
                              t.character_owner_hash, t.description,
                              t.premium_paid_until::timestamptz,
                              t.name_fetched_at::timestamptz,
                              t.contracts_fetched_at::timestamptz,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, name text, corporation_id bigint,
                             alliance_id bigint, user_id bigint,
                             character_owner_hash text, description text,
                             premium_paid_until text, name_fetched_at text,
                             contracts_fetched_at text, created_at text, updated_at text)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "esi_tokens",
            mysql: "select cast(t.id as signed) as id, cast(json_object(
                        'id', cast(t.id as signed),
                        'character_id', cast(t.character_id as signed),
                        'access_token', t.access_token,
                        'refresh_token', t.refresh_token,
                        'token_type', t.token_type,
                        'character_owner_hash', t.character_owner_hash,
                        'scopes', (select json_arrayagg(s.name)
                                   from esi_token_scope ts
                                   join esi_scopes s on s.id = ts.esi_scope_id
                                   where ts.esi_token_id = t.id),
                        'expires_at', date_format(t.expires_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'created_at', date_format(t.created_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from esi_tokens t where t.id > ? order by t.id limit ?",
            postgres: "insert into esi_tokens
                        (id, character_id, access_token, refresh_token, token_type,
                         character_owner_hash, scopes, expires_at, created_at)
                       select t.id, t.character_id, coalesce(t.access_token, ''),
                              coalesce(t.refresh_token, ''),
                              coalesce(t.token_type, 'Bearer'),
                              coalesce(t.character_owner_hash, ''),
                              coalesce((select array_agg(scope)
                                        from jsonb_array_elements_text(
                                            coalesce(t.scopes, '[]'::jsonb)) scope),
                                       '{}'),
                              coalesce(t.expires_at::timestamptz, now()),
                              coalesce(t.created_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, character_id bigint, access_token text,
                             refresh_token text, token_type text,
                             character_owner_hash text, scopes jsonb,
                             expires_at text, created_at text)
                       where exists (select 1 from characters c where c.id = t.character_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "modules",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'type_id', cast(type_id as signed),
                        'source_type_id', cast(source_type_id as signed),
                        'mutaplasmid_id', cast(mutaplasmid_id as signed),
                        'creator_id', cast(creator_id as signed),
                        'estimated_value', cast(estimated_value as double),
                        'estimated_value_updated_at',
                            date_format(estimated_value_updated_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'average_fraction', average_fraction,
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from modules where id > ? order by id limit ?",
            // The live contract link is deliberately dropped: the
            // snapshot's market is stale, the region sweep relinks.
            postgres: "insert into modules
                        (id, type_id, source_type_id, mutaplasmid_id, creator_id,
                         estimated_value, estimated_value_updated_at, average_fraction,
                         created_at, updated_at)
                       select t.id, t.type_id, t.source_type_id, t.mutaplasmid_id,
                              (select c.id from characters c where c.id = t.creator_id),
                              t.estimated_value, t.estimated_value_updated_at::timestamptz,
                              t.average_fraction,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, type_id bigint, source_type_id bigint,
                             mutaplasmid_id bigint, creator_id bigint,
                             estimated_value float8, estimated_value_updated_at text,
                             average_fraction float8, created_at text, updated_at text)
                       where exists (select 1 from types ty where ty.id = t.type_id)
                         and exists (select 1 from types st where st.id = t.source_type_id)
                         and exists (select 1 from mutaplasmids mp
                                     where mp.id = t.mutaplasmid_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "mutated_attributes",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'module_id', cast(module_id as signed),
                        'attribute_id', cast(attribute_id as signed),
                        'type_id', cast(type_id as signed),
                        'value', value,
                        'base_value', base_value,
                        'fraction', fraction,
                        'fraction_type', fraction_type,
                        'fraction_absolute', fraction_absolute,
                        'bar', cast(bar as signed),
                        'is_virtual', cast(is_virtual as signed)
                    ) as char) as j
                    from mutated_attributes where id > ? order by id limit ?",
            postgres: "insert into mutated_attributes
                        (id, module_id, attribute_id, type_id, value, base_value,
                         fraction, fraction_type, fraction_absolute, bar, is_virtual)
                       select t.id, t.module_id, t.attribute_id, t.type_id, t.value,
                              t.base_value, t.fraction, t.fraction_type,
                              t.fraction_absolute, t.bar::smallint, t.is_virtual <> 0
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, module_id bigint, attribute_id bigint,
                             type_id bigint, value float8, base_value float8,
                             fraction float8, fraction_type float8,
                             fraction_absolute float8, bar int, is_virtual int)
                       where exists (select 1 from modules m where m.id = t.module_id)
                         and exists (select 1 from attributes a where a.id = t.attribute_id)
                         and exists (select 1 from types ty where ty.id = t.type_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "historic_contracts",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'status', status,
                        'region_id', cast(region_id as signed),
                        'start_location_id', cast(start_location_id as signed),
                        'issuer_id', cast(issuer_id as signed),
                        'issuer_corporation_id', cast(issuer_corporation_id as signed),
                        'for_corporation', cast(for_corporation as signed),
                        'type', type,
                        'title', title,
                        'date_issued', date_format(date_issued, '%Y-%m-%dT%H:%i:%SZ'),
                        'date_expired', date_format(date_expired, '%Y-%m-%dT%H:%i:%SZ'),
                        'price', price,
                        'buyout', buyout,
                        'highest_bid', highest_bid,
                        'unified_price', unified_price,
                        'asking_for_items', cast(asking_for_items as signed),
                        'abyssal_modules_count', cast(abyssal_modules_count as signed),
                        'non_abyssal_modules_count', cast(non_abyssal_modules_count as signed),
                        'plex_count', cast(plex_count as signed),
                        'ignore_for_training', cast(ignore_for_training as signed),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from historic_contracts where id > ? order by id limit ?",
            postgres: "insert into historic_contracts
                        (id, status, region_id, start_location_id, issuer_id,
                         issuer_corporation_id, for_corporation, type, title,
                         date_issued, date_expired, price, buyout, highest_bid,
                         unified_price, asking_for_items, abyssal_modules_count,
                         non_abyssal_modules_count, plex_count, ignore_for_training,
                         created_at, updated_at)
                       select t.id, coalesce(t.status, 'unknown'), t.region_id,
                              t.start_location_id, t.issuer_id, t.issuer_corporation_id,
                              t.for_corporation <> 0, coalesce(t.type, 'item_exchange'),
                              t.title, t.date_issued::timestamptz,
                              t.date_expired::timestamptz, t.price, t.buyout,
                              t.highest_bid, t.unified_price, t.asking_for_items <> 0,
                              t.abyssal_modules_count, t.non_abyssal_modules_count,
                              t.plex_count, t.ignore_for_training <> 0,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, status text, region_id bigint,
                             start_location_id bigint, issuer_id bigint,
                             issuer_corporation_id bigint, for_corporation int,
                             type text, title text, date_issued text, date_expired text,
                             price float8, buyout float8, highest_bid float8,
                             unified_price float8, asking_for_items int,
                             abyssal_modules_count int, non_abyssal_modules_count int,
                             plex_count int, ignore_for_training int,
                             created_at text, updated_at text)
                       where exists (select 1 from characters c where c.id = t.issuer_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "historic_contract_items",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'historic_contract_id', cast(historic_contract_id as signed),
                        'record_id', cast(record_id as signed),
                        'type_id', cast(type_id as signed),
                        'item_id', cast(item_id as signed)
                    ) as char) as j
                    from historic_contract_items where id > ? order by id limit ?",
            postgres: "insert into historic_contract_items
                        (id, historic_contract_id, record_id, type_id, item_id)
                       select t.id, t.historic_contract_id, t.record_id, t.type_id,
                              t.item_id
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, historic_contract_id bigint, record_id bigint,
                             type_id bigint, item_id bigint)
                       where exists (select 1 from historic_contracts hc
                                     where hc.id = t.historic_contract_id)
                         and exists (select 1 from types ty where ty.id = t.type_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "market_histories",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'type_id', cast(type_id as signed),
                        'region_id', cast(region_id as signed),
                        'date', date_format(date, '%Y-%m-%d'),
                        'average', cast(average as double),
                        'highest', cast(highest as double),
                        'lowest', cast(lowest as double),
                        'order_count', cast(order_count as signed),
                        'volume', cast(volume as signed)
                    ) as char) as j
                    from market_histories where id > ? order by id limit ?",
            postgres: "insert into market_histories
                        (id, type_id, region_id, date, average, highest, lowest,
                         order_count, volume)
                       select t.id, t.type_id, t.region_id, t.date::date, t.average,
                              t.highest, t.lowest, t.order_count, t.volume
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, type_id bigint, region_id bigint, date text,
                             average float8, highest float8, lowest float8,
                             order_count bigint, volume bigint)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "estimator_statistics",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'type_id', cast(type_id as signed),
                        'name', name,
                        'data_count', cast(data_count as signed),
                        'r2', r2,
                        'mae', mae,
                        'nmae', nmae,
                        'last_trained_at', date_format(last_trained_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'data_statistics', data_statistics,
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from estimator_statistics where id > ? order by id limit ?",
            postgres: "insert into estimator_statistics
                        (id, type_id, name, data_count, r2, mae, nmae, last_trained_at,
                         data_statistics, created_at, updated_at)
                       select t.id, t.type_id, coalesce(t.name, ''), t.data_count, t.r2,
                              t.mae, t.nmae, t.last_trained_at::timestamptz,
                              t.data_statistics,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, type_id bigint, name text, data_count bigint,
                             r2 float8, mae float8, nmae float8, last_trained_at text,
                             data_statistics jsonb, created_at text, updated_at text)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "collections",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'identifier', identifier,
                        'name', name,
                        'description', description,
                        'visibility', visibility,
                        'character_id', cast(character_id as signed),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from collections where id > ? order by id limit ?",
            postgres: "insert into collections
                        (id, identifier, name, description, visibility, character_id,
                         created_at, updated_at)
                       select t.id, coalesce(t.identifier, ''), coalesce(t.name, ''),
                              t.description, coalesce(t.visibility, 'private'),
                              t.character_id,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, identifier text, name text, description text,
                             visibility text, character_id bigint,
                             created_at text, updated_at text)
                       where exists (select 1 from characters c where c.id = t.character_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "collection_modules",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'collection_id', cast(collection_id as signed),
                        'module_id', cast(module_id as signed),
                        'note', note,
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from collection_modules where id > ? order by id limit ?",
            postgres: "insert into collection_modules
                        (id, collection_id, module_id, note, created_at, updated_at)
                       select t.id, t.collection_id, t.module_id, t.note,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, collection_id bigint, module_id bigint,
                             note text, created_at text, updated_at text)
                       where exists (select 1 from collections c
                                     where c.id = t.collection_id)
                         and exists (select 1 from modules m where m.id = t.module_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "asset_imports",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'character_id', cast(character_id as signed),
                        'status', status,
                        'step', step,
                        'assets_count', cast(assets_count as signed),
                        'assets_corporation_count', cast(assets_corporation_count as signed),
                        'abyssal_modules_count', cast(abyssal_modules_count as signed),
                        'abyssal_modules_imported_count',
                            cast(abyssal_modules_imported_count as signed),
                        'abyssal_modules_failed_count',
                            cast(abyssal_modules_failed_count as signed),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from asset_imports where id > ? order by id limit ?",
            postgres: "insert into asset_imports
                        (id, character_id, status, step, assets_count,
                         assets_corporation_count, abyssal_modules_count,
                         abyssal_modules_imported_count, abyssal_modules_failed_count,
                         created_at, updated_at)
                       select t.id, t.character_id, coalesce(t.status, ''),
                              coalesce(t.step, ''), coalesce(t.assets_count, 0),
                              coalesce(t.assets_corporation_count, 0),
                              coalesce(t.abyssal_modules_count, 0),
                              coalesce(t.abyssal_modules_imported_count, 0),
                              coalesce(t.abyssal_modules_failed_count, 0),
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, character_id bigint, status text, step text,
                             assets_count int, assets_corporation_count int,
                             abyssal_modules_count int,
                             abyssal_modules_imported_count int,
                             abyssal_modules_failed_count int,
                             created_at text, updated_at text)
                       where exists (select 1 from characters c where c.id = t.character_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            // Second pass over characters: the latest-import pointer can
            // only land after asset_imports exists (circular FK pair).
            name: "characters.latest_asset_import_id",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'latest_asset_import_id', cast(latest_asset_import_id as signed)
                    ) as char) as j
                    from characters
                    where latest_asset_import_id is not null and id > ?
                    order by id limit ?",
            postgres: "update characters c
                       set latest_asset_import_id = t.latest_asset_import_id
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, latest_asset_import_id bigint)
                       where c.id = t.id
                         and exists (select 1 from asset_imports ai
                                     where ai.id = t.latest_asset_import_id)",
        },
        TableSpec {
            name: "assets",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'character_id', cast(character_id as signed),
                        'corporation_id', cast(corporation_id as signed),
                        'item_id', cast(item_id as signed),
                        'type_id', cast(type_id as signed),
                        'name', name,
                        'location_id', cast(location_id as signed),
                        'location_flag', location_flag,
                        'location_type', location_type,
                        'quantity', cast(quantity as signed),
                        'index', cast(`index` as signed),
                        'is_abyssal', cast(is_abyssal as signed),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from assets where id > ? order by id limit ?",
            postgres: "insert into assets
                        (id, character_id, corporation_id, item_id, type_id, name,
                         location_id, location_flag, location_type, quantity, \"index\",
                         is_abyssal, created_at, updated_at)
                       select t.id, t.character_id, t.corporation_id, t.item_id,
                              t.type_id, t.name, t.location_id, t.location_flag,
                              t.location_type, coalesce(t.quantity, 1),
                              coalesce(t.\"index\", 0), t.is_abyssal <> 0,
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, character_id bigint, corporation_id bigint,
                             item_id bigint, type_id bigint, name text,
                             location_id bigint, location_flag text, location_type text,
                             quantity bigint, \"index\" int, is_abyssal int,
                             created_at text, updated_at text)
                       where exists (select 1 from characters c where c.id = t.character_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "public_assets",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'character_id', cast(character_id as signed),
                        'asset_id', cast(asset_id as signed),
                        'module_id', cast(module_id as signed),
                        'created_at', date_format(created_at, '%Y-%m-%dT%H:%i:%SZ'),
                        'updated_at', date_format(updated_at, '%Y-%m-%dT%H:%i:%SZ')
                    ) as char) as j
                    from public_assets where id > ? order by id limit ?",
            // The self-referencing parent pointer lands in the second
            // pass below, once every row exists.
            postgres: "insert into public_assets
                        (id, character_id, asset_id, module_id, created_at, updated_at)
                       select t.id, t.character_id, t.asset_id,
                              (select m.id from modules m where m.id = t.module_id),
                              coalesce(t.created_at::timestamptz, now()),
                              coalesce(t.updated_at::timestamptz, now())
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, character_id bigint, asset_id bigint,
                             module_id bigint, created_at text, updated_at text)
                       where exists (select 1 from characters c where c.id = t.character_id)
                         and exists (select 1 from assets a where a.id = t.asset_id)
                       on conflict (id) do nothing",
        },
        TableSpec {
            name: "public_assets.public_parent_id",
            mysql: "select cast(id as signed) as id, cast(json_object(
                        'id', cast(id as signed),
                        'public_parent_id', cast(public_parent_id as signed)
                    ) as char) as j
                    from public_assets
                    where public_parent_id is not null and id > ?
                    order by id limit ?",
            postgres: "update public_assets p
                       set public_parent_id = t.public_parent_id
                       from jsonb_to_recordset($1::jsonb) as t
                            (id bigint, public_parent_id bigint)
                       where p.id = t.id
                         and exists (select 1 from public_assets parent
                                     where parent.id = t.public_parent_id)",
        },
    ]
}

/// Wipes the domain tables the import owns. Reference/SDE and scheduler
/// tables stay untouched.
pub async fn wipe_domain_tables(pg: &PgPool) -> sqlx::Result<()> {
    sqlx::query(&format!("truncate table {WIPED_TABLES} cascade"))
        .execute(pg)
        .await?;
    Ok(())
}

/// MySQL pages fetched ahead of the Postgres COPY writer.
const PIPELINE_DEPTH: usize = 4;

/// The unlogged one-column staging table each table streams through.
const STAGING_TABLE: &str = "_legacy_stage";

/// Pumps one table spec through the Postgres bulk path: the MySQL keyset
/// walk streams JSON lines into an unlogged staging table via COPY (no
/// indexes, no synchronous WAL), then a single set-based insert lands
/// the table — reference filters become hash joins instead of per-row
/// probes. Returns (imported, skipped) — skipped rows failed a reference
/// filter (or were already present).
pub async fn import_table(
    mysql: &MySqlPool,
    pg: &PgPool,
    spec: &TableSpec,
) -> sqlx::Result<(u64, u64)> {
    let mut connection = pg.acquire().await?;
    // A bootstrap can simply re-run after a crash, so the load skips the
    // synchronous WAL flush.
    sqlx::query("set synchronous_commit = off").execute(&mut *connection).await?;
    sqlx::query(&format!("drop table if exists {STAGING_TABLE}"))
        .execute(&mut *connection)
        .await?;
    sqlx::query(&format!("create unlogged table {STAGING_TABLE} (j jsonb)"))
        .execute(&mut *connection)
        .await?;

    // Producer: keyset pages from MySQL, a few in flight ahead of the
    // COPY writer.
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<Vec<(i64, String)>>(PIPELINE_DEPTH);
    let mysql = mysql.clone();
    let mysql_query = spec.mysql;
    let producer = tokio::spawn(async move {
        let mut last_id = 0i64;
        loop {
            let rows: Vec<(i64, String)> = sqlx::query_as(mysql_query)
                .bind(last_id)
                .bind(BATCH_SIZE)
                .fetch_all(&mysql)
                .await?;
            let Some((batch_last, _)) = rows.last() else {
                return Ok::<_, sqlx::Error>(());
            };
            last_id = *batch_last;
            if sender.send(rows).await.is_err() {
                return Ok(());
            }
        }
    });

    // COPY the JSON lines verbatim: CSV format with control-character
    // quote/delimiter bytes that cannot appear in JSON text, so no
    // escaping pass is needed.
    let mut staged = 0u64;
    let mut copy = connection
        .copy_in_raw(&format!(
            "copy {STAGING_TABLE} (j) from stdin
             with (format csv, quote e'\\x01', delimiter e'\\x02')",
        ))
        .await?;
    while let Some(rows) = receiver.recv().await {
        staged += rows.len() as u64;
        let mut chunk = String::with_capacity(rows.len() * 160);
        for (_, json) in &rows {
            chunk.push_str(json);
            chunk.push('\n');
        }
        if let Err(error) = copy.send(chunk.as_bytes()).await {
            copy.abort("copy failed").await.ok();
            return Err(error);
        }
    }
    copy.finish().await?;
    producer.await.expect("producer task")?;

    // One set-based insert per table: the spec's recordset source is
    // swapped for the staged rows.
    let landed = spec.postgres.replace(
        "jsonb_to_recordset($1::jsonb)",
        &format!("{STAGING_TABLE}, lateral jsonb_to_record({STAGING_TABLE}.j)"),
    );
    let imported = sqlx::query(&landed).execute(&mut *connection).await?.rows_affected();

    sqlx::query(&format!("drop table if exists {STAGING_TABLE}"))
        .execute(&mut *connection)
        .await?;

    Ok((imported, staged - imported.min(staged)))
}

/// Bumps every imported table's id sequence past the imported ids so
/// native inserts do not collide with legacy rows.
pub async fn fix_sequences(pg: &PgPool) -> sqlx::Result<()> {
    for table in SEQUENCED_TABLES {
        let sequence: Option<String> =
            sqlx::query_scalar("select pg_get_serial_sequence($1, 'id')")
                .bind(table)
                .fetch_one(pg)
                .await?;
        if let Some(sequence) = sequence {
            sqlx::query(&format!(
                "select setval('{sequence}',
                     greatest((select coalesce(max(id), 0) from {table}), 1))",
            ))
            .execute(pg)
            .await?;
        }
    }
    Ok(())
}

/// Runs the whole import: wipe, every table in order, sequence fixes.
pub async fn run_import(mysql: &MySqlPool, pg: &PgPool) -> sqlx::Result<ImportReport> {
    wipe_domain_tables(pg).await?;

    let mut report = ImportReport::default();
    for spec in table_specs() {
        let started = std::time::Instant::now();
        let (imported, skipped) = import_table(mysql, pg, &spec).await?;
        println!(
            "  {}: {imported} imported, {skipped} skipped ({:.1?})",
            spec.name,
            started.elapsed(),
        );
        report.tables.push(TableReport { table: spec.name, imported, skipped });
    }

    fix_sequences(pg).await?;

    Ok(report)
}

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub sampled: u64,
    pub matching: u64,
    /// Modules whose imported attributes no longer recompute exactly —
    /// expected for old rolls when the SDE's mutaplasmid data moved
    /// since the legacy computed them.
    pub drifted: u64,
    /// Modules whose type/mutaplasmid combination is unknown to the
    /// current reference data.
    pub uncomputable: u64,
}

/// Recomputes a random sample of imported modules through our own
/// mutation math and compares against the imported attribute rows at
/// 1e-9 relative tolerance. Drift is reported, never fatal: the legacy
/// values are the historical record, our math validates the copy.
pub async fn validate_sample(
    pg: &PgPool,
    reference: &ReferenceData,
    sample: i64,
) -> sqlx::Result<ValidationReport> {
    let modules: Vec<(i64, i64, i64)> = sqlx::query_as(
        "select id, source_type_id, mutaplasmid_id from modules
         order by random() limit $1",
    )
    .bind(sample)
    .fetch_all(pg)
    .await?;

    let mut report = ValidationReport::default();
    for (module_id, source_type_id, mutaplasmid_id) in modules {
        report.sampled += 1;

        let rows: Vec<(i64, f64, f64, f64, f64, bool)> = sqlx::query_as(
            "select attribute_id, value, fraction, fraction_type, fraction_absolute,
                    is_virtual
             from mutated_attributes where module_id = $1",
        )
        .bind(module_id)
        .fetch_all(pg)
        .await?;

        let Some(context) = reference.context(mutaplasmid_id, source_type_id) else {
            report.uncomputable += 1;
            continue;
        };

        let dogma: Vec<DogmaAttribute> = rows
            .iter()
            .filter(|(.., is_virtual)| !is_virtual)
            .map(|(attribute_id, value, ..)| DogmaAttribute {
                attribute_id: *attribute_id,
                value: *value,
            })
            .collect();
        let results = calculate(&context, &dogma);

        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
        let matches = rows.iter().all(|(attribute_id, _, fraction, fraction_type, fraction_absolute, _)| {
            results.iter().any(|result| {
                result.attribute_id == *attribute_id
                    && close(result.fraction, *fraction)
                    && close(result.fraction_type, *fraction_type)
                    && close(result.fraction_absolute, *fraction_absolute)
            })
        }) && rows.len() == results.len();

        if matches {
            report.matching += 1;
        } else {
            report.drifted += 1;
        }
    }

    Ok(report)
}
