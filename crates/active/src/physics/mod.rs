//! Rigid-body physics in 3D, run by [Rapier](https://rapier.rs): bodies with a mass,
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

mod gravity;
mod shape;

use std::collections::BTreeSet;

pub use gravity::Attraction;
/// Rapier itself, for what [`PhysicsWorld::rapier_mut`] gives access to (joints, ray
/// casts, collision groups).
pub use rapier3d as rapier;
pub use shape::{Collider, Shape};

use crate::scene::math::{Quat, Vec3};
use rapier3d::prelude as rp;

/// How a body moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// Moved by forces, gravity and collisions.
    #[default]
    Dynamic,
    /// Never moves (a floor, a wall); infinitely heavy.
    Static,
    /// Moved by its velocity only, pushing what it meets (a platform, a door, a paddle);
    /// infinitely heavy.
    Kinematic,
}

/// A rigid body: a collider, a mass, a position, velocities and the forces on it.
#[derive(Clone, Debug)]
pub struct Body {
    /// How it moves.
    pub kind: BodyKind,
    collider: Collider,
    /// Where its frame is (the origin of its collider's frame, not its center of mass).
    pub position: Vec3,
    /// How it is turned.
    pub rotation: Quat,
    /// The velocity of its center of mass, in m/s.
    pub linear_velocity: Vec3,
    /// Its rotation speed, in rad/s, around the axis it points along.
    pub angular_velocity: Vec3,
    /// The force applied during the next step, in newtons (cleared after it).
    pub force: Vec3,
    /// The torque applied during the next step, in N·m (cleared after it).
    pub torque: Vec3,
    mass: f32,
    inertia: Vec3,
    /// How much it bounces, from 0 (not at all) to 1 (keeps all its speed).
    pub restitution: f32,
    /// How much it grips, from 0 (ice) to 1 and more (rubber).
    pub friction: f32,
    /// How much it resists rolling and spinning on what it touches (the rolling
    /// resistance coefficient: 0.001 for steel on steel, 0.01 for a rubber ball on
    /// concrete, the default; 0 rolls forever).
    pub rolling_friction: f32,
    /// How fast it slows down by itself, per second, as in thick air or water (0: not at
    /// all, the default, as in a vacuum).
    pub linear_damping: f32,
    /// How fast its rotation slows down by itself, per second (0 by default).
    pub angular_damping: f32,
    /// How much gravity pulls it (0: it floats, -1: it falls up).
    pub gravity_scale: f32,
    sleeping: bool,
}

impl Body {
    /// A body of this kind, with a mass of its volume at density 1 (kg/m³ times m³).
    pub fn new(kind: BodyKind, collider: Collider) -> Body {
        let mut body = Body {
            kind,
            collider,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
            mass: 0.0,
            inertia: Vec3::ZERO,
            restitution: 0.2,
            friction: 0.5,
            rolling_friction: 0.01,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
        };
        body.set_density(1.0);
        body
    }

    /// A body moved by forces and collisions.
    pub fn dynamic(collider: Collider) -> Body {
        Body::new(BodyKind::Dynamic, collider)
    }

    /// A body that never moves.
    pub fn fixed(collider: Collider) -> Body {
        Body::new(BodyKind::Static, collider)
    }

    /// A body moved by its velocity only.
    pub fn kinematic(collider: Collider) -> Body {
        Body::new(BodyKind::Kinematic, collider)
    }

    /// The same body, placed here.
    pub fn at(mut self, position: Vec3) -> Body {
        self.position = position;
        self
    }

    /// The same body, turned.
    pub fn turned(mut self, rotation: Quat) -> Body {
        self.rotation = rotation;
        self
    }

    /// The same body, moving.
    pub fn moving(mut self, velocity: Vec3) -> Body {
        self.linear_velocity = velocity;
        self
    }

    /// The same body, with this mass (in kg).
    pub fn with_mass(mut self, mass: f32) -> Body {
        self.set_mass(mass);
        self
    }

    /// The same body, with this density (kg/m³: 1000 is water, 7800 steel).
    pub fn with_density(mut self, density: f32) -> Body {
        self.set_density(density);
        self
    }

    /// Its collider.
    pub fn collider(&self) -> &Collider {
        &self.collider
    }

    /// Changes its collider, keeping its density.
    pub fn set_collider(&mut self, collider: Collider) {
        let density = self.density();
        self.collider = collider;
        self.set_density(density);
    }

    /// Its mass, in kg.
    pub fn mass(&self) -> f32 {
        self.mass
    }

    /// Changes its mass (its inertia follows).
    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass.max(1e-6);
        self.inertia = self.collider.shape.inertia(self.mass);
    }

    /// Its mass per volume.
    pub fn density(&self) -> f32 {
        let volume = self.collider.shape.volume();
        if volume > 0.0 {
            self.mass / volume
        } else {
            1.0
        }
    }

    /// Changes its density (its mass follows its volume).
    pub fn set_density(&mut self, density: f32) {
        let volume = self.collider.shape.volume().max(1e-9);
        self.set_mass(volume * density.max(1e-6));
    }

    /// Its moments of inertia, along the axes of its shape.
    pub fn inertia(&self) -> Vec3 {
        self.inertia
    }

    /// Where its mass is centered, in the world.
    pub fn center_of_mass(&self) -> Vec3 {
        self.position + self.rotation * self.collider.center_of_mass()
    }

    /// Its kinetic energy, in joules.
    pub fn kinetic_energy(&self) -> f32 {
        if self.kind != BodyKind::Dynamic {
            return 0.0;
        }
        let w = self.inertia_frame().inverse() * self.angular_velocity;
        0.5 * self.mass * self.linear_velocity.length_squared()
            + 0.5
                * (self.inertia.x * w.x * w.x
                    + self.inertia.y * w.y * w.y
                    + self.inertia.z * w.z * w.z)
    }

    /// Its angular momentum around its center of mass, in kg·m²/s: with no torque, it
    /// never changes, even while the rotation axis of a tumbling body wobbles.
    pub fn angular_momentum(&self) -> Vec3 {
        if self.kind != BodyKind::Dynamic {
            return Vec3::ZERO;
        }
        let q = self.inertia_frame();
        let w = q.inverse() * self.angular_velocity;
        q * w.mul_vec(self.inertia)
    }

    /// Pushes it during the next step, at its center of mass.
    pub fn apply_force(&mut self, force: Vec3) {
        self.force += force;
        self.wake_up();
    }

    /// Pushes it during the next step at a point of the world: it also turns.
    pub fn apply_force_at(&mut self, force: Vec3, point: Vec3) {
        self.force += force;
        self.torque += (point - self.center_of_mass()).cross(force);
        self.wake_up();
    }

    /// Turns it during the next step.
    pub fn apply_torque(&mut self, torque: Vec3) {
        self.torque += torque;
        self.wake_up();
    }

    /// Changes its velocity at once, as a kick would (in N·s).
    pub fn apply_impulse(&mut self, impulse: Vec3) {
        if self.kind == BodyKind::Dynamic {
            self.linear_velocity += impulse / self.mass;
            self.wake_up();
        }
    }

    /// The same, at a point of the world: it also starts turning.
    pub fn apply_impulse_at(&mut self, impulse: Vec3, point: Vec3) {
        if self.kind == BodyKind::Dynamic {
            self.linear_velocity += impulse / self.mass;
            let angular = (point - self.center_of_mass()).cross(impulse);
            self.angular_velocity += self.inverse_inertia(angular);
            self.wake_up();
        }
    }

    /// Whether it sleeps: still for a while, it is not simulated until touched.
    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    /// Wakes it up.
    pub fn wake_up(&mut self) {
        self.sleeping = false;
    }

    fn inertia_frame(&self) -> Quat {
        self.rotation * self.collider.rotation
    }

    /// `I⁻¹ v` in the world (zero for bodies that do not turn).
    fn inverse_inertia(&self, v: Vec3) -> Vec3 {
        if self.kind != BodyKind::Dynamic {
            return Vec3::ZERO;
        }
        let q = self.inertia_frame();
        let local = q.inverse() * v;
        let inv = |i: f32| if i > 0.0 { 1.0 / i } else { 0.0 };
        q * Vec3::new(
            local.x * inv(self.inertia.x),
            local.y * inv(self.inertia.y),
            local.z * inv(self.inertia.z),
        )
    }
}

/// A body of a [`PhysicsWorld`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyId {
    index: u32,
    generation: u32,
}

/// Two bodies started or stopped touching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionEvent {
    /// They touch.
    Started(BodyId, BodyId),
    /// They no longer touch.
    Ended(BodyId, BodyId),
}

/// What was last sent to Rapier, or read back from it: what differs was changed by hand.
#[derive(Clone, Debug)]
struct Synced {
    kind: BodyKind,
    collider: Collider,
    mass: f32,
    inertia: Vec3,
    position: Vec3,
    rotation: Quat,
    linear_velocity: Vec3,
    angular_velocity: Vec3,
    friction: f32,
    restitution: f32,
    linear_damping: f32,
    angular_damping: f32,
    gravity_scale: f32,
    sleeping: bool,
}

impl Synced {
    fn of(b: &Body) -> Synced {
        Synced {
            kind: b.kind,
            collider: b.collider,
            mass: b.mass,
            inertia: b.inertia,
            position: b.position,
            rotation: b.rotation,
            linear_velocity: b.linear_velocity,
            angular_velocity: b.angular_velocity,
            friction: b.friction,
            restitution: b.restitution,
            linear_damping: b.linear_damping,
            angular_damping: b.angular_damping,
            gravity_scale: b.gravity_scale,
            sleeping: b.sleeping,
        }
    }
}

struct Entry {
    body: Body,
    handle: rp::RigidBodyHandle,
    collider: rp::ColliderHandle,
    synced: Synced,
}

struct Slot {
    generation: u32,
    entry: Option<Entry>,
}

/// The bodies, and what moves them.
pub struct PhysicsWorld {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The acceleration of gravity (9.81 m/s² down by default).
    pub gravity: Vec3,
    /// Substeps per sixtieth of a second, the substeps of Rapier's solver: more is more
    /// precise, and slower (4). A longer step is cut into more substeps, so that the
    /// motion does not depend on the frame rate.
    pub substeps: u32,
    /// Solver passes per substep: more makes stacks stiffer, and is slower (1).
    pub iterations: u32,
    /// Whether bodies attract each other, and how much.
    pub attraction: Option<Attraction>,
    engine: rp::PhysicsWorld,
    touching: BTreeSet<(u32, u32)>,
    events: Vec<CollisionEvent>,
    // The gravity and attraction of the last step: when they change, everything wakes.
    last_gravity: Vec3,
    last_attraction: Option<Attraction>,
}

impl Default for PhysicsWorld {
    fn default() -> PhysicsWorld {
        PhysicsWorld::new()
    }
}

impl std::fmt::Debug for PhysicsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicsWorld")
            .field("bodies", &self.len())
            .field("gravity", &self.gravity)
            .finish()
    }
}

/// Closer than this (m), two surfaces touch: what Rapier lets bodies overlap by.
const TOUCH: f32 = 0.005;
/// Below this speed (m/s) for half a second, a group of touching bodies sleeps.
const SLEEP_SPEED: f32 = 0.02;
/// The most steps of a sixtieth of a second in one step: a longer step is less precise.
const MAX_STEPS: u32 = 256;
/// A ring collides as this many capsules along its tube.
const RING_SEGMENTS: usize = 24;
/// A cut cone collides as the convex hull of two polygons of this many sides.
const FRUSTUM_SIDES: usize = 24;

impl PhysicsWorld {
    /// An empty world, with Earth's gravity.
    pub fn new() -> PhysicsWorld {
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        let mut engine = rp::PhysicsWorld::new();
        engine.gravity = vector(gravity);
        PhysicsWorld {
            slots: Vec::new(),
            free: Vec::new(),
            gravity,
            substeps: 4,
            iterations: 1,
            attraction: None,
            engine,
            touching: BTreeSet::new(),
            events: Vec::new(),
            last_gravity: gravity,
            last_attraction: None,
        }
    }

    /// Adds a body.
    pub fn add(&mut self, mut body: Body) -> BodyId {
        let index = match self.free.pop() {
            Some(index) => index,
            None => {
                self.slots.push(Slot {
                    generation: 0,
                    entry: None,
                });
                (self.slots.len() - 1) as u32
            }
        };
        body.sleeping = false;
        let rigid = rp::RigidBodyBuilder::new(body_type(body.kind))
            .pose(pose(body.position, body.rotation))
            .linvel(vector(body.linear_velocity))
            .angvel(vector(body.angular_velocity))
            .linear_damping(body.linear_damping)
            .angular_damping(body.angular_damping)
            .gravity_scale(body.gravity_scale)
            .additional_mass_properties(mass_properties(&body))
            .ccd_enabled(body.kind == BodyKind::Dynamic)
            .gyroscopic_forces_enabled(true)
            .user_data(index as u128);
        let handle = self.engine.insert_body(rigid);
        // Rapier lets a body sleep below 0.1 m/s: a box sliding slowly on ice would stop
        // dead. Here, below 2 cm/s.
        self.engine.bodies[handle]
            .activation_mut()
            .normalized_linear_threshold = SLEEP_SPEED / 2.0;
        let collider = self
            .engine
            .insert_collider(collider_of(&body, index), Some(handle));
        self.slots[index as usize].entry = Some(Entry {
            synced: Synced::of(&body),
            body,
            handle,
            collider,
        });
        BodyId {
            index,
            generation: self.slots[index as usize].generation,
        }
    }

    /// Removes a body and gives it back.
    pub fn remove(&mut self, id: BodyId) -> Option<Body> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let entry = slot.entry.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        // What rested on it falls.
        for &(a, b) in &self.touching {
            let other = if a == id.index {
                b
            } else if b == id.index {
                a
            } else {
                continue;
            };
            if let Some(e) = self.slots[other as usize].entry.as_ref() {
                self.engine.wake_up(e.handle, true);
            }
        }
        self.touching
            .retain(|&(a, b)| a != id.index && b != id.index);
        self.engine.remove_body_with_colliders(entry.handle, true);
        Some(entry.body)
    }

    /// A body.
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.entry.as_ref().map(|e| &e.body)
        } else {
            None
        }
    }

    /// A body, to change.
    pub fn body_mut(&mut self, id: BodyId) -> Option<&mut Body> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.entry.as_mut().map(|e| &mut e.body)
        } else {
            None
        }
    }

    /// How many bodies there are.
    pub fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bodies and their ids.
    pub fn bodies(&self) -> impl Iterator<Item = (BodyId, &Body)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.entry.as_ref().map(|e| {
                (
                    BodyId {
                        index: i as u32,
                        generation: s.generation,
                    },
                    &e.body,
                )
            })
        })
    }

    fn id_of(&self, index: u32) -> BodyId {
        BodyId {
            index,
            generation: self.slots[index as usize].generation,
        }
    }

    /// The bodies that started or stopped touching during the last step.
    pub fn events(&self) -> &[CollisionEvent] {
        &self.events
    }

    /// The pairs of bodies touching now.
    pub fn contacts(&self) -> impl Iterator<Item = (BodyId, BodyId)> + '_ {
        self.touching
            .iter()
            .map(|&(a, b)| (self.id_of(a), self.id_of(b)))
    }

    /// The total momentum of the dynamic bodies, in kg·m/s.
    pub fn momentum(&self) -> Vec3 {
        self.bodies()
            .filter(|(_, b)| b.kind == BodyKind::Dynamic)
            .fold(Vec3::ZERO, |sum, (_, b)| sum + b.linear_velocity * b.mass)
    }

    /// The total angular momentum of the dynamic bodies around the origin, in kg·m²/s.
    pub fn angular_momentum(&self) -> Vec3 {
        self.bodies()
            .filter(|(_, b)| b.kind == BodyKind::Dynamic)
            .fold(Vec3::ZERO, |sum, (_, b)| {
                sum + b.angular_momentum() + b.center_of_mass().cross(b.linear_velocity * b.mass)
            })
    }

    /// The total kinetic energy of the dynamic bodies, in joules.
    pub fn kinetic_energy(&self) -> f32 {
        self.bodies().map(|(_, b)| b.kinetic_energy()).sum()
    }

    /// The Rapier world underneath, to read what this module does not show (contact
    /// points, ray casts through its query pipeline). Bodies and colliders carry the index
    /// of their [`BodyId`] in their `user_data`.
    pub fn rapier(&self) -> &rp::PhysicsWorld {
        &self.engine
    }

    /// The Rapier world underneath, to change: joints, collision groups, solver
    /// settings. What is changed there on a body is kept, and read back into its
    /// [`Body`] after the next [`step`](Self::step); what is changed on the [`Body`] wins
    /// over it.
    pub fn rapier_mut(&mut self) -> &mut rp::PhysicsWorld {
        &mut self.engine
    }

    /// Moves the world forward by `dt` seconds. Any step works: it is cut into steps of
    /// at most a sixtieth of a second, each solved in [`substeps`](Self::substeps), so a
    /// long frame gives the same motion as several short ones.
    pub fn step(&mut self, dt: f32) {
        self.events.clear();
        if dt <= 0.0 || !dt.is_finite() {
            return;
        }
        self.send_changes();
        // A little tolerance: 1/60 s in f32 is not exactly a sixtieth.
        let steps = ((dt * 60.0 - 1e-3).ceil().max(1.0) as u32).min(MAX_STEPS);
        let h = dt / steps as f32;
        let params = &mut self.engine.integration_parameters;
        params.dt = h;
        params.num_solver_iterations = self.substeps.max(1) as usize;
        params.num_internal_pgs_iterations = self.iterations.max(1) as usize;
        self.engine.gravity = vector(self.gravity);
        // Attraction, which changes with the positions, is applied as leapfrog does: half
        // its pull before the move, half after, from the new positions (second order, and
        // orbits stay closed).
        let mut pull = self.attraction_accelerations();
        for _ in 0..steps {
            self.kick(&pull, 0.5 * h);
            self.apply_forces();
            let free = self.free_flight();
            self.engine.step();
            self.follow_free_flights(&free, h);
            self.resist_rolling();
            self.update_touching();
            if self.attraction.is_some() {
                pull = self.attraction_accelerations();
                self.kick(&pull, 0.5 * h);
            }
        }
        self.read_back();
        self.last_gravity = self.gravity;
        self.last_attraction = self.attraction;
    }

    /// Sends Rapier what was changed by hand since the last step, and wakes it up.
    fn send_changes(&mut self) {
        let everything =
            self.gravity != self.last_gravity || self.attraction != self.last_attraction;
        if everything {
            self.engine.wake_up_all(true);
        }
        let engine = &mut self.engine;
        for entry in self.slots.iter_mut().filter_map(|s| s.entry.as_mut()) {
            let (b, s) = (&entry.body, &mut entry.synced);
            let rb = &mut engine.bodies[entry.handle];
            let mut wake = b.force != Vec3::ZERO || b.torque != Vec3::ZERO;
            if b.kind != s.kind {
                rb.set_body_type(body_type(b.kind), true);
                rb.enable_ccd(b.kind == BodyKind::Dynamic);
                wake = true;
            }
            if b.collider != s.collider {
                let c = &mut engine.colliders[entry.collider];
                c.set_shape(shape_of(&b.collider.shape));
                c.set_position_wrt_parent(pose(b.collider.offset, b.collider.rotation));
                wake = true;
            }
            if b.collider != s.collider || b.mass != s.mass || b.inertia != s.inertia {
                rb.set_additional_mass_properties(mass_properties(b), true);
                wake = true;
            }
            if b.position != s.position || b.rotation != s.rotation {
                rb.set_position(pose(b.position, b.rotation), true);
                wake = true;
            }
            if b.linear_velocity != s.linear_velocity {
                rb.set_linvel(vector(b.linear_velocity), true);
                wake = true;
            }
            if b.angular_velocity != s.angular_velocity {
                rb.set_angvel(vector(b.angular_velocity), true);
                wake = true;
            }
            if b.linear_damping != s.linear_damping {
                rb.set_linear_damping(b.linear_damping);
            }
            if b.angular_damping != s.angular_damping {
                rb.set_angular_damping(b.angular_damping);
            }
            if b.gravity_scale != s.gravity_scale {
                rb.set_gravity_scale(b.gravity_scale, true);
                wake = true;
            }
            if b.friction != s.friction || b.restitution != s.restitution {
                let c = &mut engine.colliders[entry.collider];
                c.set_friction(b.friction.max(0.0));
                c.set_restitution(b.restitution.clamp(0.0, 1.0));
            }
            if s.sleeping && !b.sleeping {
                wake = true;
            }
            if wake && b.kind != BodyKind::Static {
                engine.bodies[entry.handle].wake_up(true);
            }
            *s = Synced::of(b);
        }
    }

    /// The forces set by hand, for the next step.
    fn apply_forces(&mut self) {
        for entry in self.slots.iter().filter_map(|s| s.entry.as_ref()) {
            let b = &entry.body;
            if b.kind != BodyKind::Dynamic {
                continue;
            }
            let rb = &mut self.engine.bodies[entry.handle];
            let force = b.force;
            rb.reset_forces(false);
            rb.reset_torques(false);
            if force != Vec3::ZERO {
                rb.add_force(vector(force), false);
            }
            if b.torque != Vec3::ZERO {
                rb.add_torque(vector(b.torque), false);
            }
        }
    }

    /// Changes the velocities of the dynamic bodies by `accelerations × dt`.
    fn kick(&mut self, accelerations: &[Vec3], dt: f32) {
        for (slot, a) in self.slots.iter().zip(accelerations) {
            let Some(e) = slot.entry.as_ref() else {
                continue;
            };
            let rb = &mut self.engine.bodies[e.handle];
            if e.body.kind == BodyKind::Dynamic && !rb.is_sleeping() && *a != Vec3::ZERO {
                let v = point(rb.linvel()) + *a * dt;
                rb.set_linvel(vector(v), false);
            }
        }
    }

    /// The awake dynamic bodies far from anything, and their uniform acceleration
    /// (gravity and the forces set by hand).
    fn free_flight(&self) -> Vec<Free> {
        let mut touching = vec![false; self.slots.len()];
        for pair in self.engine.narrow_phase.contact_pairs() {
            // Close enough for Rapier to keep contact points: not free (a body that rests
            // lifts off its support by a hair at times, and must not be moved then).
            if pair.manifolds().iter().all(|m| m.points.is_empty()) {
                continue;
            }
            for c in [pair.collider1, pair.collider2] {
                if let Some(c) = self.engine.colliders.get(c) {
                    if let Some(t) = touching.get_mut(c.user_data as usize) {
                        *t = true;
                    }
                }
            }
        }
        self.slots
            .iter()
            .zip(touching)
            .filter_map(|(slot, touching)| {
                let e = slot.entry.as_ref()?;
                let b = &e.body;
                let rb = &self.engine.bodies[e.handle];
                (b.kind == BodyKind::Dynamic && !touching && !rb.is_sleeping()).then(|| Free {
                    handle: e.handle,
                    accel: self.gravity * b.gravity_scale + b.force / b.mass,
                    // Turning freely: no torque, no damping, and an inertia that differs
                    // along its axes (otherwise Rapier's turn is already exact).
                    spin: (b.torque == Vec3::ZERO
                        && b.angular_damping == 0.0
                        && !is_isotropic(b.inertia))
                    .then(|| (rotation(*rb.rotation()), point(rb.angvel()))),
                })
            })
            .collect()
    }

    /// Rapier moves a body with its velocity at the end of each substep (semi-implicit
    /// Euler), which lands a thrown ball `a·h·T/2` too far along its acceleration after
    /// `T` seconds of flight, and turns it with an approximate gyroscopic term, which lets
    /// the angular momentum of a tumbling body drift. The bodies far from anything follow
    /// their exact parabola instead, and turn by the exact free rotation.
    fn follow_free_flights(&mut self, free: &[Free], h: f32) {
        let substeps = self.substeps.max(1);
        let substep = h / substeps as f32;
        for f in free {
            let rb = &mut self.engine.bodies[f.handle];
            if rb.is_sleeping() {
                continue;
            }
            let com = point(rb.center_of_mass()) + f.accel * (-0.5 * h * substep);
            let props = rb.mass_properties().local_mprops;
            let mut turn = rotation(*rb.rotation());
            if let Some((mut q, mut w)) = f.spin {
                let frame = rotation(props.principal_inertia_local_frame);
                let inv = point(props.inv_principal_inertia);
                let of = |i: f32| if i > 0.0 { 1.0 / i } else { 0.0 };
                let inertia = Vec3::new(of(inv.x), of(inv.y), of(inv.z));
                for _ in 0..substeps {
                    (q, w) = free_rotation(q, frame, inertia, w, substep);
                }
                turn = q;
                rb.set_angvel(vector(w), false);
            }
            let origin = com - turn * point(props.local_com);
            rb.set_position(pose(origin, turn), false);
        }
    }

    fn attraction_accelerations(&self) -> Vec<Vec3> {
        let Some(cfg) = self.attraction else {
            return Vec::new();
        };
        let (mut positions, mut masses) = (
            Vec::with_capacity(self.slots.len()),
            Vec::with_capacity(self.slots.len()),
        );
        for slot in &self.slots {
            match &slot.entry {
                Some(e) if e.body.kind == BodyKind::Dynamic => {
                    positions.push(point(self.engine.bodies[e.handle].center_of_mass()));
                    masses.push(e.body.mass);
                }
                _ => {
                    positions.push(Vec3::ZERO);
                    masses.push(0.0);
                }
            }
        }
        gravity::accelerations(&positions, &masses, &cfg)
    }

    /// Rolling resistance, which Rapier does not have: at each contact, the ground
    /// deforms a little under a round body and pushes back ahead of where it touches,
    /// a torque of at most `rolling_friction × normal force × lever arm` against its
    /// rolling and its spinning.
    fn resist_rolling(&mut self) {
        let engine = &mut self.engine;
        let mut turns: Vec<(rp::RigidBodyHandle, Vec3)> = Vec::new();
        for pair in engine.narrow_phase.contact_pairs() {
            let (Some(c1), Some(c2)) = (
                engine.colliders.get(pair.collider1),
                engine.colliders.get(pair.collider2),
            ) else {
                continue;
            };
            let (Some(h1), Some(h2)) = (c1.parent(), c2.parent()) else {
                continue;
            };
            let entry = |c: &rp::Collider| {
                self.slots
                    .get(c.user_data as usize)
                    .and_then(|s| s.entry.as_ref())
            };
            let (Some(e1), Some(e2)) = (entry(c1), entry(c2)) else {
                continue;
            };
            let rolling = e1.body.rolling_friction.max(e2.body.rolling_friction);
            if rolling <= 0.0 {
                continue;
            }
            let (b1, b2) = (&engine.bodies[h1], &engine.bodies[h2]);
            let dynamic1 = b1.is_dynamic() && !b1.is_sleeping();
            let dynamic2 = b2.is_dynamic() && !b2.is_sleeping();
            if !dynamic1 && !dynamic2 {
                continue;
            }
            for manifold in pair.manifolds() {
                let pressed: f32 = manifold.points.iter().map(|p| p.data.impulse).sum();
                if pressed <= 0.0 || manifold.data.solver_contacts.is_empty() {
                    continue;
                }
                let n = point(manifold.data.normal);
                let contact = point(c1.position() * manifold.points[0].local_p1);
                // The lever arm: from the contact plane to the center of the body that
                // rolls (the one that moves, or the smaller arm of two).
                let arm = |b: &rp::RigidBody| (point(b.center_of_mass()) - contact).dot(n).abs();
                let lever = match (dynamic1, dynamic2) {
                    (true, false) => arm(b1),
                    (false, true) => arm(b2),
                    _ => arm(b1).min(arm(b2)),
                };
                let limit = rolling * pressed * lever.max(0.0);
                if limit <= 0.0 {
                    continue;
                }
                let w1 = if dynamic1 {
                    point(b1.angvel())
                } else {
                    Vec3::ZERO
                };
                let w2 = if dynamic2 {
                    point(b2.angvel())
                } else {
                    Vec3::ZERO
                };
                let inv1 = |v: Vec3| {
                    if dynamic1 {
                        inverse_inertia(b1, v)
                    } else {
                        Vec3::ZERO
                    }
                };
                let inv2 = |v: Vec3| {
                    if dynamic2 {
                        inverse_inertia(b2, v)
                    } else {
                        Vec3::ZERO
                    }
                };
                // The angular impulse that would stop their relative turn, around the
                // two tangents (rolling) and the normal (spinning), each held to the
                // limit.
                let relative = w1 - w2;
                let spin = n * relative.dot(n);
                let roll = relative - spin;
                let mut impulse = Vec3::ZERO;
                for (turn, cap) in [(roll, limit), (spin, limit)] {
                    let speed = turn.length();
                    if speed <= 1e-9 {
                        continue;
                    }
                    let axis = turn / speed;
                    let k = axis.dot(inv1(axis)) + axis.dot(inv2(axis));
                    if k <= 0.0 {
                        continue;
                    }
                    impulse -= axis * (speed / k).min(cap);
                }
                if dynamic1 {
                    turns.push((h1, inv1(impulse)));
                }
                if dynamic2 {
                    turns.push((h2, -inv2(impulse)));
                }
            }
        }
        for (handle, turn) in turns {
            let b = &mut engine.bodies[handle];
            let w = point(b.angvel()) + turn;
            b.set_angvel(vector(w), false);
        }
    }

    /// The pairs touching now, and the events of those that started or stopped.
    fn update_touching(&mut self) {
        let engine = &self.engine;
        let mut now = BTreeSet::new();
        for pair in engine.narrow_phase.contact_pairs() {
            let (Some(c1), Some(c2)) = (
                engine.colliders.get(pair.collider1),
                engine.colliders.get(pair.collider2),
            ) else {
                continue;
            };
            let (a, b) = (c1.user_data as u32, c2.user_data as u32);
            let key = (a.min(b), a.max(b));
            let awake = |c: &rp::Collider| {
                c.parent()
                    .and_then(|h| engine.bodies.get(h))
                    .is_some_and(|b| !b.is_fixed() && !b.is_sleeping())
            };
            let touching = if awake(c1) || awake(c2) {
                // Rapier keeps the contacts of a pair while they are close, and the
                // distances it stores date from when it last computed them: the
                // distance now is between the anchors of each contact on both bodies.
                pair.manifolds().iter().any(|m| {
                    m.data.solver_contacts.iter().any(|c| {
                        let (p1, p2) = m.data.solver_contact_world_points(c, &engine.bodies);
                        (p2 - p1).dot(m.data.normal) <= TOUCH
                    })
                })
            } else {
                // Asleep, they stay as they were.
                self.touching.contains(&key)
            };
            if touching {
                now.insert(key);
            }
        }
        let mut events: Vec<CollisionEvent> = now
            .difference(&self.touching)
            .map(|&(a, b)| CollisionEvent::Started(self.id_of(a), self.id_of(b)))
            .chain(
                self.touching
                    .difference(&now)
                    .map(|&(a, b)| CollisionEvent::Ended(self.id_of(a), self.id_of(b))),
            )
            .collect();
        events.sort_by_key(|e| match e {
            CollisionEvent::Started(a, b) | CollisionEvent::Ended(a, b) => (*a, *b),
        });
        self.events.extend(events);
        self.touching = now;
    }

    /// The bodies as Rapier left them; the forces of the step are spent.
    fn read_back(&mut self) {
        for entry in self.slots.iter_mut().filter_map(|s| s.entry.as_mut()) {
            let rb = &self.engine.bodies[entry.handle];
            let b = &mut entry.body;
            b.position = point(rb.translation());
            b.rotation = rotation(*rb.rotation());
            b.linear_velocity = point(rb.linvel());
            b.angular_velocity = point(rb.angvel());
            b.sleeping = rb.is_sleeping();
            b.force = Vec3::ZERO;
            b.torque = Vec3::ZERO;
            entry.synced = Synced::of(b);
        }
    }
}

/// A body far from anything during a step.
struct Free {
    handle: rp::RigidBodyHandle,
    /// Its uniform acceleration.
    accel: Vec3,
    /// Its rotation and angular velocity before the step, when it turns freely.
    spin: Option<(Quat, Vec3)>,
}

/// Whether a body turns the same way around every axis (a ball, a cube).
fn is_isotropic(i: Vec3) -> bool {
    let same = |x: f32, y: f32| (x - y).abs() <= 1e-6 * x.abs().max(y.abs());
    same(i.x, i.y) && same(i.y, i.z)
}

/// `h` seconds of free rotation of a body turned by `rotation`, its principal axes
/// turned by `frame` in it: a body whose inertia differs along its axes changes its
/// rotation axis by itself (a thrown book wobbles, a spin around its middle axis flips
/// over) while keeping its angular momentum and its energy. The motion is split into
/// turns around each of its principal axes, each exact, in the symmetric order x, y, z,
/// y, x (Dullweber, Leimkuhler and McLachlan, 1997): the angular momentum is kept
/// exactly, and the energy without drift.
fn free_rotation(rotation: Quat, frame: Quat, i: Vec3, w: Vec3, h: f32) -> (Quat, Vec3) {
    if i.x <= 0.0 || i.y <= 0.0 || i.z <= 0.0 {
        return (rotation, w);
    }
    let mut q = rotation * frame;
    // The angular momentum in the body's principal frame.
    let mut l = (q.inverse() * w).mul_vec(i);
    for (axis, inertia, part) in [
        (Vec3::X, i.x, 0.5),
        (Vec3::Y, i.y, 0.5),
        (Vec3::Z, i.z, 1.0),
        (Vec3::Y, i.y, 0.5),
        (Vec3::X, i.x, 0.5),
    ] {
        // Turning around its own axis at L_k / I_k, the body sees its angular momentum
        // turn the other way.
        let turn = Quat::from_axis_angle(axis, part * h * l.dot(axis) / inertia);
        q *= turn;
        l = turn.inverse() * l;
    }
    let q = q.normalize();
    (
        (q * frame.inverse()).normalize(),
        q * Vec3::new(l.x / i.x, l.y / i.y, l.z / i.z),
    )
}

/// `I⁻¹ v` of a Rapier body, in the world.
fn inverse_inertia(b: &rp::RigidBody, v: Vec3) -> Vec3 {
    let props = b.mass_properties().local_mprops;
    let frame = *b.rotation() * props.principal_inertia_local_frame;
    let local = frame.inverse() * vector(v);
    point(frame * (local * props.inv_principal_inertia))
}

fn vector(v: Vec3) -> rp::Vector {
    rp::Vector::new(v.x, v.y, v.z)
}

fn point(v: rp::Vector) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

fn rotation(q: rp::Rotation) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

fn pose(position: Vec3, rotation: Quat) -> rp::Pose {
    let q = rp::Rotation::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w);
    rp::Pose::from_parts(vector(position), q.normalize())
}

fn body_type(kind: BodyKind) -> rp::RigidBodyType {
    match kind {
        BodyKind::Dynamic => rp::RigidBodyType::Dynamic,
        BodyKind::Static => rp::RigidBodyType::Fixed,
        BodyKind::Kinematic => rp::RigidBodyType::KinematicVelocityBased,
    }
}

/// The exact mass, center of mass and inertia of a body, which its collider (of zero
/// density) does not change.
fn mass_properties(b: &Body) -> rp::MassProperties {
    let c = &b.collider;
    let q = c.rotation;
    rp::MassProperties::with_principal_inertia_frame(
        vector(c.center_of_mass()),
        b.mass,
        vector(b.inertia),
        rp::Rotation::from_xyzw(q.x, q.y, q.z, q.w).normalize(),
    )
}

fn collider_of(b: &Body, index: u32) -> rp::ColliderBuilder {
    rp::ColliderBuilder::new(shape_of(&b.collider.shape))
        .position(pose(b.collider.offset, b.collider.rotation))
        .density(0.0)
        .friction(b.friction.max(0.0))
        .restitution(b.restitution.clamp(0.0, 1.0))
        // The grip of two materials is the geometric mean of theirs; a bounce is the
        // bounciest of the two.
        .friction_combine_rule(rp::CoefficientCombineRule::GeometricMean)
        .restitution_combine_rule(rp::CoefficientCombineRule::Max)
        .user_data(index as u128)
}

/// The Rapier shape of a [`Shape`].
fn shape_of(shape: &Shape) -> rp::SharedShape {
    let tiny = 1e-4;
    match *shape {
        Shape::Sphere { radius } => rp::SharedShape::ball(radius.max(tiny)),
        Shape::Cuboid { half_extents: e } => {
            rp::SharedShape::cuboid(e.x.max(tiny), e.y.max(tiny), e.z.max(tiny))
        }
        Shape::Cylinder {
            radius,
            half_height,
        } => rp::SharedShape::cylinder(half_height.max(tiny), radius.max(tiny)),
        Shape::Cone {
            radius,
            half_height,
        } => rp::SharedShape::cone(half_height.max(tiny), radius.max(tiny)),
        Shape::Capsule {
            radius,
            half_height,
        } => rp::SharedShape::capsule_y(half_height.max(0.0), radius.max(tiny)),
        Shape::Frustum {
            radius_top,
            radius_bottom,
            half_height,
        } => {
            let h = half_height.max(tiny);
            let mut points = Vec::with_capacity(2 * FRUSTUM_SIDES);
            for i in 0..FRUSTUM_SIDES {
                let a = i as f32 / FRUSTUM_SIDES as f32 * std::f32::consts::TAU;
                let (s, c) = a.sin_cos();
                points.push(rp::Vector::new(radius_top * c, h, radius_top * s));
                points.push(rp::Vector::new(radius_bottom * c, -h, radius_bottom * s));
            }
            rp::SharedShape::convex_hull(&points).unwrap_or_else(|| {
                rp::SharedShape::cylinder(h, radius_top.max(radius_bottom).max(tiny))
            })
        }
        Shape::Torus {
            major_radius,
            minor_radius,
        } => {
            let tube = minor_radius.max(tiny);
            let at = |i: usize| {
                let a = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
                rp::Vector::new(major_radius * a.cos(), 0.0, major_radius * a.sin())
            };
            let parts = (0..RING_SEGMENTS)
                .map(|i| {
                    (
                        rp::Pose::IDENTITY,
                        rp::SharedShape::capsule(at(i), at(i + 1), tube),
                    )
                })
                .collect();
            rp::SharedShape::compound(parts)
        }
    }
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
