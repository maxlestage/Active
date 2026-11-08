// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! The scalar N-particle constraint shared by the soft-body element kinds (distance,
//! dihedral bending, cell volume), and the block constraint of the corotational elastic cells.

mod soft_elastic_constraint;
mod soft_element_linalg;
mod soft_neo_hookean;
mod soft_scalar_constraint;
#[cfg(test)]
mod test;

pub(crate) use self::soft_elastic_constraint::*;
pub(crate) use self::soft_element_linalg::*;
pub(crate) use self::soft_neo_hookean::*;
pub(crate) use self::soft_scalar_constraint::*;
