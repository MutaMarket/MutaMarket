-- Per-table autovacuum thresholds for the tables that churn hardest.
--
-- The stock 20% dead-tuple trigger left `assets` sitting at 400k dead
-- rows for days (18.7% of live rows on 2026-09-11, just under the
-- trigger). Dead rows keep the visibility map dirty, and a dirty
-- visibility map is what forces a heap scan where an index-only scan
-- would do: the admin console's nine count scans read about 1.3 GB for
-- that reason. 5% keeps these tables leaner, at the price of more
-- background vacuum I/O.
--
-- Tables outside this list keep the cluster defaults; add one only with
-- a dead-row measurement behind it (pg_stat_user_tables).
ALTER TABLE assets SET (
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_scale_factor = 0.02
);

ALTER TABLE modules SET (
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_scale_factor = 0.02
);

ALTER TABLE characters SET (
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_scale_factor = 0.02
);

ALTER TABLE character_contracts SET (
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_scale_factor = 0.02
);
