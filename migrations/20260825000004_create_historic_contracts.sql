-- Contracts that vanished from the public feed, archived with their
-- final status (the legacy historic_contracts: contract-history tabs and
-- the estimator's training-data source). Items are copied so modules can
-- be traced across finished contracts.
create table historic_contracts (
    id bigint primary key,
    status text not null,
    region_id bigint not null,
    start_location_id bigint,
    issuer_id bigint not null references characters (id),
    issuer_corporation_id bigint,
    for_corporation boolean not null default false,
    type text not null,
    title text,
    date_issued timestamptz,
    date_expired timestamptz,
    price double precision,
    buyout double precision,
    highest_bid double precision,
    unified_price double precision,
    asking_for_items boolean not null default false,
    abyssal_modules_count integer not null default 0,
    non_abyssal_modules_count integer not null default 0,
    plex_count integer not null default 0,
    ignore_for_training boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table historic_contract_items (
    id bigserial primary key,
    historic_contract_id bigint not null
        references historic_contracts (id) on delete cascade,
    record_id bigint not null,
    type_id bigint not null references types (id),
    item_id bigint not null,
    unique (historic_contract_id, record_id)
);

create index historic_contract_items_item_id_index
    on historic_contract_items (item_id);

-- Modules whose sale qualifies as estimator training data: sold alone
-- (PLEX payment allowed) in a completed contract. Filled by the
-- training-data sweep; rows are dropped when an admin disqualifies the
-- contract.
create table training_modules (
    id bigserial primary key,
    module_id bigint not null unique references modules (id) on delete cascade,
    historic_contract_id bigint not null
        references historic_contracts (id) on delete cascade,
    issued_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
