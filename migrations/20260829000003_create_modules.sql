CREATE TABLE modules (
    id bigint NOT NULL,
    type_id bigint NOT NULL,
    source_type_id bigint,
    mutaplasmid_id bigint,
    creator_id bigint,
    average_fraction double precision,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    estimated_value double precision,
    estimated_value_updated_at timestamp with time zone,
    latest_contract_id bigint,
    latest_contract_price double precision
);

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_pkey PRIMARY KEY (id);

CREATE INDEX modules_average_fraction_index ON modules USING btree (average_fraction NULLS FIRST, id NULLS FIRST);

CREATE INDEX modules_created_at_index ON modules USING btree (created_at);

CREATE INDEX modules_creator_id_index ON modules USING btree (creator_id);

CREATE INDEX modules_estimated_value_index ON modules USING btree (estimated_value NULLS FIRST, id NULLS FIRST);

CREATE INDEX modules_estimated_value_updated_at_index ON modules USING btree (estimated_value_updated_at);

CREATE INDEX modules_latest_contract_id_index ON modules USING btree (latest_contract_id);

CREATE INDEX modules_latest_contract_price_index ON modules USING btree (latest_contract_price NULLS FIRST, id NULLS FIRST);

CREATE INDEX modules_mutaplasmid_id_index ON modules USING btree (mutaplasmid_id);

CREATE INDEX modules_source_type_id_index ON modules USING btree (source_type_id);

CREATE INDEX modules_type_id_index ON modules USING btree (type_id);

CREATE TABLE mutated_attributes (
    id bigserial NOT NULL,
    module_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    type_id bigint NOT NULL,
    value double precision NOT NULL,
    base_value double precision NOT NULL,
    fraction double precision NOT NULL,
    fraction_type double precision NOT NULL,
    fraction_absolute double precision NOT NULL,
    bar smallint DEFAULT 0 NOT NULL,
    is_virtual boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY mutated_attributes
    ADD CONSTRAINT mutated_attributes_module_id_attribute_id_key UNIQUE (module_id, attribute_id);

ALTER TABLE ONLY mutated_attributes
    ADD CONSTRAINT mutated_attributes_pkey PRIMARY KEY (id);

CREATE INDEX mutated_attributes_attribute_id_index ON mutated_attributes USING btree (attribute_id);

CREATE INDEX mutated_attributes_bar_index ON mutated_attributes USING btree (bar);

CREATE INDEX mutated_attributes_type_attribute_value_index ON mutated_attributes USING btree (type_id, attribute_id, value, module_id);

CREATE TABLE notes (
    id bigserial NOT NULL,
    user_id bigint NOT NULL,
    module_id bigint NOT NULL,
    content text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY notes
    ADD CONSTRAINT notes_pkey PRIMARY KEY (id);

ALTER TABLE ONLY notes
    ADD CONSTRAINT notes_user_id_module_id_key UNIQUE (user_id, module_id);

CREATE TABLE module_pricing (
    id bigserial NOT NULL,
    module_id bigint NOT NULL,
    user_id bigint NOT NULL,
    price double precision NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY module_pricing
    ADD CONSTRAINT module_pricing_module_id_user_id_key UNIQUE (module_id, user_id);

ALTER TABLE ONLY module_pricing
    ADD CONSTRAINT module_pricing_pkey PRIMARY KEY (id);

CREATE TABLE public_module_ownerships (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    module_id bigint NOT NULL,
    public_asset_id bigint,
    contract_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY public_module_ownerships
    ADD CONSTRAINT public_module_ownerships_character_id_module_id_key UNIQUE (character_id, module_id);

ALTER TABLE ONLY public_module_ownerships
    ADD CONSTRAINT public_module_ownerships_pkey PRIMARY KEY (id);

CREATE INDEX public_module_ownerships_module_index ON public_module_ownerships USING btree (module_id, character_id, public_asset_id, contract_id);

ALTER TABLE ONLY module_pricing
    ADD CONSTRAINT module_pricing_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id);

ALTER TABLE ONLY module_pricing
    ADD CONSTRAINT module_pricing_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES characters(id);

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_mutaplasmid_id_fkey FOREIGN KEY (mutaplasmid_id) REFERENCES mutaplasmids(id);

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_source_type_id_fkey FOREIGN KEY (source_type_id) REFERENCES types(id);

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY mutated_attributes
    ADD CONSTRAINT mutated_attributes_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id);

ALTER TABLE ONLY mutated_attributes
    ADD CONSTRAINT mutated_attributes_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY mutated_attributes
    ADD CONSTRAINT mutated_attributes_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY notes
    ADD CONSTRAINT notes_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id);

ALTER TABLE ONLY notes
    ADD CONSTRAINT notes_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY public_module_ownerships
    ADD CONSTRAINT public_module_ownerships_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;
