-- Accounts, EVE characters, their SSO tokens and server-side sessions.
-- Users have no password: authentication is EVE SSO only, identity per
-- character is tracked via the owner hash.

create table users (
    id bigserial primary key,
    name text not null,
    is_admin boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table characters (
    id bigint primary key,
    name text not null default '',
    corporation_id bigint,
    alliance_id bigint,
    user_id bigint references users (id) on delete set null,
    character_owner_hash text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table esi_tokens (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    access_token text not null,
    refresh_token text not null,
    token_type text not null default 'Bearer',
    character_owner_hash text not null,
    scopes text[] not null default '{}',
    expires_at timestamptz not null,
    created_at timestamptz not null default now()
);

create table sessions (
    token text primary key,
    user_id bigint not null references users (id) on delete cascade,
    active_character_id bigint references characters (id) on delete set null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null
);

-- Module creators are characters. Backfill stub rows for already-ingested
-- modules, then enforce the relation like the legacy schema does.
insert into characters (id, name)
select distinct creator_id, ''
from modules
where creator_id is not null
on conflict (id) do nothing;

alter table modules
    add constraint modules_creator_id_fkey
    foreign key (creator_id) references characters (id);
