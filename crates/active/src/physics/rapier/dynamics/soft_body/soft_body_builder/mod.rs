// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! The soft-body builder: its settings struct, the builder struct itself and the module wiring.

mod soft_body_build;
mod soft_body_builder;
mod soft_body_builder_mesh_helpers;
mod soft_body_builder_settings;
mod soft_body_builder_shapes;

pub use self::soft_body_builder::{SoftBodyBuilder, SoftBodyParticleSettings};
pub(crate) use self::soft_body_builder_mesh_helpers::surface_edge_table;
pub(crate) use self::soft_body_builder_mesh_helpers::{
    cell_faces, element_vertices, element_vertices_mut, surface_element_cells, surface_is_closed,
    surface_rings, surface_vertex_elements,
};
