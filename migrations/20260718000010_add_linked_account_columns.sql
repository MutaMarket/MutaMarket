-- Linked Twitch / Discord / Patreon accounts on users, mirroring the
-- legacy users-table columns the link callbacks write. The legacy
-- `*_is_public` visibility toggles arrive with the settings feature that
-- uses them. Indexes match the legacy schema (discord_id,
-- discord_channel_id and twitch_id are indexed there; patreon_id is not).

alter table users
    add column discord_id bigint,
    add column discord_name text,
    add column discord_avatar text,
    add column discord_channel_id bigint,
    add column twitch_id bigint,
    add column twitch_name text,
    add column twitch_avatar text,
    add column twitch_email text,
    add column patreon_id bigint,
    add column patreon_name text,
    add column patreon_avatar text,
    add column patreon_email text,
    add column patreon_nickname text;

create index users_discord_id_index on users (discord_id);
create index users_discord_channel_id_index on users (discord_channel_id);
create index users_twitch_id_index on users (twitch_id);
