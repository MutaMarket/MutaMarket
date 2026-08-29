-- The raffle prize pool, the legacy raffle_items table in its final
-- shape (the later unique-code migration is folded in). Status values:
-- 0 paid out, 1 pending, 2 active (drawn, awaiting claim), 3 claimed
-- (see raffles::STATUS_*); new items start pending.
CREATE TABLE raffle_items (
    id bigserial NOT NULL,
    winner_id bigint,
    type_id bigint,
    name text,
    description text,
    icon_url text,
    quantity integer DEFAULT 1 NOT NULL,
    code text NOT NULL,
    status integer DEFAULT 1 NOT NULL,
    expires_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY raffle_items
    ADD CONSTRAINT raffle_items_pkey PRIMARY KEY (id);

ALTER TABLE ONLY raffle_items
    ADD CONSTRAINT raffle_items_code_unique UNIQUE (code);

ALTER TABLE ONLY raffle_items
    ADD CONSTRAINT raffle_items_winner_id_fkey FOREIGN KEY (winner_id) REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL;

ALTER TABLE ONLY raffle_items
    ADD CONSTRAINT raffle_items_type_id_fkey FOREIGN KEY (type_id) REFERENCES types(id);

CREATE INDEX raffle_items_winner_id_index ON raffle_items USING btree (winner_id);
