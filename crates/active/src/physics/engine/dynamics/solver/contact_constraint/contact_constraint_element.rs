// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
use crate::physics::engine::dynamics::solver::SolverVel;
use crate::physics::engine::math::{DIM, TangentImpulse};
use crate::physics::engine::utils::{AngularInertiaOps, CrossProduct};
use crate::physics::engine::utils::{ComponentMul, DotProduct, ScalarType};

#[derive(Copy, Clone, Debug)]
pub(crate) struct ContactConstraintTangentPart<N: ScalarType> {
    pub torque_dir1: [N::AngVector; DIM - 1],
    pub torque_dir2: [N::AngVector; DIM - 1],
    pub ii_torque_dir1: [N::AngVector; DIM - 1],
    pub ii_torque_dir2: [N::AngVector; DIM - 1],
    pub rhs: [N; DIM - 1],
    pub rhs_wo_bias: [N; DIM - 1],
    pub impulse: na::Vector2<N>,
    pub impulse_accumulator: na::Vector2<N>,
    pub r: [N; DIM],
}

impl<N: ScalarType> ContactConstraintTangentPart<N> {
    pub fn zero() -> Self {
        Self {
            torque_dir1: [Default::default(); DIM - 1],
            torque_dir2: [Default::default(); DIM - 1],
            ii_torque_dir1: [Default::default(); DIM - 1],
            ii_torque_dir2: [Default::default(); DIM - 1],
            rhs: [N::zero(); DIM - 1],
            rhs_wo_bias: [N::zero(); DIM - 1],
            impulse: na::zero(),
            impulse_accumulator: na::zero(),
            r: [N::zero(); DIM],
        }
    }

    /// Total impulse applied during the step: the sum, over every solver substep, of the
    /// impulse that substep ended on (the warm-start impulse it re-applied included).
    #[inline]
    pub fn total_impulse(&self) -> TangentImpulse<N> {
        self.impulse_accumulator + self.impulse
    }

    #[inline]
    pub fn warmstart(
        &mut self,
        tangents1: [&N::Vector; DIM - 1],
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        {
            solver_vel1.linear += (*tangents1[0] * self.impulse[0]
                + *tangents1[1] * self.impulse[1])
                .component_mul(im1);
            solver_vel1.angular +=
                self.ii_torque_dir1[0] * self.impulse[0] + self.ii_torque_dir1[1] * self.impulse[1];

            solver_vel2.linear += (*tangents1[0] * -self.impulse[0]
                + *tangents1[1] * -self.impulse[1])
                .component_mul(im2);
            solver_vel2.angular +=
                self.ii_torque_dir2[0] * self.impulse[0] + self.ii_torque_dir2[1] * self.impulse[1];
        }
    }

    #[inline]
    pub fn solve(
        &mut self,
        tangents1: [&N::Vector; DIM - 1],
        im1: &N::Vector,
        im2: &N::Vector,
        limit: N,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        {
            let dvel_0 = tangents1[0].gdot(solver_vel1.linear)
                + self.torque_dir1[0].gdot(solver_vel1.angular)
                - tangents1[0].gdot(solver_vel2.linear)
                + self.torque_dir2[0].gdot(solver_vel2.angular)
                + self.rhs[0];
            let dvel_1 = tangents1[1].gdot(solver_vel1.linear)
                + self.torque_dir1[1].gdot(solver_vel1.angular)
                - tangents1[1].gdot(solver_vel2.linear)
                + self.torque_dir2[1].gdot(solver_vel2.angular)
                + self.rhs[1];

            // Exact coupled 2×2 central-friction solve: `Δλ = -K⁻¹·dvel` with
            // `K` the tangent effective-mass matrix. A 1D solve along `dvel` leaves an
            // orthogonal residual when `K` is anisotropic; iterating it rotates energy between
            // the tangents and pumps the friction-only mode of large stacks into ejection.
            let k11 = self.r[0];
            let k22 = self.r[1];
            let k12 = self.r[2] * N::splat(0.5);
            let inv_det = crate::physics::engine::utils::simd_inv(k11 * k22 - k12 * k12);
            let delta_impulse = na::vector![
                (k22 * dvel_0 - k12 * dvel_1) * inv_det,
                (k11 * dvel_1 - k12 * dvel_0) * inv_det
            ];
            let new_impulse = self.impulse - delta_impulse;
            let new_impulse = {
                let _disable_fe_except =
                        crate::physics::engine::utils::DisableFloatingPointExceptionsFlags::
                        disable_floating_point_exceptions();
                new_impulse.simd_cap_magnitude(limit)
            };

            let dlambda = new_impulse - self.impulse;
            self.impulse = new_impulse;

            solver_vel1.linear +=
                (*tangents1[0] * dlambda[0] + *tangents1[1] * dlambda[1]).component_mul(im1);
            solver_vel1.angular +=
                self.ii_torque_dir1[0] * dlambda[0] + self.ii_torque_dir1[1] * dlambda[1];

            solver_vel2.linear +=
                (*tangents1[0] * -dlambda[0] + *tangents1[1] * -dlambda[1]).component_mul(im2);
            solver_vel2.angular +=
                self.ii_torque_dir2[0] * dlambda[0] + self.ii_torque_dir2[1] * dlambda[1];
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ContactConstraintNormalPart<N: ScalarType> {
    pub torque_dir1: N::AngVector,
    pub torque_dir2: N::AngVector,
    pub ii_torque_dir1: N::AngVector,
    pub ii_torque_dir2: N::AngVector,
    pub rhs: N,
    pub rhs_wo_bias: N,
    pub impulse: N,
    pub impulse_accumulator: N,
    pub r: N,
    /// Per-point softness (see `ContactConstraintNormalPartSlim::cfm_factor`):
    /// separated (speculative) points are solved rigidly.
    pub cfm_factor: N,
    // For coupled constraint pairs, even constraints store the
    // diagonal of the projected mass matrix. Odd constraints
    // store the off-diagonal element of the projected mass matrix,
    // as well as the off-diagonal element of the inverse projected mass matrix.
}

impl<N: ScalarType> ContactConstraintNormalPart<N> {
    pub fn zero() -> Self {
        Self {
            torque_dir1: Default::default(),
            torque_dir2: Default::default(),
            ii_torque_dir1: Default::default(),
            ii_torque_dir2: Default::default(),
            rhs: N::zero(),
            rhs_wo_bias: N::zero(),
            impulse: N::zero(),
            impulse_accumulator: N::zero(),
            r: N::zero(),
            cfm_factor: N::zero(),
        }
    }

    /// Total impulse applied during the step: the sum, over every solver substep, of the
    /// impulse that substep ended on (the warm-start impulse it re-applied included).
    #[inline]
    pub fn total_impulse(&self) -> N {
        self.impulse_accumulator + self.impulse
    }

    #[inline]
    pub fn warmstart(
        &mut self,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) {
        solver_vel1.linear += dir1.component_mul(im1) * self.impulse;
        solver_vel1.angular += self.ii_torque_dir1 * self.impulse;

        solver_vel2.linear += dir1.component_mul(im2) * -self.impulse;
        solver_vel2.angular += self.ii_torque_dir2 * self.impulse;
    }

    #[inline]
    pub fn solve(
        &mut self,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let dvel = dir1.gdot(solver_vel1.linear) + self.torque_dir1.gdot(solver_vel1.angular)
            - dir1.gdot(solver_vel2.linear)
            + self.torque_dir2.gdot(solver_vel2.angular)
            + self.rhs;
        let new_impulse = self.cfm_factor * (self.impulse - self.r * dvel).simd_max(N::zero());
        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;

        solver_vel1.linear += dir1.component_mul(im1) * dlambda;
        solver_vel1.angular += self.ii_torque_dir1 * dlambda;

        solver_vel2.linear += dir1.component_mul(im2) * -dlambda;
        solver_vel2.angular += self.ii_torque_dir2 * dlambda;
    }

    /// End-of-step restitution (box2d-style): drives the relative normal velocity toward
    /// `-seed` (`restitution * approach_velocity`, captured at prepare), rigidly and AFTER
    /// all substeps — inside the substep rhs the speculative slack truncates the bounce.
    ///
    /// Gated per lane on the point having applied a normal impulse this step.
    #[inline]
    pub fn solve_restitution(
        &mut self,
        seed: N,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let dvel = dir1.gdot(solver_vel1.linear) + self.torque_dir1.gdot(solver_vel1.angular)
            - dir1.gdot(solver_vel2.linear)
            + self.torque_dir2.gdot(solver_vel2.angular)
            + seed;
        let gate = seed.simd_lt(N::zero()) & self.total_impulse().simd_gt(N::zero());
        let new_impulse = (self.impulse - self.r * dvel).simd_max(N::zero());
        let new_impulse = new_impulse.select(gate, self.impulse);
        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;

        solver_vel1.linear += dir1.component_mul(im1) * dlambda;
        solver_vel1.angular += self.ii_torque_dir1 * dlambda;

        solver_vel2.linear += dir1.component_mul(im2) * -dlambda;
        solver_vel2.angular += self.ii_torque_dir2 * dlambda;
    }
}

/*
 * "Slim" variants for the twist-friction (Simplified) model.
 */
#[derive(Copy, Clone, Debug)]
pub(crate) struct ContactConstraintNormalPartSlim<N: ScalarType> {
    /// Angular jacobian wrt the first body.
    pub torque_dir1: N::AngVector,
    /// Angular jacobian wrt the second body.
    pub torque_dir2: N::AngVector,
    /// Equals to `ii1 * torque_dir1`.
    pub ii_torque_dir1: N::AngVector,
    /// Equals to `ii2 * torque_dir2`.
    pub ii_torque_dir2: N::AngVector,
    pub rhs: N,
    pub rhs_wo_bias: N,
    pub impulse: N,
    pub impulse_accumulator: N,
    pub r: N,
    /// Per-point softness. Separated (speculative) points are solved RIGIDLY
    /// (`massScale = 1`, `impulseScale = 0`): perfectly inelastic touchdowns damp stack
    /// rocking; a constraint-wide cfm under-stops each touchdown and pumps tall stacks.
    pub cfm_factor: N,
}

impl<N: ScalarType> ContactConstraintNormalPartSlim<N>
where
    N::Vector: CrossProduct<N::Vector, Result = N::AngVector>,
{
    /// Total impulse applied during the step: the sum, over every solver substep, of the
    /// impulse that substep ended on (the warm-start impulse it re-applied included).
    #[inline]
    pub fn total_impulse(&self) -> N {
        self.impulse_accumulator + self.impulse
    }

    #[inline]
    pub fn warmstart(
        &mut self,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) {
        solver_vel1.linear += dir1.component_mul(im1) * self.impulse;
        solver_vel1.angular += self.ii_torque_dir1 * self.impulse;

        solver_vel2.linear += dir1.component_mul(im2) * -self.impulse;
        solver_vel2.angular += self.ii_torque_dir2 * self.impulse;
    }

    #[inline]
    pub fn solve(
        &mut self,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let dvel = dir1.gdot(solver_vel1.linear) + self.torque_dir1.gdot(solver_vel1.angular)
            - dir1.gdot(solver_vel2.linear)
            + self.torque_dir2.gdot(solver_vel2.angular)
            + self.rhs;
        let new_impulse = self.cfm_factor * (self.impulse - self.r * dvel).simd_max(N::zero());
        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;

        solver_vel1.linear += dir1.component_mul(im1) * dlambda;
        solver_vel1.angular += self.ii_torque_dir1 * dlambda;

        solver_vel2.linear += dir1.component_mul(im2) * -dlambda;
        solver_vel2.angular += self.ii_torque_dir2 * dlambda;
    }

    /// See [`ContactConstraintNormalPart::solve_restitution`].
    #[inline]
    pub fn solve_restitution(
        &mut self,
        seed: N,
        dir1: &N::Vector,
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let dvel = dir1.gdot(solver_vel1.linear) + self.torque_dir1.gdot(solver_vel1.angular)
            - dir1.gdot(solver_vel2.linear)
            + self.torque_dir2.gdot(solver_vel2.angular)
            + seed;
        let gate = seed.simd_lt(N::zero()) & self.total_impulse().simd_gt(N::zero());
        let new_impulse = (self.impulse - self.r * dvel).simd_max(N::zero());
        let new_impulse = new_impulse.select(gate, self.impulse);
        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;

        solver_vel1.linear += dir1.component_mul(im1) * dlambda;
        solver_vel1.angular += self.ii_torque_dir1 * dlambda;

        solver_vel2.linear += dir1.component_mul(im2) * -dlambda;
        solver_vel2.angular += self.ii_torque_dir2 * dlambda;
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ContactConstraintTangentPartSlim<N: ScalarType> {
    /// World-space lever arm of the friction center wrt the first body's center of mass.
    pub dp1: N::Vector,
    /// World-space lever arm of the friction center wrt the second body's center of mass.
    pub dp2: N::Vector,
    /// Angular jacobians wrt the first body (one per tangent), cached at build time.
    pub torque_dir1: [N::AngVector; 2],
    /// Angular jacobians wrt the second body (one per tangent), cached at build time.
    pub torque_dir2: [N::AngVector; 2],
    /// `ii1 * torque_dir1[j]`, cached at build time.
    pub ii_torque_dir1: [N::AngVector; 2],
    /// `ii2 * torque_dir2[j]`, cached at build time.
    pub ii_torque_dir2: [N::AngVector; 2],
    pub rhs: [N; 2],
    pub rhs_wo_bias: [N; 2],
    pub impulse: na::Vector2<N>,
    pub impulse_accumulator: na::Vector2<N>,
    pub r: [N; 3],
}

impl<N: ScalarType> ContactConstraintTangentPartSlim<N>
where
    N::Vector: CrossProduct<N::Vector, Result = N::AngVector>,
{
    #[inline]
    pub fn warmstart(
        &mut self,
        tangents1: [&N::Vector; 2],
        im1: &N::Vector,
        im2: &N::Vector,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) {
        let (ii_torque_dir1_0, ii_torque_dir1_1) = (self.ii_torque_dir1[0], self.ii_torque_dir1[1]);
        let (ii_torque_dir2_0, ii_torque_dir2_1) = (self.ii_torque_dir2[0], self.ii_torque_dir2[1]);

        solver_vel1.linear +=
            (*tangents1[0] * self.impulse[0] + *tangents1[1] * self.impulse[1]).component_mul(im1);
        solver_vel1.angular +=
            ii_torque_dir1_0 * self.impulse[0] + ii_torque_dir1_1 * self.impulse[1];

        solver_vel2.linear += (*tangents1[0] * -self.impulse[0] + *tangents1[1] * -self.impulse[1])
            .component_mul(im2);
        solver_vel2.angular +=
            ii_torque_dir2_0 * self.impulse[0] + ii_torque_dir2_1 * self.impulse[1];
    }

    #[inline]
    pub fn solve(
        &mut self,
        tangents1: [&N::Vector; 2],
        im1: &N::Vector,
        im2: &N::Vector,
        limit: N,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let (torque_dir1_0, torque_dir1_1) = (self.torque_dir1[0], self.torque_dir1[1]);
        let (torque_dir2_0, torque_dir2_1) = (self.torque_dir2[0], self.torque_dir2[1]);
        let (ii_torque_dir1_0, ii_torque_dir1_1) = (self.ii_torque_dir1[0], self.ii_torque_dir1[1]);
        let (ii_torque_dir2_0, ii_torque_dir2_1) = (self.ii_torque_dir2[0], self.ii_torque_dir2[1]);

        let dvel_0 = tangents1[0].gdot(solver_vel1.linear)
            + torque_dir1_0.gdot(solver_vel1.angular)
            - tangents1[0].gdot(solver_vel2.linear)
            + torque_dir2_0.gdot(solver_vel2.angular)
            + self.rhs[0];
        let dvel_1 = tangents1[1].gdot(solver_vel1.linear)
            + torque_dir1_1.gdot(solver_vel1.angular)
            - tangents1[1].gdot(solver_vel2.linear)
            + torque_dir2_1.gdot(solver_vel2.angular)
            + self.rhs[1];

        // Exact coupled 2×2 solve — see `ContactConstraintTangentPart::solve`
        // for why the directional approximation must not be used here.
        let k11 = self.r[0];
        let k22 = self.r[1];
        let k12 = self.r[2] * N::splat(0.5);
        let inv_det = crate::physics::engine::utils::simd_inv(k11 * k22 - k12 * k12);
        let delta_impulse = na::vector![
            (k22 * dvel_0 - k12 * dvel_1) * inv_det,
            (k11 * dvel_1 - k12 * dvel_0) * inv_det
        ];
        let new_impulse = self.impulse - delta_impulse;
        let new_impulse = {
            let _disable_fe_except =
                    crate::physics::engine::utils::DisableFloatingPointExceptionsFlags::
                    disable_floating_point_exceptions();
            new_impulse.simd_cap_magnitude(limit)
        };

        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;

        solver_vel1.linear +=
            (*tangents1[0] * dlambda[0] + *tangents1[1] * dlambda[1]).component_mul(im1);
        solver_vel1.angular += ii_torque_dir1_0 * dlambda[0] + ii_torque_dir1_1 * dlambda[1];

        solver_vel2.linear +=
            (*tangents1[0] * -dlambda[0] + *tangents1[1] * -dlambda[1]).component_mul(im2);
        solver_vel2.angular += ii_torque_dir2_0 * dlambda[0] + ii_torque_dir2_1 * dlambda[1];
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ContactConstraintTwistPartSlim<N: ScalarType> {
    pub rhs: N,
    pub impulse: N,
    pub impulse_accumulator: N,
    pub r: N,
}

impl<N: ScalarType> ContactConstraintTwistPartSlim<N> {
    #[inline]
    pub fn warmstart(
        &mut self,
        twist_dir1: &N::AngVector,
        ii1: &N::AngInertia,
        ii2: &N::AngInertia,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) {
        let ii_twist_dir1 = ii1.transform_vector(*twist_dir1);
        let ii_twist_dir2 = ii2.transform_vector(*twist_dir1);
        solver_vel1.angular += ii_twist_dir1 * self.impulse;
        solver_vel2.angular -= ii_twist_dir2 * self.impulse;
    }

    #[inline]
    pub fn solve(
        &mut self,
        twist_dir1: &N::AngVector,
        ii1: &N::AngInertia,
        ii2: &N::AngInertia,
        limit: N,
        solver_vel1: &mut SolverVel<N>,
        solver_vel2: &mut SolverVel<N>,
    ) where
        N::AngVector: DotProduct<N::AngVector, Result = N>,
    {
        let ii_twist_dir1 = ii1.transform_vector(*twist_dir1);
        let ii_twist_dir2 = ii2.transform_vector(*twist_dir1);

        let dvel = twist_dir1.gdot(solver_vel1.angular - solver_vel2.angular) + self.rhs;
        let new_impulse = (self.impulse - self.r * dvel).simd_clamp(-limit, limit);
        let dlambda = new_impulse - self.impulse;
        self.impulse = new_impulse;
        solver_vel1.angular += ii_twist_dir1 * dlambda;
        solver_vel2.angular -= ii_twist_dir2 * dlambda;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::physics::engine::dynamics::solver::SolverVel;
    use crate::physics::engine::math::{Real, Vector};

    fn vels() -> (SolverVel<Real>, SolverVel<Real>) {
        let mut v1 = SolverVel::<Real>::zero();
        let mut v2 = SolverVel::<Real>::zero();
        {
            v1.linear = Vector::new(0.3, -1.2, 0.5);
            v1.angular = Vector::new(0.7, 0.1, -0.3);
            v2.linear = Vector::new(-0.4, 0.9, -0.6);
            v2.angular = Vector::new(-0.2, 0.4, 0.6);
        }
        (v1, v2)
    }

    fn dir_im() -> (Vector, Vector, Vector) {
        {
            (
                Vector::new(0.0, 1.0, 0.0),
                Vector::new(0.5, 0.5, 0.5),
                Vector::new(0.25, 0.25, 0.25),
            )
        }
    }

    /// An inactive tangent slot is `impulse = 0`, `limit = 0` and, in 3D, `r = [1, 1, 0]`
    /// (the coupled solve divides by an `r`-weighted form: a zero `r` gives `inf`, and capping
    /// an infinite vector to the zero limit is NaN). The solve must stay a finite no-op.
    #[test]
    fn inactive_tangent_slot_is_finite_noop() {
        let (_dir1, im1, im2) = dir_im();

        let mut part = ContactConstraintTangentPart::<Real>::zero();
        {
            part.r = [1.0, 1.0, 0.0];
            part.rhs = [3.0, -2.0];
        }

        let tangents1_v = [Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0)];
        let tangents1 = [&tangents1_v[0], &tangents1_v[1]];

        let (mut v1, mut v2) = vels();
        let (v1_before, v2_before) = vels();
        part.solve(tangents1, &im1, &im2, 0.0, &mut v1, &mut v2);

        assert_eq!(part.impulse, TangentImpulse::<Real>::zeros());
        assert_eq!(v1.linear, v1_before.linear);
        assert_eq!(v1.angular, v1_before.angular);
        assert_eq!(v2.linear, v2_before.linear);
        assert_eq!(v2.angular, v2_before.angular);
        assert!(part.impulse.iter().all(|x| x.is_finite()));
    }
}
