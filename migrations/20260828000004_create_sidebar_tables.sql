-- The sidebar's data: bookmarks (2024_09_30), the in-app advertisement
-- rotation (2025_06_03 + the 2026_06_19 scheduling columns) and the
-- recommended-gear affiliate rotation (2026_08_14).

create table bookmarks (
    id bigserial primary key,
    user_id bigint not null references users (id) on delete cascade,
    type_id bigint references types (id) on delete cascade,
    name text not null,
    query text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index bookmarks_user_idx on bookmarks (user_id);

create table advertisements (
    id bigserial primary key,
    name text not null,
    description text,
    image_url text,
    link text,
    active boolean not null default true,
    starts_at timestamptz,
    expires_at timestamptz,
    priority integer not null default 0,
    size text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table gear_items (
    id bigserial primary key,
    name text not null,
    description text,
    image_url text,
    link text not null,
    active boolean not null default true,
    priority integer not null default 0,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
