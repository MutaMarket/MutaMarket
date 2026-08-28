CREATE TABLE users (
    id bigserial NOT NULL,
    name text NOT NULL,
    is_admin boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    discord_id bigint,
    discord_name text,
    discord_avatar text,
    discord_channel_id bigint,
    twitch_id bigint,
    twitch_name text,
    twitch_avatar text,
    twitch_email text,
    patreon_id bigint,
    patreon_name text,
    patreon_avatar text,
    patreon_email text,
    patreon_nickname text,
    discord_is_public boolean DEFAULT false NOT NULL,
    twitch_is_public boolean DEFAULT false NOT NULL,
    patreon_is_public boolean DEFAULT false NOT NULL,
    is_patreon_member boolean DEFAULT false NOT NULL
);

ALTER TABLE ONLY users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);

CREATE INDEX users_discord_channel_id_index ON users USING btree (discord_channel_id);

CREATE INDEX users_discord_id_index ON users USING btree (discord_id);

CREATE INDEX users_is_patreon_member_index ON users USING btree (is_patreon_member);

CREATE INDEX users_patreon_id_index ON users USING btree (patreon_id);

CREATE INDEX users_twitch_id_index ON users USING btree (twitch_id);

CREATE TABLE characters (
    id bigint NOT NULL,
    name text DEFAULT ''::text NOT NULL,
    corporation_id bigint,
    alliance_id bigint,
    user_id bigint,
    character_owner_hash text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    description text,
    premium_paid_until timestamp with time zone,
    name_fetched_at timestamp with time zone,
    contracts_fetched_at timestamp with time zone,
    latest_asset_import_id bigint,
    premium_paid_total double precision DEFAULT 0 NOT NULL,
    premium_payment_rest double precision DEFAULT 0 NOT NULL
);

ALTER TABLE ONLY characters
    ADD CONSTRAINT characters_pkey PRIMARY KEY (id);

CREATE INDEX characters_character_owner_hash_index ON characters USING btree (character_owner_hash);

CREATE INDEX characters_contracts_fetched_at_index ON characters USING btree (contracts_fetched_at);

CREATE INDEX characters_corporation_id_index ON characters USING btree (corporation_id);

CREATE INDEX characters_name_fetched_at_index ON characters USING btree (name_fetched_at);

CREATE INDEX characters_name_index ON characters USING btree (name);

CREATE INDEX characters_premium_paid_until_index ON characters USING btree (premium_paid_until) WHERE (premium_paid_until IS NOT NULL);

CREATE INDEX characters_user_id_index ON characters USING btree (user_id);

CREATE TABLE sessions (
    token text NOT NULL,
    user_id bigint NOT NULL,
    active_character_id bigint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL
);

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_pkey PRIMARY KEY (token);

CREATE TABLE esi_tokens (
    id bigserial NOT NULL,
    character_id bigint NOT NULL,
    access_token text NOT NULL,
    refresh_token text NOT NULL,
    token_type text DEFAULT 'Bearer'::text NOT NULL,
    character_owner_hash text NOT NULL,
    scopes text[] DEFAULT '{}'::text[] NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY esi_tokens
    ADD CONSTRAINT esi_tokens_pkey PRIMARY KEY (id);

CREATE INDEX esi_tokens_character_id_index ON esi_tokens USING btree (character_id);

CREATE TABLE notify_characters (
    id bigserial NOT NULL,
    user_id bigint NOT NULL,
    character_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY notify_characters
    ADD CONSTRAINT notify_characters_pkey PRIMARY KEY (id);

ALTER TABLE ONLY notify_characters
    ADD CONSTRAINT notify_characters_user_id_key UNIQUE (user_id);

CREATE TABLE blocked_users (
    id bigserial NOT NULL,
    blocker_id bigint NOT NULL,
    blocked_id bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE ONLY blocked_users
    ADD CONSTRAINT blocked_users_blocker_id_blocked_id_key UNIQUE (blocker_id, blocked_id);

ALTER TABLE ONLY blocked_users
    ADD CONSTRAINT blocked_users_pkey PRIMARY KEY (id);

ALTER TABLE ONLY blocked_users
    ADD CONSTRAINT blocked_users_blocked_id_fkey FOREIGN KEY (blocked_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY blocked_users
    ADD CONSTRAINT blocked_users_blocker_id_fkey FOREIGN KEY (blocker_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY characters
    ADD CONSTRAINT characters_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE ONLY esi_tokens
    ADD CONSTRAINT esi_tokens_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY notify_characters
    ADD CONSTRAINT notify_characters_character_id_fkey FOREIGN KEY (character_id) REFERENCES characters(id) ON DELETE CASCADE;

ALTER TABLE ONLY notify_characters
    ADD CONSTRAINT notify_characters_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_active_character_id_fkey FOREIGN KEY (active_character_id) REFERENCES characters(id) ON DELETE SET NULL;

ALTER TABLE ONLY sessions
    ADD CONSTRAINT sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
