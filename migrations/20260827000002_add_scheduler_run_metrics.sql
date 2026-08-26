-- Named per-run metrics (e.g. character-assets: found/imported/failed),
-- so the job cards can chart multiple series per run instead of the one
-- items number.
alter table scheduler_runs add column metrics jsonb;
