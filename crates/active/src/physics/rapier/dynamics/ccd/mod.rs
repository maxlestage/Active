// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
pub use self::ccd_solver::CCDSolver;
pub(crate) use self::sweeps::shape_never_ccd_swept;

mod ccd_solver;
mod sweeps;
