-- Request activity, aggregated. There is deliberately no per-request
-- row: the console's questions are all answerable from these two tables
-- at a fraction of the volume. Written by the activity-flush job from
-- the in-memory recorder in src/activity/mod.rs.

-- Request volume per hour, per matched route, split by whether the
-- request carried a live session. `route` is the method plus the route
-- pattern ('GET /api/module-page/{module}'), never a concrete URL, so
-- no module id, character id or search term is ever stored.
CREATE TABLE activity_hours (
    hour timestamp with time zone NOT NULL,
    route text NOT NULL,
    signed_in boolean NOT NULL,
    requests bigint DEFAULT 0 NOT NULL,
    errors bigint DEFAULT 0 NOT NULL,
    total_ms bigint DEFAULT 0 NOT NULL
);

ALTER TABLE ONLY activity_hours
    ADD CONSTRAINT activity_hours_pkey PRIMARY KEY (hour, route, signed_in);

CREATE INDEX activity_hours_hour_index ON activity_hours USING btree (hour);

-- One row per signed-in user per UTC day: the only per-user record.
--
-- It deliberately has no route column. Adding one would turn an activity
-- counter into a browsing history for a named person, which is a
-- different thing to hold about someone than "was here, made N
-- requests". The limit is the design, not an oversight.
CREATE TABLE user_activity_days (
    user_id bigint NOT NULL,
    day date NOT NULL,
    requests bigint DEFAULT 0 NOT NULL
);

ALTER TABLE ONLY user_activity_days
    ADD CONSTRAINT user_activity_days_pkey PRIMARY KEY (user_id, day);

CREATE INDEX user_activity_days_day_index ON user_activity_days USING btree (day);

ALTER TABLE ONLY user_activity_days
    ADD CONSTRAINT user_activity_days_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
