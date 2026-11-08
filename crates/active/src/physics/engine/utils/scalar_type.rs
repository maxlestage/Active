// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! ScalarType trait for generic scalar types in the engine.

use crate::physics::engine::math::SimdReal;
use crate::physics::engine::math::{AngularInertia, Matrix, Pose, Real, Rotation, Vector};
use crate::physics::engine::utils::SimdSelect;
use crate::physics::engine::utils::{
    AngularInertiaOps, ComponentMul, CrossProduct, CrossProductMatrix, DIM_MINUS_ONE, DotProduct,
    MatrixColumn, OrthonormalBasis, PoseOps, RotationOps,
};
use core::fmt::Debug;
use core::ops::{Add, AddAssign, DivAssign, Index, Mul, MulAssign, Neg, Sub, SubAssign};
use na::SimdRealField;

/// Trait for types that can be used as scalars in the generic code supporting both
/// the scalar and AoSoA SIMD pattern.
///
/// This trait is mostly for internal use only. No other implementations of this trait
/// is expected.
pub trait ScalarType:
    SimdRealField<Element = Real>
    + Copy
    + CrossProduct<Self::Vector, Result = Self::Vector>
    + DotProduct<Self, Result = Self>
{
    /// The pose type (position + rotation) for this scalar.
    type Pose: Copy + PoseOps<Self> + Mul<Self::Vector, Output = Self::Vector>;
    /// The vector type for this scalar (3D).
    type Vector: Copy
        + Debug
        + Default
        + Neg<Output = Self::Vector>
        + Add<Self::Vector, Output = Self::Vector>
        + Sub<Self::Vector, Output = Self::Vector>
        + AddAssign<Self::Vector>
        + SubAssign<Self::Vector>
        + MulAssign<Self>
        + DivAssign<Self>
        + Mul<Self, Output = Self::Vector>
        + Index<usize, Output = Self>
        + DotProduct<Self::Vector, Result = Self>
        + OrthonormalBasis<Basis = [Self::Vector; DIM_MINUS_ONE]>
        + CrossProduct<Self::Vector, Result = Self::Vector>
        + CrossProductMatrix<CrossMat = Self::Matrix, CrossMatTr = Self::Matrix>
        + SimdSelect<Self>
        + ComponentMul
        + Into<Self::AngVector>; // In 3D Vec and Ang are technically the same.
    /// The angular vector type for this scalar (3D).
    type AngVector: Copy
        + Debug
        + Default
        + Add<Self::AngVector, Output = Self::AngVector>
        + Sub<Self::AngVector, Output = Self::AngVector>
        + AddAssign<Self::AngVector>
        + SubAssign<Self::AngVector>
        + MulAssign<Self>
        + DivAssign<Self>
        + Mul<Self, Output = Self::AngVector>
        + DotProduct<Self::AngVector, Result = Self>;
    /// The matrix type for this scalar.
    type Matrix: Copy
        + Debug
        + MatrixColumn<Column = Self::Vector>
        + MulAssign<Self>
        + Mul<Self::Matrix, Output = Self::Matrix>;
    /// The angular inertia type for this scalar.
    type AngInertia: AngularInertiaOps<Self, AngVector = Self::AngVector>;
    /// The rotation type for this scalar.
    type Rotation: RotationOps<Self>;
}

impl ScalarType for Real {
    type Pose = Pose;
    type Vector = Vector;
    type AngVector = Vector;
    type Matrix = Matrix;
    type AngInertia = AngularInertia;
    type Rotation = Rotation;
}

impl ScalarType for SimdReal {
    type Pose = na::Isometry3<SimdReal>;
    type Vector = na::Vector3<SimdReal>;
    type AngVector = na::Vector3<SimdReal>;
    type Matrix = na::Matrix3<SimdReal>;
    type AngInertia = parry::utils::SdpMatrix3<SimdReal>;
    type Rotation = na::UnitQuaternion<SimdReal>;
}
