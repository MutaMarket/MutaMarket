-- The notification outbox. The legacy EveMail channel sent in-game
-- mails inline (and outside production only logged "Would send mail");
-- here every notification becomes a persisted row and the
-- notification-delivery job decides the transport: real ESI mail in
-- production (NOTIFY_DELIVERY=esi), a simulated delivery everywhere
-- else - inspectable on the admin dashboard and in tests.
create table notification_outbox (
    id bigserial primary key,
    user_id bigint not null references users (id) on delete cascade,
    -- Which notification this is (offer-received, messages-received, ...).
    kind text not null,
    subject text not null,
    body text not null,
    payload jsonb,
    created_at timestamptz not null default now(),
    delivered_at timestamptz,
    -- How it left the outbox: 'esi' or 'simulated'.
    delivery text,
    error text
);

create index notification_outbox_pending_idx on notification_outbox (id) where delivered_at is null;
