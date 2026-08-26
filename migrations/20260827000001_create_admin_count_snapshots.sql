-- Periodic samples of the admin dashboard's database counts, recorded by
-- the count-snapshots scheduler job (5-minute cadence like the legacy
-- SnapshotCommand) so the dashboard can chart them over time. Pruned to
-- a bounded window by the job itself.
create table admin_count_snapshots (
    id bigserial primary key,
    taken_at timestamptz not null default now(),
    modules bigint not null,
    modules_without_estimate bigint not null,
    contracts bigint not null,
    contract_items bigint not null,
    characters bigint not null,
    users bigint not null,
    assets bigint not null,
    public_ownerships bigint not null,
    market_history_days bigint not null
);

create index admin_count_snapshots_taken_at_index on admin_count_snapshots (taken_at);
