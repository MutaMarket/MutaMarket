CREATE TABLE contracts (
    id bigint NOT NULL,
    region_id bigint NOT NULL,
    start_location_id bigint,
    issuer_id bigint NOT NULL,
    issuer_corporation_id bigint,
    for_corporation boolean DEFAULT false NOT NULL,
    type text NOT NULL,
    title text,
    date_issued timestamp with time zone,
    date_expired timestamp with time zone,
    price double precision,
    buyout double precision,
    highest_bid double precision,
    unified_price double precision,
    asking_for_items boolean DEFAULT false NOT NULL,
    abyssal_modules_count integer DEFAULT 0 NOT NULL,
    non_abyssal_modules_count integer DEFAULT 0 NOT NULL,
    plex_count integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    items_synced_at timestamp with time zone
);

ALTER TABLE ONLY contracts
    ADD CONSTRAINT contracts_pkey PRIMARY KEY (id);

CREATE INDEX contracts_abyssal_modules_count_index ON contracts USING btree (abyssal_modules_count);

CREATE INDEX contracts_date_expired_index ON contracts USING btree (date_expired);

CREATE INDEX contracts_date_issued_index ON contracts USING btree (date_issued);

CREATE INDEX contracts_issuer_id_index ON contracts USING btree (issuer_id);

CREATE INDEX contracts_region_id_index ON contracts USING btree (region_id);

CREATE INDEX contracts_type_index ON contracts USING btree (type);

CREATE INDEX contracts_unified_price_index ON contracts USING btree (unified_price);

CREATE TABLE contract_items (
    id bigserial NOT NULL,
    contract_id bigint NOT NULL,
    record_id bigint NOT NULL,
    type_id bigint NOT NULL,
    item_id bigint NOT NULL
);

ALTER TABLE ONLY contract_items
    ADD CONSTRAINT contract_items_contract_id_record_id_key UNIQUE (contract_id, record_id);

ALTER TABLE ONLY contract_items
    ADD CONSTRAINT contract_items_pkey PRIMARY KEY (id);

CREATE TABLE contract_imports (
    id bigserial NOT NULL,
    region_id bigint NOT NULL,
    contracts_total_count integer DEFAULT 0 NOT NULL,
    contracts_invalidated_count integer DEFAULT 0 NOT NULL,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY contract_imports
    ADD CONSTRAINT contract_imports_pkey PRIMARY KEY (id);

CREATE TABLE character_contracts (
    id bigint NOT NULL,
    issuer_id bigint NOT NULL,
    issuer_corporation_id bigint,
    for_corporation boolean DEFAULT false NOT NULL,
    type text NOT NULL,
    title text,
    date_issued timestamp with time zone,
    date_expired timestamp with time zone,
    price double precision,
    buyout double precision,
    acceptor_id bigint,
    acceptor_type text DEFAULT 'character'::text,
    assignee_id bigint,
    availability text NOT NULL,
    date_accepted timestamp with time zone,
    date_completed timestamp with time zone,
    status text NOT NULL,
    volume double precision,
    highest_bid double precision,
    unified_price double precision,
    asking_for_items boolean DEFAULT false NOT NULL,
    abyssal_modules_count integer DEFAULT 0 NOT NULL,
    non_abyssal_modules_count integer DEFAULT 0 NOT NULL,
    plex_count integer DEFAULT 0 NOT NULL,
    items_synced_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY character_contracts
    ADD CONSTRAINT character_contracts_pkey PRIMARY KEY (id);

CREATE INDEX character_contracts_availability_index ON character_contracts USING btree (availability);

CREATE INDEX character_contracts_issuer_index ON character_contracts USING btree (id, issuer_id);

CREATE TABLE character_contract_items (
    id bigserial NOT NULL,
    character_contract_id bigint NOT NULL,
    type_id bigint NOT NULL,
    record_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY character_contract_items
    ADD CONSTRAINT character_contract_items_character_contract_id_record_id_key UNIQUE (character_contract_id, record_id);

ALTER TABLE ONLY character_contract_items
    ADD CONSTRAINT character_contract_items_pkey PRIMARY KEY (id);

CREATE TABLE historic_contracts (
    id bigint NOT NULL,
    status text NOT NULL,
    region_id bigint NOT NULL,
    start_location_id bigint,
    issuer_id bigint NOT NULL,
    issuer_corporation_id bigint,
    for_corporation boolean DEFAULT false NOT NULL,
    type text NOT NULL,
    title text,
    date_issued timestamp with time zone,
    date_expired timestamp with time zone,
    price double precision,
    buyout double precision,
    highest_bid double precision,
    unified_price double precision,
    asking_for_items boolean DEFAULT false NOT NULL,
    abyssal_modules_count integer DEFAULT 0 NOT NULL,
    non_abyssal_modules_count integer DEFAULT 0 NOT NULL,
    plex_count integer DEFAULT 0 NOT NULL,
    ignore_for_training boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY historic_contracts
    ADD CONSTRAINT historic_contracts_pkey PRIMARY KEY (id);

CREATE INDEX historic_contracts_date_issued_index ON historic_contracts USING btree (date_issued);

CREATE INDEX historic_contracts_issuer_id_index ON historic_contracts USING btree (issuer_id);

CREATE INDEX historic_contracts_status_index ON historic_contracts USING btree (status);

CREATE INDEX historic_contracts_unified_price_index ON historic_contracts USING btree (unified_price);

CREATE TABLE historic_contract_items (
    id bigserial NOT NULL,
    historic_contract_id bigint NOT NULL,
    record_id bigint NOT NULL,
    type_id bigint NOT NULL,
    item_id bigint NOT NULL
);

ALTER TABLE ONLY historic_contract_items
    ADD CONSTRAINT historic_contract_items_historic_contract_id_record_id_key UNIQUE (historic_contract_id, record_id);

ALTER TABLE ONLY historic_contract_items
    ADD CONSTRAINT historic_contract_items_pkey PRIMARY KEY (id);

CREATE INDEX historic_contract_items_item_id_index ON historic_contract_items USING btree (item_id);

CREATE INDEX historic_contract_items_type_id_index ON historic_contract_items USING btree (type_id);

CREATE TABLE contract_review_history (
    id bigserial NOT NULL,
    historic_contract_id bigint NOT NULL,
    user_id bigint NOT NULL,
    previous_status text,
    new_status text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY contract_review_history
    ADD CONSTRAINT contract_review_history_pkey PRIMARY KEY (id);

CREATE INDEX contract_review_history_contract_index ON contract_review_history USING btree (historic_contract_id, created_at);

CREATE INDEX contract_review_history_user_index ON contract_review_history USING btree (user_id, created_at);

ALTER TABLE ONLY character_contract_items
    ADD CONSTRAINT character_contract_items_character_contract_id_fkey FOREIGN KEY (character_contract_id) REFERENCES character_contracts(id) ON DELETE CASCADE;

ALTER TABLE ONLY character_contract_items
    ADD CONSTRAINT character_contract_items_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY character_contracts
    ADD CONSTRAINT character_contracts_issuer_id_fkey FOREIGN KEY (issuer_id) REFERENCES characters(id);

ALTER TABLE ONLY contract_imports
    ADD CONSTRAINT contract_imports_region_id_fkey FOREIGN KEY (region_id) REFERENCES regions(id);

ALTER TABLE ONLY contract_items
    ADD CONSTRAINT contract_items_contract_id_fkey FOREIGN KEY (contract_id) REFERENCES contracts(id) ON DELETE CASCADE;

ALTER TABLE ONLY contract_items
    ADD CONSTRAINT contract_items_item_id_fkey FOREIGN KEY (item_id) REFERENCES modules(id);

ALTER TABLE ONLY contract_items
    ADD CONSTRAINT contract_items_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY contract_review_history
    ADD CONSTRAINT contract_review_history_historic_contract_id_fkey FOREIGN KEY (historic_contract_id) REFERENCES historic_contracts(id) ON DELETE CASCADE;

ALTER TABLE ONLY contract_review_history
    ADD CONSTRAINT contract_review_history_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY contracts
    ADD CONSTRAINT contracts_issuer_id_fkey FOREIGN KEY (issuer_id) REFERENCES characters(id);

ALTER TABLE ONLY contracts
    ADD CONSTRAINT contracts_region_id_fkey FOREIGN KEY (region_id) REFERENCES regions(id);

ALTER TABLE ONLY historic_contract_items
    ADD CONSTRAINT historic_contract_items_historic_contract_id_fkey FOREIGN KEY (historic_contract_id) REFERENCES historic_contracts(id) ON DELETE CASCADE;

ALTER TABLE ONLY historic_contract_items
    ADD CONSTRAINT historic_contract_items_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

ALTER TABLE ONLY historic_contracts
    ADD CONSTRAINT historic_contracts_issuer_id_fkey FOREIGN KEY (issuer_id) REFERENCES characters(id);

ALTER TABLE ONLY modules
    ADD CONSTRAINT modules_latest_contract_id_fkey FOREIGN KEY (latest_contract_id) REFERENCES contracts(id) ON DELETE SET NULL;

ALTER TABLE ONLY public_module_ownerships
    ADD CONSTRAINT public_module_ownerships_contract_id_fkey FOREIGN KEY (contract_id) REFERENCES contracts(id) ON DELETE CASCADE;
