// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! Regression test for #717: `ColliderBuilder::trimesh` used to panic on
//! invalid input (e.g. an empty index buffer). It now returns a `Result`.

use active::physics::engine::prelude::*;

fn unit_triangle_vertices() -> Vec<Vector> {
    vec![
        Vector::new(0.0, 0.0, 0.0),
        Vector::new(1.0, 0.0, 0.0),
        Vector::new(0.0, 1.0, 0.0),
    ]
}

#[test]
fn trimesh_empty_input_returns_err() {
    // The original panic: "A triangle mesh must contain at least one triangle."
    assert!(ColliderBuilder::trimesh(vec![], vec![]).is_err());
    assert!(ColliderBuilder::trimesh(unit_triangle_vertices(), vec![]).is_err());
}

#[test]
fn trimesh_valid_input_returns_ok() {
    let builder = ColliderBuilder::trimesh(unit_triangle_vertices(), vec![[0, 1, 2]]);
    assert!(builder.is_ok());
}
