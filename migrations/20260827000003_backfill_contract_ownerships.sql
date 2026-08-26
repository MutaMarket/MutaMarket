-- Backfills the contract-sourced ownership rows the ingestion missed
-- between the legacy cutover and the port of the legacy
-- after_public_contract_item trigger (the equivalent of the legacy
-- seed_public_ownerships_table.sql contract half).
insert into public_module_ownerships (character_id, module_id, contract_id, created_at)
select ct.issuer_id, ci.item_id, ci.contract_id, coalesce(ct.created_at, now())
from contract_items ci
join contracts ct on ct.id = ci.contract_id
where ci.item_id is not null
  and exists (select 1 from modules where id = ci.item_id)
  and exists (select 1 from characters where id = ct.issuer_id)
on conflict (character_id, module_id) do nothing;
