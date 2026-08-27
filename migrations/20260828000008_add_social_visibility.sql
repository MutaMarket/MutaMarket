-- The settings page's show-on-profiles toggles (legacy users table
-- columns discord_is_public / twitch_is_public / patreon_is_public,
-- default hidden).

alter table users
    add column discord_is_public boolean not null default false,
    add column twitch_is_public boolean not null default false,
    add column patreon_is_public boolean not null default false;
