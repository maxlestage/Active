// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
pub use manager::IslandManager;

pub(crate) use island::Island;
pub(crate) use persistent::{INVALID_ISLAND, ImpulseJointIslandEvent, PersistentIslands};
pub(crate) use substep_groups::SolveGroup;

mod global_split;
mod island;
mod local_split;
mod manager;
mod persistent;
mod sleep;
mod substep_groups;
