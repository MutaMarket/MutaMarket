-- Mutated ("abyssal") modules and their computed per-attribute results.
-- The id is the EVE item id. creator_id stays a plain column until the
-- characters table lands; contract and estimator columns come with their
-- features.

create table modules (
    id bigint primary key,
    type_id bigint not null references types (id),
    source_type_id bigint references types (id),
    mutaplasmid_id bigint references mutaplasmids (id),
    creator_id bigint,
    average_fraction double precision,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table mutated_attributes (
    id bigserial primary key,
    module_id bigint not null references modules (id) on delete cascade,
    attribute_id bigint not null references attributes (id),
    type_id bigint not null references types (id),
    value double precision not null,
    base_value double precision not null,
    -- Roll quality -1..1 within the module's own mutaplasmid.
    fraction double precision not null,
    -- Within all mutaplasmids producing the abyssal type.
    fraction_type double precision not null,
    -- Within all (source type x mutaplasmid) combinations, 0..1.
    fraction_absolute double precision not null,
    -- -1 brown bar, 0 none, 1 gold bar, 2 diamond bar.
    bar smallint not null default 0,
    is_virtual boolean not null default false,
    unique (module_id, attribute_id)
);
