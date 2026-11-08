// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
pub use joint_velocity_constraint::{JointSolverBody, MotorParameters, WritebackId};

pub use any_joint_constraint::AnyJointConstraintMut;
pub use generic_joint_constraint::GenericJointConstraint;
pub(crate) use generic_joint_constraint_builder::GenericJointConstraintBuilder;
pub use generic_joint_constraint_builder::{
    JointGenericExternalConstraintBuilder, JointGenericInternalConstraintBuilder, LinkOrBodyRef,
};
pub(crate) use joint_constraint_builder::JointConstraintBuilder;
pub(crate) use joint_constraint_builder::JointConstraintBuilderSimd;
pub use joint_constraint_helper::{AngularLimitParams, JointConstraintHelper};
pub use joint_constraints_set::JointConstraintsSet;

mod any_joint_constraint;
mod generic_joint_constraint;
mod generic_joint_constraint_builder;
mod joint_constraint_builder;
mod joint_constraint_helper;
mod joint_constraints_set;
mod joint_velocity_constraint;

/// Whether this rigid body is a soft-frame proxy with no angular response (a rank-deficient
/// cluster: one particle, or a collinear one in 3D): its zero reduced inverse inertia would turn
/// an angular joint row into a one-sided constraint freezing the *other* body's orientation.
pub(crate) fn soft_frame_angular_degenerate(
    rb: &crate::physics::rapier::dynamics::RigidBody,
) -> bool {
    if !rb.is_soft_frame() {
        return false;
    }
    let ii = &rb.mass_properties().effective_world_inv_inertia;
    {
        ii.m11 == 0.0
            && ii.m12 == 0.0
            && ii.m13 == 0.0
            && ii.m22 == 0.0
            && ii.m23 == 0.0
            && ii.m33 == 0.0
    }
}

/// Whether the joint constrains any angular axis (locked, limited, motorized or coupled).
pub(crate) fn joint_uses_angular_axes(
    data: &crate::physics::rapier::dynamics::GenericJoint,
) -> bool {
    let ang = crate::physics::rapier::dynamics::JointAxesMask::ANG_AXES;
    !((data.locked_axes | data.limit_axes | data.motor_axes | data.coupled_axes) & ang).is_empty()
}

/// Strips the angular axes from a joint's lowered data when one side is an angular-degenerate
/// soft frame (see [`soft_frame_angular_degenerate`]): the joint keeps its translational
/// behavior, and the documented rule is that a rank-deficient cluster has no orientation.
pub(crate) fn strip_soft_frame_angular_axes(
    data: &mut crate::physics::rapier::dynamics::GenericJoint,
    rb1: &crate::physics::rapier::dynamics::RigidBody,
    rb2: &crate::physics::rapier::dynamics::RigidBody,
) {
    if soft_frame_angular_degenerate(rb1) || soft_frame_angular_degenerate(rb2) {
        let ang = crate::physics::rapier::dynamics::JointAxesMask::ANG_AXES;
        data.locked_axes &= !ang;
        data.limit_axes &= !ang;
        data.motor_axes &= !ang;
        data.coupled_axes &= !ang;
    }
}
