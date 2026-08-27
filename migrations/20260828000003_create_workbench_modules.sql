-- The workbench, the legacy 2024_10_24 workbench_modules table: a
-- per-user scratch set of modules to compare and share.
create table workbench_modules (
    id bigserial primary key,
    user_id bigint not null references users (id) on delete cascade,
    module_id bigint not null references modules (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (user_id, module_id)
);
