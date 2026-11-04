//! The narrow phase: the contact points of two shapes that may touch.
//!
//! Balls and boxes have exact formulas (and boxes a whole contact face, so that they
//! rest flat and stack); every other pair goes through GJK, which tells whether two
//! convex shapes overlap, and EPA, which finds how deep and in which direction.

use super::shape::{Placed, Shape};
use crate::three::math::{Quat, Vec3};

/// Contacts are kept until the shapes are this far apart (meters): a body resting on
/// another keeps its contact instead of losing and finding it again each step. Moving
/// bodies get a wider margin, as far as they can go in a substep, so that the solver
/// stops them at the surface instead of letting them through (speculative contacts).
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

/// The contacts of `a` and `b`, if they overlap or are less than `margin` apart.
pub(crate) fn collide(a: &Placed, b: &Placed, margin: f32) -> Contacts {
    match (a.shape, b.shape) {
        (Shape::Sphere { radius: ra }, Shape::Sphere { radius: rb }) => {
            sphere_sphere(a.center, ra, b.center, rb, margin)
                .map_or(Contacts::None, |c| Contacts::Full(vec![c]))
        }
        (Shape::Sphere { radius }, Shape::Cuboid { half_extents }) => {
            sphere_box(a.center, radius, b, half_extents, margin)
                .map_or(Contacts::None, |c| Contacts::Full(vec![c]))
        }
        (Shape::Cuboid { half_extents }, Shape::Sphere { radius }) => {
            sphere_box(b.center, radius, a, half_extents, margin)
                .map_or(Contacts::None, |c| Contacts::Full(vec![flip(c)]))
        }
        (Shape::Cuboid { half_extents: ha }, Shape::Cuboid { half_extents: hb }) => {
            let points = box_box(a, ha, b, hb, margin);
            if points.is_empty() {
                Contacts::None
            } else {
                Contacts::Full(points)
            }
        }
        (
            Shape::Capsule {
                radius: ra,
                half_height: ha,
            },
            Shape::Capsule {
                radius: rb,
                half_height: hb,
            },
        ) => {
            let (a0, a1) = segment(a, ha);
            let (b0, b1) = segment(b, hb);
            let (pa, pb) = closest_on_segments(a0, a1, b0, b1);
            round_contact(a, b, pa, ra, pb, rb, margin)
        }
        (
            Shape::Capsule {
                radius: ra,
                half_height: ha,
            },
            Shape::Sphere { radius: rb },
        ) => {
            let (a0, a1) = segment(a, ha);
            round_contact(
                a,
                b,
                closest_on_segment(a0, a1, b.center),
                ra,
                b.center,
                rb,
                margin,
            )
        }
        (
            Shape::Sphere { radius: ra },
            Shape::Capsule {
                radius: rb,
                half_height: hb,
            },
        ) => {
            let (b0, b1) = segment(b, hb);
            round_contact(
                a,
                b,
                a.center,
                ra,
                closest_on_segment(b0, b1, a.center),
                rb,
                margin,
            )
        }
        (
            Shape::Torus {
                major_radius,
                minor_radius,
            },
            _,
        ) => ring(a, major_radius, minor_radius, b, margin, true),
        (
            _,
            Shape::Torus {
                major_radius,
                minor_radius,
            },
        ) => ring(b, major_radius, minor_radius, a, margin, false),
        _ => match convex(a, b, margin) {
            Some(c) => faces(a, b, c, margin),
            None => Contacts::None,
        },
    }
}

/// The contact of two convex shapes that overlap or are less than `margin` apart. Both
/// are rounded by half the margin (a ball rolled over their surface), so that within the
/// margin they overlap, by at least the margin when they touch: GJK finds the overlap and
/// EPA its depth and normal, which are then well defined. Measured right at a distance of
/// zero, the direction between two nearly touching shapes is noise, worse still next to a
/// large body in single precision (Bullet and Jolt round their shapes the same way).
fn convex(a: &Placed, b: &Placed, margin: f32) -> Option<ContactPoint> {
    let mut c = epa(a, b, gjk(a, b, margin)?, margin)?;
    c.depth -= margin;
    Some(c)
}

/// How many pieces a ring collides as.
const RING_PIECES: usize = 24;

/// A ring is not convex: it collides as short capsules along its tube, so that its hole
/// is open (a ball goes through it, it hangs on a peg, it lies flat on its tube). The
/// contacts of the pieces near `other` make one contact area.
fn ring(
    ring: &Placed,
    major: f32,
    minor: f32,
    other: &Placed,
    margin: f32,
    ring_first: bool,
) -> Contacts {
    // Its convex hull first (a coin with rounded edges): apart, or touching with the
    // outside of its tube, or lying flat on a face, the ring touches as its hull does,
    // and one test is enough.
    let hull = if ring_first {
        convex(ring, other, margin)
    } else {
        convex(other, ring, margin).map(flip)
    };
    let Some(c) = hull else {
        return Contacts::None;
    };
    // `c` from the ring towards `other`.
    let axis = ring.rotation * Vec3::Y;
    let local = c.point_a - ring.center;
    let radial = (local - axis * local.dot(axis)).length();
    // (Shallow only: deep in the hull, something may be in the hole.)
    let shallow = c.depth <= 0.5 * minor;
    let outside = radial >= major * 0.999;
    // Lying flat on a face that covers its whole rim: the hole is closed by that face.
    let flat = c.normal.dot(axis).abs() > 0.95 && {
        let face = support_face(other, -c.normal);
        plane(&face, -c.normal).is_some()
            && support_face(ring, c.normal)
                .iter()
                .all(|&p| over(&face, c.normal, p))
    };
    if shallow && (outside || flat) {
        return if ring_first {
            faces(ring, other, c, margin)
        } else {
            faces(other, ring, flip(c), margin)
        };
    }
    let half = std::f32::consts::PI / RING_PIECES as f32;
    // Each piece's axis is a chord: at this radius, its middle is as far inside the
    // tube's center line as its ends are outside.
    let radius = 2.0 * major / (1.0 + half.cos());
    let half_length = radius * half.sin();
    let piece = Shape::Capsule {
        radius: minor,
        half_height: half_length,
    };
    let piece_reach = half_length + minor;
    let mut points = Vec::new();
    for k in 0..RING_PIECES {
        let angle = (2 * k + 1) as f32 * half;
        let (sin, cos) = angle.sin_cos();
        let middle = Vec3::new(cos, 0.0, sin) * (radius * half.cos());
        let center = ring.center + ring.rotation * middle;
        // Beyond the plane that bounds `other` on its side, farther than it can reach:
        // apart.
        let toward = center - other.center;
        let length = toward.length();
        if length > 1e-9 {
            let d = toward / length;
            if (center - other.support(d)).dot(d) > piece_reach + margin {
                continue;
            }
        }
        let tangent = Vec3::new(-sin, 0.0, cos);
        let placed = Placed {
            shape: piece,
            center,
            rotation: (ring.rotation * Quat::from_unit_vectors(Vec3::Y, tangent)).normalize(),
        };
        let found = if ring_first {
            collide(&placed, other, margin)
        } else {
            collide(other, &placed, margin)
        };
        match found {
            Contacts::None => {}
            Contacts::Single(p) => points.push(p),
            Contacts::Full(ps) => points.extend(ps),
        }
    }
    match points.len() {
        0 => Contacts::None,
        1 => Contacts::Single(points[0]),
        _ => {
            reduce(&mut points);
            Contacts::Full(points)
        }
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
        Shape::Frustum {
            radius_top,
            radius_bottom,
            half_height,
        } => {
            if d.y > FLAT && radius_top > 0.0 {
                rim(radius_top, half_height)
            } else if d.y < -FLAT && radius_bottom > 0.0 {
                rim(radius_bottom, -half_height)
            } else {
                // Along its side: the top and bottom points of the slant line.
                let top = flat(d) * radius_top + Vec3::new(0.0, half_height, 0.0);
                let bottom = flat(d) * radius_bottom - Vec3::new(0.0, half_height, 0.0);
                if (bottom - top).normalize().dot(d).abs() < 1.0 - FLAT {
                    vec![world(top), world(bottom)]
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
/// segment), to a tenth of a millimeter.
fn over(face: &[Vec3], n: Vec3, p: Vec3) -> bool {
    if face.len() < 3 {
        return true;
    }
    // The distance of p to each edge's line, signed by side.
    let side = |i: usize| {
        let (a, b) = (face[i], face[(i + 1) % face.len()]);
        let edge = b - a;
        edge.cross(p - a).dot(n) / edge.length().max(1e-12)
    };
    let eps = 1e-4;
    (0..face.len()).all(|i| side(i) >= -eps) || (0..face.len()).all(|i| side(i) <= eps)
}

/// The contact area of two shapes near each other along `c.normal`. A flat face (the
/// larger, if both are) is the reference: the other shape's touching feature (a face, a
/// side, a point) is clipped by its edges (Sutherland–Hodgman), and each point is measured
/// from its plane, along its own normal: over a face, the contact normal is the face's,
/// which GJK only approaches (and approaches badly next to a large body, in single
/// precision). Two sides lying along each other touch along their common part. A single
/// point otherwise.
fn faces(a: &Placed, b: &Placed, c: ContactPoint, margin: f32) -> Contacts {
    let n = c.normal;
    let face_a = support_face(a, n);
    let face_b = support_face(b, -n);
    let extent = |f: &[Vec3]| {
        let center = f.iter().fold(Vec3::ZERO, |s, p| s + *p) / f.len().max(1) as f32;
        f.iter().map(|p| (*p - center).length()).fold(0.0, f32::max)
    };
    let a_is_reference = match (face_a.len() >= 3, face_b.len() >= 3) {
        (true, true) => extent(&face_a) >= extent(&face_b),
        (true, false) => true,
        (false, true) => false,
        (false, false) => return segments(&face_a, &face_b, c, margin),
    };
    let (reference, incident) = if a_is_reference {
        (face_a, face_b)
    } else {
        (face_b, face_a)
    };
    let outward = if a_is_reference { n } else { -n };
    let Some(plane) = plane(&reference, outward) else {
        return Contacts::Single(c);
    };
    // The contact normal, from A to B.
    let normal = if a_is_reference { plane } else { -plane };
    // A single point: the one GJK found (a support point along a flat side would be any
    // of it).
    let incident = if incident.len() < 2 {
        vec![if a_is_reference { c.point_b } else { c.point_a }]
    } else {
        incident
    };
    let center = reference.iter().fold(Vec3::ZERO, |s, p| s + *p) / reference.len() as f32;
    let reach = extent(&incident);
    let mut clipped = incident;
    for (i, &p) in reference.iter().enumerate() {
        let q = reference[(i + 1) % reference.len()];
        let mut side = (q - p).cross(plane).normalize();
        if side.dot(center - p) > 0.0 {
            side = -side;
        }
        clipped = clip(&clipped, p, side, 0.0);
        if clipped.is_empty() {
            return Contacts::Single(c);
        }
    }
    let origin = reference[0];
    let contact = |q: Vec3| {
        // How far q is from the reference plane (positive: apart).
        let s = (q - origin).dot(plane);
        if a_is_reference {
            ContactPoint {
                point_a: q - plane * s,
                point_b: q,
                normal,
                depth: -s,
            }
        } else {
            ContactPoint {
                point_a: q,
                point_b: q - plane * s,
                normal,
                depth: -s,
            }
        }
    };
    let mut points: Vec<ContactPoint> = Vec::new();
    for q in clipped {
        let point = contact(q);
        let seen = points
            .iter()
            .any(|o| (o.point_a - point.point_a).length_squared() < 1e-12);
        if point.depth >= -margin && !seen {
            points.push(point);
        }
    }
    // No point can be much deeper than the shapes overlap (a face tilted by up to 18°
    // from the normal goes a third of its size deeper): otherwise the face is the wrong
    // one, and the contact is kept as found.
    if points
        .iter()
        .any(|p| p.depth > c.depth + 0.01 + 0.35 * reach)
    {
        return Contacts::Single(c);
    }
    match points.len() {
        0 => return Contacts::Single(c),
        1 => return Contacts::Single(points[0]),
        _ => {}
    }
    // A rim of eight points stands for a circle: the deepest point, found by GJK or EPA,
    // may lie between them.
    let deepest = points
        .iter()
        .map(|p| p.depth)
        .fold(f32::NEG_INFINITY, f32::max);
    let found = contact(if a_is_reference { c.point_b } else { c.point_a });
    if found.depth > deepest + 1e-3 * extent(&reference).max(1e-3)
        && over(
            &reference,
            plane,
            if a_is_reference { c.point_b } else { c.point_a },
        )
    {
        points.push(found);
    }
    // A rim's eight points are all kept: they are fixed on the body, so they are the same
    // from one substep to the next, and so are their impulses; four of them would change
    // with each little rock, and the body would never settle.
    if points.len() > 8 {
        reduce(&mut points);
    }
    Contacts::Full(points)
}

/// The outward normal of a flat face (three points or more), if it is turned towards
/// `toward`.
fn plane(face: &[Vec3], toward: Vec3) -> Option<Vec3> {
    if face.len() < 3 {
        return None;
    }
    // Newell's method: robust for any polygon.
    let mut normal = Vec3::ZERO;
    for (i, p) in face.iter().enumerate() {
        let q = face[(i + 1) % face.len()];
        normal += Vec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    let length = normal.length();
    if length < 1e-12 {
        return None;
    }
    let mut normal = normal / length;
    if normal.dot(toward) < 0.0 {
        normal = -normal;
    }
    // A face is taken when within 18° (as `support_face` finds faces): over it, the
    // contact normal is the face's.
    (normal.dot(toward) > 0.95).then_some(normal)
}

/// Two sides (of capsules, cylinders, cones) or points facing each other: lying along
/// each other, they touch along their common part; otherwise at the one point found.
fn segments(face_a: &[Vec3], face_b: &[Vec3], c: ContactPoint, margin: f32) -> Contacts {
    if face_a.len() != 2 || face_b.len() != 2 {
        return Contacts::Single(c);
    }
    let (da, db) = (face_a[1] - face_a[0], face_b[1] - face_b[0]);
    if da.cross(db).length() > 0.05 * da.length() * db.length() {
        return Contacts::Single(c);
    }
    let u = da.normalize();
    let common = clip(&clip(face_b, face_a[0], -u, 0.0), face_a[1], u, 0.0);
    let n = c.normal;
    let mut points: Vec<ContactPoint> = Vec::new();
    for p in common {
        // How far apart along the normal, from the plane through A's nearest point.
        let s = (p - c.point_a).dot(n);
        if s <= margin
            && !points
                .iter()
                .any(|o| (o.point_b - p).length_squared() < 1e-12)
        {
            points.push(ContactPoint {
                point_a: p - n * s,
                point_b: p,
                normal: n,
                depth: -s,
            });
        }
    }
    if points.len() < 2 {
        return Contacts::Single(c);
    }
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

fn sphere_sphere(ca: Vec3, ra: f32, cb: Vec3, rb: f32, margin: f32) -> Option<ContactPoint> {
    let d = cb - ca;
    let dist = d.length();
    let depth = ra + rb - dist;
    if depth < -margin {
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
fn sphere_box(
    center: Vec3,
    radius: f32,
    b: &Placed,
    half: Vec3,
    margin: f32,
) -> Option<ContactPoint> {
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
    if depth < -margin {
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
fn box_box(a: &Placed, ha: Vec3, b: &Placed, hb: Vec3, margin: f32) -> Vec<ContactPoint> {
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
            if o < -margin {
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
            if o < -margin {
                return Vec::new();
            }
            if o < best_edge.0 {
                best_edge = (o, Axis::Edge(i, j), l);
            }
        }
    }
    // Faces are preferred: an edge axis wins only when clearly better, so that a box
    // lying on another does not switch between the two cases.
    // (A centimeter, or less for small boxes: a 1 cm box would never get its edges.)
    let size = [ha.x, ha.y, ha.z, hb.x, hb.y, hb.z]
        .into_iter()
        .fold(f32::INFINITY, f32::min);
    if best_edge.0 < best_face.0 - (0.02 * size).min(0.01) {
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
            (s <= margin).then(|| {
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

/// The axis of a capsule, in the world.
fn segment(capsule: &Placed, half_height: f32) -> (Vec3, Vec3) {
    let axis = capsule.rotation * Vec3::new(0.0, half_height, 0.0);
    (capsule.center - axis, capsule.center + axis)
}

/// The point of the segment `[p, q]` nearest to `x`.
fn closest_on_segment(p: Vec3, q: Vec3, x: Vec3) -> Vec3 {
    let d = q - p;
    let len2 = d.length_squared();
    if len2 < 1e-14 {
        return p;
    }
    p + d * ((x - p).dot(d) / len2).clamp(0.0, 1.0)
}

/// Two rounded shapes (capsules, balls) whose cores' nearest points are `pa` and `pb`:
/// their contact, the whole line where two capsules lie side by side.
fn round_contact(
    a: &Placed,
    b: &Placed,
    pa: Vec3,
    ra: f32,
    pb: Vec3,
    rb: f32,
    margin: f32,
) -> Contacts {
    let gap = pb - pa;
    let dist = gap.length();
    let depth = ra + rb - dist;
    if depth < -margin {
        return Contacts::None;
    }
    let normal = if dist > 1e-6 {
        gap / dist
    } else {
        // Cores crossing: out along the line between the centers, or up.
        let d = b.center - a.center;
        if d.length_squared() > 1e-12 {
            d.normalize()
        } else {
            Vec3::Y
        }
    };
    faces(
        a,
        b,
        ContactPoint {
            point_a: pa + normal * ra,
            point_b: pb - normal * rb,
            normal,
            depth,
        },
        margin,
    )
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

/// A point of the Minkowski difference of the shapes rounded by `round` in all (the points
/// of the shapes themselves are kept, on their real surfaces).
fn support(a: &Placed, b: &Placed, dir: Vec3, round: f32) -> Support {
    let pa = a.support(dir);
    let pb = b.support(-dir);
    let length = dir.length();
    let out = if length > 0.0 {
        dir * (round / length)
    } else {
        Vec3::ZERO
    };
    Support {
        p: pa - pb + out,
        a: pa,
        b: pb,
    }
}

fn same_direction(a: Vec3, b: Vec3) -> bool {
    a.dot(b) > 0.0
}

/// GJK: a tetrahedron of the Minkowski difference around the origin, if the shapes
/// overlap. The newest point is first.
fn gjk(a: &Placed, b: &Placed, round: f32) -> Option<Vec<Support>> {
    let mut dir = b.center - a.center;
    if dir.length_squared() < 1e-12 {
        dir = Vec3::X;
    }
    let mut simplex = vec![support(a, b, dir, round)];
    dir = -simplex[0].p;
    for _ in 0..64 {
        // Only the direction matters: normalized, the tests do not depend on the size of
        // the shapes (a 1 cm one would fall under any fixed threshold).
        let len = dir.length();
        if !(len > 0.0 && len.is_finite()) {
            return None; // the origin is on the simplex: touching, not overlapping
        }
        dir = dir / len;
        let p = support(a, b, dir, round);
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
        // |ab × ao × ab| = |ab|²·|ao|·sin: the origin is on the segment when the sine is
        // about zero, whatever the size.
        let scale = ab.length_squared() * ab.length_squared() * ao.length_squared();
        if dir.length_squared() <= 1e-12 * scale {
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
            return line(s, dir); // with its fallback when the origin is on the edge
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

/// The outward normal and distance to the origin of each face, outward meaning away from
/// a point inside the polytope (not from the origin, which may lie on a face: the sign of
/// its distance is then noise, and a face turned inward sends EPA astray).
fn face_normals(points: &[Support], faces: &[[usize; 3]], inside: Vec3) -> Vec<(Vec3, f32)> {
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
            let n = if n.dot(a - inside) < 0.0 { -n } else { n };
            (n, n.dot(a))
        })
        .collect()
}

/// EPA: grows the tetrahedron of GJK into the Minkowski difference until its face
/// nearest to the origin is on the boundary: that face gives the normal and the depth.
fn epa(a: &Placed, b: &Placed, simplex: Vec<Support>, round: f32) -> Option<ContactPoint> {
    let mut points = simplex;
    // The polytope only grows: the center of the first tetrahedron stays inside.
    let inside = (points[0].p + points[1].p + points[2].p + points[3].p) * 0.25;
    let mut faces: Vec<[usize; 3]> = vec![[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]];
    let mut normals = face_normals(&points, &faces, inside);
    let mut found = None;
    for _ in 0..64 {
        let (best, &(normal, distance)) = normals
            .iter()
            .enumerate()
            .min_by(|x, y| x.1.1.total_cmp(&y.1.1))?;
        if !distance.is_finite() {
            return found;
        }
        let [i, j, k] = faces[best];
        let (wa, wb, wc) = barycentric(normal * distance, points[i].p, points[j].p, points[k].p);
        found = Some(ContactPoint {
            point_a: points[i].a * wa + points[j].a * wb + points[k].a * wc,
            point_b: points[i].b * wa + points[j].b * wb + points[k].b * wc,
            normal,
            depth: distance,
        });
        let s = support(a, b, normal, round);
        if s.p.dot(normal) - distance < 1e-4 {
            return found;
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
            return found;
        }
        let new = points.len();
        points.push(s);
        let added: Vec<[usize; 3]> = edges.iter().map(|&(u, v)| [u, v, new]).collect();
        normals.extend(face_normals(&points, &added, inside));
        faces.extend(added);
    }
    // Not converged (a large round shape): the nearest face so far is still a good answer.
    found
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::shape::Collider;

    fn gjk_epa(a: &Placed, b: &Placed) -> Option<ContactPoint> {
        epa(a, b, gjk(a, b, 0.0)?, 0.0)
    }

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
        let Contacts::Full(points) = collide(&floor, &cube, MARGIN) else {
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
        let Contacts::Full(points) = collide(&cube, &floor, MARGIN) else {
            panic!()
        };
        assert!(points.iter().all(|p| (p.normal + Vec3::Y).length() < 1e-5));
        // Turned 45° about the vertical, still flat: four points.
        let turned = Placed::new(
            &Collider::cuboid(1.0, 1.0, 1.0),
            Vec3::new(0.0, 0.49, 0.0),
            Quat::from_rotation_y(0.785),
        );
        let Contacts::Full(points) = collide(&floor, &turned, MARGIN) else {
            panic!()
        };
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn a_ball_against_a_box() {
        let cube = placed(Collider::cuboid(2.0, 2.0, 2.0), Vec3::ZERO);
        let ball = placed(Collider::sphere(0.5), Vec3::new(0.0, 1.4, 0.0));
        let Contacts::Full(points) = collide(&ball, &cube, MARGIN) else {
            panic!()
        };
        assert!((points[0].depth - 0.1).abs() < 1e-5);
        assert!(
            (points[0].normal + Vec3::Y).length() < 1e-5,
            "from the ball to the box: down"
        );
    }

    // Cases found by an independent audit, checked against reference geometry.

    fn at(shape: Shape, c: (f32, f32, f32), q: (f32, f32, f32, f32)) -> Placed {
        Placed {
            shape,
            center: Vec3::new(c.0, c.1, c.2),
            rotation: Quat::from_xyzw(q.0, q.1, q.2, q.3).normalize(),
        }
    }

    fn deepest(c: Contacts) -> Option<ContactPoint> {
        match c {
            Contacts::None => None,
            Contacts::Single(p) => Some(p),
            Contacts::Full(ps) => ps.into_iter().max_by(|a, b| a.depth.total_cmp(&b.depth)),
        }
    }

    #[test]
    fn overlaps_of_round_shapes_have_their_true_depth() {
        // EPA used to turn faces inward when the origin lay on one: a depth ten times too
        // large, or no contact at all.
        let ball =
            |r: f32, c: (f32, f32, f32)| at(Shape::Sphere { radius: r }, c, (0.0, 0.0, 0.0, 1.0));
        let cases = [
            (
                ball(0.95651674, (0.9780129, 2.2806866, 2.2090993)),
                at(
                    Shape::Cylinder {
                        radius: 0.93281734,
                        half_height: 0.6728567,
                    },
                    (-0.45480567, 1.4705058, 3.0651872),
                    (-0.063014105, 0.026377587, 0.690265, 0.72032475),
                ),
                0.198,
            ),
            (
                ball(0.44424, (1.78895, -0.03913, 0.51581)),
                at(
                    Shape::Cylinder {
                        radius: 0.31444,
                        half_height: 0.65800,
                    },
                    (1.88832, 0.20300, 0.21192),
                    (0.67105, 0.72019, 0.14170, 0.10459),
                ),
                0.358,
            ),
        ];
        for (a, b, depth) in cases {
            for (x, y, flipped) in [(&a, &b, false), (&b, &a, true)] {
                let c = deepest(collide(x, y, MARGIN)).expect("they overlap");
                assert!(
                    (c.depth - depth).abs() < 0.01,
                    "{flipped}: {} instead of {depth}",
                    c.depth
                );
            }
        }
    }

    #[test]
    fn small_boxes_meet_edge_to_edge() {
        // Two 2 cm cubes, one turned around z, the other around x: their edges cross.
        let a = at(
            Shape::Cuboid {
                half_extents: Vec3::splat(0.01),
            },
            (0.0, 0.0, 0.0),
            (
                0.0,
                0.0,
                (std::f32::consts::PI / 8.0).sin(),
                (std::f32::consts::PI / 8.0).cos(),
            ),
        );
        let b = at(
            Shape::Cuboid {
                half_extents: Vec3::splat(0.01),
            },
            (0.0, 0.027284, 0.0),
            (
                (std::f32::consts::PI / 8.0).sin(),
                0.0,
                0.0,
                (std::f32::consts::PI / 8.0).cos(),
            ),
        );
        let c = deepest(collide(&a, &b, MARGIN)).expect("they touch");
        assert!((c.depth - 0.001).abs() < 1e-4, "{c:?}");
        assert!((c.normal - Vec3::Y).length() < 1e-3, "{c:?}");
    }

    #[test]
    fn a_long_capsule_on_a_box_touches_along_a_line() {
        // Not at the box's four corners, as if the capsule were a plane.
        let cube = placed(Collider::cuboid(1.0, 1.0, 1.0), Vec3::new(0.0, -0.5, 0.0));
        let capsule = Placed {
            shape: Shape::Capsule {
                radius: 0.3,
                half_height: 2.0,
            },
            center: Vec3::new(0.0, 0.29, 0.0),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        };
        for (a, b) in [(&cube, &capsule), (&capsule, &cube)] {
            let Contacts::Full(points) = collide(a, b, MARGIN) else {
                panic!("a line of contact");
            };
            assert_eq!(points.len(), 2, "{points:?}");
            for p in &points {
                let on_box = if std::ptr::eq(a, &cube) {
                    p.point_a
                } else {
                    p.point_b
                };
                assert!(
                    (on_box.x.abs() - 0.5).abs() < 1e-3 && on_box.z.abs() < 1e-3,
                    "{p:?}"
                );
                assert!((p.depth - 0.01).abs() < 1e-3, "{p:?}");
            }
        }
    }

    #[test]
    fn a_can_over_a_table_edge_rests_on_all_it_covers() {
        // A table whose edge is at x = 1, a can of radius 0.5 standing at x = 0.7: its
        // contact area reaches the edge, 0.3 m beyond its center of mass.
        let table = placed(
            Collider::cuboid(10.0, 1.0, 10.0),
            Vec3::new(-4.0, -0.5, 0.0),
        );
        let can = placed(Collider::cylinder(0.5, 1.0), Vec3::new(0.7, 0.499, 0.0));
        let Contacts::Full(points) = collide(&table, &can, MARGIN) else {
            panic!("a contact area");
        };
        let reach = points.iter().map(|p| p.point_a.x).fold(f32::MIN, f32::max);
        assert!((reach - 1.0).abs() < 1e-3, "{points:?}");
    }
}
