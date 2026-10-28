//! The integration suites that change process env vars. Writing the
//! environment while another thread reads it is undefined behaviour, so
//! they stay out of the parallel `integration` binary and each test here
//! holds `common::lock_env` for its whole run.

#[path = "../integration/common/mod.rs"]
mod common;

mod discord_invites;
mod discord_notifications;
mod esi_buckets;
mod esi_failures;
mod og;
