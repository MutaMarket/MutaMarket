CREATE TABLE collections (
    id bigserial NOT NULL,
    identifier text NOT NULL,
    name text NOT NULL,
    description text,
    visibility text NOT NULL,
    character_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    auto_sync boolean DEFAULT false NOT NULL,
    last_synced_at timestamp with time zone
);

ALTER TABLE ONLY collections
    ADD CONSTRAINT collections_identifier_key UNIQUE (identifier);

ALTER TABLE ONLY collections
    ADD CONSTRAINT collections_pkey PRIMARY KEY (id);

CREATE INDEX collections_character_id_index ON collections USING btree (character_id);

CREATE INDEX collections_visibility_index ON collections USING btree (visibility);

CREATE TABLE collection_modules (
    id bigserial NOT NULL,
    collection_id bigint NOT NULL,
    module_id bigint NOT NULL,
    note text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY collection_modules
    ADD CONSTRAINT collection_modules_collection_id_module_id_key UNIQUE (collection_id, module_id);

ALTER TABLE ONLY collection_modules
    ADD CONSTRAINT collection_modules_pkey PRIMARY KEY (id);

CREATE INDEX collection_modules_module_id_index ON collection_modules USING btree (module_id);

CREATE TABLE collection_notes (
    id bigserial NOT NULL,
    collection_id bigint NOT NULL,
    user_id bigint NOT NULL,
    module_id bigint NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY collection_notes
    ADD CONSTRAINT collection_notes_collection_id_module_id_key UNIQUE (collection_id, module_id);

ALTER TABLE ONLY collection_notes
    ADD CONSTRAINT collection_notes_pkey PRIMARY KEY (id);

CREATE TABLE collection_locations (
    id bigserial NOT NULL,
    collection_id bigint NOT NULL,
    asset_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY collection_locations
    ADD CONSTRAINT collection_locations_collection_id_asset_id_key UNIQUE (collection_id, asset_id);

ALTER TABLE ONLY collection_locations
    ADD CONSTRAINT collection_locations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY collection_locations
    ADD CONSTRAINT collection_locations_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE;

ALTER TABLE ONLY collection_modules
    ADD CONSTRAINT collection_modules_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE;

ALTER TABLE ONLY collection_modules
    ADD CONSTRAINT collection_modules_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY collection_notes
    ADD CONSTRAINT collection_notes_collection_id_fkey FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE;

ALTER TABLE ONLY collection_notes
    ADD CONSTRAINT collection_notes_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY collection_notes
    ADD CONSTRAINT collection_notes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY collections
    ADD CONSTRAINT collections_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id);
