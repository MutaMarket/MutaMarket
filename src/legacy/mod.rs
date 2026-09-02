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
//! Mechanics: dump-restore speed via the Postgres bulk path. Each table
//! streams typed rows out of MySQL, gets cleaned in Rust (unsigned ids
//! cast signed, tinyint booleans, decimals to doubles, datetimes as UTC
//! strings, rows whose references are missing on our side filtered
//! against pre-loaded id sets), and lands through a single `COPY ...
//! FROM STDIN` per table. Because the Rust-side filters uphold
//! referential integrity, the per-row FK triggers are switched off
//! during the load where the connection is allowed to. Skipped rows are
//! counted and reported — never silently dropped.

use std::collections::HashSet;

use futures_util::TryStreamExt;
use sqlx::{FromRow, MySqlPool, PgPool};

use crate::mutation::calculator::{DogmaAttribute, calculate};
use crate::mutation::reference::ReferenceData;

/// COPY payload flushed to Postgres once the buffer reaches this size.
const COPY_CHUNK_BYTES: usize = 1 << 20;

/// How many modules the post-import validation recomputes through the
/// mutation math.
pub const VALIDATION_SAMPLE: i64 = 200;

/// The MySQL datetime-to-UTC-ISO rendering used by every select.
const DATE_FORMAT: &str = "'%Y-%m-%dT%H:%i:%SZ'";

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

/// The imported tables in load order, for the dry-run listing.
pub const IMPORT_TABLES: &[&str] = &[
    "users",
    "characters",
    "donations",
    "esi_tokens",
    "modules",
    "mutated_attributes",
    "historic_contracts",
    "historic_contract_items",
    "market_histories",
    "estimator_statistics",
    "collections",
    "collection_modules",
    "asset_imports",
    "characters.latest_asset_import_id",
    "assets",
    "public_assets",
    "public_assets.public_parent_id",
    "public_module_ownerships",
];

/// The domain tables the import owns, wiped before loading. Everything
/// here either comes from the legacy snapshot or is rebuilt by our own
/// jobs afterwards (contracts by the region sweep, training modules by
/// the training sweep). Referencing tables not listed are cleared by the
/// cascade.
const WIPED_TABLES: &str = "users, sessions, characters, donations, esi_tokens,
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
    "donations",
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
    "public_module_ownerships",
];

/// One line of COPY text format: tab-separated fields, `\N` for NULL,
/// backslash escapes for the delimiter characters.
struct CopyLine<'a> {
    buf: &'a mut String,
    first: bool,
}

impl<'a> CopyLine<'a> {
    fn new(buf: &'a mut String) -> Self {
        Self { buf, first: true }
    }

    fn sep(&mut self) {
        if !self.first {
            self.buf.push('\t');
        }
        self.first = false;
    }

    fn text(&mut self, value: Option<&str>) {
        self.sep();
        match value {
            None => self.buf.push_str("\\N"),
            Some(value) => {
                for ch in value.chars() {
                    match ch {
                        '\\' => self.buf.push_str("\\\\"),
                        '\t' => self.buf.push_str("\\t"),
                        '\n' => self.buf.push_str("\\n"),
                        '\r' => self.buf.push_str("\\r"),
                        _ => self.buf.push(ch),
                    }
                }
            }
        }
    }

    fn int(&mut self, value: Option<i64>) {
        self.sep();
        match value {
            None => self.buf.push_str("\\N"),
            Some(value) => self.buf.push_str(&value.to_string()),
        }
    }

    fn float(&mut self, value: Option<f64>) {
        self.sep();
        match value {
            None => self.buf.push_str("\\N"),
            // Rust's shortest-roundtrip Display keeps doubles exact.
            Some(value) => self.buf.push_str(&value.to_string()),
        }
    }

    fn boolean(&mut self, value: bool) {
        self.sep();
        self.buf.push(if value { 't' } else { 'f' });
    }

    fn end(self) {
        self.buf.push('\n');
    }
}

/// Streams one MySQL select through `encode` into a Postgres COPY. The
/// encoder returns false to skip a row (a failed reference filter);
/// imported/skipped are counted from that.
async fn copy_table<R, F>(
    mysql: &MySqlPool,
    pg: &PgPool,
    table: &'static str,
    select: &str,
    copy: &str,
    mut encode: F,
) -> sqlx::Result<TableReport>
where
    R: Send + Unpin + for<'r> FromRow<'r, sqlx::mysql::MySqlRow>,
    F: FnMut(R, &mut String) -> bool,
{
    let started = std::time::Instant::now();
    let mut connection = pg.acquire().await?;
    // A bootstrap can simply re-run after a crash, so the load skips the
    // synchronous WAL flush; and with integrity upheld by the Rust-side
    // filters, per-row FK triggers are disabled where the role allows
    // (superuser). Without the privilege the copy just runs triggered.
    sqlx::query("set synchronous_commit = off")
        .execute(&mut *connection)
        .await?;
    let replica_role = sqlx::query("set session_replication_role = replica")
        .execute(&mut *connection)
        .await
        .is_ok();

    let (mut imported, mut skipped) = (0u64, 0u64);
    let mut sink = connection.copy_in_raw(copy).await?;
    {
        let mut rows = sqlx::query_as::<_, R>(select).fetch(mysql);
        let mut buf = String::with_capacity(COPY_CHUNK_BYTES + 4096);
        while let Some(row) = rows.try_next().await? {
            if encode(row, &mut buf) {
                imported += 1;
            } else {
                skipped += 1;
            }
            if buf.len() >= COPY_CHUNK_BYTES {
                if let Err(error) = sink.send(buf.as_bytes()).await {
                    sink.abort("copy failed").await.ok();
                    return Err(error);
                }
                buf.clear();
            }
        }
        if !buf.is_empty()
            && let Err(error) = sink.send(buf.as_bytes()).await
        {
            sink.abort("copy failed").await.ok();
            return Err(error);
        }
    }
    sink.finish().await?;

    if replica_role {
        sqlx::query("set session_replication_role = origin")
            .execute(&mut *connection)
            .await?;
    }

    println!(
        "  {table}: {imported} imported, {skipped} skipped ({:.1?})",
        started.elapsed()
    );
    Ok(TableReport {
        table,
        imported,
        skipped,
    })
}

/// A second-pass pointer update: (id, target) pairs already filtered.
async fn update_pairs(
    pg: &PgPool,
    table: &'static str,
    sql: &str,
    pairs: Vec<(i64, i64)>,
    skipped: u64,
) -> sqlx::Result<TableReport> {
    let started = std::time::Instant::now();
    let imported = sqlx::query(sql)
        .bind(pairs.iter().map(|(id, _)| *id).collect::<Vec<_>>())
        .bind(pairs.iter().map(|(_, target)| *target).collect::<Vec<_>>())
        .execute(pg)
        .await?
        .rows_affected();
    println!(
        "  {table}: {imported} imported, {skipped} skipped ({:.1?})",
        started.elapsed()
    );
    Ok(TableReport {
        table,
        imported,
        skipped,
    })
}

async fn id_set(pg: &PgPool, sql: &str) -> sqlx::Result<HashSet<i64>> {
    Ok(sqlx::query_scalar::<_, i64>(sql)
        .fetch_all(pg)
        .await?
        .into_iter()
        .collect())
}

/// Wipes the domain tables the import owns. Reference/SDE and scheduler
/// tables stay untouched.
pub async fn wipe_domain_tables(pg: &PgPool) -> sqlx::Result<()> {
    sqlx::query(&format!("truncate table {WIPED_TABLES} cascade"))
        .execute(pg)
        .await?;
    Ok(())
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

#[derive(FromRow)]
struct UserRow {
    id: i64,
    name: Option<String>,
    is_admin: Option<i64>,
    discord_id: Option<i64>,
    discord_name: Option<String>,
    discord_avatar: Option<String>,
    discord_channel_id: Option<i64>,
    twitch_id: Option<i64>,
    twitch_name: Option<String>,
    twitch_avatar: Option<String>,
    twitch_email: Option<String>,
    patreon_id: Option<i64>,
    patreon_name: Option<String>,
    patreon_avatar: Option<String>,
    patreon_email: Option<String>,
    patreon_nickname: Option<String>,
    is_patreon_member: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct CharacterRow {
    id: i64,
    name: Option<String>,
    corporation_id: Option<i64>,
    alliance_id: Option<i64>,
    user_id: Option<i64>,
    character_owner_hash: Option<String>,
    description: Option<String>,
    premium_paid_until: Option<String>,
    premium_paid_total: Option<f64>,
    premium_payment_rest: Option<f64>,
    name_fetched_at: Option<String>,
    contracts_fetched_at: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct DonationRow {
    id: i64,
    character_id: i64,
    journal_id: Option<i64>,
    amount: Option<f64>,
    date: Option<String>,
    confirmation_sent: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct TokenRow {
    id: i64,
    character_id: i64,
    access_token: Option<String>,
    refresh_token: Option<String>,
    token_type: Option<String>,
    character_owner_hash: Option<String>,
    scopes: Option<String>,
    expires_at: Option<String>,
    created_at: Option<String>,
}

#[derive(FromRow)]
struct ModuleRow {
    id: i64,
    type_id: i64,
    source_type_id: i64,
    mutaplasmid_id: i64,
    creator_id: Option<i64>,
    estimated_value: Option<f64>,
    estimated_value_updated_at: Option<String>,
    average_fraction: Option<f64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct AttributeRow {
    id: i64,
    module_id: i64,
    attribute_id: i64,
    type_id: i64,
    value: f64,
    base_value: f64,
    fraction: f64,
    fraction_type: f64,
    fraction_absolute: f64,
    bar: i64,
    is_virtual: i64,
}

#[derive(FromRow)]
struct HistoricContractRow {
    id: i64,
    status: Option<String>,
    region_id: i64,
    start_location_id: Option<i64>,
    issuer_id: i64,
    issuer_corporation_id: Option<i64>,
    for_corporation: Option<i64>,
    contract_type: Option<String>,
    title: Option<String>,
    date_issued: Option<String>,
    date_expired: Option<String>,
    price: Option<f64>,
    buyout: Option<f64>,
    highest_bid: Option<f64>,
    unified_price: Option<f64>,
    asking_for_items: Option<i64>,
    abyssal_modules_count: i64,
    non_abyssal_modules_count: i64,
    plex_count: i64,
    ignore_for_training: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct HistoricItemRow {
    id: i64,
    historic_contract_id: i64,
    record_id: i64,
    type_id: i64,
    item_id: i64,
}

#[derive(FromRow)]
struct MarketHistoryRow {
    id: i64,
    type_id: i64,
    region_id: i64,
    date: Option<String>,
    average: f64,
    highest: f64,
    lowest: f64,
    order_count: i64,
    volume: i64,
}

#[derive(FromRow)]
struct EstimatorStatisticRow {
    id: i64,
    type_id: i64,
    name: Option<String>,
    data_count: i64,
    r2: Option<f64>,
    mae: Option<f64>,
    nmae: Option<f64>,
    last_trained_at: Option<String>,
    data_statistics: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct CollectionRow {
    id: i64,
    identifier: Option<String>,
    name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    character_id: i64,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct CollectionModuleRow {
    id: i64,
    collection_id: i64,
    module_id: i64,
    note: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct AssetImportRow {
    id: i64,
    character_id: i64,
    status: Option<String>,
    step: Option<String>,
    assets_count: Option<i64>,
    assets_corporation_count: Option<i64>,
    abyssal_modules_count: Option<i64>,
    abyssal_modules_imported_count: Option<i64>,
    abyssal_modules_failed_count: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct AssetRow {
    id: i64,
    character_id: i64,
    corporation_id: Option<i64>,
    item_id: i64,
    type_id: i64,
    name: Option<String>,
    location_id: Option<i64>,
    location_flag: Option<String>,
    location_type: Option<String>,
    quantity: Option<i64>,
    item_index: Option<i64>,
    is_abyssal: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct OwnershipRow {
    id: i64,
    character_id: i64,
    module_id: i64,
    public_asset_id: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(FromRow)]
struct PublicAssetRow {
    id: i64,
    character_id: i64,
    asset_id: i64,
    module_id: Option<i64>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

/// Runs the whole import: wipe, every table in FK order through the COPY
/// path, the two pointer passes, sequence fixes.
pub async fn run_import(mysql: &MySqlPool, pg: &PgPool) -> sqlx::Result<ImportReport> {
    wipe_domain_tables(pg).await?;

    // The reference side of the filters: ids owned by the SDE import.
    let types = id_set(pg, "select id from types").await?;
    let mutaplasmids = id_set(pg, "select id from mutaplasmids").await?;
    let attributes = id_set(pg, "select id from attributes").await?;

    // Every NOT NULL timestamp falls back to one shared import stamp.
    let now: String = sqlx::query_scalar("select now()::text")
        .fetch_one(pg)
        .await?;
    let now = now.as_str();
    let ts = |line: &mut CopyLine, value: &Option<String>| {
        line.text(Some(value.as_deref().unwrap_or(now)));
    };

    let mut report = ImportReport::default();

    let mut users = HashSet::new();
    report.tables.push(
        copy_table::<UserRow, _>(
            mysql,
            pg,
            "users",
            &format!(
                "select cast(id as signed) as id, name, cast(is_admin as signed) as is_admin,
                        cast(discord_id as signed) as discord_id, discord_name, discord_avatar,
                        cast(discord_channel_id as signed) as discord_channel_id,
                        cast(twitch_id as signed) as twitch_id, twitch_name, twitch_avatar,
                        twitch_email, cast(patreon_id as signed) as patreon_id, patreon_name,
                        patreon_avatar, patreon_email, patreon_nickname,
                        cast(is_patreon_member as signed) as is_patreon_member,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from users",
            ),
            "copy users (id, name, is_admin, discord_id, discord_name, discord_avatar,
                 discord_channel_id, twitch_id, twitch_name, twitch_avatar, twitch_email,
                 patreon_id, patreon_name, patreon_avatar, patreon_email, patreon_nickname,
                 is_patreon_member, created_at, updated_at) from stdin",
            |row, buf| {
                users.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.text(Some(row.name.as_deref().unwrap_or("")));
                line.boolean(row.is_admin.unwrap_or(0) != 0);
                line.int(row.discord_id);
                line.text(row.discord_name.as_deref());
                line.text(row.discord_avatar.as_deref());
                line.int(row.discord_channel_id);
                line.int(row.twitch_id);
                line.text(row.twitch_name.as_deref());
                line.text(row.twitch_avatar.as_deref());
                line.text(row.twitch_email.as_deref());
                line.int(row.patreon_id);
                line.text(row.patreon_name.as_deref());
                line.text(row.patreon_avatar.as_deref());
                line.text(row.patreon_email.as_deref());
                line.text(row.patreon_nickname.as_deref());
                line.boolean(row.is_patreon_member.unwrap_or(0) != 0);
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut characters = HashSet::new();
    report.tables.push(
        copy_table::<CharacterRow, _>(
            mysql,
            pg,
            "characters",
            &format!(
                "select cast(id as signed) as id, name,
                        cast(corporation_id as signed) as corporation_id,
                        cast(alliance_id as signed) as alliance_id,
                        cast(user_id as signed) as user_id, character_owner_hash, description,
                        date_format(premium_paid_until, {DATE_FORMAT}) as premium_paid_until,
                        cast(premium_paid_total as double) as premium_paid_total,
                        cast(premium_payment_rest as double) as premium_payment_rest,
                        date_format(name_fetched_at, {DATE_FORMAT}) as name_fetched_at,
                        date_format(contracts_fetched_at, {DATE_FORMAT}) as contracts_fetched_at,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from characters",
            ),
            "copy characters (id, name, corporation_id, alliance_id, user_id,
                 character_owner_hash, description, premium_paid_until, premium_paid_total,
                 premium_payment_rest, name_fetched_at,
                 contracts_fetched_at, created_at, updated_at) from stdin",
            |row, buf| {
                characters.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.text(Some(row.name.as_deref().unwrap_or("")));
                line.int(row.corporation_id);
                line.int(row.alliance_id);
                // Unknown user links are nulled, not skipped.
                line.int(row.user_id.filter(|user| users.contains(user)));
                line.text(row.character_owner_hash.as_deref());
                line.text(row.description.as_deref());
                line.text(row.premium_paid_until.as_deref());
                line.float(Some(row.premium_paid_total.unwrap_or(0.0)));
                line.float(Some(row.premium_payment_rest.unwrap_or(0.0)));
                line.text(row.name_fetched_at.as_deref());
                line.text(row.contracts_fetched_at.as_deref());
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<DonationRow, _>(
            mysql,
            pg,
            "donations",
            &format!(
                "select cast(id as signed) as id,
                        cast(character_id as signed) as character_id,
                        cast(journal_id as signed) as journal_id,
                        cast(amount as double) as amount,
                        date_format(date, {DATE_FORMAT}) as date,
                        cast(confirmation_sent as signed) as confirmation_sent,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from donations",
            ),
            "copy donations (id, character_id, journal_id, amount, date, confirmation_sent,
                 created_at, updated_at) from stdin",
            |row, buf| {
                // Donations of characters missing from the snapshot are
                // skipped (the FK would reject them).
                if !characters.contains(&row.character_id) {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.int(row.journal_id);
                line.float(Some(row.amount.unwrap_or(0.0)));
                ts(&mut line, &row.date);
                line.boolean(row.confirmation_sent.unwrap_or(0) != 0);
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<TokenRow, _>(
            mysql,
            pg,
            "esi_tokens",
            &format!(
                "select cast(t.id as signed) as id,
                        cast(t.character_id as signed) as character_id,
                        t.access_token, t.refresh_token, t.token_type, t.character_owner_hash,
                        (select group_concat(s.name separator ',')
                         from esi_token_scope ts
                         join esi_scopes s on s.id = ts.esi_scope_id
                         where ts.esi_token_id = t.id) as scopes,
                        date_format(t.expires_at, {DATE_FORMAT}) as expires_at,
                        date_format(t.created_at, {DATE_FORMAT}) as created_at
                 from esi_tokens t",
            ),
            "copy esi_tokens (id, character_id, access_token, refresh_token, token_type,
                 character_owner_hash, scopes, expires_at, created_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.text(Some(row.access_token.as_deref().unwrap_or("")));
                line.text(Some(row.refresh_token.as_deref().unwrap_or("")));
                line.text(Some(row.token_type.as_deref().unwrap_or("Bearer")));
                line.text(Some(row.character_owner_hash.as_deref().unwrap_or("")));
                // Scope names are plain identifiers, so the array
                // literal needs no quoting.
                line.text(Some(&format!(
                    "{{{}}}",
                    row.scopes.as_deref().unwrap_or("")
                )));
                ts(&mut line, &row.expires_at);
                ts(&mut line, &row.created_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut modules = HashSet::new();
    report.tables.push(
        copy_table::<ModuleRow, _>(
            mysql,
            pg,
            "modules",
            &format!(
                "select cast(id as signed) as id, cast(type_id as signed) as type_id,
                        cast(source_type_id as signed) as source_type_id,
                        cast(mutaplasmid_id as signed) as mutaplasmid_id,
                        cast(creator_id as signed) as creator_id,
                        cast(estimated_value as double) as estimated_value,
                        date_format(estimated_value_updated_at, {DATE_FORMAT})
                            as estimated_value_updated_at,
                        average_fraction,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from modules",
            ),
            // The live contract link is deliberately dropped: the
            // snapshot's market is stale, the region sweep relinks.
            "copy modules (id, type_id, source_type_id, mutaplasmid_id, creator_id,
                 estimated_value, estimated_value_updated_at, average_fraction,
                 created_at, updated_at) from stdin",
            |row, buf| {
                if !types.contains(&row.type_id)
                    || !types.contains(&row.source_type_id)
                    || !mutaplasmids.contains(&row.mutaplasmid_id)
                {
                    return false;
                }
                modules.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.type_id));
                line.int(Some(row.source_type_id));
                line.int(Some(row.mutaplasmid_id));
                line.int(
                    row.creator_id
                        .filter(|creator| characters.contains(creator)),
                );
                line.float(row.estimated_value);
                line.text(row.estimated_value_updated_at.as_deref());
                line.float(row.average_fraction);
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<AttributeRow, _>(
            mysql,
            pg,
            "mutated_attributes",
            "select cast(id as signed) as id, cast(module_id as signed) as module_id,
                    cast(attribute_id as signed) as attribute_id,
                    cast(type_id as signed) as type_id, value, base_value, fraction,
                    fraction_type, fraction_absolute, cast(bar as signed) as bar,
                    cast(is_virtual as signed) as is_virtual
             from mutated_attributes",
            "copy mutated_attributes (id, module_id, attribute_id, type_id, value,
                 base_value, fraction, fraction_type, fraction_absolute, bar, is_virtual)
             from stdin",
            |row, buf| {
                if !modules.contains(&row.module_id)
                    || !attributes.contains(&row.attribute_id)
                    || !types.contains(&row.type_id)
                {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.module_id));
                line.int(Some(row.attribute_id));
                line.int(Some(row.type_id));
                line.float(Some(row.value));
                line.float(Some(row.base_value));
                line.float(Some(row.fraction));
                line.float(Some(row.fraction_type));
                line.float(Some(row.fraction_absolute));
                line.int(Some(row.bar));
                line.boolean(row.is_virtual != 0);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut historic = HashSet::new();
    report.tables.push(
        copy_table::<HistoricContractRow, _>(
            mysql,
            pg,
            "historic_contracts",
            &format!(
                "select cast(id as signed) as id, status,
                        cast(region_id as signed) as region_id,
                        cast(start_location_id as signed) as start_location_id,
                        cast(issuer_id as signed) as issuer_id,
                        cast(issuer_corporation_id as signed) as issuer_corporation_id,
                        cast(for_corporation as signed) as for_corporation,
                        type as contract_type, title,
                        date_format(date_issued, {DATE_FORMAT}) as date_issued,
                        date_format(date_expired, {DATE_FORMAT}) as date_expired,
                        price, buyout, highest_bid, unified_price,
                        cast(asking_for_items as signed) as asking_for_items,
                        cast(abyssal_modules_count as signed) as abyssal_modules_count,
                        cast(non_abyssal_modules_count as signed) as non_abyssal_modules_count,
                        cast(plex_count as signed) as plex_count,
                        cast(ignore_for_training as signed) as ignore_for_training,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from historic_contracts",
            ),
            "copy historic_contracts (id, status, region_id, start_location_id, issuer_id,
                 issuer_corporation_id, for_corporation, type, title, date_issued,
                 date_expired, price, buyout, highest_bid, unified_price, asking_for_items,
                 abyssal_modules_count, non_abyssal_modules_count, plex_count,
                 ignore_for_training, created_at, updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.issuer_id) {
                    return false;
                }
                historic.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.text(Some(row.status.as_deref().unwrap_or("unknown")));
                line.int(Some(row.region_id));
                line.int(row.start_location_id);
                line.int(Some(row.issuer_id));
                line.int(row.issuer_corporation_id);
                line.boolean(row.for_corporation.unwrap_or(0) != 0);
                line.text(Some(
                    row.contract_type.as_deref().unwrap_or("item_exchange"),
                ));
                line.text(row.title.as_deref());
                line.text(row.date_issued.as_deref());
                line.text(row.date_expired.as_deref());
                line.float(row.price);
                line.float(row.buyout);
                line.float(row.highest_bid);
                line.float(row.unified_price);
                line.boolean(row.asking_for_items.unwrap_or(0) != 0);
                line.int(Some(row.abyssal_modules_count));
                line.int(Some(row.non_abyssal_modules_count));
                line.int(Some(row.plex_count));
                line.boolean(row.ignore_for_training.unwrap_or(0) != 0);
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<HistoricItemRow, _>(
            mysql,
            pg,
            "historic_contract_items",
            "select cast(id as signed) as id,
                    cast(historic_contract_id as signed) as historic_contract_id,
                    cast(record_id as signed) as record_id,
                    cast(type_id as signed) as type_id, cast(item_id as signed) as item_id
             from historic_contract_items",
            "copy historic_contract_items (id, historic_contract_id, record_id, type_id,
                 item_id) from stdin",
            |row, buf| {
                if !historic.contains(&row.historic_contract_id) || !types.contains(&row.type_id) {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.historic_contract_id));
                line.int(Some(row.record_id));
                line.int(Some(row.type_id));
                line.int(Some(row.item_id));
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<MarketHistoryRow, _>(
            mysql,
            pg,
            "market_histories",
            "select cast(id as signed) as id, cast(type_id as signed) as type_id,
                    cast(region_id as signed) as region_id,
                    date_format(date, '%Y-%m-%d') as date,
                    cast(average as double) as average, cast(highest as double) as highest,
                    cast(lowest as double) as lowest,
                    cast(order_count as signed) as order_count,
                    cast(volume as signed) as volume
             from market_histories",
            "copy market_histories (id, type_id, region_id, date, average, highest, lowest,
                 order_count, volume) from stdin",
            |row, buf| {
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.type_id));
                line.int(Some(row.region_id));
                line.text(row.date.as_deref());
                line.float(Some(row.average));
                line.float(Some(row.highest));
                line.float(Some(row.lowest));
                line.int(Some(row.order_count));
                line.int(Some(row.volume));
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<EstimatorStatisticRow, _>(
            mysql,
            pg,
            "estimator_statistics",
            &format!(
                "select cast(id as signed) as id, cast(type_id as signed) as type_id, name,
                        cast(data_count as signed) as data_count, r2, mae, nmae,
                        date_format(last_trained_at, {DATE_FORMAT}) as last_trained_at,
                        cast(data_statistics as char) as data_statistics,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from estimator_statistics",
            ),
            "copy estimator_statistics (id, type_id, name, data_count, r2, mae, nmae,
                 last_trained_at, data_statistics, created_at, updated_at) from stdin",
            |row, buf| {
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.type_id));
                line.text(Some(row.name.as_deref().unwrap_or("")));
                line.int(Some(row.data_count));
                line.float(row.r2);
                line.float(row.mae);
                line.float(row.nmae);
                line.text(row.last_trained_at.as_deref());
                line.text(row.data_statistics.as_deref());
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut collections = HashSet::new();
    report.tables.push(
        copy_table::<CollectionRow, _>(
            mysql,
            pg,
            "collections",
            &format!(
                "select cast(id as signed) as id, identifier, name, description, visibility,
                        cast(character_id as signed) as character_id,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from collections",
            ),
            "copy collections (id, identifier, name, description, visibility, character_id,
                 created_at, updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) {
                    return false;
                }
                collections.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.text(Some(row.identifier.as_deref().unwrap_or("")));
                line.text(Some(row.name.as_deref().unwrap_or("")));
                line.text(row.description.as_deref());
                line.text(Some(row.visibility.as_deref().unwrap_or("private")));
                line.int(Some(row.character_id));
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    report.tables.push(
        copy_table::<CollectionModuleRow, _>(
            mysql,
            pg,
            "collection_modules",
            &format!(
                "select cast(id as signed) as id, cast(collection_id as signed) as collection_id,
                        cast(module_id as signed) as module_id, note,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from collection_modules",
            ),
            "copy collection_modules (id, collection_id, module_id, note, created_at,
                 updated_at) from stdin",
            |row, buf| {
                if !collections.contains(&row.collection_id) || !modules.contains(&row.module_id) {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.collection_id));
                line.int(Some(row.module_id));
                line.text(row.note.as_deref());
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut asset_imports = HashSet::new();
    report.tables.push(
        copy_table::<AssetImportRow, _>(
            mysql,
            pg,
            "asset_imports",
            &format!(
                "select cast(id as signed) as id, cast(character_id as signed) as character_id,
                        status, step, cast(assets_count as signed) as assets_count,
                        cast(assets_corporation_count as signed) as assets_corporation_count,
                        cast(abyssal_modules_count as signed) as abyssal_modules_count,
                        cast(abyssal_modules_imported_count as signed)
                            as abyssal_modules_imported_count,
                        cast(abyssal_modules_failed_count as signed)
                            as abyssal_modules_failed_count,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from asset_imports",
            ),
            "copy asset_imports (id, character_id, status, step, assets_count,
                 assets_corporation_count, abyssal_modules_count,
                 abyssal_modules_imported_count, abyssal_modules_failed_count,
                 created_at, updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) {
                    return false;
                }
                asset_imports.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.text(Some(row.status.as_deref().unwrap_or("")));
                line.text(Some(row.step.as_deref().unwrap_or("")));
                line.int(Some(row.assets_count.unwrap_or(0)));
                line.int(Some(row.assets_corporation_count.unwrap_or(0)));
                line.int(Some(row.abyssal_modules_count.unwrap_or(0)));
                line.int(Some(row.abyssal_modules_imported_count.unwrap_or(0)));
                line.int(Some(row.abyssal_modules_failed_count.unwrap_or(0)));
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    // Second pass over characters: the latest-import pointer can only
    // land after asset_imports exists (circular FK pair).
    let pointer_rows: Vec<(i64, i64)> = sqlx::query_as(
        "select cast(id as signed), cast(latest_asset_import_id as signed)
         from characters where latest_asset_import_id is not null",
    )
    .fetch_all(mysql)
    .await?;
    let total = pointer_rows.len() as u64;
    let pairs: Vec<(i64, i64)> = pointer_rows
        .into_iter()
        .filter(|(_, import)| asset_imports.contains(import))
        .collect();
    let skipped = total - pairs.len() as u64;
    report.tables.push(
        update_pairs(
            pg,
            "characters.latest_asset_import_id",
            "update characters c set latest_asset_import_id = t.import
             from unnest($1::bigint[], $2::bigint[]) as t(id, import)
             where c.id = t.id",
            pairs,
            skipped,
        )
        .await?,
    );

    let mut assets = HashSet::new();
    report.tables.push(
        copy_table::<AssetRow, _>(
            mysql,
            pg,
            "assets",
            &format!(
                "select cast(id as signed) as id, cast(character_id as signed) as character_id,
                        cast(corporation_id as signed) as corporation_id,
                        cast(item_id as signed) as item_id, cast(type_id as signed) as type_id,
                        name, cast(location_id as signed) as location_id, location_flag,
                        location_type, cast(quantity as signed) as quantity,
                        cast(`index` as signed) as item_index,
                        cast(is_abyssal as signed) as is_abyssal,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from assets",
            ),
            "copy assets (id, character_id, corporation_id, item_id, type_id, name,
                 location_id, location_flag, location_type, quantity, \"index\", is_abyssal,
                 created_at, updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) {
                    return false;
                }
                assets.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.int(row.corporation_id);
                line.int(Some(row.item_id));
                line.int(Some(row.type_id));
                line.text(row.name.as_deref());
                line.int(Some(row.location_id.unwrap_or(0)));
                line.text(Some(row.location_flag.as_deref().unwrap_or("")));
                line.text(Some(row.location_type.as_deref().unwrap_or("")));
                line.int(Some(row.quantity.unwrap_or(1)));
                line.int(Some(row.item_index.unwrap_or(0)));
                line.boolean(row.is_abyssal.unwrap_or(0) != 0);
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let mut public_assets = HashSet::new();
    report.tables.push(
        copy_table::<PublicAssetRow, _>(
            mysql,
            pg,
            "public_assets",
            &format!(
                "select cast(id as signed) as id, cast(character_id as signed) as character_id,
                        cast(asset_id as signed) as asset_id,
                        cast(module_id as signed) as module_id,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from public_assets",
            ),
            // The self-referencing parent pointer lands in the second
            // pass below, once every row exists.
            "copy public_assets (id, character_id, asset_id, module_id, created_at,
                 updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) || !assets.contains(&row.asset_id) {
                    return false;
                }
                public_assets.insert(row.id);
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.int(Some(row.asset_id));
                line.int(row.module_id.filter(|module| modules.contains(module)));
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    let parent_rows: Vec<(i64, i64)> = sqlx::query_as(
        "select cast(id as signed), cast(public_parent_id as signed)
         from public_assets where public_parent_id is not null",
    )
    .fetch_all(mysql)
    .await?;
    let total = parent_rows.len() as u64;
    let pairs: Vec<(i64, i64)> = parent_rows
        .into_iter()
        .filter(|(id, parent)| public_assets.contains(id) && public_assets.contains(parent))
        .collect();
    let skipped = total - pairs.len() as u64;
    report.tables.push(
        update_pairs(
            pg,
            "public_assets.public_parent_id",
            "update public_assets p set public_parent_id = t.parent
             from unnest($1::bigint[], $2::bigint[]) as t(id, parent)
             where p.id = t.id",
            pairs,
            skipped,
        )
        .await?,
    );

    let mut seen_ownerships: HashSet<(i64, i64)> = HashSet::new();
    report.tables.push(
        copy_table::<OwnershipRow, _>(
            mysql,
            pg,
            "public_module_ownerships",
            &format!(
                "select cast(id as signed) as id,
                        cast(character_id as signed) as character_id,
                        cast(module_id as signed) as module_id,
                        cast(public_asset_id as signed) as public_asset_id,
                        date_format(created_at, {DATE_FORMAT}) as created_at,
                        date_format(updated_at, {DATE_FORMAT}) as updated_at
                 from public_module_ownerships",
            ),
            // The contract link is deliberately dropped like the module
            // one: the snapshot's live market is stale. Both links
            // cascade on delete in the schema, so a row whose only link
            // was that contract (or a skipped public asset) is dropped
            // with it; the region sweep recreates the rows of contracts
            // still live.
            "copy public_module_ownerships (id, character_id, module_id, public_asset_id,
                 created_at, updated_at) from stdin",
            |row, buf| {
                if !characters.contains(&row.character_id) || !modules.contains(&row.module_id) {
                    return false;
                }
                let Some(public_asset_id) = row
                    .public_asset_id
                    .filter(|asset| public_assets.contains(asset))
                else {
                    return false;
                };
                // Legacy tolerated duplicate (character, module) pairs;
                // our unique constraint keeps the first occurrence.
                if !seen_ownerships.insert((row.character_id, row.module_id)) {
                    return false;
                }
                let mut line = CopyLine::new(buf);
                line.int(Some(row.id));
                line.int(Some(row.character_id));
                line.int(Some(row.module_id));
                line.int(Some(public_asset_id));
                ts(&mut line, &row.created_at);
                ts(&mut line, &row.updated_at);
                line.end();
                true
            },
        )
        .await?,
    );

    fix_sequences(pg).await?;

    // Fresh planner statistics: autovacuum takes a while to catch up
    // with 15M new rows, and stale stats make every query slow until it
    // does.
    sqlx::query("analyze").execute(pg).await?;

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
        let matches = rows.iter().all(
            |(attribute_id, _, fraction, fraction_type, fraction_absolute, _)| {
                results.iter().any(|result| {
                    result.attribute_id == *attribute_id
                        && close(result.fraction, *fraction)
                        && close(result.fraction_type, *fraction_type)
                        && close(result.fraction_absolute, *fraction_absolute)
                })
            },
        ) && rows.len() == results.len();

        if matches {
            report.matching += 1;
        } else {
            report.drifted += 1;
        }
    }

    Ok(report)
}
