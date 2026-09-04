-- characters.latest_asset_import_id references asset_imports with ON
-- DELETE SET NULL, so every deleted import row makes Postgres look for
-- the characters pointing at it. Without an index that is a sequential
-- scan of all characters per deleted row, which made pruning the
-- historic import backlog crawl and would tax every import's prune.
CREATE INDEX characters_latest_asset_import_id_index
    ON characters (latest_asset_import_id);
