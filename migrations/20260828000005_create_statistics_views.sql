-- Materialized statistics for the unified /statistics page: the
-- overview aggregates cost ~1s of scans over 1.7M modules and the
-- leaderboard ranks ~54k creators per request, so both are
-- precomputed and refreshed by the statistics-views scheduler job.

-- One row of market-wide totals; the time-window counts are "as of
-- refreshed_at". Bar markers mirror src/modules/stats.rs (gold 1,
-- brown -1, diamond 2), counted archive-wide like the all-modules page.
create materialized view statistics_overview as
select
    1::bigint as id,
    (select count(*) from modules) as total_count,
    (select count(*) from modules where latest_contract_id is not null) as listed_count,
    (select count(*) from modules where created_at >= now() - interval '1 hour') as added_last_hour_count,
    (select count(*) from modules where created_at >= now() - interval '1 day') as added_last_day_count,
    (select count(*) from modules where created_at >= now() - interval '7 days') as added_last_week_count,
    (select count(*) from contracts where abyssal_modules_count > 0) as contracts_count,
    (select count(*) from contracts where type = 'item_exchange' and abyssal_modules_count > 0) as item_exchanges_count,
    (select count(*) from contracts where type = 'auction' and abyssal_modules_count > 0) as auctions_count,
    (select count(distinct a.module_id) from mutated_attributes a where a.bar = 1) as goldbars_count,
    (select count(distinct a.module_id) from mutated_attributes a where a.bar = -1) as brownbars_count,
    (select count(distinct a.module_id) from mutated_attributes a where a.bar = 2) as diamondbars_count,
    (select coalesce(sum(estimated_value), 0) from modules) as total_value,
    (select coalesce(avg(estimated_value), 0) from modules) as average_value,
    (select count(distinct creator_id) from modules where creator_id is not null) as creators_count,
    (select count(*) from characters) as characters_count,
    now() as refreshed_at;

-- refresh ... concurrently needs a unique index.
create unique index statistics_overview_id on statistics_overview (id);

-- Creation counts per (creator, type): the leaderboard aggregates
-- these few-hundred-k rows instead of the modules table, both for the
-- global ranking and the per-type scope.
create materialized view statistics_creator_type_counts as
select creator_id, type_id, count(*) as modules_created_count
from modules
where creator_id is not null
group by creator_id, type_id;

create unique index statistics_creator_type_counts_key
    on statistics_creator_type_counts (creator_id, type_id);
create index statistics_creator_type_counts_type
    on statistics_creator_type_counts (type_id);
