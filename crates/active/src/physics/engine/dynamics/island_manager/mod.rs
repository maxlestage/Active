// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
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
