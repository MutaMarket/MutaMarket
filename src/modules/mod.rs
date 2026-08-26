//! The module domain: ingesting mutated modules, persisting their computed
//! roll-quality results, and presenting them.

/// Meta level rides on dogma attribute 633 (the legacy meta-level icon
/// id); used by the source-type tables and the filter panel.
pub const META_LEVEL_ATTRIBUTE_ID: i64 = 633;

pub mod ingest;
pub mod link;
pub mod queries;
pub mod search;
pub mod stats;
pub mod view;
