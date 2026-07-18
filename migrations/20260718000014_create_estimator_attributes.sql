-- The value estimator backend: the per-type feature list of the ML price
-- models (legacy estimator_attributes table), plus the estimator_statistics
-- columns the legacy grew later (nmae from 2025_04_10_090724, and the
-- created_at/updated_at timestamps its Eloquent model always carried) so
-- /api/estimator-statistics can emit the full legacy key set.

create table estimator_attributes (
    id bigserial primary key,
    type_id bigint not null references types (id) on delete cascade,
    attribute_id bigint not null references attributes (id) on delete cascade,
    -- The legacy table has no unique key (its seeder guards via
    -- firstOrCreate); the constraint makes the ported seeder idempotent.
    unique (type_id, attribute_id)
);

alter table estimator_statistics
    add column nmae double precision,
    add column created_at timestamptz not null default now(),
    add column updated_at timestamptz not null default now();
