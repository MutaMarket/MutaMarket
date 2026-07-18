-- Crash-safe item ingestion: a contract's items are fetched until this is
-- set, so a crash between the contract upsert and the item sync just means
-- a retry on the next cycle instead of a permanently itemless contract.

alter table contracts
    add column items_synced_at timestamptz;
