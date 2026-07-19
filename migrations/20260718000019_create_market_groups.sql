-- Market group hierarchy, imported for the legacy nameable-type filter:
-- only types under the Ships/Containers market groups get asset names
-- requested from ESI.

create table market_groups (
    id bigint primary key,
    parent_id bigint
);

alter table types
    add column market_group_id bigint;
