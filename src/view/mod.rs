//! Serializable view DTOs shared between the server handlers and the
//! frontend-facing payloads. They live outside the feature-gated domain
//! modules so both server and client builds can name them.

pub mod docs;
pub mod nav;
pub mod offers;
pub mod personal;
pub mod public_api;
pub mod social;
