//! The roll-quality math for mutated ("abyssal") modules, ported from the
//! legacy Laravel services in `app/Services/Modules/`. The behavior is pinned
//! by the legacy characterization fixtures (`tests/fixtures/module_parsing`),
//! so any change here must keep those snapshots passing.
//!
//! The pipeline: a [`context::MutationContext`] carries all reference data
//! for one (mutaplasmid, source type) combination, and [`calculator::calculate`]
//! turns a module's raw ESI dogma attributes into one
//! [`calculator::AttributeMutationResult`] per rollable attribute.

pub mod calculator;
pub mod context;
pub mod reference;

mod bars;
mod derived;
pub(crate) mod fractions;

pub use bars::AttributeBar;
pub use calculator::{AttributeMutationResult, DogmaAttribute, average_fraction, calculate};
