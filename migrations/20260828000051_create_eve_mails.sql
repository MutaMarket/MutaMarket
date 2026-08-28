-- EVE mail ingestion, the legacy eve_mails / eve_mail_recipients /
-- eve_mail_module tables: mails received by the service character, the
-- characters involved in them, and the abyssal modules linked in their
-- bodies (the mail-based appraisal flow).
create table eve_mails (
    id bigint primary key,
    character_id bigint not null references characters (id) on delete cascade,
    is_read boolean not null default false,
    subject text not null,
    timestamp timestamptz not null,
    body text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index eve_mails_is_read_idx on eve_mails (is_read);
create index eve_mails_timestamp_idx on eve_mails (timestamp);

create table eve_mail_recipients (
    id bigserial primary key,
    eve_mail_id bigint not null references eve_mails (id) on delete cascade,
    character_id bigint not null references characters (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (eve_mail_id, character_id)
);

create table eve_mail_module (
    id bigserial primary key,
    eve_mail_id bigint not null references eve_mails (id) on delete cascade,
    module_id bigint not null references modules (id) on delete cascade,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    unique (eve_mail_id, module_id)
);

-- Modules-processed replies address the mail's sender, who need not be
-- a MutaMarket user: outbox rows may now target a character directly
-- (user_id stays for user-addressed notifications).
alter table notification_outbox
    alter column user_id drop not null,
    add column recipient_character_id bigint references characters (id) on delete cascade;
