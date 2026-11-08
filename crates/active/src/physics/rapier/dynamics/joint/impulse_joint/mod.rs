// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
pub use self::impulse_joint::ImpulseJoint;
pub use self::impulse_joint_handle::ImpulseJointHandle;
pub use self::impulse_joint_set::ImpulseJointSet;
pub(crate) use self::impulse_joint_set::{JointGraphEdge, JointIndex};

mod impulse_joint;
mod impulse_joint_handle;
mod impulse_joint_set;
