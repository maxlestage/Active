//! Shapes: what collides, and where the mass is.

use crate::three::math::{Quat, Vec3};

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
    /// A capsule along `y`: a cylinder with half balls at its ends.
    Capsule {
        /// Its radius.
        radius: f32,
        /// Half the height of its straight part.
        half_height: f32,
    },
    /// A ring lying around `y`. It collides as its convex hull (a coin with rounded
    /// edges): nothing goes through the hole.
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
            Shape::Capsule {
                radius,
                half_height,
            } => {
                // A cylinder of the whole height: close enough for how it moves.
                let h = 2.0 * (half_height + radius);
                let side = mass * (3.0 * radius * radius + h * h) / 12.0;
                Vec3::new(side, 0.5 * mass * radius * radius, side)
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
