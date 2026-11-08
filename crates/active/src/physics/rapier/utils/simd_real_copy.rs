// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! SimdRealCopy trait for real numbers used by Rapier.

use crate::physics::rapier::math::Real;
use simba::simd::SimdRealField;

/// The trait for real numbers used by Rapier.
///
/// This includes `f32`, `f64` and their related SIMD types.
pub trait SimdRealCopy: SimdRealField<Element = Real> + Copy {}
impl<T: SimdRealField<Element = Real> + Copy> SimdRealCopy for T {}
