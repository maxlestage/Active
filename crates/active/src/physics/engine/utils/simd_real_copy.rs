// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! SimdRealCopy trait for real numbers used by the engine.

use crate::physics::engine::math::Real;
use simba::simd::SimdRealField;

/// The trait for real numbers used by the engine.
///
/// This includes `f32`, `f64` and their related SIMD types.
pub trait SimdRealCopy: SimdRealField<Element = Real> + Copy {}
impl<T: SimdRealField<Element = Real> + Copy> SimdRealCopy for T {}
