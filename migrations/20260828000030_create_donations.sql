-- The donations ledger and the characters premium-payment columns, the
-- legacy 2024_07_08_130746_create_donations_table (journal_id nullable
-- since 2024_11_27) plus the premium columns of the legacy characters
-- table. Legacy stored ISK as decimal(50,2); here amounts are double
-- precision like every other money column (contracts.price etc.).

alter table characters
    add column premium_paid_total double precision not null default 0,
    add column premium_payment_rest double precision not null default 0;

create table donations (
    id bigserial primary key,
    character_id bigint not null references characters (id)
        on update cascade on delete cascade,
    -- The ESI wallet-journal entry id; the ingestion's idempotency key.
    -- Nullable like legacy (manually granted donations have none).
    journal_id bigint,
    amount double precision not null,
    date timestamptz not null,
    confirmation_sent boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index donations_journal_id_index on donations (journal_id);
create index donations_date_index on donations (date);
create index donations_confirmation_sent_index on donations (confirmation_sent);
-- MySQL indexed the FK implicitly; the per-character aggregates
-- (top-donor lists, repeat-donor counts) lean on it.
create index donations_character_id_index on donations (character_id);
