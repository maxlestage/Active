// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Structures related to dynamics: bodies, impulse_joints, etc.

pub use self::ccd::CCDSolver;
pub use self::coefficient_combine_rule::CoefficientCombineRule;
pub use self::integration_parameters::FrictionModel;
pub use self::integration_parameters::{IntegrationParameters, SpringCoefficients};
pub use self::island_manager::IslandManager;
pub(crate) use self::island_manager::{INVALID_ISLAND, ImpulseJointIslandEvent, PersistentIslands};

pub(crate) use self::joint::JointGraphEdge;
pub(crate) use self::joint::JointIndex;
pub use self::joint::*;
pub use self::rigid_body_components::*;
pub use self::rigid_body_handle::RigidBodyHandle;
pub(crate) use self::rigid_body_set::ModifiedRigidBodies;
pub(crate) use self::solver::StagedIslandSolver;
pub use parry::mass_properties::MassProperties;

pub use self::rigid_body::{RigidBody, RigidBodyBuilder};
pub use self::rigid_body_set::{BodyPair, RigidBodySet};
pub(crate) use self::soft_body::soft_body_crossing_tests;
pub use self::soft_body::*;

mod ccd;
mod coefficient_combine_rule;
mod integration_parameters;
mod island_manager;
mod joint;
mod rigid_body_components;
mod rigid_body_handle;
pub(crate) mod solver;

mod rigid_body;
mod rigid_body_set;
mod soft_body;
