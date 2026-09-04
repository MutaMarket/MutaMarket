-- The contract-review page picks a random unknown-status, single-abyssal
-- item-exchange contract. A partial index on exactly that predicate turns
-- the scan into an index-only scan over the eligible rows, so the base
-- page no longer hashes every historic contract item (1.8s -> a few ms).
CREATE INDEX historic_contracts_reviewable_idx
    ON historic_contracts (id)
    WHERE type = 'item_exchange'
      AND status = 'unknown'
      AND abyssal_modules_count = 1
      AND non_abyssal_modules_count = 0;
