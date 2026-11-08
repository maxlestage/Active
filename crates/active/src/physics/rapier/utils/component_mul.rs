// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! ComponentMul trait for element-wise vector operations.

use crate::physics::rapier::math::Vector;
use crate::physics::rapier::utils::SimdRealCopy;
use na::{Vector2, Vector3};

/// Extension trait for element-wise vector operations
pub trait ComponentMul: Sized {
    /// Element-wise multiplication
    fn component_mul(&self, other: &Self) -> Self;
}

impl ComponentMul for Vector {
    #[inline]
    fn component_mul(&self, other: &Self) -> Self {
        *self * *other
    }
}

// Nalgebra vector implementations for SIMD support
impl<N: SimdRealCopy> ComponentMul for Vector2<N> {
    #[inline]
    fn component_mul(&self, other: &Self) -> Self {
        nalgebra::Vector2::component_mul(self, other)
    }
}

impl<N: SimdRealCopy> ComponentMul for Vector3<N> {
    #[inline]
    fn component_mul(&self, other: &Self) -> Self {
        nalgebra::Vector3::component_mul(self, other)
    }
}
