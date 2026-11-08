// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
use crate::physics::engine::alloc_prelude::*;
use crate::physics::engine::dynamics::RigidBodyVelocity;
use crate::physics::engine::math::{DVector, Real};

/// A temporary workspace for various updates of the multibody.
#[derive(Clone, Debug)]
pub(crate) struct MultibodyWorkspace {
    pub accs: Vec<RigidBodyVelocity<Real>>,
    pub ndofs_vec: DVector,
}

impl MultibodyWorkspace {
    /// Create an empty workspace.
    pub fn new() -> Self {
        MultibodyWorkspace {
            accs: Vec::new(),
            ndofs_vec: DVector::zeros(0),
        }
    }

    /// Resize the workspace so it is enough for `nlinks` links.
    pub fn resize(&mut self, nlinks: usize, ndofs: usize) {
        self.accs.resize(nlinks, RigidBodyVelocity::zero());
        self.ndofs_vec = DVector::zeros(ndofs)
    }
}
