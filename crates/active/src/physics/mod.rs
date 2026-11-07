//! Rigid-body physics in 3D: bodies with a mass, forces and torques, gravity, collisions
//! between every shape of [`three`](crate::scene) and [`ecs`](crate::ecs) (balls, boxes,
//! cylinders, cones, capsules, rings), friction and bounce, and bodies that attract each
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
//! How it works. Each step is cut into substeps (4 per sixtieth of a second by default,
//! more for a longer step, so the motion does not depend on the frame rate). In each:
//!
//! 1. Forces and gravity change the velocities (semi-implicit Euler, the velocity form of
//!    Verlet integration). Attraction pulls half before the move and half after, as
//!    leapfrog does, which keeps orbits closed.
//! 2. The grid of the broad phase finds the bodies whose boxes overlap, the boxes
//!    stretched by how far each body can go in the substep.
//! 3. The narrow phase finds their contact points, also when they are still apart but
//!    may meet during the substep (speculative contacts: nothing goes through a thin
//!    wall): exact formulas for balls, boxes and capsules, and for the other shapes GJK and
//!    EPA on the shapes rounded by that margin (the normal is then well defined even where
//!    they barely touch); then the touching face of one shape clipped by the flat face of
//!    the other, along its exact normal, so that bodies rest flat.
//! 4. The solver applies impulses at the contacts (sequential impulses, starting from
//!    the previous ones): friction in a true Coulomb cone, rolling resistance, and
//!    contacts that are stiff, heavily damped springs (the soft step of Box2D v3), so
//!    that stacks stand still instead of trembling.
//! 5. The bodies move: along the exact parabola in free flight, and turning by exact
//!    rotations around their principal axes, which keeps the angular momentum of a
//!    tumbling body (it wobbles and flips as a real one does).
//! 6. A last pass removes the speed used to push overlaps apart, then the contacts that
//!    were hit fast enough bounce by exactly their restitution.
//!
//! Groups of touching bodies that stay still fall asleep together and cost nothing,
//! until something touches them or the code changes them. Units are SI: meters,
//! kilograms, seconds, newtons.

mod broad;
mod gravity;
mod narrow;
mod shape;

use std::collections::{BTreeMap, HashSet};

pub use gravity::Attraction;
pub use shape::{Collider, Shape};

use crate::scene::math::{Quat, Vec3};
use narrow::{ContactPoint, Contacts};
use shape::Placed;

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
    rest_time: f32,
    // Where the last step left it: a body moved or changed by hand since then wakes up,
    // and wakes what touches it.
    last_position: Vec3,
    last_rotation: Quat,
    changed: bool,
    // The uniform acceleration of this substep (gravity, forces), and whether a contact
    // pressed on it: in free flight, it moves along the exact parabola.
    accel: Vec3,
    pressed: bool,
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
            rest_time: 0.0,
            last_position: Vec3::ZERO,
            last_rotation: Quat::IDENTITY,
            changed: true,
            accel: Vec3::ZERO,
            pressed: false,
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
        self.changed = true;
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
        self.rest_time = 0.0;
    }

    fn is_awake_dynamic(&self) -> bool {
        self.kind == BodyKind::Dynamic && !self.sleeping
    }

    fn inverse_mass(&self) -> f32 {
        if self.is_awake_dynamic() {
            1.0 / self.mass
        } else {
            0.0
        }
    }

    fn inertia_frame(&self) -> Quat {
        self.rotation * self.collider.rotation
    }

    /// `I⁻¹ v` in the world (zero for bodies that do not turn).
    fn inverse_inertia(&self, v: Vec3) -> Vec3 {
        if !self.is_awake_dynamic() {
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

    fn placed(&self) -> Placed {
        Placed::new(&self.collider, self.position, self.rotation)
    }

    fn motion(&self) -> f32 {
        self.linear_velocity.length_squared() + self.angular_velocity.length_squared()
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

struct Slot {
    generation: u32,
    body: Option<Body>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Contact {
    local_a: Vec3,
    local_b: Vec3,
    normal: Vec3,
    depth: f32,
    normal_impulse: f32,
    tangent_impulse: [f32; 2],
    // Solver data, computed before each solve.
    ra: Vec3,
    rb: Vec3,
    tangents: [Vec3; 2],
    normal_mass: f32,
    tangent_mass: [f32; 2],
    /// The normal velocity before the solve: how fast they came together.
    approach: f32,
    /// The largest normal impulse of the solve: zero if they never actually met.
    max_impulse: f32,
}

struct Manifold {
    contacts: Vec<Contact>,
    friction: f32,
    restitution: f32,
    rolling: f32,
    /// The angular impulse resisting rolling (around the two tangents) and spinning
    /// (around the normal), kept from step to step like the contact impulses.
    roll_impulse: [f32; 3],
    roll_axes: [Vec3; 3],
    roll_mass: [f32; 3],
    /// The lever arm of the rolling resistance: from the contact to the center of the
    /// body that rolls.
    roll_arm: f32,
}

impl Manifold {
    fn new(contacts: Vec<Contact>, friction: f32, restitution: f32, rolling: f32) -> Manifold {
        Manifold {
            contacts,
            friction,
            restitution,
            rolling,
            roll_impulse: [0.0; 3],
            roll_axes: [Vec3::ZERO; 3],
            roll_mass: [0.0; 3],
            roll_arm: 0.0,
        }
    }
}

/// The bodies, and what moves them.
pub struct PhysicsWorld {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The acceleration of gravity (9.81 m/s² down by default).
    pub gravity: Vec3,
    /// Substeps per sixtieth of a second: more is more precise, and slower (4). A longer
    /// step is cut into more substeps, so that the motion does not depend on the frame
    /// rate.
    pub substeps: u32,
    /// Solver passes per substep: more makes stacks stiffer (6).
    pub iterations: u32,
    /// Whether bodies attract each other, and how much.
    pub attraction: Option<Attraction>,
    manifolds: BTreeMap<(u32, u32), Manifold>,
    touching: HashSet<(u32, u32)>,
    /// The pairs that touched during one of the substeps of this step: a bounce can begin
    /// and end within a step.
    touched: HashSet<(u32, u32)>,
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

/// Above this approach speed, contacts bounce (m/s): slower, a bounce would be too small
/// to see, and resting bodies would tremble.
const BOUNCE_THRESHOLD: f32 = 0.5;
/// Contacts are very stiff springs, heavily damped: the overlaps they push out, and the
/// rocking of a tall stack, fade away instead of ringing (the soft step of Box2D v3).
/// Their frequency (Hz), twice as high against a body that does not move, and their
/// damping ratio.
const CONTACT_HERTZ: f32 = 30.0;
const CONTACT_DAMPING: f32 = 10.0;
/// The fastest an overlap is pushed out (m/s).
const MAX_PUSH: f32 = 3.0;
/// Below these speeds (m/s, rad/s) for half a second, a group of touching bodies sleeps.
const SLEEP_SPEED: f32 = 0.02;
const SLEEP_SPIN: f32 = 0.05;
const SLEEP_TIME: f32 = 0.5;
/// The most substeps of one step: a longer step than that is less precise.
const MAX_SUBSTEPS: u32 = 1024;

impl PhysicsWorld {
    /// An empty world, with Earth's gravity.
    pub fn new() -> PhysicsWorld {
        PhysicsWorld {
            slots: Vec::new(),
            free: Vec::new(),
            gravity: Vec3::new(0.0, -9.81, 0.0),
            substeps: 4,
            iterations: 6,
            attraction: None,
            manifolds: BTreeMap::new(),
            touching: HashSet::new(),
            touched: HashSet::new(),
            events: Vec::new(),
            last_gravity: Vec3::new(0.0, -9.81, 0.0),
            last_attraction: None,
        }
    }

    /// Adds a body.
    pub fn add(&mut self, body: Body) -> BodyId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.body = Some(body);
            return BodyId {
                index,
                generation: slot.generation,
            };
        }
        self.slots.push(Slot {
            generation: 0,
            body: Some(body),
        });
        BodyId {
            index: (self.slots.len() - 1) as u32,
            generation: 0,
        }
    }

    /// Removes a body and gives it back.
    pub fn remove(&mut self, id: BodyId) -> Option<Body> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let body = slot.body.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        // What rested on it falls.
        let partners: Vec<u32> = self
            .manifolds
            .keys()
            .filter_map(|&(a, b)| {
                if a == id.index {
                    Some(b)
                } else if b == id.index {
                    Some(a)
                } else {
                    None
                }
            })
            .collect();
        for i in partners {
            if let Some(b) = self.slots[i as usize].body.as_mut() {
                b.wake_up();
            }
        }
        self.manifolds
            .retain(|&(a, b), _| a != id.index && b != id.index);
        self.touching
            .retain(|&(a, b)| a != id.index && b != id.index);
        Some(body)
    }

    /// A body.
    pub fn body(&self, id: BodyId) -> Option<&Body> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.body.as_ref()
        } else {
            None
        }
    }

    /// A body, to change.
    pub fn body_mut(&mut self, id: BodyId) -> Option<&mut Body> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.body.as_mut()
        } else {
            None
        }
    }

    /// How many bodies there are.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.body.is_some()).count()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bodies and their ids.
    pub fn bodies(&self) -> impl Iterator<Item = (BodyId, &Body)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.body.as_ref().map(|b| {
                (
                    BodyId {
                        index: i as u32,
                        generation: s.generation,
                    },
                    b,
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

    /// Moves the world forward by `dt` seconds. Any step works: it is cut into substeps
    /// of at most 1/240 s (with the default [`substeps`](Self::substeps)), so a long
    /// frame gives the same motion as several short ones.
    pub fn step(&mut self, dt: f32) {
        self.events.clear();
        if dt <= 0.0 || !dt.is_finite() {
            return;
        }
        self.wake_changed();
        let per_frame = self.substeps.max(1);
        // A little tolerance: 1/60 s in f32 is not exactly a sixtieth.
        let wanted = (dt * 60.0 * per_frame as f32 - 1e-3).ceil();
        let substeps = (wanted.max(0.0) as u32).clamp(per_frame, MAX_SUBSTEPS);
        let h = dt / substeps as f32;
        let relax = (self.iterations / 2).max(1);
        // Attraction, which changes with the positions, is applied as leapfrog does: half
        // its pull before the move, half after, from the new positions (second order, and
        // orbits stay closed).
        let mut pull = self.attraction_accelerations();
        for _ in 0..substeps {
            self.integrate_velocities(h, &pull);
            self.detect(h);
            let mut manifolds = std::mem::take(&mut self.manifolds);
            let mut motions = self.motions();
            Self::prepare(&mut manifolds, &motions);
            Self::warm_start(&manifolds, &mut motions);
            for _ in 0..self.iterations.max(1) {
                Self::solve(&mut manifolds, &mut motions, h, true);
            }
            self.store(&motions);
            self.integrate_positions(h);
            self.refresh(&mut motions);
            // The speed used to push bodies apart is not kept: it would make them jump.
            for _ in 0..relax {
                Self::solve(&mut manifolds, &mut motions, h, false);
            }
            // Bounces last, once the contacts hold: only the contacts that were really
            // reached bounce, and by exactly their restitution.
            for _ in 0..relax {
                Self::bounce(&mut manifolds, &mut motions);
            }
            self.store(&motions);
            self.manifolds = manifolds;
            if self.attraction.is_some() {
                pull = self.attraction_accelerations();
                self.kick(&pull, 0.5 * h);
            }
        }
        self.sleep(dt);
        self.update_touching();
        self.last_gravity = self.gravity;
        self.last_attraction = self.attraction;
        for slot in &mut self.slots {
            if let Some(b) = slot.body.as_mut() {
                b.force = Vec3::ZERO;
                b.torque = Vec3::ZERO;
                b.last_position = b.position;
                b.last_rotation = b.rotation;
            }
        }
    }

    /// Wakes the sleeping bodies that were changed by hand since the last step (a
    /// velocity, a force, a new place), those touching a body moved or changed by hand,
    /// and every body when gravity changed.
    fn wake_changed(&mut self) {
        let everything =
            self.gravity != self.last_gravity || self.attraction != self.last_attraction;
        let mut moved = HashSet::new();
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(b) = slot.body.as_mut() else {
                continue;
            };
            let by_hand =
                b.changed || b.position != b.last_position || b.rotation != b.last_rotation;
            b.changed = false;
            if by_hand {
                moved.insert(i as u32);
            }
            let pushed = b.linear_velocity != Vec3::ZERO
                || b.angular_velocity != Vec3::ZERO
                || b.force != Vec3::ZERO
                || b.torque != Vec3::ZERO;
            if b.sleeping && (everything || by_hand || pushed) {
                b.wake_up();
            }
        }
        if moved.is_empty() {
            return;
        }
        let partners: Vec<u32> = self
            .manifolds
            .keys()
            .flat_map(|&(a, b)| {
                [
                    moved.contains(&a).then_some(b),
                    moved.contains(&b).then_some(a),
                ]
            })
            .flatten()
            .collect();
        for i in partners {
            if let Some(b) = self.slots[i as usize].body.as_mut() {
                b.wake_up();
            }
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
            match &slot.body {
                Some(b) if b.kind == BodyKind::Dynamic => {
                    positions.push(b.center_of_mass());
                    masses.push(b.mass);
                }
                _ => {
                    positions.push(Vec3::ZERO);
                    masses.push(0.0);
                }
            }
        }
        gravity::accelerations(&positions, &masses, &cfg)
    }

    fn integrate_velocities(&mut self, h: f32, pull: &[Vec3]) {
        let gravity = self.gravity;
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(b) = slot.body.as_mut() else {
                continue;
            };
            if !b.is_awake_dynamic() {
                continue;
            }
            let uniform = gravity * b.gravity_scale + b.force / b.mass;
            b.accel = uniform;
            b.pressed = false;
            let mut a = uniform;
            if let Some(p) = pull.get(i) {
                a += *p * 0.5;
            }
            b.linear_velocity += a * h;
            let torque = b.torque;
            b.angular_velocity += b.inverse_inertia(torque) * h;
            if b.linear_damping > 0.0 {
                b.linear_velocity *= 1.0 / (1.0 + h * b.linear_damping);
            }
            if b.angular_damping > 0.0 {
                b.angular_velocity *= 1.0 / (1.0 + h * b.angular_damping);
            }
            // Limits that keep a bad frame from blowing everything up.
            let speed = b.linear_velocity.length();
            if speed > 500.0 {
                b.linear_velocity *= 500.0 / speed;
            }
            let spin = b.angular_velocity.length();
            if spin > 100.0 {
                b.angular_velocity *= 100.0 / spin;
            }
        }
    }

    /// The second half of the pull of attraction.
    fn kick(&mut self, pull: &[Vec3], dt: f32) {
        for (slot, p) in self.slots.iter_mut().zip(pull) {
            if let Some(b) = slot.body.as_mut() {
                if b.is_awake_dynamic() {
                    b.linear_velocity += *p * dt;
                }
            }
        }
    }

    fn integrate_positions(&mut self, h: f32) {
        for slot in &mut self.slots {
            let Some(b) = slot.body.as_mut() else {
                continue;
            };
            let moves = match b.kind {
                BodyKind::Dynamic => !b.sleeping,
                BodyKind::Kinematic => true,
                BodyKind::Static => false,
            };
            if !moves {
                continue;
            }
            // Under a uniform acceleration and nothing else, x + v h + a h² / 2 is exact
            // (the velocity already has its a h): free flights keep their energy, and
            // a thrown ball reaches its height. Attraction, which changes with the
            // position, keeps the plain step, which keeps orbits closed.
            let velocity = if b.kind == BodyKind::Dynamic && !b.pressed {
                b.linear_velocity - b.accel * (0.5 * h)
            } else {
                b.linear_velocity
            };
            let com = b.center_of_mass() + velocity * h;
            let w = b.angular_velocity;
            let spin = w.length();
            if b.kind == BodyKind::Dynamic && !is_isotropic(b.inertia) && spin > 0.0 {
                free_rotation(b, h);
            } else if spin > 0.0 {
                // The exact turn of angle |ω| h around ω, about the center of mass.
                b.rotation = (Quat::from_axis_angle(w / spin, spin * h) * b.rotation).normalize();
            }
            b.position = com - b.rotation * b.collider.center_of_mass();
        }
    }

    /// Finds the contacts of this substep, keeping the impulses of the contacts that were
    /// already there.
    fn detect(&mut self, h: f32) {
        let mut placed: Vec<Option<(Placed, shape::Aabb, f32)>> =
            Vec::with_capacity(self.slots.len());
        let mut indices = Vec::new();
        let mut boxes = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            let entry = slot.body.as_ref().map(|b| {
                let p = b.placed();
                // As far as any of its points can go in a substep.
                let reach = b.collider.shape.bounding_radius() + b.collider.offset.length();
                let sweep = if b.kind == BodyKind::Static || b.sleeping {
                    0.0
                } else {
                    (b.linear_velocity.length() + b.angular_velocity.length() * reach) * h
                };
                (p, p.aabb(narrow::MARGIN + sweep), sweep)
            });
            if let Some((_, aabb, _)) = &entry {
                indices.push(i as u32);
                boxes.push(*aabb);
            }
            placed.push(entry);
        }
        let mut seen = HashSet::new();
        for (x, y) in broad::overlapping_pairs(&boxes) {
            let (i, j) = (indices[x], indices[y]);
            let (Some(a), Some(b)) = (
                self.slots[i as usize].body.as_ref(),
                self.slots[j as usize].body.as_ref(),
            ) else {
                continue;
            };
            let active = |s: &Body, o: &Body| {
                s.is_awake_dynamic()
                    || (s.kind == BodyKind::Kinematic
                        && s.motion() > 0.0
                        && o.kind == BodyKind::Dynamic)
            };
            if !(active(a, b) || active(b, a))
                || (a.kind != BodyKind::Dynamic && b.kind != BodyKind::Dynamic)
            {
                continue;
            }
            let (Some((pa, _, sweep_a)), Some((pb, _, sweep_b))) =
                (&placed[i as usize], &placed[j as usize])
            else {
                continue;
            };
            let found = narrow::collide(pa, pb, narrow::MARGIN + sweep_a + sweep_b);
            if matches!(found, Contacts::None) {
                continue;
            }
            // A body awake, however slow, wakes a sleeping one it touches: a sleeping body
            // is not a wall.
            let wake = |s: &Body| {
                s.is_awake_dynamic() || (s.kind == BodyKind::Kinematic && s.motion() > 0.0)
            };
            let (wake_b, wake_a) = (wake(a) && b.sleeping, wake(b) && a.sleeping);
            let (pos_a, rot_a, pos_b, rot_b) = (a.position, a.rotation, b.position, b.rotation);
            let (friction, restitution, rolling) = (
                (a.friction.max(0.0) * b.friction.max(0.0)).sqrt(),
                a.restitution.max(b.restitution).clamp(0.0, 1.0),
                a.rolling_friction.max(b.rolling_friction).max(0.0),
            );
            if wake_a {
                if let Some(a) = self.slots[i as usize].body.as_mut() {
                    a.wake_up();
                }
            }
            if wake_b {
                if let Some(b) = self.slots[j as usize].body.as_mut() {
                    b.wake_up();
                }
            }
            let to_local = |p: Vec3, pos: Vec3, rot: Quat| rot.inverse() * (p - pos);
            let new_contact = |c: &ContactPoint| Contact {
                local_a: to_local(c.point_a, pos_a, rot_a),
                local_b: to_local(c.point_b, pos_b, rot_b),
                normal: c.normal,
                depth: c.depth,
                ..Contact::default()
            };
            let (previous, roll_impulse) = self
                .manifolds
                .remove(&(i, j))
                .map(|m| (m.contacts, m.roll_impulse))
                .unwrap_or_default();
            let contacts = match found {
                Contacts::None => continue,
                Contacts::Full(points) => points
                    .iter()
                    .map(|p| {
                        let mut c = new_contact(p);
                        // The same point as before keeps its impulses.
                        if let Some(old) = previous
                            .iter()
                            .find(|o| (o.local_a - c.local_a).length_squared() < 0.0025)
                        {
                            c.normal_impulse = old.normal_impulse;
                            c.tangent_impulse = old.tangent_impulse;
                        }
                        c
                    })
                    .collect(),
                Contacts::Single(point) => {
                    // Keep the earlier points that still hold, to stand on a face.
                    let n = point.normal;
                    let mut kept: Vec<Contact> = previous
                        .into_iter()
                        .filter_map(|mut c| {
                            let wa = pos_a + rot_a * c.local_a;
                            let wb = pos_b + rot_b * c.local_b;
                            let gap = wb - wa;
                            let separation = gap.dot(n);
                            let drift = (gap - n * separation).length();
                            (separation < narrow::MARGIN && drift < 0.05).then(|| {
                                c.normal = n;
                                c.depth = -separation;
                                c
                            })
                        })
                        .collect();
                    let fresh = new_contact(&point);
                    match kept
                        .iter_mut()
                        .find(|c| (c.local_a - fresh.local_a).length_squared() < 0.0004)
                    {
                        Some(c) => {
                            c.local_a = fresh.local_a;
                            c.local_b = fresh.local_b;
                            c.depth = fresh.depth;
                        }
                        None => kept.push(fresh),
                    }
                    if kept.len() > 4 {
                        let mut points: Vec<ContactPoint> = kept
                            .iter()
                            .map(|c| ContactPoint {
                                point_a: pos_a + rot_a * c.local_a,
                                point_b: pos_b + rot_b * c.local_b,
                                normal: c.normal,
                                depth: c.depth,
                            })
                            .collect();
                        narrow::reduce(&mut points);
                        kept.retain(|c| {
                            let wa = pos_a + rot_a * c.local_a;
                            points
                                .iter()
                                .any(|p| (p.point_a - wa).length_squared() < 1e-10)
                        });
                    }
                    kept
                }
            };
            seen.insert((i, j));
            if contacts.iter().any(|c| c.depth > -narrow::MARGIN * 0.5) {
                self.touched.insert((i, j));
            }
            // Touching, or about to: not in free flight.
            if contacts.iter().any(|c| c.depth > -narrow::MARGIN) {
                for k in [i, j] {
                    if let Some(body) = self.slots[k as usize].body.as_mut() {
                        body.pressed = true;
                    }
                }
            }
            let mut manifold = Manifold::new(contacts, friction, restitution, rolling);
            manifold.roll_impulse = roll_impulse;
            self.manifolds.insert((i, j), manifold);
        }
        // Pairs no longer near: their contacts go. Pairs of sleeping bodies keep theirs.
        let slots = &self.slots;
        self.manifolds.retain(|&(a, b), _| {
            seen.contains(&(a, b)) || {
                let asleep = |i: u32| {
                    slots[i as usize]
                        .body
                        .as_ref()
                        .is_none_or(|b| !b.is_awake_dynamic())
                };
                asleep(a) && asleep(b)
            }
        });
    }

    /// The motions of the bodies, for the solver (same indices as the slots).
    fn motions(&self) -> Vec<Motion> {
        self.slots
            .iter()
            .map(|s| s.body.as_ref().map_or(Motion::NONE, Motion::of))
            .collect()
    }

    /// Writes the solver's velocities back into the bodies.
    fn store(&mut self, motions: &[Motion]) {
        for (slot, m) in self.slots.iter_mut().zip(motions) {
            if let Some(b) = slot.body.as_mut() {
                if b.is_awake_dynamic() {
                    b.linear_velocity = m.v;
                    b.angular_velocity = m.w;
                    b.pressed |= m.pressed;
                }
            }
        }
    }

    /// Reads the bodies' new places (and turns: a free rotation changes ω) after they moved.
    fn refresh(&self, motions: &mut [Motion]) {
        for (slot, m) in self.slots.iter().zip(motions.iter_mut()) {
            if let Some(b) = slot.body.as_ref() {
                m.position = b.position;
                m.rotation = b.rotation;
                m.com = b.center_of_mass();
                m.v = b.linear_velocity;
                m.w = b.angular_velocity;
            }
        }
    }

    fn prepare(manifolds: &mut BTreeMap<(u32, u32), Manifold>, motions: &[Motion]) {
        for (&(i, j), m) in manifolds.iter_mut() {
            let (a, b) = (&motions[i as usize], &motions[j as usize]);
            let (mut arm_a, mut arm_b) = (0.0, 0.0);
            for c in &mut m.contacts {
                let wa = a.position + a.rotation * c.local_a;
                let wb = b.position + b.rotation * c.local_b;
                let point = (wa + wb) * 0.5;
                c.ra = point - a.com;
                c.rb = point - b.com;
                let n = c.normal;
                // The rolling radius: from the center down to the contact plane.
                arm_a += c.ra.dot(n).abs();
                arm_b += c.rb.dot(n).abs();
                let k = |dir: Vec3| {
                    let ka = a.turn(c.ra.cross(dir)).cross(c.ra);
                    let kb = b.turn(c.rb.cross(dir)).cross(c.rb);
                    let k = a.inv_mass + b.inv_mass + dir.dot(ka + kb);
                    if k > 0.0 { 1.0 / k } else { 0.0 }
                };
                c.normal_mass = k(n);
                let [t1, t2] = tangents(n);
                c.tangents = [t1, t2];
                c.tangent_mass = [k(t1), k(t2)];
                c.approach = relative_velocity(a, b, c.ra, c.rb).dot(n);
                c.max_impulse = 0.0;
            }
            if let Some(first) = m.contacts.first() {
                let count = m.contacts.len() as f32;
                // The body that rolls is the dynamic one, or the smaller of two.
                m.roll_arm = match (a.dynamic, b.dynamic) {
                    (true, true) => arm_a.min(arm_b) / count,
                    (true, false) => arm_a / count,
                    _ => arm_b / count,
                };
                let n = first.normal;
                let [t1, t2] = tangents(n);
                m.roll_axes = [t1, t2, n];
                for (axis, mass) in m.roll_axes.iter().zip(m.roll_mass.iter_mut()) {
                    let k = axis.dot(a.turn(*axis) + b.turn(*axis));
                    *mass = if k > 0.0 { 1.0 / k } else { 0.0 };
                }
            }
        }
    }

    fn warm_start(manifolds: &BTreeMap<(u32, u32), Manifold>, motions: &mut [Motion]) {
        for (&(i, j), m) in manifolds {
            let (mut a, mut b) = (motions[i as usize], motions[j as usize]);
            for c in &m.contacts {
                let p = c.normal * c.normal_impulse
                    + c.tangents[0] * c.tangent_impulse[0]
                    + c.tangents[1] * c.tangent_impulse[1];
                apply(&mut a, &mut b, c.ra, c.rb, p);
            }
            let turn = m.roll_axes[0] * m.roll_impulse[0]
                + m.roll_axes[1] * m.roll_impulse[1]
                + m.roll_axes[2] * m.roll_impulse[2];
            apply_angular(&mut a, &mut b, turn);
            motions[i as usize] = a;
            motions[j as usize] = b;
        }
    }

    fn solve(
        manifolds: &mut BTreeMap<(u32, u32), Manifold>,
        motions: &mut [Motion],
        h: f32,
        use_bias: bool,
    ) {
        let (soft_pair, soft_fixed) = (
            Softness::new(CONTACT_HERTZ, CONTACT_DAMPING, h),
            Softness::new(2.0 * CONTACT_HERTZ, CONTACT_DAMPING, h),
        );
        for (&(i, j), m) in manifolds.iter_mut() {
            let (mut a, mut b) = (motions[i as usize], motions[j as usize]);
            let soft = if !use_bias {
                Softness::RIGID
            } else if a.inv_mass == 0.0 || b.inv_mass == 0.0 {
                soft_fixed
            } else {
                soft_pair
            };
            // Rolling and spinning resistance: a torque of at most the coefficient times
            // the normal force times the lever arm.
            if m.rolling > 0.0 && !m.contacts.is_empty() {
                let pressed: f32 = m.contacts.iter().map(|c| c.normal_impulse).sum();
                let limit = m.rolling * m.roll_arm * pressed;
                let dw = b.w - a.w;
                let old = m.roll_impulse;
                let mut new = [0.0; 3];
                for k in 0..3 {
                    new[k] = old[k] - m.roll_mass[k] * dw.dot(m.roll_axes[k]);
                }
                let rolling = (new[0] * new[0] + new[1] * new[1]).sqrt();
                if rolling > limit {
                    let s = limit / rolling;
                    new[0] *= s;
                    new[1] *= s;
                }
                new[2] = new[2].clamp(-limit, limit);
                m.roll_impulse = new;
                let turn = m.roll_axes[0] * (new[0] - old[0])
                    + m.roll_axes[1] * (new[1] - old[1])
                    + m.roll_axes[2] * (new[2] - old[2]);
                apply_angular(&mut a, &mut b, turn);
            }
            for c in &mut m.contacts {
                // Friction first, bounded by the normal impulse in every direction alike
                // (Coulomb's cone): the contact itself is solved last, to hold best.
                let limit = m.friction * c.normal_impulse;
                let dv = relative_velocity(&a, &b, c.ra, c.rb);
                let old = c.tangent_impulse;
                let mut new = [
                    old[0] - c.tangent_mass[0] * dv.dot(c.tangents[0]),
                    old[1] - c.tangent_mass[1] * dv.dot(c.tangents[1]),
                ];
                let length = (new[0] * new[0] + new[1] * new[1]).sqrt();
                if length > limit {
                    let s = if length > 0.0 { limit / length } else { 0.0 };
                    new[0] *= s;
                    new[1] *= s;
                }
                c.tangent_impulse = new;
                let p = c.tangents[0] * (new[0] - old[0]) + c.tangents[1] * (new[1] - old[1]);
                apply(&mut a, &mut b, c.ra, c.rb, p);

                let n = c.normal;
                // The gap now, the bodies having moved since the contact was found.
                let wa = a.position + a.rotation * c.local_a;
                let wb = b.position + b.rotation * c.local_b;
                let separation = (wb - wa).dot(n);
                // Apart (a speculative contact): they may come closer until they touch.
                // Overlapping: pushed apart by the spring.
                let (target, soft) = if separation > 0.0 {
                    (-separation / h, Softness::RIGID)
                } else {
                    ((-soft.bias_rate * separation).min(MAX_PUSH), soft)
                };
                let vn = relative_velocity(&a, &b, c.ra, c.rb).dot(n);
                let lambda = -c.normal_mass * soft.mass_scale * (vn - target)
                    - soft.impulse_scale * c.normal_impulse;
                let total = (c.normal_impulse + lambda).max(0.0);
                let applied = total - c.normal_impulse;
                c.normal_impulse = total;
                c.max_impulse = c.max_impulse.max(total);
                apply(&mut a, &mut b, c.ra, c.rb, n * applied);
                if total > 0.0 {
                    a.pressed = true;
                    b.pressed = true;
                }
            }
            motions[i as usize] = a;
            motions[j as usize] = b;
        }
    }

    /// Restitution: the contacts that met fast enough leave as fast as they came, times
    /// the restitution.
    fn bounce(manifolds: &mut BTreeMap<(u32, u32), Manifold>, motions: &mut [Motion]) {
        for (&(i, j), m) in manifolds.iter_mut() {
            if m.restitution <= 0.0
                || !m
                    .contacts
                    .iter()
                    .any(|c| c.approach < -BOUNCE_THRESHOLD && c.max_impulse > 0.0)
            {
                continue;
            }
            let (mut a, mut b) = (motions[i as usize], motions[j as usize]);
            for c in &mut m.contacts {
                if c.approach >= -BOUNCE_THRESHOLD || c.max_impulse <= 0.0 {
                    continue;
                }
                let vn = relative_velocity(&a, &b, c.ra, c.rb).dot(c.normal);
                let lambda = -c.normal_mass * (vn + m.restitution * c.approach);
                let total = (c.normal_impulse + lambda).max(0.0);
                let applied = total - c.normal_impulse;
                c.normal_impulse = total;
                c.max_impulse = c.max_impulse.max(total);
                apply(&mut a, &mut b, c.ra, c.rb, c.normal * applied);
            }
            motions[i as usize] = a;
            motions[j as usize] = b;
        }
    }

    /// Groups of touching bodies that stay still together fall asleep together: a body
    /// alone could not, since what it rests on may still move.
    fn sleep(&mut self, dt: f32) {
        let n = self.slots.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn root(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        // Groups that something moving carries cannot sleep.
        let mut carried = vec![false; n];
        for (&(i, j), m) in &self.manifolds {
            if !m.contacts.iter().any(|c| c.depth > -narrow::MARGIN) {
                continue;
            }
            let (Some(a), Some(b)) = (
                self.slots[i as usize].body.as_ref(),
                self.slots[j as usize].body.as_ref(),
            ) else {
                continue;
            };
            let (i, j) = (i as usize, j as usize);
            match (a.kind, b.kind) {
                (BodyKind::Dynamic, BodyKind::Dynamic) => {
                    let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                    parent[ri] = rj;
                }
                (BodyKind::Dynamic, BodyKind::Kinematic) if b.motion() > 0.0 => carried[i] = true,
                (BodyKind::Kinematic, BodyKind::Dynamic) if a.motion() > 0.0 => carried[j] = true,
                _ => {}
            }
        }
        let mut restless = vec![false; n];
        for (i, slot) in self.slots.iter_mut().enumerate() {
            let Some(b) = slot.body.as_mut() else {
                continue;
            };
            if !b.is_awake_dynamic() {
                continue;
            }
            if b.linear_velocity.length() < SLEEP_SPEED && b.angular_velocity.length() < SLEEP_SPIN
            {
                b.rest_time += dt;
            } else {
                b.rest_time = 0.0;
            }
            if b.rest_time < SLEEP_TIME || carried[i] {
                let r = root(&mut parent, i);
                restless[r] = true;
            }
        }
        for i in 0..n {
            let r = root(&mut parent, i);
            let Some(b) = self.slots[i].body.as_mut() else {
                continue;
            };
            if b.is_awake_dynamic() && !restless[r] {
                b.sleeping = true;
                b.linear_velocity = Vec3::ZERO;
                b.angular_velocity = Vec3::ZERO;
            }
        }
    }

    fn update_touching(&mut self) {
        // Touching during the step, or still resting (sleeping pairs keep their contacts).
        let mut now = std::mem::take(&mut self.touched);
        now.extend(
            self.manifolds
                .iter()
                .filter(|(_, m)| m.contacts.iter().any(|c| c.depth > -narrow::MARGIN * 0.5))
                .map(|(&k, _)| k),
        );
        let mut events: Vec<CollisionEvent> = now
            .difference(&self.touching)
            .map(|&(a, b)| CollisionEvent::Started(self.id_of(a), self.id_of(b)))
            .chain(self.touching.difference(&now).filter_map(|&(a, b)| {
                // A removed body ends its contacts silently.
                let alive = |i: u32| self.slots[i as usize].body.is_some();
                (alive(a) && alive(b)).then(|| CollisionEvent::Ended(self.id_of(a), self.id_of(b)))
            }))
            .collect();
        events.sort_by_key(|e| match e {
            CollisionEvent::Started(a, b) | CollisionEvent::Ended(a, b) => (*a, *b),
        });
        self.events = events;
        self.touching = now;
    }
}

/// Whether a body turns the same way around every axis (a ball, a cube).
fn is_isotropic(i: Vec3) -> bool {
    let same = |x: f32, y: f32| (x - y).abs() <= 1e-6 * x.abs().max(y.abs());
    same(i.x, i.y) && same(i.y, i.z)
}

/// `h` seconds of free rotation: a body whose inertia differs along its axes changes its
/// rotation axis by itself (a thrown book wobbles, a spin around its middle axis flips
/// over) while keeping its angular momentum and its energy. The motion is split into
/// turns around each of its principal axes, each exact, in the symmetric order x, y, z,
/// y, x (Dullweber, Leimkuhler and McLachlan, 1997): the angular momentum is kept
/// exactly, and the energy without drift.
fn free_rotation(b: &mut Body, h: f32) {
    let i = b.inertia;
    if i.x <= 0.0 || i.y <= 0.0 || i.z <= 0.0 {
        return;
    }
    let mut q = b.inertia_frame();
    // The angular momentum in the body's frame.
    let mut l = (q.inverse() * b.angular_velocity).mul_vec(i);
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
    b.rotation = (q * b.collider.rotation.inverse()).normalize();
    b.angular_velocity = q * Vec3::new(l.x / i.x, l.y / i.y, l.z / i.z);
}

/// What the solver needs of a body, copied out for a substep: its velocities change, the
/// rest is read. Bodies that do not move have no inverse mass (sleeping ones too).
#[derive(Clone, Copy)]
struct Motion {
    v: Vec3,
    w: Vec3,
    inv_mass: f32,
    /// The inverse of its inertia tensor in the world, `R I⁻¹ Rᵀ`.
    inv_inertia: [[f32; 3]; 3],
    position: Vec3,
    rotation: Quat,
    com: Vec3,
    dynamic: bool,
    pressed: bool,
}

impl Motion {
    const NONE: Motion = Motion {
        v: Vec3::ZERO,
        w: Vec3::ZERO,
        inv_mass: 0.0,
        inv_inertia: [[0.0; 3]; 3],
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        com: Vec3::ZERO,
        dynamic: false,
        pressed: false,
    };

    fn of(b: &Body) -> Motion {
        let mut inv_inertia = [[0.0; 3]; 3];
        if b.is_awake_dynamic() {
            let q = b.inertia_frame();
            let axes = [q * Vec3::X, q * Vec3::Y, q * Vec3::Z];
            let inv = |i: f32| if i > 0.0 { 1.0 / i } else { 0.0 };
            let d = [inv(b.inertia.x), inv(b.inertia.y), inv(b.inertia.z)];
            for (r, row) in inv_inertia.iter_mut().enumerate() {
                for (c, cell) in row.iter_mut().enumerate() {
                    *cell = (0..3)
                        .map(|k| component(axes[k], r) * d[k] * component(axes[k], c))
                        .sum();
                }
            }
        }
        Motion {
            v: b.linear_velocity,
            w: b.angular_velocity,
            inv_mass: b.inverse_mass(),
            inv_inertia,
            position: b.position,
            rotation: b.rotation,
            com: b.center_of_mass(),
            dynamic: b.kind == BodyKind::Dynamic,
            pressed: false,
        }
    }

    /// `I⁻¹ l`: how an angular impulse `l` changes its rotation speed.
    fn turn(&self, l: Vec3) -> Vec3 {
        let m = &self.inv_inertia;
        Vec3::new(
            m[0][0] * l.x + m[0][1] * l.y + m[0][2] * l.z,
            m[1][0] * l.x + m[1][1] * l.y + m[1][2] * l.z,
            m[2][0] * l.x + m[2][1] * l.y + m[2][2] * l.z,
        )
    }
}

fn component(v: Vec3, i: usize) -> f32 {
    match i {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn relative_velocity(a: &Motion, b: &Motion, ra: Vec3, rb: Vec3) -> Vec3 {
    (b.v + b.w.cross(rb)) - (a.v + a.w.cross(ra))
}

/// A soft constraint: a spring of some frequency and damping, solved implicitly over a
/// substep (Erin Catto, "Solver2D", 2024).
#[derive(Clone, Copy)]
struct Softness {
    bias_rate: f32,
    mass_scale: f32,
    impulse_scale: f32,
}

impl Softness {
    const RIGID: Softness = Softness {
        bias_rate: 0.0,
        mass_scale: 1.0,
        impulse_scale: 0.0,
    };

    fn new(hertz: f32, damping: f32, h: f32) -> Softness {
        // No faster than a quarter of the substep rate, or it would not be stable.
        let omega = 2.0 * std::f32::consts::PI * hertz.min(0.25 / h);
        let a1 = 2.0 * damping + h * omega;
        let a2 = h * omega * a1;
        let a3 = 1.0 / (1.0 + a2);
        Softness {
            bias_rate: omega / a1,
            mass_scale: a2 * a3,
            impulse_scale: a3,
        }
    }
}

/// Two directions perpendicular to `n` and to each other.
fn tangents(n: Vec3) -> [Vec3; 2] {
    let t1 = if n.x.abs() < 0.57 {
        Vec3::X.cross(n)
    } else {
        Vec3::Y.cross(n)
    }
    .normalize();
    [t1, n.cross(t1)]
}

/// Turns `b` by the angular impulse `l` and `a` by `-l`.
fn apply_angular(a: &mut Motion, b: &mut Motion, l: Vec3) {
    a.w -= a.turn(l);
    b.w += b.turn(l);
}

/// Applies `p` to `b` and `-p` to `a`, at their contact arms.
fn apply(a: &mut Motion, b: &mut Motion, ra: Vec3, rb: Vec3, p: Vec3) {
    a.v -= p * a.inv_mass;
    a.w -= a.turn(ra.cross(p));
    b.v += p * b.inv_mass;
    b.w += b.turn(rb.cross(p));
}

#[cfg(test)]
mod laws;
#[cfg(test)]
mod tests;
