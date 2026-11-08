// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! SimdDot and SimdLength traits for generalized dot product and length.

use crate::physics::rapier::math::SimdReal;
use crate::physics::rapier::math::{Real, Vector};
use crate::physics::rapier::utils::SimdRealCopy;
use na::{Vector1, Vector2, Vector3};
use simba::simd::SimdRealField;

/// Trait for computing generalized dot products.
pub trait DotProduct<Rhs>: Sized + Copy {
    /// The result type of the dot product.
    type Result: SimdRealField;
    /// Computes the generalized dot product of `self` with `rhs`.
    fn gdot(&self, rhs: Rhs) -> Self::Result;
}

/// Trait for computing generalized lengths.
pub trait SimdLength: DotProduct<Self> {
    /// Computes the SIMD length of this value.
    fn simd_length(&self) -> Self::Result {
        use simba::simd::SimdComplexField;
        self.gdot(*self).simd_sqrt()
    }
}

impl<T: DotProduct<T>> SimdLength for T {}

impl<N: SimdRealCopy> DotProduct<Vector3<N>> for Vector3<N> {
    type Result = N;

    fn gdot(&self, rhs: Vector3<N>) -> Self::Result {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}

impl<N: SimdRealCopy> DotProduct<Vector2<N>> for Vector2<N> {
    type Result = N;

    fn gdot(&self, rhs: Vector2<N>) -> Self::Result {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl<N: SimdRealCopy> DotProduct<Vector1<N>> for N {
    type Result = N;

    fn gdot(&self, rhs: Vector1<N>) -> Self::Result {
        *self * rhs.x
    }
}

impl DotProduct<Real> for Real {
    type Result = Real;

    fn gdot(&self, rhs: Real) -> Self::Result {
        *self * rhs
    }
}

impl DotProduct<SimdReal> for SimdReal {
    type Result = SimdReal;

    fn gdot(&self, rhs: SimdReal) -> Self::Result {
        *self * rhs
    }
}

impl<N: SimdRealCopy> DotProduct<N> for Vector1<N> {
    type Result = N;

    fn gdot(&self, rhs: N) -> Self::Result {
        self.x * rhs
    }
}

// Glam implementations for concrete Vector type
impl DotProduct<Vector> for Vector {
    type Result = Real;

    fn gdot(&self, rhs: Vector) -> Self::Result {
        self.dot(rhs)
    }
}
