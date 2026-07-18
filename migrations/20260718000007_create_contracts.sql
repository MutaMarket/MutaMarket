-- Public contract ingestion: regions (fan-out targets from the SDE),
-- contracts with their classification stats and unified price, the abyssal
-- items linking contracts to modules, per-run import bookkeeping, and
-- market histories (PLEX average feeds the unified price).

create table regions (
    id bigint primary key,
    name text not null default ''
);

create table contracts (
    id bigint primary key,
    region_id bigint not null references regions (id),
    start_location_id bigint,
    issuer_id bigint not null references characters (id),
    issuer_corporation_id bigint,
    for_corporation boolean not null default false,
    -- ESI contract type: auction or item_exchange (others are irrelevant).
    type text not null,
    title text,
    date_issued timestamptz,
    date_expired timestamptz,
    price double precision,
    buyout double precision,
    highest_bid double precision,
    -- Auction and item-exchange prices normalized (PLEX included).
    unified_price double precision,
    asking_for_items boolean not null default false,
    abyssal_modules_count integer not null default 0,
    non_abyssal_modules_count integer not null default 0,
    plex_count integer not null default 0,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index contracts_region_id_index on contracts (region_id);
create index contracts_type_index on contracts (type);

-- Only the abyssal module items of a contract are stored.
create table contract_items (
    id bigserial primary key,
    contract_id bigint not null references contracts (id) on delete cascade,
    record_id bigint not null,
    type_id bigint not null references types (id),
    item_id bigint not null references modules (id),
    unique (contract_id, record_id)
);

create table contract_imports (
    id bigserial primary key,
    region_id bigint not null references regions (id),
    contracts_total_count integer not null default 0,
    contracts_invalidated_count integer not null default 0,
    expires_at timestamptz,
    created_at timestamptz not null default now()
);

create table market_histories (
    id bigserial primary key,
    type_id bigint not null references types (id),
    region_id bigint not null references regions (id),
    date date not null,
    average double precision not null,
    highest double precision not null,
    lowest double precision not null,
    order_count bigint not null default 0,
    volume bigint not null default 0,
    unique (type_id, region_id, date)
);

alter table modules
    add column latest_contract_id bigint references contracts (id) on delete set null;
