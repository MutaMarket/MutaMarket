-- The legacy MySQL index inventory, applied to the columns our ported
-- queries actually use. MySQL creates an index for every foreign key
-- implicitly; Postgres does not, so with the production volumes imported
-- (1.75M modules, 8.9M attribute rows) the FK joins and the browse
-- sorts were sequential scans. Builds take a couple of minutes on the
-- imported dataset, once, at migration time.

-- modules: type-scoped browsing, sorts, visibility and the jobs.
create index if not exists modules_type_id_index on modules (type_id);
create index if not exists modules_source_type_id_index on modules (source_type_id);
create index if not exists modules_mutaplasmid_id_index on modules (mutaplasmid_id);
create index if not exists modules_creator_id_index on modules (creator_id);
create index if not exists modules_latest_contract_id_index on modules (latest_contract_id);
create index if not exists modules_estimated_value_index on modules (estimated_value);
create index if not exists modules_average_fraction_index on modules (average_fraction);
create index if not exists modules_estimated_value_updated_at_index
    on modules (estimated_value_updated_at);
-- Legacy had no created_at index because its stat counts were cached
-- hourly; ours computes the added-today strip per request.
create index if not exists modules_created_at_index on modules (created_at);

-- mutated_attributes: the legacy attribute-bounds search composite, the
-- bar marker for the gold/diamond/brown filters and stats, and the FK.
create index if not exists mutated_attributes_type_attribute_value_index
    on mutated_attributes (type_id, attribute_id, value, module_id);
create index if not exists mutated_attributes_bar_index on mutated_attributes (bar);
create index if not exists mutated_attributes_attribute_id_index
    on mutated_attributes (attribute_id);

-- contracts: price sorting, the stats strip and issuer joins.
create index if not exists contracts_unified_price_index on contracts (unified_price);
create index if not exists contracts_abyssal_modules_count_index
    on contracts (abyssal_modules_count);
create index if not exists contracts_date_issued_index on contracts (date_issued);
create index if not exists contracts_date_expired_index on contracts (date_expired);
create index if not exists contracts_issuer_id_index on contracts (issuer_id);

-- historic_contracts: the training sweep and the contract-history and
-- future historic-sales pages.
create index if not exists historic_contracts_status_index on historic_contracts (status);
create index if not exists historic_contracts_date_issued_index
    on historic_contracts (date_issued);
create index if not exists historic_contracts_unified_price_index
    on historic_contracts (unified_price);
create index if not exists historic_contracts_issuer_id_index
    on historic_contracts (issuer_id);
create index if not exists historic_contract_items_type_id_index
    on historic_contract_items (type_id);

-- characters: SSO login (owner hash), the per-request premium check
-- (user_id), name search, and the sync jobs' pending scans.
create index if not exists characters_character_owner_hash_index
    on characters (character_owner_hash);
create index if not exists characters_user_id_index on characters (user_id);
create index if not exists characters_name_index on characters (name);
create index if not exists characters_corporation_id_index on characters (corporation_id);
create index if not exists characters_contracts_fetched_at_index
    on characters (contracts_fetched_at);
create index if not exists characters_name_fetched_at_index
    on characters (name_fetched_at);

-- assets / public_assets: module-to-asset resolution and the FK cascade
-- paths hit by every asset re-import.
create index if not exists assets_item_id_index on assets (item_id);
create index if not exists public_assets_asset_id_index on public_assets (asset_id);
create index if not exists public_assets_public_parent_id_index
    on public_assets (public_parent_id);

-- esi_tokens / collections: per-character token and collection lookups.
create index if not exists esi_tokens_character_id_index on esi_tokens (character_id);
create index if not exists collections_character_id_index on collections (character_id);
create index if not exists collection_modules_module_id_index
    on collection_modules (module_id);

-- training_modules: the sweep's cleanup join and sale-date ordering.
create index if not exists training_modules_historic_contract_id_index
    on training_modules (historic_contract_id);
create index if not exists training_modules_issued_at_index
    on training_modules (issued_at);
