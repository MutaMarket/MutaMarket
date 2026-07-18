-- Per-abyssal-type roll extremes across every producing mutaplasmid,
-- aggregated from mutaplasmid_type_statistics like the legacy
-- AbyssalTypeStatisticsService; served by /api/abyssal-type-statistics.

create table abyssal_type_statistics (
    id bigserial primary key,
    type_id bigint not null references types (id),
    attribute_id bigint not null references attributes (id),
    best double precision not null,
    worst double precision not null,
    high_is_good boolean not null default false,
    is_virtual boolean not null default false,
    unique (type_id, attribute_id)
);
