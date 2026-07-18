-- A logged-in character's personal contracts from ESI, ported from the
-- legacy character_contracts + character_contract_items tables. Personal
-- contracts live apart from the public contracts table; the availability
-- column (public/personal/corporation/alliance) is what marks a contract
-- as private, there is no boolean flag. Rows are only ever upserted,
-- never pruned, like the legacy import. (The legacy last_fetched_at
-- column is not ported: nothing ever wrote it.)

create table character_contracts (
    id bigint primary key,
    issuer_id bigint not null references characters (id),
    issuer_corporation_id bigint,
    for_corporation boolean not null default false,
    -- ESI contract type: auction, item_exchange, courier, ...
    type text not null,
    title text,
    date_issued timestamptz,
    date_expired timestamptz,
    price double precision,
    buyout double precision,
    acceptor_id bigint,
    -- character/corporation/alliance, resolved via universe names.
    acceptor_type text default 'character',
    assignee_id bigint,
    -- public, personal, corporation, alliance or unknown.
    availability text not null,
    date_accepted timestamptz,
    date_completed timestamptz,
    -- The raw ESI status string (outstanding, finished, deleted, ...).
    status text not null,
    volume double precision,
    highest_bid double precision,
    unified_price double precision,
    asking_for_items boolean not null default false,
    abyssal_modules_count integer not null default 0,
    non_abyssal_modules_count integer not null default 0,
    plex_count integer not null default 0,
    -- Crash-safe item ingestion (a divergence like the public contracts
    -- one): items are owed until this is set, so the fetch is derived
    -- from domain state instead of the legacy new-ids diff.
    items_synced_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index character_contracts_availability_index on character_contracts (availability);
create index character_contracts_issuer_index on character_contracts (id, issuer_id);

-- Only the abyssal module items of a contract are stored; the character
-- items endpoint carries no item ids, so unlike public contract items
-- there is no module link.
create table character_contract_items (
    id bigserial primary key,
    character_contract_id bigint not null references character_contracts (id) on delete cascade,
    type_id bigint not null references types (id),
    record_id bigint not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (character_contract_id, record_id)
);

-- Oldest-fetched-first fan-out ordering, like the legacy
-- characters.contracts_fetched_at.
alter table characters
    add column contracts_fetched_at timestamptz;
