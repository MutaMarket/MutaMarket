-- Detail of failed outgoing ESI requests, so the admin console's error
-- counts can be opened and read. Bounded by row count and by age from
-- src/esi/failures.rs; the exact per-minute counts live in the
-- in-memory telemetry, this is a sampled set of the failures behind
-- them.
--
-- Request headers are deliberately absent: authenticated calls carry a
-- bearer token, and a table with nowhere to put a header cannot leak
-- one. `authenticated` records that a token was sent, not which.
CREATE TABLE esi_failures (
    id bigserial NOT NULL,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL,
    -- The telemetry bucket key, e.g. 'contracts/public'.
    endpoint text NOT NULL,
    method text NOT NULL,
    -- Full URL with its query; token-ish parameters redacted.
    url text NOT NULL,
    -- NULL when no response arrived at all.
    status integer,
    -- 'timeout' | 'connect' | 'decode' | 'body' | 'request', set only
    -- when no response arrived.
    error_kind text,
    error_message text,
    duration_ms bigint NOT NULL,
    authenticated boolean NOT NULL,
    -- 'job:region-contracts' | 'http:GET /api/modules/{module}'.
    caller text,
    -- No foreign key: scheduler_runs prunes to RUN_HISTORY_KEEP rows
    -- per job and would orphan or block this insert.
    scheduler_run_id bigint,
    response_headers jsonb,
    response_body text,
    -- Length before truncation, so the console can say what it is not
    -- showing.
    response_bytes bigint,
    request_body text,
    request_bytes bigint
);

ALTER TABLE ONLY esi_failures
    ADD CONSTRAINT esi_failures_pkey PRIMARY KEY (id);

CREATE INDEX esi_failures_occurred_at_index ON esi_failures USING btree (occurred_at DESC);
