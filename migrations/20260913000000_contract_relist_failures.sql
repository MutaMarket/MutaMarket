-- The later contract that put one of this contract's modules back on the
-- market under the same seller: proof the item came back, so this
-- contract never completed. Points at a public or an archived contract
-- (ids are shared and rows move between the two tables), so no foreign
-- key.
ALTER TABLE historic_contracts ADD COLUMN relisted_by_contract_id bigint;
