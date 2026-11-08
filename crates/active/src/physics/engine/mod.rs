// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! The physics engine of Active: rigid bodies, colliders, contacts, joints, islands
//! that sleep, and the queries (rays, shapes) on them. [`crate::physics`] wraps it.
//!
//! This directory is under the Apache License 2.0 (its `LICENSE`), not Active's license.

#![allow(unsafe_code, rust_2018_idioms, unused_braces, clippy::all)]
#![deny(bare_trait_objects)]
#![warn(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)] // TODO: remove this? I find that in the math code using indices adds clarity.
#![allow(clippy::module_inception)]

pub use ::parry3d as parry;

/// Internal prelude re-exporting alloc types for no_std compatibility.
#[doc(hidden)]
#[allow(unused_imports)]
pub(crate) mod alloc_prelude {
    pub use alloc::{boxed::Box, string::String, string::ToString, vec, vec::Vec};
}

pub use ::nalgebra as na;
pub(crate) use ::num;

pub use parry::glamx;

#[allow(unused_macros)]
macro_rules! gather(
    ($callback: expr) => { array!($callback) }
);

#[allow(unused_macros)]
macro_rules! array(
    ($callback: expr) => {
        {
            #[inline(always)]
            #[allow(dead_code)]
            fn create_arr<T>(callback: impl FnMut(usize) -> T) -> [T; SIMD_WIDTH] {
                // Width-agnostic: `N` is inferred from `[T; SIMD_WIDTH]`, covering the
                // 1-, 4-, and 8-lane builds alike.
                core::array::from_fn(callback)
            }

            create_arr($callback)
        }
    }
);

#[allow(unused_macros)]
macro_rules! par_iter {
    ($t: expr) => {{
        let it = $t.iter();

        it
    }};
}

#[allow(unused_macros)]
macro_rules! par_iter_mut {
    ($t: expr) => {{
        let it = $t.iter_mut();

        it
    }};
}

// macro_rules! par_chunks_mut {
//     ($t: expr, $sz: expr) => {{
//         #[cfg(not(feature = "parallel"))]
//         let it = $t.chunks_mut($sz);
//
//         #[cfg(feature = "parallel")]
//         let it = $t.par_chunks_mut($sz);
//         it
//     }};
// }

#[allow(unused_macros)]
macro_rules! try_ret {
    ($val: expr) => {
        try_ret!($val, ())
    };
    ($val: expr, $ret: expr) => {
        if let Some(val) = $val {
            val
        } else {
            return $ret;
        }
    };
}

// macro_rules! try_continue {
//     ($val: expr) => {
//         if let Some(val) = $val {
//             val
//         } else {
//             continue;
//         }
//     };
// }

#[allow(dead_code)]
pub(crate) const INVALID_U32: u32 = u32::MAX;
#[allow(dead_code)]
pub(crate) const INVALID_USIZE: usize = INVALID_U32 as usize;

/// The version of the engine.
pub const VERSION: &str = "0.36.1";

pub mod control;
pub mod counters;
pub mod data;
pub mod dynamics;
pub mod geometry;
pub mod pipeline;
pub mod utils;

/// Elementary mathematical entities (vectors, matrices, isometries, etc).
pub mod math {
    pub use parry::math::*;

    // Re-export glam from parry for direct access
    pub use parry::glamx;

    /// Creates a rotation from an angular vector.
    ///
    /// In 2D, the angular vector is a scalar angle in radians.
    /// In 3D, the angular vector is a scaled axis-angle (axis * angle).
    #[inline]
    pub fn rotation_from_angle(angle: AngVector) -> Rotation {
        Rotation::from_scaled_axis(angle)
    }

    // Generic nalgebra type aliases for SIMD/generic code (where N is SimdReal or similar)
    // These use nalgebra types which support generic scalars
    // Note: These override the non-generic versions above when used with <T> syntax

    /// Generic vector type (nalgebra) for SoA SIMD code
    pub type SimdVector<N> = na::Vector3<N>;
    /// Generic angular vector type (nalgebra) for SoA SIMD code
    pub type SimdAngVector<N> = na::Vector3<N>;
    /// Generic point type (nalgebra) for SoA SIMD code
    pub type SimdPoint<N> = na::Point3<N>;
    /// Generic isometry type (nalgebra) for SoA SIMD code
    pub type SimdPose<N> = na::Isometry3<N>;
    /// Generic rotation type (nalgebra) for SoA SIMD code
    pub type SimdRotation<N> = na::UnitQuaternion<N>;
    /// Generic angular inertia type for SoA SIMD code (scalar in 2D, SdpMatrix3 in 3D)
    pub type SimdAngularInertia<N> = parry::utils::SdpMatrix3<N>;
    /// Generic 2D/3D square matrix for SoA SIMD code
    pub type SimdMatrix<N> = na::Matrix3<N>;

    // Dimension types for nalgebra matrix operations (used in multibody code)
    /// The dimension type constant (U3 for 3D).
    pub type Dim = na::U3;
    /// The angular dimension type constant (U3 for 3D).
    pub type AngDim = na::U3;

    /// Dynamic vector type for multibody/solver code
    pub type DVector = na::DVector<Real>;
    /// Dynamic matrix type for multibody/solver code
    pub type DMatrix = na::DMatrix<Real>;

    /*
     * 2D
     */

    /*
     * 3D
     */
    /// Max number of pairs of contact points from the same
    /// contact manifold that can be solved as part of a
    /// single contact constraint.
    pub const MAX_MANIFOLD_POINTS: usize = 4;

    /// The type of a constraint Jacobian in twist coordinates.
    pub type Jacobian<N> = na::Matrix6xX<N>;

    /// The type of a slice of the constraint Jacobian in twist coordinates.
    pub type JacobianView<'a, N> = na::MatrixView6xX<'a, N>;

    /// The type of a mutable slice of the constraint Jacobian in twist coordinates.
    pub type JacobianViewMut<'a, N> = na::MatrixViewMut6xX<'a, N>;

    /// The type of impulse applied for friction constraints.
    pub type TangentImpulse<N> = na::Vector2<N>;

    /// The maximum number of possible rotations and translations of a rigid body.
    pub const SPATIAL_DIM: usize = 6;

    /// The maximum number of rotational degrees of freedom of a rigid-body.
    pub const ANG_DIM: usize = 3;
}

/// Prelude containing the common types defined by the engine.
///
/// The `nalgebra` crate and its `vector!`/`point!` macros are re-exported by this prelude,
/// so the macros can be used without declaring `nalgebra` as an explicit dependency:
///
/// ```
/// use active::physics::engine::prelude::*;
///
/// // No `use nalgebra` anywhere: the macros must still resolve.
/// let v = vector![1.0, 2.0, 3.0];
/// let p = point![1.0, 2.0, 3.0];
/// assert_eq!(v, nalgebra::Vector3::new(1.0, 2.0, 3.0));
/// assert_eq!(p, nalgebra::Point3::new(1.0, 2.0, 3.0));
/// ```
pub mod prelude {
    pub use crate::physics::engine::dynamics::*;
    pub use crate::physics::engine::geometry::*;
    pub use crate::physics::engine::math::*;
    pub use crate::physics::engine::pipeline::*;
    pub use na::{point, vector};
    pub extern crate nalgebra;
}
