-- EVE reference data (from CCP's SDE via the legacy export for now).
-- Only the columns the mutation math and basic display need; UI-facing
-- columns get added by later migrations as pages are ported.

create table attributes (
    id bigint primary key,
    name text not null,
    high_is_good boolean not null default false,
    derived boolean not null default false,
    derived_operation text,
    derived_attributes bigint[]
);

create table types (
    id bigint primary key,
    name text not null,
    published boolean not null default false
);

create table type_attributes (
    id bigint primary key,
    type_id bigint not null references types (id),
    attribute_id bigint not null references attributes (id),
    value double precision,
    unique (type_id, attribute_id)
);

create table mutaplasmids (
    id bigint primary key,
    name text not null,
    output_type_id bigint not null references types (id)
);

create table mutaplasmid_attributes (
    id bigint primary key,
    mutaplasmid_id bigint not null references mutaplasmids (id),
    attribute_id bigint not null references attributes (id),
    value_min double precision not null,
    value_max double precision not null,
    high_is_good boolean,
    is_virtual boolean not null default false,
    unique (mutaplasmid_id, attribute_id)
);

create table mutaplasmid_input_types (
    id bigint primary key,
    mutaplasmid_id bigint not null references mutaplasmids (id),
    type_id bigint not null references types (id),
    unique (mutaplasmid_id, type_id)
);

create table mutaplasmid_type_statistics (
    id bigint primary key,
    type_id bigint not null references types (id),
    mutaplasmid_id bigint not null references mutaplasmids (id),
    attribute_id bigint not null references attributes (id),
    best double precision not null,
    worst double precision not null,
    high_is_good boolean not null default false,
    is_virtual boolean not null default false,
    unique (type_id, mutaplasmid_id, attribute_id)
);
