// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
//! Shape constructors of the soft-body builder: meshes, ropes, cloths, cuboids, spheres, polygons, disks and grids.

use crate::physics::engine::alloc_prelude::*;
use crate::physics::engine::dynamics::SpringCoefficients;
use crate::physics::engine::geometry::ColliderBuilder;
use crate::physics::engine::math::{DIM, Real, Vector};
use na::SimdRealField;
use parry::utils::hashmap::HashMap;

use super::super::{SoftBodyCellModel, SoftBodyMaterial};
use super::soft_body_builder_mesh_helpers::icosphere;
use super::soft_body_builder_mesh_helpers::mean_edge_length;
use super::soft_body_builder_mesh_helpers::{cell_faces, drop_stray_pieces};
use super::{SoftBodyBuilder, SoftBodyParticleSettings};

impl SoftBodyBuilder {
    /// A builder over the given world-space particle positions, with no element.
    pub fn new(positions: Vec<Vector>) -> Self {
        Self {
            positions,
            masses: Vec::new(),
            particle_mass: 1.0,
            pinned: Vec::new(),
            edges: Vec::new(),
            bend_edges: Vec::new(),
            tension_only_edges: Vec::new(),
            edge_softness: Vec::new(),
            edge_tear_resistance: Vec::new(),
            dihedrals: Vec::new(),
            cells: Vec::new(),
            surface: Vec::new(),
            material: SoftBodyMaterial::default(),
            cell_model: SoftBodyCellModel::default(),
            volume_preservation: false,
            volume_factor: 1.0,
            shape_matching: false,
            self_contacts: false,
            oriented: None,
            particle_radius: 0.01,
            skin: None,
            wire: Vec::new(),
            skin_collision: false,
            collider_template: Some(ColliderBuilder::ball(0.05).density(0.0)),
            particle_settings: SoftBodyParticleSettings::default(),
            user_data: 0,
        }
    }

    /*
     * Generators.
     */

    /// A triangle-mesh soft body without volumetric cells (`None` if empty): vertices are
    /// particles, edges are structural and shape matching holds the shape. 3D: the triangles are
    /// the surface (dihedral bending across interior edges); 2D: the boundary is the surface.
    pub fn trimesh(vertices: Vec<Vector>, indices: Vec<[u32; 3]>) -> Option<Self> {
        if vertices.is_empty() || indices.is_empty() {
            return None;
        }
        {
            let mut builder = Self::new(vertices).surface(indices);
            let edges = builder.surface_edges();
            let dihedrals = builder.surface_dihedrals();
            let mean_edge = mean_edge_length(&builder.positions, &edges);
            builder = builder
                .edges(edges)
                .dihedrals(dihedrals)
                .shape_matching(true)
                .particle_radius(mean_edge * 0.5);
            Some(builder)
        }
    }

    /// A rope of `num_particles` particles from `start` to `end`: structural edges between
    /// neighbors, bending edges between second neighbors; in 2D the segments are also the surface.
    /// The particle radius (the rope's thickness) defaults to half the segment length.
    pub fn rope(start: Vector, end: Vector, num_particles: usize) -> Self {
        let num_particles = num_particles.max(2);
        let positions: Vec<Vector> = (0..num_particles)
            .map(|i| start.lerp(end, i as Real / (num_particles - 1) as Real))
            .collect();
        let edges: Vec<[u32; 2]> = (0..num_particles as u32 - 1).map(|i| [i, i + 1]).collect();
        let bend_edges = (0..num_particles as u32)
            .filter_map(|i| (i + 2 < num_particles as u32).then_some([i, i + 2]))
            .collect();
        let segment = (end - start).length() / (num_particles - 1) as Real;
        let builder = Self::new(positions).particle_radius(segment * 0.5);
        // The rope collides through its own segments: a polyline surface in 2D, a wire in 3D.
        let builder = builder.wire(edges.clone());
        builder.edges(edges).bend_edges(bend_edges)
    }

    /// A rectangular cloth of `nu × nv` particles (3D only): `origin + i * du + j * dv`, with
    /// structural, shear and bending edges, and a triangle surface for rendering. The particle
    /// radius defaults to half the smallest edge length so the particles tile the surface.
    pub fn cloth(origin: Vector, du: Vector, dv: Vector, nu: usize, nv: usize) -> Self {
        let (nu, nv) = (nu.max(2), nv.max(2));
        let idx = |i: usize, j: usize| (i * nv + j) as u32;
        let mut positions = Vec::with_capacity(nu * nv);
        for i in 0..nu {
            for j in 0..nv {
                positions.push(origin + du * i as Real + dv * j as Real);
            }
        }
        let mut edges = Vec::new();
        let mut bend_edges = Vec::new();
        let mut surface = Vec::new();
        for i in 0..nu {
            for j in 0..nv {
                if i + 1 < nu {
                    edges.push([idx(i, j), idx(i + 1, j)]);
                }
                if j + 1 < nv {
                    edges.push([idx(i, j), idx(i, j + 1)]);
                }
                if i + 1 < nu && j + 1 < nv {
                    // Shear edges (both diagonals).
                    edges.push([idx(i, j), idx(i + 1, j + 1)]);
                    edges.push([idx(i + 1, j), idx(i, j + 1)]);
                    // Two triangles per quad, alternating diagonals.
                    if (i + j) % 2 == 0 {
                        surface.push([idx(i, j), idx(i + 1, j), idx(i + 1, j + 1)]);
                        surface.push([idx(i, j), idx(i + 1, j + 1), idx(i, j + 1)]);
                    } else {
                        surface.push([idx(i, j), idx(i + 1, j), idx(i, j + 1)]);
                        surface.push([idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)]);
                    }
                }
                if i + 2 < nu {
                    bend_edges.push([idx(i, j), idx(i + 2, j)]);
                }
                if j + 2 < nv {
                    bend_edges.push([idx(i, j), idx(i, j + 2)]);
                }
            }
        }
        let radius = du.length().min(dv.length()) * 0.5;
        Self::new(positions)
            .edges(edges)
            .bend_edges(bend_edges)
            .surface(surface)
            .particle_radius(radius)
    }

    /// A cloth tube: `num_along` rings of `num_around` particles around the segment `origin` to
    /// `origin + axis`, radius linear from `radius_start` to `radius_end`; edges and surface of
    /// [`Self::cloth`] closed around the axis. Ring-major particles: ring `0` is `0..num_around`.
    pub fn cloth_tube(
        origin: Vector,
        axis: Vector,
        radius_start: Real,
        radius_end: Real,
        num_around: usize,
        num_along: usize,
    ) -> Self {
        use crate::physics::engine::utils::OrthonormalBasis;
        let (na, nl) = (num_around.max(3), num_along.max(2));
        let dir = axis.try_normalize().unwrap_or(Vector::Y);
        let [u, v] = dir.orthonormal_basis();
        let idx = |ring: usize, k: usize| (ring * na + (k % na)) as u32;
        let mut positions = Vec::with_capacity(na * nl);
        for ring in 0..nl {
            let t = ring as Real / (nl - 1) as Real;
            let radius = radius_start + (radius_end - radius_start) * t;
            let center = origin + axis * t;
            for k in 0..na {
                let a = k as Real / na as Real * Real::simd_two_pi();
                positions.push(center + (u * a.cos() + v * a.sin()) * radius);
            }
        }
        let mut edges = Vec::new();
        let mut bend_edges = Vec::new();
        let mut surface = Vec::new();
        for ring in 0..nl {
            for k in 0..na {
                edges.push([idx(ring, k), idx(ring, k + 1)]);
                bend_edges.push([idx(ring, k), idx(ring, k + 2)]);
                if ring + 1 < nl {
                    edges.push([idx(ring, k), idx(ring + 1, k)]);
                    edges.push([idx(ring, k), idx(ring + 1, k + 1)]);
                    edges.push([idx(ring, k + 1), idx(ring + 1, k)]);
                    if (ring + k) % 2 == 0 {
                        surface.push([idx(ring, k), idx(ring + 1, k), idx(ring + 1, k + 1)]);
                        surface.push([idx(ring, k), idx(ring + 1, k + 1), idx(ring, k + 1)]);
                    } else {
                        surface.push([idx(ring, k), idx(ring + 1, k), idx(ring, k + 1)]);
                        surface.push([idx(ring + 1, k), idx(ring + 1, k + 1), idx(ring, k + 1)]);
                    }
                }
                if ring + 2 < nl {
                    bend_edges.push([idx(ring, k), idx(ring + 2, k)]);
                }
            }
        }
        let ring_step = radius_start.min(radius_end) * Real::simd_two_pi() / na as Real;
        let radius = ring_step.min(axis.length() / (nl - 1) as Real) * 0.5;
        Self::new(positions)
            .edges(edges)
            .bend_edges(bend_edges)
            .surface(surface)
            .particle_radius(radius)
    }

    /// [`Self::cloth`] with anisotropic stiffness: the edges along `du` (warp), along `dv`
    /// (weft) and the diagonals (shear) get their own softness (the material's `bend_softness`
    /// still applies to the bending edges).
    pub fn cloth_anisotropic(
        origin: Vector,
        du: Vector,
        dv: Vector,
        nu: usize,
        nv: usize,
        warp: SpringCoefficients<Real>,
        weft: SpringCoefficients<Real>,
        shear: SpringCoefficients<Real>,
    ) -> Self {
        let mut builder = Self::cloth(origin, du, dv, nu, nv);
        let (nu, nv) = (nu.max(2), nv.max(2));
        let idx = |i: usize, j: usize| (i * nv + j) as u32;
        // Same edge order as `cloth`.
        let mut k = 0u32;
        for i in 0..nu {
            for j in 0..nv {
                if i + 1 < nu {
                    debug_assert_eq!(builder.edges[k as usize], [idx(i, j), idx(i + 1, j)]);
                    builder.edge_softness.push((k, warp));
                    k += 1;
                }
                if j + 1 < nv {
                    builder.edge_softness.push((k, weft));
                    k += 1;
                }
                if i + 1 < nu && j + 1 < nv {
                    builder.edge_softness.push((k, shear));
                    builder.edge_softness.push((k + 1, shear));
                    k += 2;
                }
            }
        }
        builder
    }

    /// A solid box tetrahedralized on a `nx × ny × nz` particle grid (3D only), with structural
    /// edges along the tetrahedra edges and the boundary triangles as surface.
    ///
    /// The particle radius defaults to half the smallest cell size.
    pub fn cuboid(center: Vector, half_extents: Vector, nx: usize, ny: usize, nz: usize) -> Self {
        let (nx, ny, nz) = (nx.max(2), ny.max(2), nz.max(2));
        let step = Vector::new(
            2.0 * half_extents.x / (nx - 1) as Real,
            2.0 * half_extents.y / (ny - 1) as Real,
            2.0 * half_extents.z / (nz - 1) as Real,
        );
        let idx = |i: usize, j: usize, k: usize| ((i * ny + j) * nz + k) as u32;
        let mut positions = Vec::with_capacity(nx * ny * nz);
        for i in 0..nx {
            for j in 0..ny {
                for k in 0..nz {
                    positions.push(
                        center - half_extents
                            + Vector::new(
                                step.x * i as Real,
                                step.y * j as Real,
                                step.z * k as Real,
                            ),
                    );
                }
            }
        }
        // Five tetrahedra per cube, split parity alternating so neighboring cubes share their
        // face diagonals.
        let mut cells = Vec::new();
        for i in 0..nx - 1 {
            for j in 0..ny - 1 {
                for k in 0..nz - 1 {
                    // Corner `c` has bits (x, y, z) = (c & 1, c >> 1 & 1, c >> 2 & 1).
                    let v: [u32; 8] = core::array::from_fn(|c| {
                        idx(i + (c & 1), j + ((c >> 1) & 1), k + ((c >> 2) & 1))
                    });
                    let tets: [[usize; 4]; 5] = if (i + j + k) % 2 == 0 {
                        [
                            [0, 3, 5, 6],
                            [1, 0, 3, 5],
                            [2, 0, 3, 6],
                            [4, 0, 5, 6],
                            [7, 3, 5, 6],
                        ]
                    } else {
                        [
                            [1, 2, 4, 7],
                            [0, 1, 2, 4],
                            [3, 1, 2, 7],
                            [5, 1, 4, 7],
                            [6, 2, 4, 7],
                        ]
                    };
                    for t in tets {
                        cells.push([v[t[0]], v[t[1]], v[t[2]], v[t[3]]]);
                    }
                }
            }
        }
        let radius = step.x.min(step.y).min(step.z) * 0.5;
        Self::new(positions).cells(cells).particle_radius(radius)
    }

    /// A volumetric body filling a closed, outward-oriented mesh (triangles in 3D, segments in 2D)
    /// with cells of size `cell_size`; the surface is their boundary. Particle radius defaults to
    /// half the mean cell edge; `None` if the mesh is empty, open or encloses nothing at that size.
    // Filling a 2D boundary goes through parry's Delaunay refinement, which the 2D build
    // enables with `std` (see the crate's feature list).
    pub fn volumetric(
        vertices: &[Vector],
        indices: &[[u32; DIM]],
        cell_size: Real,
    ) -> Option<Self> {
        Self::volumetric_with(
            vertices,
            indices,
            &parry::transformation::VolumeMeshParameters::new(cell_size),
        )
    }

    /// The same as [`Self::volumetric`], with the meshing parameters spelled out: element size,
    /// the cover's smoothing, guard and subdivision, and whether the surface alone is covered
    /// (the crust).
    // Filling a 2D boundary goes through parry's Delaunay refinement, which the 2D build
    // enables with `std` (see the crate's feature list).
    pub fn volumetric_with(
        vertices: &[Vector],
        indices: &[[u32; DIM]],
        params: &parry::transformation::VolumeMeshParameters,
    ) -> Option<Self> {
        let mut mesh = parry::transformation::volume_mesh(vertices, indices, params)?;
        let components = mesh.connected_components();
        let count = components.iter().max().map(|id| *id as usize + 1)?;
        if count > 1 {
            drop_stray_pieces(&mut mesh, &components, count);
        }
        Self::from_cells(mesh)
    }

    /// A body of the cells of a filled mesh, with the particle radius the mesh asks for.
    fn from_cells(mesh: parry::transformation::VolumeMesh) -> Option<Self> {
        let mut builder = Self::new(mesh.vertices).cells(mesh.cells);
        // Half the mean edge of the *boundary*: with a subdivided cover the interior edges are
        // longer than the boundary's, and it is the boundary the radius is a thickness for.
        let mut counts: HashMap<[u32; DIM], u32> = HashMap::default();
        for cell in &builder.cells {
            for face in cell_faces(*cell) {
                let mut key = face;
                key.sort_unstable();
                *counts.entry(key).or_insert(0) += 1;
            }
        }
        let mut seen: HashMap<[u32; 2], ()> = HashMap::default();
        let mut edges = Vec::new();
        for (face, count) in &counts {
            if *count != 1 {
                continue;
            }
            for k in 0..DIM {
                let (a, b) = (face[k], face[(k + 1) % DIM]);
                let key = [a.min(b), a.max(b)];
                if seen.insert(key, ()).is_none() {
                    edges.push(key);
                }
            }
        }
        let radius = mean_edge_length(&builder.positions, &edges) * 0.5;
        builder = builder.particle_radius(radius);
        Some(builder)
    }

    /// The same as [`Self::volumetric`], keeping the boundary mesh as the body's skin: the cells
    /// hold it and the body is drawn as it, so thin features survive a cell size that cannot
    /// resolve them; it still collides through its cells, so contacts are as coarse as they are.
    // Filling a 2D boundary goes through parry's Delaunay refinement, which the 2D build
    // enables with `std` (see the crate's feature list).
    pub fn volumetric_skinned(
        vertices: &[Vector],
        indices: &[[u32; DIM]],
        cell_size: Real,
    ) -> Option<Self> {
        let builder = Self::volumetric(vertices, indices, cell_size)?;
        Some(builder.skin(vertices.to_vec(), indices.to_vec()))
    }

    /// A hollow sphere (3D only): an icosphere surface with `subdivisions` refinement levels,
    /// structural edges along the triangle edges, dihedral bending constraints across them, and
    /// volume preservation. The particle radius defaults to half the mean edge length.
    pub fn sphere(center: Vector, radius: Real, subdivisions: usize) -> Self {
        let (positions, triangles) = icosphere(center, radius, subdivisions);
        let mut builder = Self::new(positions).surface(triangles);
        let dihedrals = builder.surface_dihedrals();
        let edges = builder.surface_edges();
        let mean_edge = edges
            .iter()
            .map(|e| (builder.positions[e[0] as usize] - builder.positions[e[1] as usize]).length())
            .sum::<Real>()
            / edges.len().max(1) as Real;
        builder = builder
            .edges(edges)
            .dihedrals(dihedrals)
            .volume_preservation(true)
            .particle_radius(mean_edge * 0.5);
        builder
    }
}
