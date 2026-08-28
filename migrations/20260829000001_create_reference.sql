CREATE TABLE sde_meta (
    key text NOT NULL,
    value text NOT NULL
);

ALTER TABLE ONLY sde_meta
    ADD CONSTRAINT sde_meta_pkey PRIMARY KEY (key);

CREATE TABLE units (
    id bigint NOT NULL,
    name text DEFAULT ''::text NOT NULL,
    display_name text DEFAULT ''::text NOT NULL
);

ALTER TABLE ONLY units
    ADD CONSTRAINT units_pkey PRIMARY KEY (id);

CREATE TABLE attributes (
    id bigint NOT NULL,
    name text NOT NULL,
    high_is_good boolean DEFAULT false NOT NULL,
    derived boolean DEFAULT false NOT NULL,
    derived_operation text,
    derived_attributes bigint[],
    display_name text DEFAULT ''::text NOT NULL,
    unit_id bigint
);

ALTER TABLE ONLY attributes
    ADD CONSTRAINT attributes_pkey PRIMARY KEY (id);

CREATE TABLE meta_groups (
    id bigint NOT NULL,
    name text DEFAULT ''::text NOT NULL
);

ALTER TABLE ONLY meta_groups
    ADD CONSTRAINT meta_groups_pkey PRIMARY KEY (id);

CREATE TABLE market_groups (
    id bigint NOT NULL,
    parent_id bigint
);

ALTER TABLE ONLY market_groups
    ADD CONSTRAINT market_groups_pkey PRIMARY KEY (id);

CREATE TABLE types (
    id bigint NOT NULL,
    name text NOT NULL,
    published boolean DEFAULT false NOT NULL,
    meta_group_id bigint,
    market_group_id bigint
);

ALTER TABLE ONLY types
    ADD CONSTRAINT types_pkey PRIMARY KEY (id);

CREATE TABLE type_attributes (
    id bigint NOT NULL,
    type_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    value double precision
);

ALTER TABLE ONLY type_attributes
    ADD CONSTRAINT type_attributes_pkey PRIMARY KEY (id);

ALTER TABLE ONLY type_attributes
    ADD CONSTRAINT type_attributes_type_id_attribute_id_key UNIQUE (type_id, attribute_id);

CREATE TABLE mutaplasmids (
    id bigint NOT NULL,
    name text NOT NULL,
    output_type_id bigint NOT NULL
);

ALTER TABLE ONLY mutaplasmids
    ADD CONSTRAINT mutaplasmids_pkey PRIMARY KEY (id);

CREATE TABLE mutaplasmid_attributes (
    id bigint NOT NULL,
    mutaplasmid_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    value_min double precision NOT NULL,
    value_max double precision NOT NULL,
    high_is_good boolean,
    is_virtual boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY mutaplasmid_attributes
    ADD CONSTRAINT mutaplasmid_attributes_mutaplasmid_id_attribute_id_key UNIQUE (mutaplasmid_id, attribute_id);

ALTER TABLE ONLY mutaplasmid_attributes
    ADD CONSTRAINT mutaplasmid_attributes_pkey PRIMARY KEY (id);

CREATE TABLE mutaplasmid_input_types (
    id bigint NOT NULL,
    mutaplasmid_id bigint NOT NULL,
    type_id bigint NOT NULL
);

ALTER TABLE ONLY mutaplasmid_input_types
    ADD CONSTRAINT mutaplasmid_input_types_mutaplasmid_id_type_id_key UNIQUE (mutaplasmid_id, type_id);

ALTER TABLE ONLY mutaplasmid_input_types
    ADD CONSTRAINT mutaplasmid_input_types_pkey PRIMARY KEY (id);

CREATE TABLE regions (
    id bigint NOT NULL,
    name text DEFAULT ''::text NOT NULL
);

ALTER TABLE ONLY regions
    ADD CONSTRAINT regions_pkey PRIMARY KEY (id);

CREATE TABLE stations (
    id bigint NOT NULL,
    name text NOT NULL,
    type_id bigint,
    solarsystem_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY stations
    ADD CONSTRAINT stations_pkey PRIMARY KEY (id);

ALTER TABLE ONLY attributes
    ADD CONSTRAINT attributes_unit_id_fkey FOREIGN KEY (unit_id) REFERENCES units(id);

ALTER TABLE ONLY mutaplasmid_attributes
    ADD CONSTRAINT mutaplasmid_attributes_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id);

ALTER TABLE ONLY mutaplasmid_attributes
    ADD CONSTRAINT mutaplasmid_attributes_mutaplasmid_id_fkey FOREIGN KEY (mutaplasmid_id) REFERENCES mutaplasmids(id);

ALTER TABLE ONLY mutaplasmid_input_types
    ADD CONSTRAINT mutaplasmid_input_types_mutaplasmid_id_fkey FOREIGN KEY (mutaplasmid_id) REFERENCES mutaplasmids(id);

ALTER TABLE ONLY mutaplasmid_input_types
    ADD CONSTRAINT mutaplasmid_input_types_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY mutaplasmids
    ADD CONSTRAINT mutaplasmids_output_type_id_fkey FOREIGN KEY (output_type_id) REFERENCES types(id);

ALTER TABLE ONLY type_attributes
    ADD CONSTRAINT type_attributes_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id);

ALTER TABLE ONLY type_attributes
    ADD CONSTRAINT type_attributes_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);
