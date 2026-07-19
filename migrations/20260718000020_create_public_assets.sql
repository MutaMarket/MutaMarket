-- Published (public) assets: a user can make an owned asset and its whole
-- descendant subtree public, exposing the abyssal modules within on the
-- character page. Ported from the legacy public_assets table + the
-- after_public_asset trigger that maintains public_module_ownerships.

create table public_assets (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    asset_id bigint not null references assets (id) on delete cascade,
    -- The top published asset this row descends from (null for the root).
    public_parent_id bigint references public_assets (id) on delete cascade,
    -- Set when the asset is an abyssal module.
    module_id bigint,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (character_id, asset_id)
);

create index public_assets_module_index on public_assets (module_id);

-- Now that public_assets exists, wire the ownership FK the collections
-- migration deferred.
alter table public_module_ownerships
    add constraint public_module_ownerships_public_asset_fk
    foreign key (public_asset_id) references public_assets (id) on delete cascade;
