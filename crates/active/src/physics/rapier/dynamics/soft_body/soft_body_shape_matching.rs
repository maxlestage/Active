// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Shape matching helpers: the rotation of the best-fit rigid transform of a particle cloud onto
//! its rest shape (polar decomposition of the mass-weighted covariance matrix).

use crate::physics::rapier::math::Real;
use crate::physics::rapier::math::{Matrix, Rotation};

/// The rotation `R` of the polar decomposition `A = R S` of a covariance matrix, warm-started
/// from `warm`: closed form in 2D; in 3D, Müller et al.'s iterative rotation extraction, which
/// never leaves the rotation group and converges in a couple of iterations from a warm start.
pub(crate) fn extract_rotation(a: Matrix, warm: Rotation) -> Rotation {
    {
        use crate::physics::rapier::math::Vector;
        const MAX_ITERS: usize = 8;
        let mut q = warm;
        for _ in 0..MAX_ITERS {
            let r = Matrix::from_quat(q);
            let cols = [
                (r.x_axis, a.x_axis),
                (r.y_axis, a.y_axis),
                (r.z_axis, a.z_axis),
            ];
            let mut omega = Vector::ZERO;
            let mut denom: Real = 0.0;
            for (rc, ac) in cols {
                omega += rc.cross(ac);
                denom += rc.dot(ac);
            }
            let omega = omega * (1.0 / (denom.abs() + 1.0e-9));
            let w = omega.length();
            // Converged once the correction is at the rounding noise of the cross products (a
            // few ulps): a tolerance below that never triggers and runs every iteration.
            if w < 16.0 * Real::EPSILON {
                break;
            }
            // The exact update `exp(ω/2) q` for a large step; a small one takes the
            // first-order quaternion `(1, ω/2)` (renormalized) instead of sin/cos: same fixed
            // point, and the next iteration absorbs the O(w³) difference.
            let step = if w > 0.1 {
                Rotation::from_axis_angle(omega / w, w)
            } else {
                Rotation::from_xyzw(0.5 * omega.x, 0.5 * omega.y, 0.5 * omega.z, 1.0)
            };
            q = (step * q).normalize();
        }
        q
    }
}
