-- Whole days of premium moved from one character to another through
-- the gifting endpoint: the audit trail of every transfer (a rewrite
-- addition, the legacy application had no way to pass premium on).
CREATE TABLE premium_gifts (
    id bigserial PRIMARY KEY,
    from_character_id bigint NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    to_character_id bigint NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    days integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE INDEX premium_gifts_from_character_id_index ON premium_gifts (from_character_id);
CREATE INDEX premium_gifts_to_character_id_index ON premium_gifts (to_character_id);
