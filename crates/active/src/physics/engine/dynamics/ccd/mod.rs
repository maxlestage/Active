// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
pub use self::ccd_solver::CCDSolver;
pub(crate) use self::sweeps::shape_never_ccd_swept;

mod ccd_solver;
mod sweeps;
