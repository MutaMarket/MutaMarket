-- The value and fraction sorts order with the MySQL-parity nulls
-- placement (asc nulls first / desc nulls last), which a default
-- btree cannot serve in either direction, so both sorts fell back to
-- scanning every module. Rebuilt like the latest_contract_price index:
-- a forward scan is "asc nulls first", a backward scan is "desc nulls
-- last". Range filters on the leading column are unaffected.

drop index modules_estimated_value_index;
create index modules_estimated_value_index
    on modules (estimated_value asc nulls first, id asc nulls first);

drop index modules_average_fraction_index;
create index modules_average_fraction_index
    on modules (average_fraction asc nulls first, id asc nulls first);
