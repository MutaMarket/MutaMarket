-- URL slug of an EVE name: lowercase, non-alphanumeric runs collapsed to
-- single dashes. Mirrors the slug format of the legacy app's routes.
create function slug(name text) returns text
language sql immutable
return trim(both '-' from regexp_replace(lower(name), '[^a-z0-9]+', '-', 'g'));

-- Quality metrics of the per-type ML price estimators. Filled by the
-- estimator training pipeline; served by /api/estimator-statistics.

create table estimator_statistics (
    id bigserial primary key,
    type_id bigint not null unique references types (id),
    name text not null,
    data_count bigint not null default 0,
    r2 double precision,
    mae double precision,
    last_trained_at timestamptz,
    data_statistics jsonb
);
