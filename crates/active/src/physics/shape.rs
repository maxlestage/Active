//! Shapes: what collides, and where the mass is.

use crate::scene::math::{Quat, Vec3};

/// A convex shape around its own origin. Round shapes stand along `y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// A ball.
    Sphere {
        /// Its radius.
        radius: f32,
    },
    /// A box.
    Cuboid {
        /// Half its size on each axis.
        half_extents: Vec3,
    },
    /// A cylinder along `y`.
    Cylinder {
        /// Its radius.
        radius: f32,
        /// Half its height.
        half_height: f32,
    },
    /// A cone along `y`, its tip up, its base at `-half_height`.
    Cone {
        /// The radius of its base.
        radius: f32,
        /// Half its height.
        half_height: f32,
    },
    /// A cone cut flat along `y` (a bucket, a lampshade): discs of two radii at
    /// `±half_height`. A radius of 0 makes a cone, pointing up or down.
    Frustum {
        /// The radius at `+half_height`.
        radius_top: f32,
        /// The radius at `-half_height`.
        radius_bottom: f32,
        /// Half its height.
        half_height: f32,
    },
    /// A capsule along `y`: a cylinder with half balls at its ends.
    Capsule {
        /// Its radius.
        radius: f32,
        /// Half the height of its straight part.
        half_height: f32,
    },
    /// A ring lying around `y`, its tube thinner than the ring is wide (`minor_radius <=
    /// major_radius`). Its hole is open: it collides as 24 short capsules along its tube.
    Torus {
        /// From the center to the middle of the tube.
        major_radius: f32,
        /// The radius of the tube.
        minor_radius: f32,
    },
}

impl Shape {
    /// Its volume.
    pub fn volume(&self) -> f32 {
        use std::f32::consts::PI;
        match *self {
            Shape::Sphere { radius } => 4.0 / 3.0 * PI * radius.powi(3),
            Shape::Cuboid { half_extents: h } => 8.0 * h.x * h.y * h.z,
            Shape::Cylinder {
                radius,
                half_height,
            } => PI * radius * radius * 2.0 * half_height,
            Shape::Cone {
                radius,
                half_height,
            } => PI * radius * radius * 2.0 * half_height / 3.0,
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => PI * frustum(radius_top, radius_bottom, half_height).squares,
            Shape::Capsule {
                radius,
                half_height,
            } => PI * radius * radius * (2.0 * half_height + 4.0 / 3.0 * radius),
            Shape::Torus {
                major_radius,
                minor_radius,
            } => 2.0 * PI * PI * major_radius * minor_radius * minor_radius,
        }
    }

    /// Where the mass is centered, in the shape's frame (only a cone is off center).
    pub fn center_of_mass(&self) -> Vec3 {
        match *self {
            Shape::Cone { half_height, .. } => Vec3::new(0.0, -half_height / 2.0, 0.0),
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => Vec3::new(
                0.0,
                frustum(radius_top, radius_bottom, half_height).center,
                0.0,
            ),
            _ => Vec3::ZERO,
        }
    }

    /// The moments of inertia of a solid of this mass, about its center of mass, along the
    /// shape's axes.
    pub fn inertia(&self, mass: f32) -> Vec3 {
        match *self {
            Shape::Sphere { radius } => Vec3::splat(0.4 * mass * radius * radius),
            Shape::Cuboid { half_extents: h } => {
                let (x, y, z) = (
                    (2.0 * h.x).powi(2),
                    (2.0 * h.y).powi(2),
                    (2.0 * h.z).powi(2),
                );
                Vec3::new(
                    mass * (y + z) / 12.0,
                    mass * (x + z) / 12.0,
                    mass * (x + y) / 12.0,
                )
            }
            Shape::Cylinder {
                radius,
                half_height,
            } => {
                let h = 2.0 * half_height;
                let side = mass * (3.0 * radius * radius + h * h) / 12.0;
                Vec3::new(side, 0.5 * mass * radius * radius, side)
            }
            Shape::Cone {
                radius,
                half_height,
            } => {
                let h = 2.0 * half_height;
                let side = mass * (3.0 / 20.0 * radius * radius + 3.0 / 80.0 * h * h);
                Vec3::new(side, 0.3 * mass * radius * radius, side)
            }
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => {
                let f = frustum(radius_top, radius_bottom, half_height);
                if f.squares <= 0.0 {
                    return Vec3::ZERO;
                }
                let side = mass * f.side / f.squares;
                Vec3::new(side, mass * f.axis / f.squares, side)
            }
            Shape::Capsule {
                radius: r,
                half_height,
            } => {
                // A cylinder and two half balls, each with its share of the mass; a half
                // ball's center of mass is 3r/8 from its flat side (parallel axes).
                let h = 2.0 * half_height;
                let pi = std::f32::consts::PI;
                let (cylinder, ball) = (pi * r * r * h, 4.0 / 3.0 * pi * r.powi(3));
                let total = cylinder + ball;
                if total <= 0.0 {
                    return Vec3::ZERO;
                }
                let (mc, mb) = (mass * cylinder / total, mass * ball / total);
                let side = mc * (r * r / 4.0 + h * h / 12.0)
                    + mb * (0.4 * r * r + h * h / 4.0 + 3.0 * h * r / 8.0);
                Vec3::new(side, mc * r * r / 2.0 + mb * 0.4 * r * r, side)
            }
            Shape::Torus {
                major_radius: big,
                minor_radius: r,
            } => {
                let side = mass * (0.5 * big * big + 5.0 / 8.0 * r * r);
                Vec3::new(side, mass * (big * big + 0.75 * r * r), side)
            }
        }
    }

    /// The point of the shape farthest along `dir` (which need not be a unit vector).
    pub fn support(&self, dir: Vec3) -> Vec3 {
        // Scaled to a largest component of 1: a tiny or a huge direction is as good as any.
        let m = dir.x.abs().max(dir.y.abs()).max(dir.z.abs());
        let dir = if m > 0.0 && m.is_finite() {
            dir / m
        } else {
            Vec3::Y
        };
        let flat = |d: Vec3| {
            let len = (d.x * d.x + d.z * d.z).sqrt();
            if len > 1e-9 {
                Vec3::new(d.x / len, 0.0, d.z / len)
            } else {
                Vec3::X
            }
        };
        let unit = |d: Vec3| {
            let len = d.length();
            if len > 1e-12 { d / len } else { Vec3::Y }
        };
        let sign = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
        match *self {
            Shape::Sphere { radius } => unit(dir) * radius,
            Shape::Cuboid { half_extents: h } => {
                Vec3::new(sign(dir.x) * h.x, sign(dir.y) * h.y, sign(dir.z) * h.z)
            }
            Shape::Cylinder {
                radius,
                half_height,
            } => flat(dir) * radius + Vec3::new(0.0, sign(dir.y) * half_height, 0.0),
            Shape::Cone {
                radius,
                half_height,
            } => {
                let tip = Vec3::new(0.0, half_height, 0.0);
                let rim = flat(dir) * radius + Vec3::new(0.0, -half_height, 0.0);
                if tip.dot(dir) >= rim.dot(dir) {
                    tip
                } else {
                    rim
                }
            }
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => {
                let top = flat(dir) * radius_top + Vec3::new(0.0, half_height, 0.0);
                let bottom = flat(dir) * radius_bottom - Vec3::new(0.0, half_height, 0.0);
                if top.dot(dir) >= bottom.dot(dir) {
                    top
                } else {
                    bottom
                }
            }
            Shape::Capsule {
                radius,
                half_height,
            } => Vec3::new(0.0, sign(dir.y) * half_height, 0.0) + unit(dir) * radius,
            Shape::Torus {
                major_radius,
                minor_radius,
            } => flat(dir) * major_radius + unit(dir) * minor_radius,
        }
    }

    /// The distance from the origin to the farthest point.
    pub fn bounding_radius(&self) -> f32 {
        match *self {
            Shape::Sphere { radius } => radius,
            Shape::Cuboid { half_extents } => half_extents.length(),
            Shape::Cylinder {
                radius,
                half_height,
            }
            | Shape::Cone {
                radius,
                half_height,
            } => (radius * radius + half_height * half_height).sqrt(),
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => (radius_top.max(radius_bottom).powi(2) + half_height * half_height).sqrt(),
            Shape::Capsule {
                radius,
                half_height,
            } => radius + half_height,
            Shape::Torus {
                major_radius,
                minor_radius,
            } => major_radius + minor_radius,
        }
    }

    /// The same shape, stretched by a scale (round shapes take the largest of the scales
    /// across their axis).
    pub fn scaled(&self, s: Vec3) -> Shape {
        let s = Vec3::new(s.x.abs(), s.y.abs(), s.z.abs());
        let across = s.x.max(s.z);
        match *self {
            Shape::Sphere { radius } => Shape::Sphere {
                radius: radius * s.x.max(s.y).max(s.z),
            },
            Shape::Cuboid { half_extents } => Shape::Cuboid {
                half_extents: half_extents.mul_vec(s),
            },
            Shape::Cylinder {
                radius,
                half_height,
            } => Shape::Cylinder {
                radius: radius * across,
                half_height: half_height * s.y,
            },
            Shape::Cone {
                radius,
                half_height,
            } => Shape::Cone {
                radius: radius * across,
                half_height: half_height * s.y,
            },
            Shape::Frustum {
                radius_top,
                radius_bottom,
                half_height,
            } => Shape::Frustum {
                radius_top: radius_top * across,
                radius_bottom: radius_bottom * across,
                half_height: half_height * s.y,
            },
            Shape::Capsule {
                radius,
                half_height,
            } => Shape::Capsule {
                radius: radius * across,
                half_height: half_height * s.y,
            },
            Shape::Torus {
                major_radius,
                minor_radius,
            } => Shape::Torus {
                major_radius: major_radius * across,
                minor_radius: minor_radius * across.min(s.y),
            },
        }
    }
}

/// The integrals of a frustum's slices along its axis, divided by π: of r² (its volume),
/// and, about its center of mass, of r⁴/2 (its inertia around the axis) and of r⁴/4 + r²
/// (y - c)² (around a side axis); and where that center is.
struct FrustumIntegrals {
    squares: f32,
    center: f32,
    axis: f32,
    side: f32,
}

fn frustum(top: f32, bottom: f32, half_height: f32) -> FrustumIntegrals {
    // Polynomials of degree 4 along the axis: three Gauss–Legendre points are exact.
    let radius = |y: f32| bottom + (top - bottom) * (y + half_height) / (2.0 * half_height);
    let nodes = [
        (-(0.6f32.sqrt()), 5.0 / 9.0),
        (0.0, 8.0 / 9.0),
        (0.6f32.sqrt(), 5.0 / 9.0),
    ];
    let integrate = |f: &dyn Fn(f32, f32) -> f32| {
        nodes
            .iter()
            .map(|&(x, w)| {
                let y = x * half_height;
                w * half_height * f(y, radius(y))
            })
            .sum::<f32>()
    };
    let squares = integrate(&|_, r| r * r);
    if squares <= 0.0 || half_height <= 0.0 {
        return FrustumIntegrals {
            squares: 0.0,
            center: 0.0,
            axis: 0.0,
            side: 0.0,
        };
    }
    let center = integrate(&|y, r| r * r * y) / squares;
    FrustumIntegrals {
        squares,
        center,
        axis: integrate(&|_, r| 0.5 * r.powi(4)),
        side: integrate(&|y, r| 0.25 * r.powi(4) + r * r * (y - center).powi(2)),
    }
}

/// A shape placed in a body: shifted and turned from the body's origin (a floor is a
/// thick slab under the visible surface, so that fast objects do not go through).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Collider {
    /// The shape.
    pub shape: Shape,
    /// Where the shape's origin is, in the body's frame.
    pub offset: Vec3,
    /// How the shape is turned in the body's frame.
    pub rotation: Quat,
}

impl Collider {
    /// A shape at the body's origin.
    pub fn new(shape: Shape) -> Collider {
        Collider {
            shape,
            offset: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }

    /// A ball.
    pub fn sphere(radius: f32) -> Collider {
        Collider::new(Shape::Sphere { radius })
    }

    /// A box of this size (not half size).
    pub fn cuboid(x: f32, y: f32, z: f32) -> Collider {
        Collider::new(Shape::Cuboid {
            half_extents: Vec3::new(x, y, z) * 0.5,
        })
    }

    /// A cylinder along `y`.
    pub fn cylinder(radius: f32, height: f32) -> Collider {
        Collider::new(Shape::Cylinder {
            radius,
            half_height: height / 2.0,
        })
    }

    /// A cone along `y`, tip up.
    pub fn cone(radius: f32, height: f32) -> Collider {
        Collider::new(Shape::Cone {
            radius,
            half_height: height / 2.0,
        })
    }

    /// A cone cut flat along `y`, of these radii at its top and bottom.
    pub fn frustum(radius_top: f32, radius_bottom: f32, height: f32) -> Collider {
        Collider::new(Shape::Frustum {
            radius_top,
            radius_bottom,
            half_height: height / 2.0,
        })
    }

    /// A capsule along `y`.
    pub fn capsule(radius: f32, length: f32) -> Collider {
        Collider::new(Shape::Capsule {
            radius,
            half_height: length / 2.0,
        })
    }

    /// The same collider, moved in the body.
    pub fn with_offset(mut self, offset: Vec3) -> Collider {
        self.offset = offset;
        self
    }

    /// The same collider, turned in the body.
    pub fn with_rotation(mut self, rotation: Quat) -> Collider {
        self.rotation = rotation;
        self
    }

    /// The same collider for a body scaled by `scale`.
    pub fn scaled(&self, scale: Vec3) -> Collider {
        // Turn the scale into the shape's axes before stretching it.
        let axes = [Vec3::X, Vec3::Y, Vec3::Z].map(|a| {
            let a = self.rotation * a;
            Vec3::new(a.x * scale.x, a.y * scale.y, a.z * scale.z).length()
        });
        Collider {
            shape: self.shape.scaled(Vec3::new(axes[0], axes[1], axes[2])),
            offset: self.offset.mul_vec(scale),
            rotation: self.rotation,
        }
    }

    /// Where the mass is centered, in the body's frame.
    pub fn center_of_mass(&self) -> Vec3 {
        self.offset + self.rotation * self.shape.center_of_mass()
    }
}

/// An axis-aligned box, for the broad phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Aabb {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

impl Aabb {
    pub(crate) fn overlaps(&self, o: &Aabb) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
            && self.min.z <= o.max.z
            && self.max.z >= o.min.z
    }
}

/// A shape where it is in the world.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Placed {
    pub(crate) shape: Shape,
    pub(crate) center: Vec3,
    pub(crate) rotation: Quat,
}

impl Placed {
    pub(crate) fn new(collider: &Collider, position: Vec3, rotation: Quat) -> Placed {
        Placed {
            shape: collider.shape,
            center: position + rotation * collider.offset,
            rotation: (rotation * collider.rotation).normalize(),
        }
    }

    /// The farthest point along a world direction.
    pub(crate) fn support(&self, dir: Vec3) -> Vec3 {
        self.center + self.rotation * self.shape.support(self.rotation.inverse() * dir)
    }

    pub(crate) fn aabb(&self, margin: f32) -> Aabb {
        let m = Vec3::splat(margin);
        let max = Vec3::new(
            self.support(Vec3::X).x,
            self.support(Vec3::Y).y,
            self.support(Vec3::Z).z,
        );
        let min = Vec3::new(
            self.support(Vec3::NEG_X).x,
            self.support(Vec3::NEG_Y).y,
            self.support(Vec3::NEG_Z).z,
        );
        Aabb {
            min: min - m,
            max: max + m,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_properties_match_the_formulas() {
        let cube = Shape::Cuboid {
            half_extents: Vec3::splat(0.5),
        };
        assert!((cube.volume() - 1.0).abs() < 1e-6);
        assert!((cube.inertia(6.0).x - 1.0).abs() < 1e-6);
        let ball = Shape::Sphere { radius: 1.0 };
        assert!((ball.volume() - 4.18879).abs() < 1e-4);
        let cone = Shape::Cone {
            radius: 1.0,
            half_height: 1.0,
        };
        assert_eq!(cone.center_of_mass(), Vec3::new(0.0, -0.5, 0.0));
        assert!((cone.volume() - 2.0943951).abs() < 1e-5);
    }

    #[test]
    fn a_capsule_turns_like_a_cylinder_and_two_half_balls() {
        // With no straight part, a ball.
        let ball = Shape::Capsule {
            radius: 0.5,
            half_height: 0.0,
        };
        let i = ball.inertia(2.0);
        assert!((i.x - 0.4 * 2.0 * 0.25).abs() < 1e-6 && (i.y - i.x).abs() < 1e-6);
        // Against a sum over a fine grid of its volume.
        let (r, hh) = (0.3f32, 0.5f32);
        let capsule = Shape::Capsule {
            radius: r,
            half_height: hh,
        };
        let (mut count, mut ix, mut iy) = (0.0f64, 0.0f64, 0.0f64);
        let n = 60;
        for a in 0..n {
            for b in 0..n * 3 {
                for c in 0..n {
                    let x = (a as f32 + 0.5) / n as f32 * 2.0 * r - r;
                    let y = (b as f32 + 0.5) / (n * 3) as f32 * 2.0 * (hh + r) - (hh + r);
                    let z = (c as f32 + 0.5) / n as f32 * 2.0 * r - r;
                    let core = y.clamp(-hh, hh);
                    let inside = x * x + (y - core).powi(2) + z * z <= r * r;
                    if inside {
                        count += 1.0;
                        ix += f64::from(y * y + z * z);
                        iy += f64::from(x * x + z * z);
                    }
                }
            }
        }
        let i = capsule.inertia(1.0);
        assert!(
            ((ix / count) as f32 - i.x).abs() < 0.002 * i.x,
            "{} {}",
            ix / count,
            i.x
        );
        assert!(
            ((iy / count) as f32 - i.y).abs() < 0.002 * i.y,
            "{} {}",
            iy / count,
            i.y
        );
    }

    #[test]
    fn a_frustum_is_a_cylinder_or_a_cone_at_the_limits() {
        let close = |a: Vec3, b: Vec3| (a - b).length() <= 1e-4 * a.length().max(1.0);
        let cylinder = Shape::Cylinder {
            radius: 0.6,
            half_height: 0.8,
        };
        let same = Shape::Frustum {
            radius_top: 0.6,
            radius_bottom: 0.6,
            half_height: 0.8,
        };
        assert!((same.volume() - cylinder.volume()).abs() < 1e-5);
        assert!(close(same.inertia(3.0), cylinder.inertia(3.0)));
        assert!(close(same.center_of_mass(), Vec3::ZERO));
        let cone = Shape::Cone {
            radius: 0.5,
            half_height: 0.7,
        };
        let pointed = Shape::Frustum {
            radius_top: 0.0,
            radius_bottom: 0.5,
            half_height: 0.7,
        };
        assert!((pointed.volume() - cone.volume()).abs() < 1e-5);
        assert!(close(pointed.center_of_mass(), cone.center_of_mass()));
        assert!(
            close(pointed.inertia(2.0), cone.inertia(2.0)),
            "{:?} {:?}",
            pointed.inertia(2.0),
            cone.inertia(2.0)
        );
        // Upside down, its mass is near the top.
        let bucket = Shape::Frustum {
            radius_top: 0.5,
            radius_bottom: 0.3,
            half_height: 0.4,
        };
        assert!(bucket.center_of_mass().y > 0.0);
        assert_eq!(
            bucket.support(Vec3::new(1.0, 0.1, 0.0)),
            Vec3::new(0.5, 0.4, 0.0)
        );
    }

    #[test]
    fn supports_are_the_extreme_points() {
        let cone = Shape::Cone {
            radius: 1.0,
            half_height: 1.0,
        };
        assert_eq!(cone.support(Vec3::Y), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(
            cone.support(Vec3::new(1.0, -0.1, 0.0)),
            Vec3::new(1.0, -1.0, 0.0)
        );
        let torus = Shape::Torus {
            major_radius: 2.0,
            minor_radius: 0.5,
        };
        assert!((torus.support(Vec3::X) - Vec3::new(2.5, 0.0, 0.0)).length() < 1e-6);
        let placed = Placed::new(
            &Collider::cuboid(2.0, 2.0, 2.0),
            Vec3::new(5.0, 0.0, 0.0),
            Quat::IDENTITY,
        );
        let aabb = placed.aabb(0.0);
        assert_eq!(
            (aabb.min, aabb.max),
            (Vec3::new(4.0, -1.0, -1.0), Vec3::new(6.0, 1.0, 1.0))
        );
    }
}
