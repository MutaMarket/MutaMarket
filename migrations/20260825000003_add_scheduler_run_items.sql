-- Each run's headline metric (contracts synced, modules imported,
-- characters named, ...), so the per-job cards can chart work per run.
alter table scheduler_runs add column items bigint;
