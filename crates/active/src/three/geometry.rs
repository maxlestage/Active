//! Geometries: vertices (positions, normals, texture coordinates) and the triangles
//! that join them, like Three.js's `BufferGeometry` and its ready-made shapes.

use std::f32::consts::{PI, TAU};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::math::{Quaternion, Vector3};
use crate::physics::{Collider, Shape};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Vertices and triangles. Shared between meshes ([`Geometry`] is an `Rc`), and uploaded
/// to the GPU once per renderer.
#[derive(Debug, Default)]
pub struct BufferGeometry {
    id: u64,
    /// `x, y, z` of each vertex.
    pub positions: Vec<f32>,
    /// The unit normal of each vertex.
    pub normals: Vec<f32>,
    /// `u, v` of each vertex.
    pub uvs: Vec<f32>,
    /// Three vertex indices per triangle, counter-clockwise when seen from the front.
    pub indices: Vec<u32>,
    /// How it collides (`None`: as its bounding box).
    collider: Option<Collider>,
    /// A surface without volume (a plane): a floor, which stays where it is.
    flat: bool,
}

/// A shared geometry.
pub type Geometry = Rc<BufferGeometry>;

impl BufferGeometry {
    /// A geometry from its arrays (normals are computed when `normals` is empty).
    pub fn new(
        positions: Vec<f32>,
        normals: Vec<f32>,
        uvs: Vec<f32>,
        indices: Vec<u32>,
    ) -> Geometry {
        let mut g = BufferGeometry {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            positions,
            normals,
            uvs,
            indices,
            collider: None,
            flat: false,
        };
        if g.normals.len() != g.positions.len() {
            g.compute_vertex_normals();
        }
        if g.uvs.len() / 2 != g.vertex_count() {
            g.uvs = vec![0.0; g.vertex_count() * 2];
        }
        Rc::new(g)
    }

    /// How it collides with other bodies ([`crate::physics`]): the exact shape for the
    /// ready-made geometries, `None` for others (they collide as their bounding box).
    pub fn collider(&self) -> Option<&Collider> {
        self.collider.as_ref()
    }

    /// Whether it is a surface without volume (a plane, a disc): physics keeps it in place.
    pub fn is_flat(&self) -> bool {
        self.flat
    }

    /// The same geometry, colliding as `collider` (copied if it is shared).
    pub fn with_collider(geometry: Geometry, collider: Collider) -> Geometry {
        with_collider(geometry, collider, false)
    }

    /// A number unique to this geometry (the key of its GPU buffers).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// The number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Smooth normals: the average of the faces around each vertex
    /// (Three.js's `computeVertexNormals`).
    pub fn compute_vertex_normals(&mut self) {
        let mut normals = vec![0.0f32; self.positions.len()];
        let p = |i: usize| {
            Vector3::new(
                self.positions[i * 3],
                self.positions[i * 3 + 1],
                self.positions[i * 3 + 2],
            )
        };
        for t in self.indices.chunks_exact(3) {
            let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let n = (p(b) - p(a)).cross(p(c) - p(a));
            for i in [a, b, c] {
                normals[i * 3] += n.x;
                normals[i * 3 + 1] += n.y;
                normals[i * 3 + 2] += n.z;
            }
        }
        for n in normals.chunks_exact_mut(3) {
            let v = Vector3::new(n[0], n[1], n[2]).normalize();
            n.copy_from_slice(&v.to_array());
        }
        self.normals = normals;
    }

    /// The smallest box around the vertices: `(min, max)`.
    pub fn bounding_box(&self) -> (Vector3, Vector3) {
        let mut min = Vector3::splat(f32::INFINITY);
        let mut max = Vector3::splat(f32::NEG_INFINITY);
        for p in self.positions.chunks_exact(3) {
            let v = Vector3::new(p[0], p[1], p[2]);
            min = min.min(v);
            max = max.max(v);
        }
        (min, max)
    }
}

/// Sets the collider of a geometry that was just made (or of a copy, if shared).
pub(crate) fn with_collider(mut geometry: Geometry, collider: Collider, flat: bool) -> Geometry {
    if Rc::get_mut(&mut geometry).is_none() {
        let g = &*geometry;
        geometry = BufferGeometry::new(
            g.positions.clone(),
            g.normals.clone(),
            g.uvs.clone(),
            g.indices.clone(),
        );
    }
    if let Some(g) = Rc::get_mut(&mut geometry) {
        g.collider = Some(collider);
        g.flat = flat;
    }
    geometry
}

/// Half the thickness of the slab under a flat surface: thick enough that fast objects
/// do not go through it in one step.
pub(crate) fn slab(size: f32) -> f32 {
    (size * 0.05).clamp(0.02, 0.5)
}

/// Builds the vertices of a geometry, then makes it.
#[derive(Default)]
struct Builder {
    positions: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
    indices: Vec<u32>,
}

impl Builder {
    fn vertex(&mut self, p: Vector3, n: Vector3, u: f32, v: f32) -> u32 {
        let i = (self.positions.len() / 3) as u32;
        self.positions.extend_from_slice(&p.to_array());
        self.normals.extend_from_slice(&n.to_array());
        self.uvs.extend_from_slice(&[u, v]);
        i
    }

    /// A grid of `(cols + 1) × (rows + 1)` vertices made by `at(u, v)`, joined in
    /// quads; `at` gives a position and a normal.
    fn grid(&mut self, cols: u32, rows: u32, at: impl Fn(f32, f32) -> (Vector3, Vector3)) {
        let first = (self.positions.len() / 3) as u32;
        for j in 0..=rows {
            for i in 0..=cols {
                let (u, v) = (i as f32 / cols as f32, j as f32 / rows as f32);
                let (p, n) = at(u, v);
                self.vertex(p, n, u, v);
            }
        }
        let row = cols + 1;
        for j in 0..rows {
            for i in 0..cols {
                let a = first + j * row + i;
                let (b, c, d) = (a + 1, a + row, a + row + 1);
                self.indices.extend_from_slice(&[a, b, d, a, d, c]);
            }
        }
    }

    fn build(self) -> Geometry {
        BufferGeometry::new(self.positions, self.normals, self.uvs, self.indices)
    }
}

/// A box centered on the origin (Three.js's `BoxGeometry`).
pub struct BoxGeometry;

// `new` gives the shared geometry, like `new THREE.BoxGeometry(...)`, not a `BoxGeometry`.
#[allow(clippy::new_ret_no_self)]
impl BoxGeometry {
    /// A box of `width` (x), `height` (y) and `depth` (z).
    pub fn new(width: f32, height: f32, depth: f32) -> Geometry {
        let (w, h, d) = (width / 2.0, height / 2.0, depth / 2.0);
        let mut b = Builder::default();
        // Chaque face : son normal, et les deux axes qui la parcourent (u vers la droite,
        // v vers le haut, vus de l'extérieur).
        let faces = [
            (
                Vector3::X,
                Vector3::new(0.0, 0.0, -d),
                Vector3::new(0.0, h, 0.0),
                w,
            ),
            (
                Vector3::NEG_X,
                Vector3::new(0.0, 0.0, d),
                Vector3::new(0.0, h, 0.0),
                w,
            ),
            (
                Vector3::Y,
                Vector3::new(w, 0.0, 0.0),
                Vector3::new(0.0, 0.0, -d),
                h,
            ),
            (
                Vector3::NEG_Y,
                Vector3::new(w, 0.0, 0.0),
                Vector3::new(0.0, 0.0, d),
                h,
            ),
            (
                Vector3::Z,
                Vector3::new(w, 0.0, 0.0),
                Vector3::new(0.0, h, 0.0),
                d,
            ),
            (
                Vector3::NEG_Z,
                Vector3::new(-w, 0.0, 0.0),
                Vector3::new(0.0, h, 0.0),
                d,
            ),
        ];
        for (normal, right, up, offset) in faces {
            let center = normal * offset;
            b.grid(1, 1, |u, v| {
                (
                    center + right * (u * 2.0 - 1.0) + up * (v * 2.0 - 1.0),
                    normal,
                )
            });
        }
        with_collider(b.build(), Collider::cuboid(width, height, depth), false)
    }
}

/// A flat rectangle in the `xy` plane, facing `+z` (Three.js's `PlaneGeometry`).
pub struct PlaneGeometry;

// `new` gives the shared geometry, like `new THREE.PlaneGeometry(...)`, not a `PlaneGeometry`.
#[allow(clippy::new_ret_no_self)]
impl PlaneGeometry {
    /// A plane of `width` × `height`.
    pub fn new(width: f32, height: f32) -> Geometry {
        let mut b = Builder::default();
        b.grid(1, 1, |u, v| {
            (
                Vector3::new((u - 0.5) * width, (v - 0.5) * height, 0.0),
                Vector3::Z,
            )
        });
        with_collider(
            b.build(),
            {
                let t = slab(width.max(height));
                Collider::new(Shape::Cuboid {
                    half_extents: Vector3::new(width / 2.0, height / 2.0, t),
                })
                .with_offset(Vector3::new(0.0, 0.0, -t))
            },
            true,
        )
    }
}

/// A sphere (Three.js's `SphereGeometry`).
pub struct SphereGeometry;

// `new` gives the shared geometry, like `new THREE.SphereGeometry(...)`, not a `SphereGeometry`.
#[allow(clippy::new_ret_no_self)]
impl SphereGeometry {
    /// A sphere of `radius`, with `width_segments` around and `height_segments` from pole
    /// to pole (32 and 16 are smooth).
    pub fn new(radius: f32, width_segments: u32, height_segments: u32) -> Geometry {
        let mut b = Builder::default();
        b.grid(width_segments.max(3), height_segments.max(2), |u, v| {
            let (phi, theta) = (u * TAU, (1.0 - v) * PI);
            // Aux pôles, exactement sur l'axe : `sin(π)` en f32 ne vaut pas 0.
            let (sin, cos) = match v {
                0.0 => (0.0, -1.0),
                1.0 => (0.0, 1.0),
                _ => theta.sin_cos(),
            };
            let n = Vector3::new(-phi.cos() * sin, cos, phi.sin() * sin);
            (n * radius, n)
        });
        with_collider(b.build(), Collider::sphere(radius), false)
    }
}

/// A cylinder along `y`, possibly narrower at the top (Three.js's `CylinderGeometry`).
pub struct CylinderGeometry;

// `new` gives the shared geometry, like `new THREE.CylinderGeometry(...)`, not a `CylinderGeometry`.
#[allow(clippy::new_ret_no_self)]
impl CylinderGeometry {
    /// A cylinder from `radius_top` to `radius_bottom`, of `height`, with
    /// `radial_segments` around; the ends are closed.
    pub fn new(radius_top: f32, radius_bottom: f32, height: f32, radial_segments: u32) -> Geometry {
        let segments = radial_segments.max(3);
        let mut b = Builder::default();
        let half = height / 2.0;
        let slope = (radius_bottom - radius_top) / height;
        b.grid(segments, 1, |u, v| {
            let theta = u * TAU;
            let r = radius_bottom + (radius_top - radius_bottom) * v;
            let (s, c) = theta.sin_cos();
            let n = Vector3::new(s, slope, c).normalize();
            (Vector3::new(r * s, -half + v * height, r * c), n)
        });
        for (top, radius) in [(true, radius_top), (false, radius_bottom)] {
            if radius <= 0.0 {
                continue;
            }
            let (y, normal) = if top {
                (half, Vector3::Y)
            } else {
                (-half, Vector3::NEG_Y)
            };
            let center = b.vertex(Vector3::new(0.0, y, 0.0), normal, 0.5, 0.5);
            let first = center + 1;
            for i in 0..=segments {
                let theta = i as f32 / segments as f32 * TAU;
                let (s, c) = theta.sin_cos();
                b.vertex(
                    Vector3::new(radius * s, y, radius * c),
                    normal,
                    s * 0.5 + 0.5,
                    c * 0.5 + 0.5,
                );
            }
            for i in 0..segments {
                let (a, bb) = (first + i, first + i + 1);
                if top {
                    b.indices.extend_from_slice(&[center, a, bb]);
                } else {
                    b.indices.extend_from_slice(&[center, bb, a]);
                }
            }
        }
        with_collider(
            b.build(),
            if radius_top <= 0.0 {
                Collider::cone(radius_bottom, height)
            } else {
                // A frustum collides as the cylinder around it.
                Collider::cylinder(radius_top.max(radius_bottom), height)
            },
            false,
        )
    }
}

/// A cone along `y`, its point at the top (Three.js's `ConeGeometry`).
pub struct ConeGeometry;

// `new` gives the shared geometry, like `new THREE.ConeGeometry(...)`, not a `ConeGeometry`.
#[allow(clippy::new_ret_no_self)]
impl ConeGeometry {
    /// A cone of base `radius`, of `height`, with `radial_segments` around.
    pub fn new(radius: f32, height: f32, radial_segments: u32) -> Geometry {
        CylinderGeometry::new(0.0, radius, height, radial_segments)
    }
}

/// A ring (Three.js's `TorusGeometry`), in the `xy` plane.
pub struct TorusGeometry;

// `new` gives the shared geometry, like `new THREE.TorusGeometry(...)`, not a `TorusGeometry`.
#[allow(clippy::new_ret_no_self)]
impl TorusGeometry {
    /// A torus of `radius` (center to the middle of the tube) and `tube` (radius of the
    /// tube), with `radial_segments` around the tube and `tubular_segments` along it.
    pub fn new(radius: f32, tube: f32, radial_segments: u32, tubular_segments: u32) -> Geometry {
        let mut b = Builder::default();
        b.grid(tubular_segments.max(3), radial_segments.max(3), |u, v| {
            let (a, t) = (u * TAU, v * TAU);
            let center = Vector3::new(radius * a.cos(), radius * a.sin(), 0.0);
            let p = Vector3::new(
                (radius + tube * t.cos()) * a.cos(),
                (radius + tube * t.cos()) * a.sin(),
                tube * t.sin(),
            );
            (p, (p - center).normalize())
        });
        with_collider(
            b.build(), // Three.js's ring lies around z.
            Collider::new(Shape::Torus {
                major_radius: radius,
                minor_radius: tube,
            })
            .with_rotation(Quaternion::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every normal has a length of 1, every index points at a vertex, and every
    /// triangle faces the way its normals point (outwards).
    fn check(g: &BufferGeometry, center: Vector3) {
        assert_eq!(g.positions.len(), g.normals.len());
        assert_eq!(g.uvs.len() / 2, g.vertex_count());
        assert!(g.indices.iter().all(|&i| (i as usize) < g.vertex_count()));
        for n in g.normals.chunks_exact(3) {
            assert!((Vector3::new(n[0], n[1], n[2]).length() - 1.0).abs() < 1e-3);
        }
        let p = |i: u32| {
            Vector3::new(
                g.positions[i as usize * 3],
                g.positions[i as usize * 3 + 1],
                g.positions[i as usize * 3 + 2],
            )
        };
        for t in g.indices.chunks_exact(3) {
            let face = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            if face.length() < 1e-9 {
                continue;
            }
            let middle = (p(t[0]) + p(t[1]) + p(t[2])) / 3.0;
            assert!(
                face.dot(middle - center) > 0.0,
                "a triangle faces inwards at {middle:?}"
            );
        }
    }

    #[test]
    fn shapes_face_outwards() {
        let cube = BoxGeometry::new(1.0, 2.0, 3.0);
        assert_eq!((cube.vertex_count(), cube.triangle_count()), (24, 12));
        assert_eq!(
            cube.bounding_box(),
            (Vector3::new(-0.5, -1.0, -1.5), Vector3::new(0.5, 1.0, 1.5))
        );
        check(&cube, Vector3::ZERO);
        check(&SphereGeometry::new(2.0, 16, 8), Vector3::ZERO);
        check(&CylinderGeometry::new(1.0, 1.0, 2.0, 12), Vector3::ZERO);
        check(&ConeGeometry::new(1.0, 2.0, 12), Vector3::ZERO);
        let plane = PlaneGeometry::new(2.0, 2.0);
        check(&plane, Vector3::new(0.0, 0.0, -1.0));
        assert_ne!(cube.id(), plane.id());
    }

    /// The triangles of a torus face the way its normals point.
    #[test]
    fn torus_winding_follows_its_normals() {
        let g = TorusGeometry::new(1.0, 0.3, 8, 16);
        let p = |i: u32| {
            Vector3::new(
                g.positions[i as usize * 3],
                g.positions[i as usize * 3 + 1],
                g.positions[i as usize * 3 + 2],
            )
        };
        let n = |i: u32| {
            Vector3::new(
                g.normals[i as usize * 3],
                g.normals[i as usize * 3 + 1],
                g.normals[i as usize * 3 + 2],
            )
        };
        for t in g.indices.chunks_exact(3) {
            let face = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
            if face.length() > 1e-9 {
                assert!(face.dot(n(t[0]) + n(t[1]) + n(t[2])) > 0.0);
            }
        }
    }

    #[test]
    fn computed_normals() {
        let g = BufferGeometry::new(
            vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            Vec::new(),
            Vec::new(),
            vec![0, 1, 2],
        );
        assert_eq!(&g.normals[..3], &[0.0, 0.0, 1.0]);
        assert_eq!(g.uvs.len(), 6);
    }
}
