//! The module domain: ingesting mutated modules, persisting their computed
//! roll-quality results, and presenting them.

#[cfg(feature = "ssr")]
pub mod ingest;
pub mod link;
#[cfg(feature = "ssr")]
pub mod queries;
pub mod view;
