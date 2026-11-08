// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
use crate::physics::rapier::dynamics::JointGraphEdge;
use crate::physics::rapier::dynamics::solver::joint_constraint::generic_joint_constraint::GenericJointConstraint;
use crate::physics::rapier::dynamics::solver::joint_constraint::joint_velocity_constraint::JointConstraint;
use crate::physics::rapier::math::Real;

use crate::physics::rapier::math::{SIMD_WIDTH, SimdReal};

#[derive(Debug)]
pub enum AnyJointConstraintMut<'a> {
    Generic(&'a mut GenericJointConstraint),
    Rigid(&'a mut JointConstraint<Real, 1>),
    SimdRigid(&'a mut JointConstraint<SimdReal, SIMD_WIDTH>),
}

impl AnyJointConstraintMut<'_> {
    pub fn writeback_impulses(&mut self, joints_all: &mut [JointGraphEdge]) {
        match self {
            Self::Rigid(c) => c.writeback_impulses(joints_all),
            Self::Generic(c) => c.writeback_impulses(joints_all),
            Self::SimdRigid(c) => c.writeback_impulses(joints_all),
        }
    }
}
