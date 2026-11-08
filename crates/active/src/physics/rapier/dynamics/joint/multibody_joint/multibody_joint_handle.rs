// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
use crate::physics::rapier::data::Index;

/// The unique handle of an multibody_joint added to a `MultibodyJointSet`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MultibodyJointHandle(pub Index);

/// The temporary index of a multibody added to a `MultibodyJointSet`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MultibodyIndex(pub Index);

impl MultibodyJointHandle {
    /// Converts this handle into its (index, generation) components.
    pub fn into_raw_parts(self) -> (u32, u32) {
        self.0.into_raw_parts()
    }

    /// Reconstructs an handle from its (index, generation) components.
    pub fn from_raw_parts(id: u32, generation: u32) -> Self {
        Self(Index::from_raw_parts(id, generation))
    }

    /// An always-invalid rigid-body handle.
    pub fn invalid() -> Self {
        Self(Index::from_raw_parts(
            crate::physics::rapier::INVALID_U32,
            crate::physics::rapier::INVALID_U32,
        ))
    }
}

impl Default for MultibodyJointHandle {
    fn default() -> Self {
        Self::invalid()
    }
}
