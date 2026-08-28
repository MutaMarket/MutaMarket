-- Personal module notes and per-collection notes, ported from the legacy
-- notes (2024_11_09_140003) and collection_notes (2024_11_15_144245)
-- tables. Like legacy, notes.module_id carries no delete cascade while
-- collection_notes.module_id does.

create table notes (
    id bigserial primary key,
    user_id bigint not null references users (id) on delete cascade,
    module_id bigint not null references modules (id),
    content text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (user_id, module_id)
);

create table collection_notes (
    id bigserial primary key,
    collection_id bigint not null references collections (id) on delete cascade,
    user_id bigint not null references users (id) on delete cascade,
    module_id bigint not null references modules (id) on delete cascade,
    content text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (collection_id, module_id)
);
