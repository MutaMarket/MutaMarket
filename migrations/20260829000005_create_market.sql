CREATE TABLE market_histories (
    id bigserial NOT NULL,
    type_id bigint NOT NULL,
    region_id bigint NOT NULL,
    date date NOT NULL,
    average double precision NOT NULL,
    highest double precision NOT NULL,
    lowest double precision NOT NULL,
    order_count bigint DEFAULT 0 NOT NULL,
    volume bigint DEFAULT 0 NOT NULL
);

ALTER TABLE ONLY market_histories
    ADD CONSTRAINT market_histories_pkey PRIMARY KEY (id);

ALTER TABLE ONLY market_histories
    ADD CONSTRAINT market_histories_type_id_region_id_date_key UNIQUE (type_id, region_id, date);

ALTER TABLE ONLY market_histories
    ADD CONSTRAINT market_histories_region_id_fkey FOREIGN KEY (region_id) REFERENCES regions(id);

ALTER TABLE ONLY market_histories
    ADD CONSTRAINT market_histories_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);
