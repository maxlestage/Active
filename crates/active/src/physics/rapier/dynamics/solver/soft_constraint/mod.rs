// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Soft-body constraints solved by the staged island solver alongside joints and contacts.

pub(crate) use self::soft_constraints_set::SoftConstraintsSet;

pub(crate) mod soft_attachment;
pub(crate) mod soft_constraints_set;
pub(crate) mod soft_contact;
mod soft_contact_assembly;
mod soft_contact_chunks;
pub(crate) mod soft_element_constraint;
mod soft_element_constraint_assembly;
