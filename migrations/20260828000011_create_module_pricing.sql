-- Per-user asking prices for modules, ported from the legacy
-- module_pricing table (2024_12_09_165810). Legacy stored decimal(50, 2);
-- the price rides as double precision here like offers.price (documented
-- divergence: ISK amounts fit comfortably and sqlx binds f64 natively).

create table module_pricing (
    id bigserial primary key,
    module_id bigint not null references modules (id),
    user_id bigint not null references users (id) on delete cascade,
    price double precision not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (module_id, user_id)
);
