-- Bookkeeping for the SDE importer: the seeded build number, so repeated
-- bootstrap runs (docker compose up) skip an unchanged SDE.

create table sde_meta (
    key text primary key,
    value text not null
);
