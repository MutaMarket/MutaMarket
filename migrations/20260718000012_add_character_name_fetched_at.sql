-- Character name sync state, like the legacy characters.name_fetched_at:
-- stub characters (created with an empty name from contract issuers and
-- module creators) are named from ESI until this is set; permanently
-- unresolvable ids get stamped without a name so they are not retried.

alter table characters
    add column name_fetched_at timestamptz;
