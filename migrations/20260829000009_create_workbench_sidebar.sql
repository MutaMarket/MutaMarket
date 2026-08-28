CREATE TABLE workbench_modules (
    id bigserial NOT NULL,
    user_id bigint NOT NULL,
    module_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY workbench_modules
    ADD CONSTRAINT workbench_modules_pkey PRIMARY KEY (id);

ALTER TABLE ONLY workbench_modules
    ADD CONSTRAINT workbench_modules_user_id_module_id_key UNIQUE (user_id, module_id);

CREATE TABLE bookmarks (
    id bigserial NOT NULL,
    user_id bigint NOT NULL,
    type_id bigint,
    name text NOT NULL,
    query text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY bookmarks
    ADD CONSTRAINT bookmarks_pkey PRIMARY KEY (id);

CREATE INDEX bookmarks_user_idx ON bookmarks USING btree (user_id);

CREATE TABLE advertisements (
    id bigserial NOT NULL,
    name text NOT NULL,
    description text,
    image_url text,
    link text,
    active boolean DEFAULT true NOT NULL,
    starts_at timestamp with time zone,
    expires_at timestamp with time zone,
    priority integer DEFAULT 0 NOT NULL,
    size text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY advertisements
    ADD CONSTRAINT advertisements_pkey PRIMARY KEY (id);

CREATE TABLE gear_items (
    id bigserial NOT NULL,
    name text NOT NULL,
    description text,
    image_url text,
    link text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    priority integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY gear_items
    ADD CONSTRAINT gear_items_pkey PRIMARY KEY (id);

ALTER TABLE ONLY bookmarks
    ADD CONSTRAINT bookmarks_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id) ON DELETE CASCADE;

ALTER TABLE ONLY bookmarks
    ADD CONSTRAINT bookmarks_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY workbench_modules
    ADD CONSTRAINT workbench_modules_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY workbench_modules
    ADD CONSTRAINT workbench_modules_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--
