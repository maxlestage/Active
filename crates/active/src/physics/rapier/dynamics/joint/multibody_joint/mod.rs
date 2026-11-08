// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! MultibodyJoints using the reduced-coordinates formalism or using constraints.

pub use self::multibody::{Multibody, MultibodyDofCoupling};
pub use self::multibody_ik::InverseKinematicsOption;
pub use self::multibody_joint::MultibodyJoint;
pub use self::multibody_joint_handle::{MultibodyIndex, MultibodyJointHandle};
pub use self::multibody_joint_set::{MultibodyJointSet, MultibodyLinkId};
pub use self::multibody_link::MultibodyLink;
pub use self::unit_multibody_joint::{
    unit_joint_friction_constraint, unit_joint_limit_constraint, unit_joint_motor_constraint,
};

mod multibody;
mod multibody_joint_handle;
mod multibody_joint_set;
mod multibody_link;
mod multibody_workspace;

mod multibody_ik;
mod multibody_joint;
mod unit_multibody_joint;

#[cfg(test)]
mod multibody_regression_tests;
