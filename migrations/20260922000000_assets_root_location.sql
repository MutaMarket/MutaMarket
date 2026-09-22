-- The station or structure an asset ultimately sits in, denormalized
-- from the container chain.
--
-- The `in-jita` filter only ever matched the current contract's start
-- station, so every listing without a live contract dropped out of the
-- result however close to Jita 4-4 it physically was: on 2026-09-22 that
-- was 140,913 published modules sitting in Jita 4-4 with no contract
-- (GitHub issue #67). Matching them needs the root of each container
-- chain, and resolving that per query is not affordable: walking down
-- from Jita 4-4 across `assets` takes 5.6 s, climbing up from the
-- published listings 0.5 s. The import writes the resolved root instead.
ALTER TABLE assets ADD COLUMN root_location_id bigint;

-- Backfill by lifting each row to its parent's location until the chain
-- leaves the table, which is far cheaper than a recursive walk per row.
-- The bound matches MAX_CONTAINER_DEPTH in src/assets/mod.rs and only
-- guards against a cycle in imported data.
DO $$
DECLARE
    lifted bigint;
BEGIN
    UPDATE assets SET root_location_id = location_id;
    FOR _ in 1..16 LOOP
        UPDATE assets a
           SET root_location_id = p.location_id
          FROM assets p
         WHERE p.character_id = a.character_id
           AND p.item_id = a.root_location_id;
        GET DIAGNOSTICS lifted = ROW_COUNT;
        EXIT WHEN lifted = 0;
    END LOOP;
END $$;

-- The listing query probes by module id and tests the root, so the id
-- leads.
CREATE INDEX assets_item_root_location_index
    ON assets (item_id, root_location_id);
