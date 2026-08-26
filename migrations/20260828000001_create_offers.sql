-- Offers and their message threads, the legacy 2024_09_05 offers +
-- messages tables, plus the blocked_users gate (2025_02_24) and the
-- notify_characters pick (2024_08_21) they depend on.
--
-- Deliberate divergence: offers carry an explicit ISK `price`. The
-- legacy dialog asked buyers to type the amount into the message text
-- ("Hey, I can offer you  ISK for it."); the rewrite prompts for the
-- price as a field and keeps the message optional alongside it.

create table offers (
    id bigserial primary key,
    sender_id bigint not null references characters (id) on delete cascade,
    receiver_id bigint not null references characters (id) on delete cascade,
    module_id bigint not null references modules (id) on delete cascade,
    price double precision not null,
    left_by_sender_at timestamptz,
    left_by_receiver_at timestamptz,
    deleted_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index offers_sender_idx on offers (sender_id) where deleted_at is null;
create index offers_receiver_idx on offers (receiver_id) where deleted_at is null;
create index offers_module_idx on offers (module_id);

create table messages (
    id bigserial primary key,
    offer_id bigint not null references offers (id) on delete cascade,
    sender_id bigint not null references characters (id) on delete cascade,
    receiver_id bigint not null references characters (id) on delete cascade,
    content text not null,
    read_at timestamptz,
    notified_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index messages_offer_idx on messages (offer_id, id desc);
create index messages_receiver_unread_idx on messages (receiver_id) where read_at is null;

create table blocked_users (
    id bigserial primary key,
    blocker_id bigint not null references users (id) on delete cascade,
    blocked_id bigint not null references users (id) on delete cascade,
    created_at timestamptz not null default now(),
    unique (blocker_id, blocked_id)
);

create table notify_characters (
    id bigserial primary key,
    user_id bigint not null unique references users (id) on delete cascade,
    character_id bigint not null references characters (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
