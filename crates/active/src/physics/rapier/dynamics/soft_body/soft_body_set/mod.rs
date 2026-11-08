// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! The `SoftBodySet`: storage of the soft bodies, their insertion and removal, and their clusters.
mod soft_body_set;
mod soft_body_set_clusters;
mod soft_body_set_insert_remove;
mod soft_body_set_proxies;
mod soft_body_set_split;
mod soft_body_set_step_sync;

pub(crate) use self::soft_body_set::SoftBodyIslandEvent;
pub use self::soft_body_set::SoftBodySet;
pub(super) use super::soft_body_builder;
pub(super) use super::{SoftBodyParticleSettings, SoftCollisionMesh};
