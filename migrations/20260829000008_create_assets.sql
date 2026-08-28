CREATE TABLE asset_imports (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    step text DEFAULT 'fetching_assets'::text NOT NULL,
    assets_count integer DEFAULT 0 NOT NULL,
    assets_corporation_count integer DEFAULT 0 NOT NULL,
    abyssal_modules_count integer DEFAULT 0 NOT NULL,
    abyssal_modules_imported_count integer DEFAULT 0 NOT NULL,
    abyssal_modules_failed_count integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY asset_imports
    ADD CONSTRAINT asset_imports_pkey PRIMARY KEY (id);

CREATE INDEX asset_imports_character_id_index ON asset_imports USING btree (character_id);

CREATE TABLE assets (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    corporation_id bigint,
    item_id bigint NOT NULL,
    type_id bigint NOT NULL,
    name text,
    location_id bigint,
    location_flag text NOT NULL,
    location_type text NOT NULL,
    quantity bigint NOT NULL,
    index bigint DEFAULT 0 NOT NULL,
    is_abyssal boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY assets
    ADD CONSTRAINT assets_character_id_item_id_key UNIQUE (character_id, item_id);

ALTER TABLE ONLY assets
    ADD CONSTRAINT assets_pkey PRIMARY KEY (id);

CREATE INDEX assets_character_location_index ON assets USING btree (character_id, item_id, location_id);

CREATE INDEX assets_item_id_index ON assets USING btree (item_id);

CREATE INDEX assets_location_id_index ON assets USING btree (location_id);

CREATE TABLE public_assets (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    asset_id bigint NOT NULL,
    public_parent_id bigint,
    module_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public_assets
    ADD CONSTRAINT public_assets_character_id_asset_id_key UNIQUE (character_id, asset_id);

ALTER TABLE ONLY public_assets
    ADD CONSTRAINT public_assets_pkey PRIMARY KEY (id);

CREATE INDEX public_assets_asset_id_index ON public_assets USING btree (asset_id);

CREATE INDEX public_assets_module_index ON public_assets USING btree (module_id);

CREATE INDEX public_assets_public_parent_id_index ON public_assets USING btree (public_parent_id);

CREATE TABLE structures (
    id bigint NOT NULL,
    name text,
    owner_id bigint,
    type_id bigint,
    solarsystem_id bigint,
    last_fetched_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY structures
    ADD CONSTRAINT structures_pkey PRIMARY KEY (id);

CREATE INDEX structures_name_index ON structures USING btree (name);

CREATE TABLE character_structure (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    structure_id bigint NOT NULL,
    could_resolve boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY character_structure
    ADD CONSTRAINT character_structure_character_id_structure_id_key UNIQUE (character_id, structure_id);

ALTER TABLE ONLY character_structure
    ADD CONSTRAINT character_structure_pkey PRIMARY KEY (id);

CREATE TABLE alliances (
    id bigint NOT NULL,
    name text NOT NULL,
    ticker text,
    creator_id bigint,
    date_founded timestamp with time zone,
    executor_corporation_id bigint,
    faction_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT alliances_check_id_valid_range CHECK (((id >= 99000000) AND (id <= 2100000000)))
);

ALTER TABLE ONLY alliances
    ADD CONSTRAINT alliances_pkey PRIMARY KEY (id);

ALTER TABLE ONLY alliances
    ADD CONSTRAINT alliances_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES characters(id);

ALTER TABLE ONLY asset_imports
    ADD CONSTRAINT asset_imports_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY assets
    ADD CONSTRAINT assets_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY character_structure
    ADD CONSTRAINT character_structure_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY character_structure
    ADD CONSTRAINT character_structure_structure_id_fkey FOREIGN KEY (structure_id) REFERENCES structures(id) ON DELETE CASCADE;

ALTER TABLE ONLY characters
    ADD CONSTRAINT characters_latest_asset_import_id_fkey FOREIGN KEY (latest_asset_import_id) REFERENCES asset_imports(id) ON DELETE SET NULL;

ALTER TABLE ONLY collection_locations
    ADD CONSTRAINT collection_locations_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE;

ALTER TABLE ONLY public_assets
    ADD CONSTRAINT public_assets_asset_id_fkey FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE;

ALTER TABLE ONLY public_assets
    ADD CONSTRAINT public_assets_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY public_assets
    ADD CONSTRAINT public_assets_public_parent_id_fkey FOREIGN KEY (public_parent_id) REFERENCES public_assets(id) ON DELETE CASCADE;

ALTER TABLE ONLY public_module_ownerships
    ADD CONSTRAINT public_module_ownerships_public_asset_fk FOREIGN KEY (public_asset_id) REFERENCES public_assets(id) ON DELETE CASCADE;
