-- Saved search alerts (a rewrite addition, the legacy application had no
-- way to follow a query): a user's filter query path, re-run by the
-- search-alerts job against the public listings that appeared since the
-- alert's last check, with one notification per run carrying the new
-- matches.
CREATE TABLE search_alerts (
    id bigserial PRIMARY KEY,
    user_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The normalized query path (no prefix, page, sort or account-relative
    -- options), unique per user so saving the same search twice is a no-op.
    query text NOT NULL,
    -- The query's type filter, kept for the settings list's label.
    type_id bigint REFERENCES types(id) ON DELETE CASCADE,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    -- The end of the last closed check window: listing rows created after
    -- this instant are the next run's candidates. Starts at the alert's
    -- creation, so listings that existed before it never fire it.
    checked_until timestamp with time zone DEFAULT now() NOT NULL,
    last_notified_at timestamp with time zone,
    notified_count bigint DEFAULT 0 NOT NULL,
    UNIQUE (user_id, query)
);

-- The alert job's candidate scan: listing rows created inside a window.
CREATE INDEX public_module_ownerships_created_at_index ON public_module_ownerships (created_at);
