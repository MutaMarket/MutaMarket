-- Observability and control state of the background scheduler (no legacy
-- counterpart): per-job pause flags and the recorded run history the
-- /admin/scheduler page reads.

create table scheduler_jobs (
    job text primary key,
    paused boolean not null default false
);

create table scheduler_runs (
    id bigserial primary key,
    job text not null,
    started_at timestamptz not null default now(),
    finished_at timestamptz,
    -- 'success' or 'error' once finished; null while running.
    outcome text,
    summary text,
    error text
);

create index scheduler_runs_job_index on scheduler_runs (job, id desc);
