-- Price sorting joined contracts per request and sorted ~14k visible
-- modules with no usable index (~350ms). The latest contract's unified
-- price is denormalized onto modules and kept in sync by triggers, the
-- legacy precedent for denormalized lookups (public_module_ownerships
-- was trigger-maintained too):
--   1. modules_copy_latest_contract_price fires whenever
--      latest_contract_id changes, including the ON DELETE SET NULL
--      clear when a contract row is removed.
--   2. contracts_propagate_unified_price follows bid-driven price
--      updates on live contracts.

alter table modules add column latest_contract_price double precision;

create function modules_copy_latest_contract_price() returns trigger as $$
begin
    if new.latest_contract_id is null then
        new.latest_contract_price := null;
    else
        select unified_price into new.latest_contract_price
        from contracts where id = new.latest_contract_id;
    end if;
    return new;
end;
$$ language plpgsql;

create trigger modules_copy_latest_contract_price
    before insert or update of latest_contract_id on modules
    for each row execute function modules_copy_latest_contract_price();

create function contracts_propagate_unified_price() returns trigger as $$
begin
    update modules set latest_contract_price = new.unified_price
    where latest_contract_id = new.id
      and latest_contract_price is distinct from new.unified_price;
    return null;
end;
$$ language plpgsql;

create trigger contracts_propagate_unified_price
    after update of unified_price on contracts
    for each row execute function contracts_propagate_unified_price();

update modules m
set latest_contract_price = c.unified_price
from contracts c
where c.id = m.latest_contract_id;

-- One index serves both directions: a forward scan is the MySQL-parity
-- "asc nulls first", a backward scan is "desc nulls last".
create index modules_latest_contract_price_index
    on modules (latest_contract_price asc nulls first, id asc nulls first);
