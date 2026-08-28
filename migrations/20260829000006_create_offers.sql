CREATE TABLE offers (
    id bigserial NOT NULL,
    sender_id bigint NOT NULL,
    receiver_id bigint NOT NULL,
    module_id bigint NOT NULL,
    price double precision NOT NULL,
    left_by_sender_at timestamp with time zone,
    left_by_receiver_at timestamp with time zone,
    deleted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY offers
    ADD CONSTRAINT offers_pkey PRIMARY KEY (id);

CREATE INDEX offers_module_idx ON offers USING btree (module_id);

CREATE INDEX offers_receiver_idx ON offers USING btree (receiver_id) WHERE (deleted_at IS NULL);

CREATE INDEX offers_sender_idx ON offers USING btree (sender_id) WHERE (deleted_at IS NULL);

CREATE TABLE messages (
    id bigserial NOT NULL,
    offer_id bigint NOT NULL,
    sender_id bigint NOT NULL,
    receiver_id bigint NOT NULL,
    content text NOT NULL,
    read_at timestamp with time zone,
    notified_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY messages
    ADD CONSTRAINT messages_pkey PRIMARY KEY (id);

CREATE INDEX messages_offer_idx ON messages USING btree (offer_id, id DESC);

CREATE INDEX messages_receiver_unread_idx ON messages USING btree (receiver_id) WHERE (read_at IS NULL);

CREATE TABLE notification_outbox (
    id bigserial NOT NULL,
    user_id bigint,
    kind text NOT NULL,
    subject text NOT NULL,
    body text NOT NULL,
    payload jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    delivered_at timestamp with time zone,
    delivery text,
    error text,
    recipient_character_id bigint
);

ALTER TABLE ONLY notification_outbox
    ADD CONSTRAINT notification_outbox_pkey PRIMARY KEY (id);

CREATE INDEX notification_outbox_pending_idx ON notification_outbox USING btree (id) WHERE (delivered_at IS NULL);

ALTER TABLE ONLY messages
    ADD CONSTRAINT messages_offer_id_fkey FOREIGN KEY (offer_id) REFERENCES offers(id) ON DELETE CASCADE;

ALTER TABLE ONLY messages
    ADD CONSTRAINT messages_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY messages
    ADD CONSTRAINT messages_sender_id_fkey FOREIGN KEY (sender_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY notification_outbox
    ADD CONSTRAINT notification_outbox_recipient_character_id_fkey FOREIGN KEY (recipient_character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY notification_outbox
    ADD CONSTRAINT notification_outbox_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY offers
    ADD CONSTRAINT offers_module_id_fkey FOREIGN KEY (module_id) REFERENCES modules(id) ON DELETE CASCADE;

ALTER TABLE ONLY offers
    ADD CONSTRAINT offers_receiver_id_fkey FOREIGN KEY (receiver_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY offers
    ADD CONSTRAINT offers_sender_id_fkey FOREIGN KEY (sender_id) REFERENCES characters(id) ON DELETE CASCADE;
