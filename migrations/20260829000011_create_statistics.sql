CREATE FUNCTION contracts_propagate_unified_price() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
begin
    update modules set latest_contract_price = new.unified_price
    where latest_contract_id = new.id
      and latest_contract_price is distinct from new.unified_price;
    return null;
end;
$$;

CREATE FUNCTION modules_copy_latest_contract_price() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
begin
    if new.latest_contract_id is null then
        new.latest_contract_price := null;
    else
        select unified_price into new.latest_contract_price
        from contracts where id = new.latest_contract_id;
    end if;
    return new;
end;
$$;

CREATE MATERIALIZED VIEW statistics_creator_type_counts AS
 SELECT creator_id,
    type_id,
    count(*) AS modules_created_count
   FROM modules
  WHERE (creator_id IS NOT NULL)
  GROUP BY creator_id, type_id
  -- Populated at creation: REFRESH CONCURRENTLY refuses unpopulated views.
  WITH DATA;

CREATE MATERIALIZED VIEW statistics_overview AS
 SELECT (1)::bigint AS id,
    ( SELECT count(*) AS count
           FROM modules) AS total_count,
    ( SELECT count(*) AS count
           FROM modules
          WHERE (modules.latest_contract_id IS NOT NULL)) AS listed_count,
    ( SELECT count(*) AS count
           FROM modules
          WHERE (modules.created_at >= (now() - '01:00:00'::interval))) AS added_last_hour_count,
    ( SELECT count(*) AS count
           FROM modules
          WHERE (modules.created_at >= (now() - '1 day'::interval))) AS added_last_day_count,
    ( SELECT count(*) AS count
           FROM modules
          WHERE (modules.created_at >= (now() - '7 days'::interval))) AS added_last_week_count,
    ( SELECT count(*) AS count
           FROM contracts
          WHERE (contracts.abyssal_modules_count > 0)) AS contracts_count,
    ( SELECT count(*) AS count
           FROM contracts
          WHERE ((contracts.type = 'item_exchange'::text) AND (contracts.abyssal_modules_count > 0))) AS item_exchanges_count,
    ( SELECT count(*) AS count
           FROM contracts
          WHERE ((contracts.type = 'auction'::text) AND (contracts.abyssal_modules_count > 0))) AS auctions_count,
    ( SELECT count(DISTINCT a.module_id) AS count
           FROM mutated_attributes a
          WHERE (a.bar = 1)) AS goldbars_count,
    ( SELECT count(DISTINCT a.module_id) AS count
           FROM mutated_attributes a
          WHERE (a.bar = '-1'::integer)) AS brownbars_count,
    ( SELECT count(DISTINCT a.module_id) AS count
           FROM mutated_attributes a
          WHERE (a.bar = 2)) AS diamondbars_count,
    ( SELECT COALESCE(sum(modules.estimated_value), (0)::double precision) AS "coalesce"
           FROM modules) AS total_value,
    ( SELECT COALESCE(avg(modules.estimated_value), (0)::double precision) AS "coalesce"
           FROM modules) AS average_value,
    ( SELECT count(DISTINCT modules.creator_id) AS count
           FROM modules
          WHERE (modules.creator_id IS NOT NULL)) AS creators_count,
    ( SELECT count(*) AS count
           FROM characters) AS characters_count,
    now() AS refreshed_at
  -- Populated at creation: REFRESH CONCURRENTLY refuses unpopulated views.
  WITH DATA;

CREATE UNIQUE INDEX statistics_creator_type_counts_key ON statistics_creator_type_counts USING btree (creator_id, type_id);

CREATE INDEX statistics_creator_type_counts_type ON statistics_creator_type_counts USING btree (type_id);

CREATE UNIQUE INDEX statistics_overview_id ON statistics_overview USING btree (id);

CREATE TRIGGER contracts_propagate_unified_price AFTER UPDATE OF unified_price ON contracts FOR EACH ROW EXECUTE FUNCTION contracts_propagate_unified_price();

CREATE TRIGGER modules_copy_latest_contract_price BEFORE INSERT OR UPDATE OF latest_contract_id ON modules FOR EACH ROW EXECUTE FUNCTION modules_copy_latest_contract_price();
