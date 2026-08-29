-- The legacy corporations table (2024_10_19_115401) in final shape.
-- alliance_id stays a raw id column with no FK, like legacy; it also
-- stays null in practice because the legacy CreateCorporationAction
-- never writes it (see src/corporations.rs). alliances.executor_corporation_id
-- likewise remains a raw id column: legacy never constrained it either.
CREATE TABLE corporations (
    id bigint NOT NULL,
    name text NOT NULL,
    ticker text,
    alliance_id bigint,
    faction_id bigint,
    ceo_id bigint,
    creator_id bigint,
    date_founded timestamp with time zone,
    description text,
    home_station_id bigint,
    member_count bigint,
    shares bigint,
    tax_rate double precision,
    url text,
    war_eligible boolean DEFAULT false,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    -- Player corporation id ranges (NPC corporations live below them and
    -- are deliberately rejected, like the legacy check).
    CONSTRAINT corporations_check_id_valid_range CHECK (((id >= 98000000 AND id <= 99000000) OR (id >= 100000000 AND id <= 2100000000)))
);

ALTER TABLE ONLY corporations
    ADD CONSTRAINT corporations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY corporations
    ADD CONSTRAINT corporations_ceo_id_fkey FOREIGN KEY (ceo_id) REFERENCES characters(id);

ALTER TABLE ONLY corporations
    ADD CONSTRAINT corporations_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES characters(id);
