// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
use crate::physics::rapier::alloc_prelude::*;
use crate::physics::rapier::dynamics::RigidBodyVelocity;
use crate::physics::rapier::math::{DVector, Real};

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
