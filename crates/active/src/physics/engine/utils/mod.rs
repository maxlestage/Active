// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! Miscellaneous utilities.

mod angular_inertia_ops;
mod component_mul;
mod copysign;
mod cross_product;
mod cross_product_matrix;
mod dot_product;
mod fp_flags;
mod index_mut2;
mod matrix_column;
mod orthonormal_basis;
mod pos_ops;
mod prefetch;
mod rotation_ops;
mod scalar_type;
mod simd_real_copy;
mod simd_select;

pub use component_mul::ComponentMul;
pub use copysign::CopySign;
pub use index_mut2::IndexMut2;
pub use matrix_column::MatrixColumn;
pub use orthonormal_basis::OrthonormalBasis;
pub use pos_ops::PoseOps;
pub(crate) use prefetch::prefetch_read;
pub use rotation_ops::RotationOps;
pub use scalar_type::ScalarType;
pub use simd_real_copy::SimdRealCopy;
pub use simd_select::SimdSelect;

pub use angular_inertia_ops::AngularInertiaOps;
pub use cross_product::CrossProduct;
pub use cross_product_matrix::CrossProductMatrix;
pub use dot_product::{DotProduct, SimdLength};
#[allow(unused_imports)]
pub(crate) use fp_flags::DisableFloatingPointExceptionsFlags;

use crate::physics::engine::math::SIMD_WIDTH;
use crate::physics::engine::math::SimdVector;
use crate::physics::engine::math::{Real, Vector};
use na::Matrix3;

/// Dimension minus one (1 for 2D, 2 for 3D).
pub const DIM_MINUS_ONE: usize = 2;

/// Try to normalize a vector and return both the normalized vector and the original length.
///
/// Returns `None` if the vector's length is below the threshold.
/// This is the glam equivalent of nalgebra's `Unit::try_new_and_get`.
pub fn try_normalize_and_get_length(v: Vector, threshold: Real) -> Option<(Vector, Real)> {
    let len = v.length();
    if len > threshold {
        Some((v / len, len))
    } else {
        None
    }
}

/// Forces a negative-zero float (or any negative-zero component) to positive zero via
/// `x + 0.0`, leaving every other value untouched.
///
/// Serialized solver state must not contain `-0.0`: whether a min/max/clamp returns `+0.0`
/// or `-0.0` for a signed-zero tie is platform-specific (SSE returns one of the operands
/// picked by argument order, NEON's `fminnm`/`fmaxnm` order the zeros), so a stored
/// signed zero breaks cross-platform snapshot determinism even though `-0.0 == +0.0`
/// dynamically.
// Unused in no-alloc builds: the contact solver, its only caller, needs alloc.
#[allow(dead_code)]
#[inline(always)]
pub(crate) fn canonicalize_zero<T>(x: T) -> T
where
    T: core::ops::Add<Output = T> + Default,
{
    { x }
}

/// Convert glam Vector to nalgebra `SimdVector<Real>`
#[inline]
pub fn vect_to_na(v: Vector) -> SimdVector<Real> {
    v.into()
}

use crate::physics::engine::math::Matrix;

/// Convert glam Matrix to nalgebra `Matrix3<Real>` (3D matrix)
#[inline]
pub fn mat_to_na(m: Matrix) -> Matrix3<Real> {
    Matrix3::new(
        m.x_axis.x, m.y_axis.x, m.z_axis.x, m.x_axis.y, m.y_axis.y, m.z_axis.y, m.x_axis.z,
        m.y_axis.z, m.z_axis.z,
    )
}

const INV_EPSILON: Real = 1.0e-20;

#[allow(dead_code)]
pub(crate) fn inv(val: Real) -> Real {
    if (-INV_EPSILON..=INV_EPSILON).contains(&val) {
        0.0
    } else {
        1.0 / val
    }
}

#[allow(dead_code)]
pub(crate) fn simd_inv<N: SimdRealCopy>(val: N) -> N {
    let eps = N::splat(INV_EPSILON);
    N::zero().select(val.simd_gt(-eps) & val.simd_lt(eps), N::one() / val)
}

#[allow(dead_code)]
pub(crate) fn select_other<T: PartialEq>(pair: (T, T), elt: T) -> T {
    if pair.0 == elt { pair.1 } else { pair.0 }
}

/// `Sync`, unless the `unsync-callbacks` feature says otherwise.
pub trait MaybeSync: Sync {}
impl<T: Sync + ?Sized> MaybeSync for T {}

/// Removes a key from a [`parry::utils::hashmap::HashMap`] without caring about the
/// resulting entry order.
///
/// The map is an `IndexMap` under `enhanced-determinism` and a `hashbrown` map otherwise,
/// and only the former has (and demands) the order-explicit `swap_remove`. Its order still
/// only depends on the sequence of operations, so swapping stays deterministic.
pub(crate) fn hashmap_remove<K, V>(
    map: &mut parry::utils::hashmap::HashMap<K, V>,
    key: &K,
) -> Option<V>
where
    K: core::hash::Hash + Eq,
{
    return map.remove(key);
}

/// Calculate the difference with smallest absolute value between the two given values.
pub fn smallest_abs_diff_between_sin_angles<N: SimdRealCopy>(a: N, b: N) -> N {
    // Select the smallest path among the two angles to reach the target.
    let s_err = a - b;
    let sgn = s_err.simd_signum();
    let s_err_complement = s_err - sgn * N::splat(2.0);
    let s_err_is_smallest = s_err.simd_abs().simd_lt(s_err_complement.simd_abs());
    s_err.select(s_err_is_smallest, s_err_complement)
}

/// Calculate the difference with smallest absolute value between the two given angles.
pub fn smallest_abs_diff_between_angles<N: SimdRealCopy>(a: N, b: N) -> N {
    // Select the smallest path among the two angles to reach the target.
    let s_err = a - b;
    let sgn = s_err.simd_signum();
    let s_err_complement = s_err - sgn * N::simd_two_pi();
    let s_err_is_smallest = s_err.simd_abs().simd_lt(s_err_complement.simd_abs());
    s_err.select(s_err_is_smallest, s_err_complement)
}

/// A single solver body's 4-scalar storage block, used to reinterpret a scalar
/// `SolverVel`/`SolverPose`/`SolverContact` as fixed 4-wide chunks for the
/// AoS↔SoA gather/scatter transpose.
///
/// This is deliberately **always** 4 lanes, independent of [`SIMD_WIDTH`]: it
/// describes one body's data layout, not the SIMD lane count. At f32 and the
/// default 4-lane width it is exactly `SimdReal`; at 8 lanes `SimdReal` widens
/// to 256-bit while a per-body block stays 128-bit.
pub(crate) type SolverBlock = simba::simd::WideF32x4;

/// One body's block as the plain array the `aos!` gather hands over — what
/// [`SolverBlock`] wraps, and what the transpose below operates on.
pub(crate) type RawBlock = wide::f32x4;

/// A 4x4 block transpose. Pure data movement — no arithmetic — so both
/// implementations below are bit-exact and interchangeable.
#[inline(always)]
fn transpose4(data: [RawBlock; 4]) -> [RawBlock; 4] {
    wide::f32x4::transpose(data)
}

/// Transposes `SIMD_WIDTH` bodies' blocks (AoS) into 4 SoA lane-vectors, one per
/// float field. Inverse of [`transpose_wide_inv`].
///
/// At 4 lanes this is a single [`transpose4`]. At 8 lanes it does two 4x4
/// transposes (bodies 0–3 / 4–7) and concatenates each field's two halves.
#[inline(always)]
pub(crate) fn transpose_wide(
    aos: [RawBlock; SIMD_WIDTH],
) -> [crate::physics::engine::math::SimdReal; 4] {
    { unsafe { core::mem::transmute(transpose4(aos)) } }
}

/// Transposes 4 SoA lane-vectors back into `SIMD_WIDTH` bodies' blocks (AoS).
/// Inverse of [`transpose_wide`].
#[inline(always)]
pub(crate) fn transpose_wide_inv(
    soa: [crate::physics::engine::math::SimdReal; 4],
) -> [RawBlock; SIMD_WIDTH] {
    {
        transpose4(unsafe {
            core::mem::transmute::<[crate::physics::engine::math::SimdReal; 4], [RawBlock; 4]>(soa)
        })
    }
}
