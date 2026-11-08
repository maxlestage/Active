// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! MatrixColumn trait for matrix column access.

use crate::physics::rapier::math::{Matrix, Vector};
use na::Scalar;

/// Extension trait for matrix column access (like nalgebra's `.column()`)
pub trait MatrixColumn {
    /// The column type returned by `column()`.
    type Column;
    /// Returns the i-th column of this matrix.
    fn column(&self, i: usize) -> Self::Column;
}

impl<T: Copy> MatrixColumn for [T; 2] {
    type Column = T;
    fn column(&self, i: usize) -> Self::Column {
        self[i]
    }
}

impl MatrixColumn for Matrix {
    type Column = Vector;
    #[inline]
    fn column(&self, i: usize) -> Self::Column {
        self.col(i)
    }
}

impl<T: Scalar> MatrixColumn for na::Matrix3<T> {
    type Column = na::Vector3<T>;
    #[inline]
    fn column(&self, i: usize) -> Self::Column {
        self.column(i).into_owned()
    }
}

impl<T: Scalar> MatrixColumn for na::Matrix2<T> {
    type Column = na::Vector2<T>;
    #[inline]
    fn column(&self, i: usize) -> Self::Column {
        self.column(i).into_owned()
    }
}
