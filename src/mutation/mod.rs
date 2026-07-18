//! The roll-quality math for mutated ("abyssal") modules, ported from the
//! legacy Laravel services in `app/Services/Modules/`. The behavior is pinned
//! by the legacy characterization fixtures (`tests/fixtures/module_parsing`),
//! so any change here must keep those snapshots passing.

pub mod calculator;
pub mod context;
#[cfg(feature = "ssr")]
pub mod reference;
