// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! SimdSelect trait for conditional selection.

use crate::physics::rapier::math::SimdReal;
use crate::physics::rapier::math::{Real, Vector};
use simba::simd::SimdValue;

/// Trait for conditional selection between two values.
pub trait SimdSelect<N: SimdValue> {
    /// Select between `self` and `if_false` based on `condition`.
    fn select(self, condition: N::SimdBool, if_false: Self) -> Self;
}

impl SimdSelect<Real> for Vector {
    #[inline]
    fn select(self, condition: bool, if_false: Self) -> Self {
        if condition { self } else { if_false }
    }
}

impl SimdSelect<SimdReal> for na::Vector3<SimdReal> {
    #[inline]
    fn select(self, condition: <SimdReal as SimdValue>::SimdBool, if_false: Self) -> Self {
        SimdValue::select(self, condition, if_false)
    }
}
