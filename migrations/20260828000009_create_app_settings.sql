-- Admin-configured application settings (key/value), starting with the
-- service character: the ESI character used for structure resolution
-- and, when those features land, donation and wallet processing.
-- Replaces hand-set env configuration (EVE_STRUCTURES_CHARACTER_ID
-- stays as a fallback).

create table app_settings (
    key text primary key,
    value text not null,
    updated_at timestamptz not null default now()
);
