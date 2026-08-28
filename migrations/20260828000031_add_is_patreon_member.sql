-- The Patreon membership flag maintained by the subscriber sync, the
-- legacy 2025_04_03_132218_add_is_patreon_member_column_to_users_table
-- (both indexes included).

alter table users
    add column is_patreon_member boolean not null default false;

create index users_is_patreon_member_index on users (is_patreon_member);
create index users_patreon_id_index on users (patreon_id);
