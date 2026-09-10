-- Detail of failed incoming requests, so an error count on the admin
-- console's activity roll-up can be opened and read. The mirror image of
-- esi_failures, which does the same for outgoing ESI calls: bounded by
-- row count and by age from src/activity/failures.rs, a sampled set of
-- the failures behind the exact counts in activity_hours.
--
-- Unlike activity_hours this does hold the concrete path and, for a
-- signed-in request, the account behind it: without them a 500 on a
-- module page says only that one happened, not on which module or to
-- whom. That is the deliberate difference between a permanent counter
-- and a seven-day debugging sample. Request headers are never stored, so
-- the table cannot hold a session cookie or a bearer token, and query
-- parameters carrying a secret are redacted before the path is written.
CREATE TABLE request_failures (
    id bigserial NOT NULL,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL,
    -- Method plus the matched route pattern, the activity_hours key:
    -- 'GET /api/modules/{module}', or 'GET (not found)'.
    route text NOT NULL,
    method text NOT NULL,
    -- Concrete path with its query, secret-ish parameters redacted.
    path text NOT NULL,
    status integer NOT NULL,
    -- The `message` of our JSON error body, when it carries one.
    error_message text,
    response_body text,
    -- Length before truncation, so the console can say what it is not
    -- showing.
    response_bytes bigint,
    duration_ms bigint NOT NULL,
    user_id bigint
);

ALTER TABLE ONLY request_failures
    ADD CONSTRAINT request_failures_pkey PRIMARY KEY (id);

CREATE INDEX request_failures_occurred_at_index ON request_failures USING btree (occurred_at DESC);

-- The console's two filters: one route, or one status class.
CREATE INDEX request_failures_route_index ON request_failures USING btree (route);
CREATE INDEX request_failures_status_index ON request_failures USING btree (status);

ALTER TABLE ONLY request_failures
    ADD CONSTRAINT request_failures_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;
