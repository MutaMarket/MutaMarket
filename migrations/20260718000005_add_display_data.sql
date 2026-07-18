-- Display data for rendering modules: dogma units (from the SDE's
-- dogmaUnits.jsonl plus app-defined units for derived attributes), display
-- names and units on attributes, and the meta group of types (drives the
-- card accent color).

create table units (
    id bigint primary key,
    name text not null default '',
    display_name text not null default ''
);

alter table attributes
    add column display_name text not null default '',
    add column unit_id bigint references units (id);

alter table types
    add column meta_group_id bigint;
