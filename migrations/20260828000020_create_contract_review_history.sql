-- Audit rows of the moderator contract review (the legacy
-- contract_review_history table): who folded an unknown historic
-- contract into which status. The legacy ip_address column is not
-- ported: the controller never wrote it.

create table contract_review_history (
    id bigserial primary key,
    historic_contract_id bigint not null
        references historic_contracts (id) on delete cascade,
    user_id bigint not null references users (id) on delete cascade,
    previous_status text,
    new_status text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index contract_review_history_contract_index
    on contract_review_history (historic_contract_id, created_at);
create index contract_review_history_user_index
    on contract_review_history (user_id, created_at);
