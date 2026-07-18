-- Character asset ingestion, ported from the legacy assets tables: the
-- stored subset of a character's assets (abyssal modules plus the
-- container chain around them) and the per-run import state machine that
-- makes the multi-step ESI fetch observable and crash-recoverable.

create table asset_imports (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    -- pending -> processing -> completed | failed (legacy AssetImportStatus).
    status text not null default 'pending',
    -- The legacy AssetImportStep values, advanced as the fetch progresses.
    step text not null default 'fetching_assets',
    assets_count integer not null default 0,
    assets_corporation_count integer not null default 0,
    abyssal_modules_count integer not null default 0,
    abyssal_modules_imported_count integer not null default 0,
    abyssal_modules_failed_count integer not null default 0,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index asset_imports_character_id_index on asset_imports (character_id);

-- The character's most recent import drives the oldest-first fan-out
-- ordering, like the legacy characters.latest_asset_import_id.
alter table characters
    add column latest_asset_import_id bigint references asset_imports (id) on delete set null;

create table assets (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    -- Set when the item lives in the corporation hangars (fetched with the
    -- corporation assets scope); personal assets carry null.
    corporation_id bigint,
    item_id bigint not null,
    -- No FK to types: the minimal SDE import does not carry every ship and
    -- container type an asset can be.
    type_id bigint not null,
    -- The player-given name (ships, containers), from the asset names
    -- endpoint.
    name text,
    location_id bigint,
    location_flag text not null,
    location_type text not null,
    quantity bigint not null,
    -- Tree traversal order, like the legacy assets.index: containers come
    -- before their contents.
    index bigint not null default 0,
    is_abyssal boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (character_id, item_id)
);

create index assets_location_id_index on assets (location_id);
create index assets_character_location_index on assets (character_id, item_id, location_id);
