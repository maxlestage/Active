//! Rigid-body physics in 3D, run by [Rapier](https://rapier.rs), copied into Active as
//! [`rapier`] (feature `physics`, on by default): bodies with a mass,
//! forces and torques, gravity, collisions between every shape of
//! [`scene`](crate::scene) and [`ecs`](crate::ecs) (balls, boxes, cylinders, cones,
//! capsules, rings), friction, rolling resistance and bounce, and bodies that attract each
//! other.
//!
//! [`ecs`](crate::ecs) gives every mesh a body (its `PhysicsPlugin` is in `DefaultPlugins`),
//! [`scene::Physics`](crate::scene::Physics) does the same for a scene, and
//! [`PhysicsWorld`] can be used directly:
//!
//! ```
//! use active::physics::*;
//! use active::scene::math::Vec3;
//!
//! let mut world = PhysicsWorld::new();
//! world.add(Body::fixed(Collider::cuboid(20.0, 1.0, 20.0)).at(Vec3::new(0.0, -0.5, 0.0)));
//! let ball = world.add(Body::dynamic(Collider::sphere(0.5)).at(Vec3::new(0.0, 3.0, 0.0)));
//! for _ in 0..240 {
//!     world.step(1.0 / 60.0); // four seconds
//! }
//! let y = world.body(ball).unwrap().position.y;
//! assert!((y - 0.5).abs() < 0.02, "it rests on the floor: {y}");
//! ```
//!
//! How it works. [`Body`] and [`Collider`] are plain values, in the units and the math
//! types of [`scene`](crate::scene): you read and change their fields between two steps.
//! [`PhysicsWorld`] keeps a Rapier body and collider for each of them, and every
//! [`step`](PhysicsWorld::step):
//!
//! 1. Sends Rapier what changed by hand since the last step (a place, a velocity, a
//!    shape, a mass, a material, a force), and wakes up what was changed.
//! 2. Cuts the step into steps of at most a sixtieth of a second (so the motion does not
//!    depend on the frame rate); each is solved by Rapier in
//!    [`substeps`](PhysicsWorld::substeps) (its TGS soft solver): broad phase, contacts
//!    (with continuous collision detection: nothing goes through a thin wall), Coulomb
//!    friction, restitution, gyroscopic forces (a tumbling brick wobbles and flips as a
//!    real one does) and islands that fall asleep.
//! 3. Adds what Rapier does not have: the attraction between bodies ([`Attraction`],
//!    a Barnes–Hut octree) and the rolling resistance of round bodies
//!    ([`Body::rolling_friction`]).
//! 4. Reads the bodies back, and the pairs that started or stopped touching
//!    ([`events`](PhysicsWorld::events)).
//!
//! The masses and inertias stay the exact ones of [`Shape`] (including the rings and the
//! cut cones Rapier has no primitive for: a ring collides as 24 capsules along its tube, a
//! cut cone as a convex hull). Units are SI: meters, kilograms, seconds, newtons.
//!
//! Without the `physics` feature (`default-features = false`), only the shapes stay
//! ([`Shape`], [`Collider`], [`Attraction`]): no engine, and no dependency.

#[cfg_attr(not(feature = "physics"), allow(dead_code))]
mod gravity;
mod shape;
#[cfg(feature = "physics")]
mod world;

pub use gravity::Attraction;
pub use shape::{Collider, Shape};
#[cfg(feature = "physics")]
pub use world::{Body, BodyId, BodyKind, CollisionEvent, PhysicsWorld};

/// The physics engine underneath: [Rapier](https://rapier.rs), copied into Active (under
/// the Apache License 2.0, see `README.md` and `LICENSE` in this directory). For what
/// [`PhysicsWorld::rapier_mut`] gives access to: joints, ray casts, collision groups.
#[cfg(feature = "physics")]
#[allow(missing_docs)]
pub mod rapier;

#[cfg(all(test, feature = "physics"))]
mod laws;
#[cfg(all(test, feature = "physics"))]
mod tests;
