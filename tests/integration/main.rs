//! The single integration-test binary: every suite lives here as a module
//! so a full `cargo test` links (and macOS Gatekeeper-scans) one
//! executable instead of ~65.
//!
//! The suites assume they never run concurrently with each other: they
//! clean shared tables (`delete from users`, ...) and a few mutate
//! process env vars. `.cargo/config.toml` pins `RUST_TEST_THREADS=1` to
//! keep that assumption true inside this shared process.

mod common;

mod abyssal_statistics;
mod account_characters;
mod admin_activity;
mod admin_scheduler;
mod admin_scopes;
mod alliances;
mod appraise;
mod assets;
mod blocked_users;
mod calculator;
mod character_contracts;
mod characters;
mod collection_auto_sync;
mod collection_locations;
mod collections;
mod contracts;
mod cross_site;
mod discord_invites;
mod discord_notifications;
mod display;
mod docs;
mod donations;
mod esi_failures;
mod estimator;
mod estimator_forest;
mod estimator_training;
mod eve_mails;
mod historic_sales;
mod i18n;
mod linked_accounts;
mod locations;
mod market_histories;
mod moderator_contracts;
mod module_api;
mod module_import;
mod module_ingestion;
mod module_parsing;
mod module_pricing;
mod nav;
mod notes;
mod offers;
mod og;
mod og_service;
mod patreon;
mod personal_contracts;
mod personal_modules;
mod premium_page;
mod public_api;
mod public_assets;
mod raffles;
mod reference_db;
mod route_guards;
mod routes;
mod sde_meta;
mod sde_pipeline;
mod search;
mod sell;
mod settings;
mod sidebar;
mod sitemap;
mod sso;
mod statistics;
mod structures;
mod tokens;
mod ui_contract;
mod workbench;
mod ws;
