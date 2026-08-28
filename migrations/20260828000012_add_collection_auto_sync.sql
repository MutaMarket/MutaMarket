-- Collection locations and auto-sync, ported from the legacy
-- 2026_01_04_103603_add_auto_sync_to_collections_table and
-- 2026_01_04_103603_create_collection_locations_table migrations: a
-- collection can bulk-manage its modules per asset location, and an
-- auto-sync collection tracks locations and rebuilds its modules from
-- them after each asset import.

alter table collections
    add column auto_sync boolean not null default false,
    add column last_synced_at timestamptz;

create table collection_locations (
    id bigserial primary key,
    collection_id bigint not null references collections (id) on delete cascade,
    asset_id bigint not null references assets (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (collection_id, asset_id)
);
