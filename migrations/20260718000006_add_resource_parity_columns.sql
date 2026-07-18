-- Columns and tables the legacy module JSON resources carry: meta group
-- names, character bio/premium data, and the estimator value columns on
-- modules (filled by the estimator milestone, null until then).

create table meta_groups (
    id bigint primary key,
    name text not null default ''
);

alter table characters
    add column description text,
    add column premium_paid_until timestamptz;

alter table modules
    add column estimated_value double precision,
    add column estimated_value_updated_at timestamptz;
