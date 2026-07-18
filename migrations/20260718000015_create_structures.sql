-- Player-owned structures resolved from ESI, ported from the legacy
-- structures + character_structure tables: id stubs from the public list
-- or asset locations, names filled per character via the structures scope,
-- with the per-character resolution outcome recorded. (The legacy
-- position_x/y/z and the stations SDE import are not ported yet: nothing
-- ported so far reads them.)

create table structures (
    id bigint primary key,
    -- Null until a character with access resolved the structure.
    name text,
    owner_id bigint,
    -- No FKs: structure hull types and solar systems are outside the
    -- minimal SDE import.
    type_id bigint,
    solarsystem_id bigint,
    last_fetched_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index structures_name_index on structures (name);

-- Which characters could (or could not) resolve a structure: docking
-- access is per character, so failures are only meaningful per character.
create table character_structure (
    id bigserial primary key,
    character_id bigint not null references characters (id) on delete cascade,
    structure_id bigint not null references structures (id) on delete cascade,
    could_resolve boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (character_id, structure_id)
);
