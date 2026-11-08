// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
/// The unique handle of a soft body added to a [`super::SoftBodySet`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SoftBodyHandle(pub crate::physics::engine::data::arena::Index);

impl SoftBodyHandle {
    /// Converts this handle into its (index, generation) components.
    pub fn into_raw_parts(self) -> (u32, u32) {
        self.0.into_raw_parts()
    }

    /// Reconstructs an handle from its (index, generation) components.
    pub fn from_raw_parts(id: u32, generation: u32) -> Self {
        Self(crate::physics::engine::data::arena::Index::from_raw_parts(
            id, generation,
        ))
    }

    /// An always-invalid soft-body handle.
    pub fn invalid() -> Self {
        Self(crate::physics::engine::data::arena::Index::from_raw_parts(
            crate::physics::engine::INVALID_U32,
            crate::physics::engine::INVALID_U32,
        ))
    }

    /// Whether this is the always-invalid handle returned by [`Self::invalid`].
    pub fn is_invalid(self) -> bool {
        self == Self::invalid()
    }
}
