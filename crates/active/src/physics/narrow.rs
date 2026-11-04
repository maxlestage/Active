//! The narrow phase: the contact points of two shapes that may touch.
//!
//! Balls and boxes have exact formulas (and boxes a whole contact face, so that they
//! rest flat and stack); every other pair goes through GJK, which tells whether two
//! convex shapes overlap, and EPA, which finds how deep and in which direction.

use super::shape::{Placed, Shape};
use crate::three::math::{Quat, Vec3};

/// Contacts are kept until the shapes are this far apart (meters): a body resting on
/// another keeps its contact instead of losing and finding it again each step.
pub(crate) const MARGIN: f32 = 0.02;

/// A point where two shapes touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContactPoint {
    /// On the first shape.
    pub(crate) point_a: Vec3,
    /// On the second.
    pub(crate) point_b: Vec3,
    /// From the first shape towards the second.
    pub(crate) normal: Vec3,
    /// How deep they overlap (negative: still apart).
    pub(crate) depth: f32,
}

/// What the narrow phase found.
#[derive(Debug)]
pub(crate) enum Contacts {
    /// Apart.
    None,
    /// The whole contact area, recomputed each time.
    Full(Vec<ContactPoint>),
    /// The deepest point only: the solver keeps the previous ones to make a contact area.
    Single(ContactPoint),
}

/// The contacts of `a` and `b`.
pub(crate) fn collide(a: &Placed, b: &Placed) -> Contacts {
    match (a.shape, b.shape) {
        (Shape::Sphere { radius: ra }, Shape::Sphere { radius: rb }) => {
            sphere_sphere(a.center, ra, b.center, rb)
                .map_or(Contacts::None, |c| Contacts::Full(vec![c]))
        }
        (Shape::Sphere { radius }, Shape::Cuboid { half_extents }) => {
            sphere_box(a.center, radius, b, half_extents)
                .map_or(Contacts::None, |c| Contacts::Full(vec![c]))
        }
        (Shape::Cuboid { half_extents }, Shape::Sphere { radius }) => {
            sphere_box(b.center, radius, a, half_extents)
                .map_or(Contacts::None, |c| Contacts::Full(vec![flip(c)]))
        }
        (Shape::Cuboid { half_extents: ha }, Shape::Cuboid { half_extents: hb }) => {
            let points = box_box(a, ha, b, hb);
            if points.is_empty() {
                Contacts::None
            } else {
                Contacts::Full(points)
            }
        }
        _ => match gjk_epa(a, b) {
            Some(c) => faces(a, b, c),
            None => Contacts::None,
        },
    }
}

/// The points of a shape's face turned towards `dir` (in the world): the corners of a box
/// face, the rim of a cylinder's cap or a cone's base, the two ends of a side, or the one
/// farthest point when no face is turned that way.
fn support_face(shape: &Placed, dir: Vec3) -> Vec<Vec3> {
    let d = (shape.rotation.inverse() * dir).normalize();
    let world = |p: Vec3| shape.center + shape.rotation * p;
    const FLAT: f32 = 0.95;
    let rim = |radius: f32, y: f32| -> Vec<Vec3> {
        (0..8)
            .map(|k| {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                world(Vec3::new(radius * a.cos(), y, radius * a.sin()))
            })
            .collect()
    };
    let flat = |d: Vec3| {
        let len = (d.x * d.x + d.z * d.z).sqrt();
        if len > 1e-6 {
            Vec3::new(d.x / len, 0.0, d.z / len)
        } else {
            Vec3::X
        }
    };
    match shape.shape {
        Shape::Cuboid { half_extents: h } => {
            let axes = [d.x, d.y, d.z];
            match (0..3).find(|&i| axes[i].abs() > FLAT) {
                Some(i) => {
                    let s = axes[i].signum();
                    let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                    [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
                        .iter()
                        .map(|&(u, v)| {
                            let mut p = [0.0; 3];
                            p[i] = s * component(h, i);
                            p[j] = u * component(h, j);
                            p[k] = v * component(h, k);
                            world(Vec3::new(p[0], p[1], p[2]))
                        })
                        .collect()
                }
                None => vec![world(shape.shape.support(d))],
            }
        }
        Shape::Cylinder {
            radius,
            half_height,
        } => {
            if d.y.abs() > FLAT {
                rim(radius, d.y.signum() * half_height)
            } else if d.y.abs() < 1.0 - FLAT {
                let side = flat(d) * radius;
                vec![
                    world(side + Vec3::new(0.0, half_height, 0.0)),
                    world(side - Vec3::new(0.0, half_height, 0.0)),
                ]
            } else {
                vec![world(shape.shape.support(d))]
            }
        }
        Shape::Cone {
            radius,
            half_height,
        } => {
            if d.y < -FLAT {
                rim(radius, -half_height)
            } else {
                // Along its side: the tip and the rim point below it.
                let tip = Vec3::new(0.0, half_height, 0.0);
                let base = flat(d) * radius - Vec3::new(0.0, half_height, 0.0);
                let slant = (base - tip).normalize();
                if slant.dot(d).abs() < 1.0 - FLAT {
                    vec![world(tip), world(base)]
                } else {
                    vec![world(shape.shape.support(d))]
                }
            }
        }
        Shape::Capsule {
            radius,
            half_height,
        } if d.y.abs() < 1.0 - FLAT => {
            let side = flat(d) * radius;
            vec![
                world(side + Vec3::new(0.0, half_height, 0.0)),
                world(side - Vec3::new(0.0, half_height, 0.0)),
            ]
        }
        Shape::Torus {
            major_radius,
            minor_radius,
        } if d.y.abs() > FLAT => rim(major_radius, d.y.signum() * minor_radius),
        _ => vec![world(shape.shape.support(d))],
    }
}

/// Whether `p`, seen along `n`, is over the polygon `face` (always for a point or a
/// segment).
fn over(face: &[Vec3], n: Vec3, p: Vec3) -> bool {
    if face.len() < 3 {
        return true;
    }
    let side = |i: usize| {
        let (a, b) = (face[i], face[(i + 1) % face.len()]);
        (b - a).cross(p - a).dot(n)
    };
    let eps = 1e-4;
    (0..face.len()).all(|i| side(i) >= -eps) || (0..face.len()).all(|i| side(i) <= eps)
}

/// The contact area of two shapes that overlap along `c.normal`: the points of the smaller
/// face that reach the plane of the other one. A single point otherwise.
fn faces(a: &Placed, b: &Placed, c: ContactPoint) -> Contacts {
    let n = c.normal;
    let face_a = support_face(a, n);
    let face_b = support_face(b, -n);
    let extent = |f: &[Vec3]| {
        let center = f.iter().fold(Vec3::ZERO, |s, p| s + *p) / f.len().max(1) as f32;
        f.iter().map(|p| (*p - center).length()).fold(0.0, f32::max)
    };
    let (ea, eb) = (extent(&face_a), extent(&face_b));
    // The smaller face touches the larger one: its points, kept where they are over it.
    let mut points: Vec<ContactPoint> = if eb <= ea {
        if face_b.len() < 2 {
            return Contacts::Single(c);
        }
        face_b
            .iter()
            .filter(|&&p| over(&face_a, n, p))
            .filter_map(|&p| {
                let s = (p - c.point_a).dot(n);
                (s <= MARGIN).then(|| ContactPoint {
                    point_a: p - n * s,
                    point_b: p,
                    normal: n,
                    depth: -s,
                })
            })
            .collect()
    } else {
        if face_a.len() < 2 {
            return Contacts::Single(c);
        }
        face_a
            .iter()
            .filter(|&&p| over(&face_b, n, p))
            .filter_map(|&p| {
                let s = (c.point_b - p).dot(n);
                (s <= MARGIN).then(|| ContactPoint {
                    point_a: p,
                    point_b: p + n * s,
                    normal: n,
                    depth: -s,
                })
            })
            .collect()
    };
    if points.len() < 2 {
        return Contacts::Single(c);
    }
    reduce(&mut points);
    Contacts::Full(points)
}

fn flip(c: ContactPoint) -> ContactPoint {
    ContactPoint {
        point_a: c.point_b,
        point_b: c.point_a,
        normal: -c.normal,
        depth: c.depth,
    }
}

fn sphere_sphere(ca: Vec3, ra: f32, cb: Vec3, rb: f32) -> Option<ContactPoint> {
    let d = cb - ca;
    let dist = d.length();
    let depth = ra + rb - dist;
    if depth < -MARGIN {
        return None;
    }
    let normal = if dist > 1e-6 { d / dist } else { Vec3::Y };
    Some(ContactPoint {
        point_a: ca + normal * ra,
        point_b: cb - normal * rb,
        normal,
        depth,
    })
}

/// A ball against a box; the normal goes from the ball to the box.
fn sphere_box(center: Vec3, radius: f32, b: &Placed, half: Vec3) -> Option<ContactPoint> {
    let local = b.rotation.inverse() * (center - b.center);
    let clamped = Vec3::new(
        local.x.clamp(-half.x, half.x),
        local.y.clamp(-half.y, half.y),
        local.z.clamp(-half.z, half.z),
    );
    let (surface, normal_from_box, depth) = if clamped == local {
        // The center is inside the box: out through the nearest face.
        let gaps = [
            half.x - local.x.abs(),
            half.y - local.y.abs(),
            half.z - local.z.abs(),
        ];
        let axis = (0..3)
            .min_by(|&i, &j| gaps[i].total_cmp(&gaps[j]))
            .unwrap_or(0);
        let mut surface = local;
        let mut normal = Vec3::ZERO;
        let side = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
        match axis {
            0 => {
                surface.x = side(local.x) * half.x;
                normal.x = side(local.x);
            }
            1 => {
                surface.y = side(local.y) * half.y;
                normal.y = side(local.y);
            }
            _ => {
                surface.z = side(local.z) * half.z;
                normal.z = side(local.z);
            }
        }
        (surface, normal, radius + gaps[axis])
    } else {
        let d = local - clamped;
        let dist = d.length();
        (clamped, d / dist, radius - dist)
    };
    if depth < -MARGIN {
        return None;
    }
    let normal = b.rotation * normal_from_box;
    // From the ball to the box: the opposite of the box's outward normal.
    Some(ContactPoint {
        point_a: center - normal * radius,
        point_b: b.center + b.rotation * surface,
        normal: -normal,
        depth,
    })
}

fn axes(q: Quat) -> [Vec3; 3] {
    [q * Vec3::X, q * Vec3::Y, q * Vec3::Z]
}

fn component(v: Vec3, i: usize) -> f32 {
    match i {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

#[derive(Clone, Copy)]
enum Axis {
    FaceA(usize),
    FaceB(usize),
    Edge(usize, usize),
}

/// Two boxes, by the separating axis theorem: the axis along which they overlap least
/// gives the normal; a face against a face gives up to four points (the incident face
/// clipped by the reference face), an edge against an edge one point.
fn box_box(a: &Placed, ha: Vec3, b: &Placed, hb: Vec3) -> Vec<ContactPoint> {
    let (ua, ub) = (axes(a.rotation), axes(b.rotation));
    let t = b.center - a.center;
    let overlap = |l: Vec3| {
        let ra: f32 = (0..3).map(|k| component(ha, k) * ua[k].dot(l).abs()).sum();
        let rb: f32 = (0..3).map(|k| component(hb, k) * ub[k].dot(l).abs()).sum();
        ra + rb - t.dot(l).abs()
    };
    let mut best_face = (f32::INFINITY, Axis::FaceA(0));
    for i in 0..3 {
        for (axis, kind) in [(ua[i], Axis::FaceA(i)), (ub[i], Axis::FaceB(i))] {
            let o = overlap(axis);
            if o < -MARGIN {
                return Vec::new();
            }
            if o < best_face.0 {
                best_face = (o, kind);
            }
        }
    }
    let mut best_edge = (f32::INFINITY, Axis::Edge(0, 0), Vec3::ZERO);
    for (i, axis_a) in ua.iter().enumerate() {
        for (j, axis_b) in ub.iter().enumerate() {
            let l = axis_a.cross(*axis_b);
            let len = l.length();
            if len < 1e-4 {
                continue; // parallel edges: the face axes cover them
            }
            let l = l / len;
            let o = overlap(l);
            if o < -MARGIN {
                return Vec::new();
            }
            if o < best_edge.0 {
                best_edge = (o, Axis::Edge(i, j), l);
            }
        }
    }
    // Faces are preferred: an edge axis wins only when clearly better, so that a box
    // lying on another does not switch between the two cases.
    if best_edge.0 < best_face.0 - 0.01 {
        let Axis::Edge(i, j) = best_edge.1 else {
            return Vec::new();
        };
        let mut l = best_edge.2;
        if l.dot(t) < 0.0 {
            l = -l;
        }
        let side = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
        let mut pa = a.center;
        let mut pb = b.center;
        for k in 0..3 {
            if k != i {
                pa += ua[k] * (side(ua[k].dot(l)) * component(ha, k));
            }
            if k != j {
                pb += ub[k] * (side(-ub[k].dot(l)) * component(hb, k));
            }
        }
        let (ea, eb) = (ua[i] * component(ha, i), ub[j] * component(hb, j));
        let (qa, qb) = closest_on_segments(pa - ea, pa + ea, pb - eb, pb + eb);
        return vec![ContactPoint {
            point_a: qa,
            point_b: qb,
            normal: l,
            depth: best_edge.0,
        }];
    }
    let (reference, incident, r_half, i_half, k, flipped) = match best_face.1 {
        Axis::FaceA(k) => (a, b, ha, hb, k, false),
        Axis::FaceB(k) => (b, a, hb, ha, k, true),
        Axis::Edge(..) => return Vec::new(),
    };
    let (ur, ui) = (axes(reference.rotation), axes(incident.rotation));
    let mut n = ur[k];
    if n.dot(incident.center - reference.center) < 0.0 {
        n = -n;
    }
    // The incident face: the one most turned against the reference normal.
    let m = (0..3)
        .max_by(|&x, &y| ui[x].dot(n).abs().total_cmp(&ui[y].dot(n).abs()))
        .unwrap_or(0);
    let face_normal = if ui[m].dot(n) > 0.0 { -ui[m] } else { ui[m] };
    let face_center = incident.center + face_normal * component(i_half, m);
    let (m1, m2) = ((m + 1) % 3, (m + 2) % 3);
    let (e1, e2) = (
        ui[m1] * component(i_half, m1),
        ui[m2] * component(i_half, m2),
    );
    let mut polygon = vec![
        face_center + e1 + e2,
        face_center - e1 + e2,
        face_center - e1 - e2,
        face_center + e1 - e2,
    ];
    let (k1, k2) = ((k + 1) % 3, (k + 2) % 3);
    for (axis, extent) in [
        (ur[k1], component(r_half, k1)),
        (-ur[k1], component(r_half, k1)),
        (ur[k2], component(r_half, k2)),
        (-ur[k2], component(r_half, k2)),
    ] {
        polygon = clip(&polygon, reference.center, axis, extent);
        if polygon.is_empty() {
            return Vec::new();
        }
    }
    let reference_face = reference.center + n * component(r_half, k);
    let mut points: Vec<ContactPoint> = polygon
        .into_iter()
        .filter_map(|p| {
            let s = (p - reference_face).dot(n);
            (s <= MARGIN).then(|| {
                let on_reference = p - n * s;
                if flipped {
                    ContactPoint {
                        point_a: p,
                        point_b: on_reference,
                        normal: -n,
                        depth: -s,
                    }
                } else {
                    ContactPoint {
                        point_a: on_reference,
                        point_b: p,
                        normal: n,
                        depth: -s,
                    }
                }
            })
        })
        .collect();
    reduce(&mut points);
    points
}

/// The part of a polygon on the inner side of a plane (`(p - center)·axis <= extent`).
fn clip(polygon: &[Vec3], center: Vec3, axis: Vec3, extent: f32) -> Vec<Vec3> {
    let inside = |p: Vec3| (p - center).dot(axis) - extent;
    let mut out = Vec::with_capacity(polygon.len() + 1);
    for (i, &p) in polygon.iter().enumerate() {
        let q = polygon[(i + 1) % polygon.len()];
        let (dp, dq) = (inside(p), inside(q));
        if dp <= 0.0 {
            out.push(p);
        }
        if (dp <= 0.0) != (dq <= 0.0) {
            out.push(p + (q - p) * (dp / (dp - dq)));
        }
    }
    out
}

/// At most four points, spread out: the deepest, the farthest from it, then the ones
/// that cover the most area.
pub(crate) fn reduce(points: &mut Vec<ContactPoint>) {
    if points.len() <= 4 {
        return;
    }
    let mid = |c: &ContactPoint| (c.point_a + c.point_b) * 0.5;
    let first = (0..points.len())
        .max_by(|&i, &j| points[i].depth.total_cmp(&points[j].depth))
        .unwrap_or(0);
    let p0 = mid(&points[first]);
    let second = (0..points.len())
        .max_by(|&i, &j| {
            (mid(&points[i]) - p0)
                .length_squared()
                .total_cmp(&(mid(&points[j]) - p0).length_squared())
        })
        .unwrap_or(0);
    let p1 = mid(&points[second]);
    let area = |p: Vec3| (p1 - p0).cross(p - p0).length_squared();
    let third = (0..points.len())
        .max_by(|&i, &j| area(mid(&points[i])).total_cmp(&area(mid(&points[j]))))
        .unwrap_or(0);
    let p2 = mid(&points[third]);
    let spread = |p: Vec3| {
        (p - p0)
            .length_squared()
            .min((p - p1).length_squared())
            .min((p - p2).length_squared())
    };
    let fourth = (0..points.len())
        .filter(|i| ![first, second, third].contains(i))
        .max_by(|&i, &j| spread(mid(&points[i])).total_cmp(&spread(mid(&points[j]))));
    let mut keep = vec![first, second, third];
    keep.extend(fourth);
    keep.sort_unstable();
    keep.dedup();
    let kept = keep.into_iter().map(|i| points[i]).collect();
    *points = kept;
}

/// The closest points of two segments `[p1, q1]` and `[p2, q2]`.
fn closest_on_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (c, b) = (d1.dot(r), d1.dot(d2));
    let denom = a * e - b * b;
    let mut s = if denom > 1e-9 {
        ((b * f - c * e) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let mut t = if e > 1e-9 { (b * s + f) / e } else { 0.0 };
    if t < 0.0 {
        t = 0.0;
        s = if a > 1e-9 {
            (-c / a).clamp(0.0, 1.0)
        } else {
            0.0
        };
    } else if t > 1.0 {
        t = 1.0;
        s = if a > 1e-9 {
            ((b - c) / a).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    (p1 + d1 * s, p2 + d2 * t)
}

// ----- GJK and EPA ----------------------------------------------------------------------

/// A point of the Minkowski difference `A - B`, with the points of A and B it comes from.
#[derive(Clone, Copy, Debug)]
struct Support {
    p: Vec3,
    a: Vec3,
    b: Vec3,
}

fn support(a: &Placed, b: &Placed, dir: Vec3) -> Support {
    let pa = a.support(dir);
    let pb = b.support(-dir);
    Support {
        p: pa - pb,
        a: pa,
        b: pb,
    }
}

fn same_direction(a: Vec3, b: Vec3) -> bool {
    a.dot(b) > 0.0
}

/// GJK: a tetrahedron of the Minkowski difference around the origin, if the shapes
/// overlap. The newest point is first.
fn gjk(a: &Placed, b: &Placed) -> Option<Vec<Support>> {
    let mut dir = b.center - a.center;
    if dir.length_squared() < 1e-12 {
        dir = Vec3::X;
    }
    let mut simplex = vec![support(a, b, dir)];
    dir = -simplex[0].p;
    for _ in 0..64 {
        if dir.length_squared() < 1e-14 {
            return None; // the origin is on the boundary: touching, not overlapping
        }
        let p = support(a, b, dir);
        if p.p.dot(dir) <= 0.0 {
            return None;
        }
        simplex.insert(0, p);
        if next_simplex(&mut simplex, &mut dir) {
            return Some(simplex);
        }
    }
    None
}

fn next_simplex(s: &mut Vec<Support>, dir: &mut Vec3) -> bool {
    match s.len() {
        2 => line(s, dir),
        3 => triangle(s, dir),
        _ => tetrahedron(s, dir),
    }
}

fn line(s: &mut Vec<Support>, dir: &mut Vec3) -> bool {
    let (a, b) = (s[0], s[1]);
    let (ab, ao) = (b.p - a.p, -a.p);
    if same_direction(ab, ao) {
        *dir = ab.cross(ao).cross(ab);
        if dir.length_squared() < 1e-12 {
            // The origin is on the segment: look sideways.
            let side = if ab.x.abs() < 0.57 * ab.length() {
                Vec3::X
            } else {
                Vec3::Y
            };
            *dir = ab.cross(side);
        }
    } else {
        *s = vec![a];
        *dir = ao;
    }
    false
}

fn triangle(s: &mut Vec<Support>, dir: &mut Vec3) -> bool {
    let (a, b, c) = (s[0], s[1], s[2]);
    let (ab, ac, ao) = (b.p - a.p, c.p - a.p, -a.p);
    let abc = ab.cross(ac);
    if same_direction(abc.cross(ac), ao) {
        if same_direction(ac, ao) {
            *s = vec![a, c];
            *dir = ac.cross(ao).cross(ac);
        } else {
            *s = vec![a, b];
            return line(s, dir);
        }
    } else if same_direction(ab.cross(abc), ao) {
        *s = vec![a, b];
        return line(s, dir);
    } else if same_direction(abc, ao) {
        *dir = abc;
    } else {
        *s = vec![a, c, b];
        *dir = -abc;
    }
    false
}

fn tetrahedron(s: &mut Vec<Support>, dir: &mut Vec3) -> bool {
    let (a, b, c, d) = (s[0], s[1], s[2], s[3]);
    let (ab, ac, ad, ao) = (b.p - a.p, c.p - a.p, d.p - a.p, -a.p);
    let (abc, acd, adb) = (ab.cross(ac), ac.cross(ad), ad.cross(ab));
    if same_direction(abc, ao) {
        *s = vec![a, b, c];
        return triangle(s, dir);
    }
    if same_direction(acd, ao) {
        *s = vec![a, c, d];
        return triangle(s, dir);
    }
    if same_direction(adb, ao) {
        *s = vec![a, d, b];
        return triangle(s, dir);
    }
    true
}

/// The outward normal and distance to the origin of each face.
fn face_normals(points: &[Support], faces: &[[usize; 3]]) -> Vec<(Vec3, f32)> {
    faces
        .iter()
        .map(|f| {
            let (a, b, c) = (points[f[0]].p, points[f[1]].p, points[f[2]].p);
            let n = (b - a).cross(c - a);
            let len = n.length();
            if len < 1e-12 {
                return (Vec3::Y, f32::INFINITY); // degenerate: never the closest
            }
            let n = n / len;
            let d = n.dot(a);
            if d < 0.0 { (-n, -d) } else { (n, d) }
        })
        .collect()
}

/// EPA: grows the tetrahedron of GJK into the Minkowski difference until its face
/// nearest to the origin is on the boundary: that face gives the normal and the depth.
fn epa(a: &Placed, b: &Placed, simplex: Vec<Support>) -> Option<ContactPoint> {
    let mut points = simplex;
    let mut faces: Vec<[usize; 3]> = vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
    let mut normals = face_normals(&points, &faces);
    for _ in 0..64 {
        let (best, &(normal, distance)) = normals
            .iter()
            .enumerate()
            .min_by(|x, y| x.1.1.total_cmp(&y.1.1))?;
        if !distance.is_finite() {
            return None;
        }
        let s = support(a, b, normal);
        let further = s.p.dot(normal) - distance;
        if further < 1e-4 || points.len() > 96 {
            let [i, j, k] = faces[best];
            let (wa, wb, wc) =
                barycentric(normal * distance, points[i].p, points[j].p, points[k].p);
            let point_a = points[i].a * wa + points[j].a * wb + points[k].a * wc;
            let point_b = points[i].b * wa + points[j].b * wb + points[k].b * wc;
            return Some(ContactPoint {
                point_a,
                point_b,
                normal,
                depth: distance,
            });
        }
        // Remove the faces the new point sees; their outline makes the new faces.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        let mut f = 0;
        while f < faces.len() {
            if same_direction(normals[f].0, s.p - points[faces[f][0]].p) {
                for (u, v) in [
                    (faces[f][0], faces[f][1]),
                    (faces[f][1], faces[f][2]),
                    (faces[f][2], faces[f][0]),
                ] {
                    match edges.iter().position(|&e| e == (v, u)) {
                        Some(shared) => {
                            edges.swap_remove(shared);
                        }
                        None => edges.push((u, v)),
                    }
                }
                faces.swap_remove(f);
                normals.swap_remove(f);
            } else {
                f += 1;
            }
        }
        if edges.is_empty() {
            return None;
        }
        let new = points.len();
        points.push(s);
        let added: Vec<[usize; 3]> = edges.iter().map(|&(u, v)| [u, v, new]).collect();
        normals.extend(face_normals(&points, &added));
        faces.extend(added);
    }
    None
}

/// The barycentric coordinates of `p` in the triangle `a b c`.
fn barycentric(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (f32, f32, f32) {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return (1.0, 0.0, 0.0);
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    (1.0 - v - w, v, w)
}

fn gjk_epa(a: &Placed, b: &Placed) -> Option<ContactPoint> {
    epa(a, b, gjk(a, b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::shape::Collider;

    fn placed(c: Collider, at: Vec3) -> Placed {
        Placed::new(&c, at, Quat::IDENTITY)
    }

    #[test]
    fn gjk_and_epa_agree_with_the_exact_formulas() {
        // Two balls through GJK/EPA (as capsules of no length) and exactly.
        let a = placed(Collider::capsule(1.0, 0.0), Vec3::ZERO);
        let b = placed(Collider::capsule(1.0, 0.0), Vec3::new(1.5, 0.0, 0.0));
        let c = gjk_epa(&a, &b).unwrap();
        assert!((c.depth - 0.5).abs() < 0.01, "{c:?}");
        assert!((c.normal - Vec3::X).length() < 0.02, "{c:?}");
        let far = placed(Collider::capsule(1.0, 0.0), Vec3::new(2.5, 0.0, 0.0));
        assert!(gjk_epa(&a, &far).is_none());
        // A cylinder standing on a slab: pushed straight up.
        let slab = placed(Collider::cuboid(10.0, 1.0, 10.0), Vec3::new(0.0, -0.5, 0.0));
        let can = placed(Collider::cylinder(0.5, 1.0), Vec3::new(0.0, 0.45, 0.0));
        let c = gjk_epa(&slab, &can).unwrap();
        assert!(
            (c.normal - Vec3::Y).length() < 1e-3 && (c.depth - 0.05).abs() < 1e-3,
            "{c:?}"
        );
    }

    #[test]
    fn a_box_on_a_box_touches_by_four_corners() {
        let floor = placed(Collider::cuboid(10.0, 1.0, 10.0), Vec3::new(0.0, -0.5, 0.0));
        let cube = placed(Collider::cuboid(1.0, 1.0, 1.0), Vec3::new(0.0, 0.49, 0.0));
        let Contacts::Full(points) = collide(&floor, &cube) else {
            panic!()
        };
        assert_eq!(points.len(), 4);
        for p in &points {
            assert!(
                (p.depth - 0.01).abs() < 1e-4 && (p.normal - Vec3::Y).length() < 1e-5,
                "{p:?}"
            );
        }
        // The other way round, the normal turns over.
        let Contacts::Full(points) = collide(&cube, &floor) else {
            panic!()
        };
        assert!(points.iter().all(|p| (p.normal + Vec3::Y).length() < 1e-5));
        // Turned 45° about the vertical, still flat: four points.
        let turned = Placed::new(
            &Collider::cuboid(1.0, 1.0, 1.0),
            Vec3::new(0.0, 0.49, 0.0),
            Quat::from_rotation_y(0.785),
        );
        let Contacts::Full(points) = collide(&floor, &turned) else {
            panic!()
        };
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn a_ball_against_a_box() {
        let cube = placed(Collider::cuboid(2.0, 2.0, 2.0), Vec3::ZERO);
        let ball = placed(Collider::sphere(0.5), Vec3::new(0.0, 1.4, 0.0));
        let Contacts::Full(points) = collide(&ball, &cube) else {
            panic!()
        };
        assert!((points[0].depth - 0.1).abs() < 1e-5);
        assert!(
            (points[0].normal + Vec3::Y).length() < 1e-5,
            "from the ball to the box: down"
        );
    }
}
