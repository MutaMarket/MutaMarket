CREATE TABLE estimator_attributes (
    id bigserial NOT NULL,
    type_id bigint NOT NULL,
    attribute_id bigint NOT NULL
);

ALTER TABLE ONLY estimator_attributes
    ADD CONSTRAINT estimator_attributes_pkey PRIMARY KEY (id);

ALTER TABLE ONLY estimator_attributes
    ADD CONSTRAINT estimator_attributes_type_id_attribute_id_key UNIQUE (type_id, attribute_id);

CREATE TABLE estimator_models (
    type_id bigint NOT NULL,
    feature_names jsonb NOT NULL,
    model bytea NOT NULL,
    trained_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY estimator_models
    ADD CONSTRAINT estimator_models_pkey PRIMARY KEY (type_id);

CREATE TABLE estimator_statistics (
    id bigserial NOT NULL,
    type_id bigint NOT NULL,
    name text NOT NULL,
    data_count bigint DEFAULT 0 NOT NULL,
    r2 double precision,
    mae double precision,
    last_trained_at timestamp with time zone,
    data_statistics jsonb,
    nmae double precision,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY estimator_statistics
    ADD CONSTRAINT estimator_statistics_pkey PRIMARY KEY (id);

ALTER TABLE ONLY estimator_statistics
    ADD CONSTRAINT estimator_statistics_type_id_key UNIQUE (type_id);

CREATE TABLE training_modules (
    id bigserial NOT NULL,
    module_id bigint NOT NULL,
    historic_contract_id bigint NOT NULL,
    issued_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY training_modules
    ADD CONSTRAINT training_modules_module_id_key UNIQUE (module_id);

ALTER TABLE ONLY training_modules
    ADD CONSTRAINT training_modules_pkey PRIMARY KEY (id);

CREATE INDEX training_modules_historic_contract_id_index ON training_modules USING btree (historic_contract_id);

CREATE INDEX training_modules_issued_at_index ON training_modules USING btree (issued_at);

CREATE TABLE abyssal_type_statistics (
    id bigserial NOT NULL,
    type_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    best double precision NOT NULL,
    worst double precision NOT NULL,
    high_is_good boolean DEFAULT false NOT NULL,
    is_virtual boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY abyssal_type_statistics
    ADD CONSTRAINT abyssal_type_statistics_pkey PRIMARY KEY (id);

ALTER TABLE ONLY abyssal_type_statistics
    ADD CONSTRAINT abyssal_type_statistics_type_id_attribute_id_key UNIQUE (type_id, attribute_id);

CREATE TABLE mutaplasmid_type_statistics (
    id bigint NOT NULL,
    type_id bigint NOT NULL,
    mutaplasmid_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    best double precision NOT NULL,
    worst double precision NOT NULL,
    high_is_good boolean DEFAULT false NOT NULL,
    is_virtual boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY mutaplasmid_type_statistics
    ADD CONSTRAINT mutaplasmid_type_statistics_pkey PRIMARY KEY (id);

ALTER TABLE ONLY mutaplasmid_type_statistics
    ADD CONSTRAINT mutaplasmid_type_statistics_type_id_mutaplasmid_id_attribut_key UNIQUE (type_id, mutaplasmid_id, attribute_id);

CREATE FUNCTION slug(name text) RETURNS text
    LANGUAGE sql IMMUTABLE
    RETURN TRIM(BOTH '-'::text FROM regexp_replace(lower(name), '[^a-z0-9]+'::text, '-'::text, 'g'::text));


SET default_tablespace = '';

SET default_table_access_method = heap;

ALTER TABLE ONLY abyssal_type_statistics
    ADD CONSTRAINT abyssal_type_statistics_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id);

ALTER TABLE ONLY abyssal_type_statistics
    ADD CONSTRAINT abyssal_type_statistics_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY estimator_attributes
    ADD CONSTRAINT estimator_attributes_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id) ON DELETE CASCADE;

ALTER TABLE ONLY estimator_attributes
    ADD CONSTRAINT estimator_attributes_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id) ON DELETE CASCADE;

ALTER TABLE ONLY estimator_models
    ADD CONSTRAINT estimator_models_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY estimator_statistics
    ADD CONSTRAINT estimator_statistics_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY mutaplasmid_type_statistics
    ADD CONSTRAINT mutaplasmid_type_statistics_attribute_id_fkey FOREIGN KEY (attribute_id) REFERENCES attributes(id);

ALTER TABLE ONLY mutaplasmid_type_statistics
    ADD CONSTRAINT mutaplasmid_type_statistics_mutaplasmid_id_fkey FOREIGN KEY (mutaplasmid_id) REFERENCES mutaplasmids(id);

ALTER TABLE ONLY mutaplasmid_type_statistics
    ADD CONSTRAINT mutaplasmid_type_statistics_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY training_modules
    ADD CONSTRAINT training_modules_historic_contract_id_fkey FOREIGN KEY (historic_contract_id) REFERENCES historic_contracts(id) ON DELETE CASCADE;

ALTER TABLE ONLY training_modules
    ADD CONSTRAINT training_modules_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;
