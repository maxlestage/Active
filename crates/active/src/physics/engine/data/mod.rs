// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! Data structures modified with guaranteed deterministic behavior after deserialization.

pub use self::arena::Arena;
pub use self::arena::Index;
pub use self::coarena::Coarena;
pub use self::modified_objects::{HasModifiedFlag, ModifiedObjects};

pub mod arena;
mod coarena;
pub(crate) mod graph;
mod modified_objects;
pub mod pubsub;
pub(crate) mod union_find;
