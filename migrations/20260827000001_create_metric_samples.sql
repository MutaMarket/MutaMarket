-- Generic time-series samples for the admin dashboard (the shape of
-- Laravel Pulse's entries): one row per (metric, instant, value),
-- written by the metric-samples scheduler job for every registered
-- `metrics::Recordable`. Pruned to a bounded window by the job itself.
create table metric_samples (
    id bigserial primary key,
    metric text not null,
    taken_at timestamptz not null default now(),
    value double precision not null
);

create index metric_samples_metric_taken_at_index on metric_samples (metric, taken_at);
