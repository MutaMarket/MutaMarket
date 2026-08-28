-- Alliance records, the legacy alliances table fed by the daily
-- app:get-alliances sweep (GetAlliancesJob -> GetAllianceJob ->
-- CreateAllianceAction).
--
-- Divergence, deliberate: legacy executor_corporation_id referenced the
-- corporations table and the create action synced the executor
-- corporation first; corporations are not ported, so the raw ESI id is
-- stored without a foreign key.
create table alliances (
    id bigint primary key,
    name text not null,
    ticker text,
    creator_id bigint references characters (id),
    date_founded timestamptz,
    executor_corporation_id bigint,
    faction_id bigint,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    -- The legacy id-range check (its two OR ranges collapse to one).
    constraint alliances_check_id_valid_range check (id between 99000000 and 2100000000)
);
