-- The premium-expiry sweep filters on premium_paid_until every five
-- minutes, while characters is dominated by stub rows whose value is
-- null (contract issuers, wallet parties, mail correspondents). The
-- partial index stays premium-holders-only and keeps the sweep off a
-- full table scan.
create index characters_premium_paid_until_index
    on characters (premium_paid_until)
    where premium_paid_until is not null;
