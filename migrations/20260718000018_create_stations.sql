-- NPC stations hosting assets. Legacy seeds these from a static SQL dump;
-- we resolve them natively from ESI's public station endpoint during asset
-- imports (stations are immutable, so fetch-once). Minimal columns for the
-- location display.

create table stations (
    id bigint primary key,
    name text not null,
    type_id bigint,
    solarsystem_id bigint,
    created_at timestamptz not null default now()
);
