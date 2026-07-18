-- Collections and public module ownerships, ported from the legacy
-- collections / collection_modules / public_module_ownerships tables
-- (minimal columns the ported features use).

create table collections (
    id bigserial primary key,
    -- Random lowercase identifier; the URL slug is {name-slug}-{identifier}
    -- and route binding matches the trailing segment, like legacy.
    identifier text not null unique,
    name text not null,
    description text,
    -- 'public' | 'private' | 'unlisted' (legacy CollectionVisibility).
    visibility text not null,
    character_id bigint not null references characters (id),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index collections_visibility_index on collections (visibility);

create table collection_modules (
    id bigserial primary key,
    collection_id bigint not null references collections (id) on delete cascade,
    module_id bigint not null references modules (id) on delete cascade,
    note text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (collection_id, module_id)
);

-- A character's publicly listed ownership of a module, fed by published
-- assets and contracts. module_id carries no foreign key, mirroring the
-- legacy migration (modules may be re-ingested independently).
create table public_module_ownerships (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    module_id bigint not null,
    -- References public_assets once the assets milestone lands.
    public_asset_id bigint,
    contract_id bigint references contracts (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (character_id, module_id)
);

create index public_module_ownerships_module_index
    on public_module_ownerships (module_id, character_id, public_asset_id, contract_id);
