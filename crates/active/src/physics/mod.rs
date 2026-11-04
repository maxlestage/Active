//! Rigid-body physics in 3D: bodies with a mass, forces and torques, gravity, collisions
//! between every shape of [`three`](crate::three) and [`ecs`](crate::ecs) (balls, boxes,
//! cylinders, cones, capsules, rings), friction and bounce, and bodies that attract each
//! other.
//!
//! [`ecs`](crate::ecs) gives every mesh a body (its `PhysicsPlugin` is in `DefaultPlugins`),
//! [`three::Physics`](crate::three::Physics) does the same for a Three.js-style scene, and
//! [`PhysicsWorld`] can be used directly:
//!
//! ```
//! use active::physics::*;
//! use active::three::math::Vec3;
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
//! How it works, step by step (each split into substeps, 4 by default): forces and
//! gravity change the velocities; the grid of the broad phase finds the bodies whose
//! boxes overlap; the narrow phase finds their contact points (exact formulas for balls
//! and boxes, GJK and EPA for the other shapes); the solver applies impulses at the
//! contacts (sequential impulses, with friction, bounce, and the impulses of the
//! previous step as a start); the bodies move; a last pass removes the speed added to
//! push overlapping bodies apart. Bodies that stay still fall asleep and cost nothing,
//! until something touches them. Units are SI: meters, kilograms, seconds, newtons.

mod broad;
mod gravity;
mod narrow;
mod shape;

use std::collections::{HashMap, HashSet};

pub use gravity::Attraction;
pub use shape::{Collider, Shape};

use crate::three::math::{Quat, Vec3};
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
    /// How fast it slows down by itself (air), per second.
    pub linear_damping: f32,
    /// How fast its rotation slows down by itself, per second.
    pub angular_damping: f32,
    /// How much gravity pulls it (0: it floats, -1: it falls up).
    pub gravity_scale: f32,
    sleeping: bool,
    rest_time: f32,
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
            linear_damping: 0.05,
            angular_damping: 0.1,
            gravity_scale: 1.0,
            sleeping: false,
            rest_time: 0.0,
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
    bounce: f32,
}

struct Manifold {
    contacts: Vec<Contact>,
    friction: f32,
    restitution: f32,
}

/// The bodies, and what moves them.
pub struct PhysicsWorld {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The acceleration of gravity (9.81 m/s² down by default).
    pub gravity: Vec3,
    /// Substeps per step: more is steadier, and slower (4).
    pub substeps: u32,
    /// Solver passes per substep: more makes stacks stiffer (6).
    pub iterations: u32,
    /// Whether bodies attract each other, and how much.
    pub attraction: Option<Attraction>,
    manifolds: HashMap<(u32, u32), Manifold>,
    touching: HashSet<(u32, u32)>,
    /// The pairs that touched during one of the substeps of this step: a bounce can begin
    /// and end within a step.
    touched: HashSet<(u32, u32)>,
    events: Vec<CollisionEvent>,
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

/// Above this approach speed, contacts bounce (m/s).
const BOUNCE_THRESHOLD: f32 = 0.5;
/// How much of an overlap is corrected per substep, and the overlap left alone (m).
const BAUMGARTE: f32 = 0.2;
const SLOP: f32 = 0.005;
const MAX_PUSH: f32 = 3.0;
/// Below these speeds for half a second, a body sleeps.
const SLEEP_SPEED2: f32 = 0.01;
const SLEEP_TIME: f32 = 0.5;

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
            manifolds: HashMap::new(),
            touching: HashSet::new(),
            touched: HashSet::new(),
            events: Vec::new(),
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

    /// Moves the world forward by `dt` seconds (a fixed step, such as 1/60 s, keeps it
    /// steady).
    pub fn step(&mut self, dt: f32) {
        self.events.clear();
        if dt <= 0.0 || !dt.is_finite() {
            return;
        }
        let substeps = self.substeps.max(1);
        let h = dt / substeps as f32;
        let pull = self.attraction_accelerations();
        for _ in 0..substeps {
            self.integrate_velocities(h, &pull);
            self.detect(h);
            let mut manifolds = std::mem::take(&mut self.manifolds);
            self.prepare(&mut manifolds);
            self.warm_start(&manifolds);
            for _ in 0..self.iterations.max(1) {
                self.solve(&mut manifolds, h, true);
            }
            self.integrate_positions(h);
            // The speed used to push bodies apart is not kept: it would make them jump.
            for _ in 0..(self.iterations / 2).max(1) {
                self.solve(&mut manifolds, h, false);
            }
            self.manifolds = manifolds;
        }
        self.sleep(dt);
        self.update_touching();
        for slot in &mut self.slots {
            if let Some(b) = slot.body.as_mut() {
                b.force = Vec3::ZERO;
                b.torque = Vec3::ZERO;
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
            let mut a = gravity * b.gravity_scale + b.force / b.mass;
            if let Some(p) = pull.get(i) {
                a += *p;
            }
            b.linear_velocity += a * h;
            let torque = b.torque;
            b.angular_velocity += b.inverse_inertia(torque) * h;
            b.linear_velocity *= 1.0 / (1.0 + h * b.linear_damping);
            b.angular_velocity *= 1.0 / (1.0 + h * b.angular_damping);
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
            let com = b.center_of_mass() + b.linear_velocity * h;
            let w = b.angular_velocity;
            if w.length_squared() > 0.0 {
                // q' = q + h/2 · (0, ω) q
                let q = b.rotation;
                let dq = Quat::from_xyzw(w.x, w.y, w.z, 0.0) * q;
                b.rotation = Quat::from_xyzw(
                    q.x + 0.5 * h * dq.x,
                    q.y + 0.5 * h * dq.y,
                    q.z + 0.5 * h * dq.z,
                    q.w + 0.5 * h * dq.w,
                )
                .normalize();
            }
            b.position = com - b.rotation * b.collider.center_of_mass();
        }
    }

    /// Finds the contacts of this substep, keeping the impulses of the contacts that were
    /// already there.
    fn detect(&mut self, h: f32) {
        let mut placed: Vec<Option<(Placed, shape::Aabb)>> = Vec::with_capacity(self.slots.len());
        let mut indices = Vec::new();
        let mut boxes = Vec::new();
        for (i, slot) in self.slots.iter().enumerate() {
            let entry = slot.body.as_ref().map(|b| {
                let p = b.placed();
                let margin = narrow::MARGIN + b.linear_velocity.length() * h;
                (p, p.aabb(margin))
            });
            if let Some((_, aabb)) = &entry {
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
            let (Some((pa, _)), Some((pb, _))) = (&placed[i as usize], &placed[j as usize]) else {
                continue;
            };
            let found = narrow::collide(pa, pb);
            if matches!(found, Contacts::None) {
                continue;
            }
            // A moving body wakes a sleeping one it touches.
            let wake = |s: &Body| s.motion() > 4.0 * SLEEP_SPEED2 || s.kind == BodyKind::Kinematic;
            let (wake_b, wake_a) = (wake(a) && b.sleeping, wake(b) && a.sleeping);
            let (pos_a, rot_a, pos_b, rot_b) = (a.position, a.rotation, b.position, b.rotation);
            let (friction, restitution) = (
                (a.friction * b.friction).sqrt(),
                a.restitution.max(b.restitution),
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
            let previous = self
                .manifolds
                .remove(&(i, j))
                .map(|m| m.contacts)
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
            self.manifolds.insert(
                (i, j),
                Manifold {
                    contacts,
                    friction,
                    restitution,
                },
            );
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

    fn pair(&mut self, a: u32, b: u32) -> (Body, Body) {
        // Taken out and put back: the solver changes both bodies of a contact.
        let ba = self.slots[a as usize]
            .body
            .take()
            .expect("a body of a contact");
        let bb = self.slots[b as usize]
            .body
            .take()
            .expect("a body of a contact");
        (ba, bb)
    }

    fn put_back(&mut self, a: u32, b: u32, ba: Body, bb: Body) {
        self.slots[a as usize].body = Some(ba);
        self.slots[b as usize].body = Some(bb);
    }

    fn prepare(&mut self, manifolds: &mut HashMap<(u32, u32), Manifold>) {
        for (&(i, j), m) in manifolds.iter_mut() {
            if self.slots[i as usize].body.is_none() || self.slots[j as usize].body.is_none() {
                continue;
            }
            let (a, b) = self.pair(i, j);
            let (com_a, com_b) = (a.center_of_mass(), b.center_of_mass());
            let (inv_ma, inv_mb) = (a.inverse_mass(), b.inverse_mass());
            for c in &mut m.contacts {
                let wa = a.position + a.rotation * c.local_a;
                let wb = b.position + b.rotation * c.local_b;
                let point = (wa + wb) * 0.5;
                c.ra = point - com_a;
                c.rb = point - com_b;
                let n = c.normal;
                let k = |dir: Vec3| {
                    let ka = a.inverse_inertia(c.ra.cross(dir)).cross(c.ra);
                    let kb = b.inverse_inertia(c.rb.cross(dir)).cross(c.rb);
                    let k = inv_ma + inv_mb + dir.dot(ka + kb);
                    if k > 0.0 { 1.0 / k } else { 0.0 }
                };
                c.normal_mass = k(n);
                let t1 = if n.x.abs() < 0.57 {
                    Vec3::X.cross(n)
                } else {
                    Vec3::Y.cross(n)
                }
                .normalize();
                let t2 = n.cross(t1);
                c.tangents = [t1, t2];
                c.tangent_mass = [k(t1), k(t2)];
                let dv = relative_velocity(&a, &b, c.ra, c.rb);
                let vn = dv.dot(n);
                c.bounce = if vn < -BOUNCE_THRESHOLD {
                    -m.restitution * vn
                } else {
                    0.0
                };
            }
            self.put_back(i, j, a, b);
        }
    }

    fn warm_start(&mut self, manifolds: &HashMap<(u32, u32), Manifold>) {
        for (&(i, j), m) in manifolds {
            if self.slots[i as usize].body.is_none() || self.slots[j as usize].body.is_none() {
                continue;
            }
            let (mut a, mut b) = self.pair(i, j);
            for c in &m.contacts {
                let p = c.normal * c.normal_impulse
                    + c.tangents[0] * c.tangent_impulse[0]
                    + c.tangents[1] * c.tangent_impulse[1];
                apply(&mut a, &mut b, c.ra, c.rb, p);
            }
            self.put_back(i, j, a, b);
        }
    }

    fn solve(&mut self, manifolds: &mut HashMap<(u32, u32), Manifold>, h: f32, use_bias: bool) {
        for (&(i, j), m) in manifolds.iter_mut() {
            if self.slots[i as usize].body.is_none() || self.slots[j as usize].body.is_none() {
                continue;
            }
            let (mut a, mut b) = self.pair(i, j);
            for c in &mut m.contacts {
                let n = c.normal;
                // Separated (speculative contact): may approach until it touches.
                // Overlapping: pushed apart, a little per substep.
                let separation = -c.depth;
                let mut target = if separation > 0.0 {
                    -separation / h
                } else if use_bias {
                    (BAUMGARTE * (c.depth - SLOP).max(0.0) / h).min(MAX_PUSH)
                } else {
                    0.0
                };
                if c.bounce > 0.0 {
                    target = target.max(c.bounce);
                }
                let vn = relative_velocity(&a, &b, c.ra, c.rb).dot(n);
                let lambda = -c.normal_mass * (vn - target);
                let total = (c.normal_impulse + lambda).max(0.0);
                let applied = total - c.normal_impulse;
                c.normal_impulse = total;
                apply(&mut a, &mut b, c.ra, c.rb, n * applied);
                // Friction, bounded by the normal impulse (Coulomb).
                let limit = m.friction * c.normal_impulse;
                for k in 0..2 {
                    let t = c.tangents[k];
                    let vt = relative_velocity(&a, &b, c.ra, c.rb).dot(t);
                    let lambda = -c.tangent_mass[k] * vt;
                    let total = (c.tangent_impulse[k] + lambda).clamp(-limit, limit);
                    let applied = total - c.tangent_impulse[k];
                    c.tangent_impulse[k] = total;
                    apply(&mut a, &mut b, c.ra, c.rb, t * applied);
                }
            }
            self.put_back(i, j, a, b);
        }
    }

    fn sleep(&mut self, dt: f32) {
        for slot in &mut self.slots {
            let Some(b) = slot.body.as_mut() else {
                continue;
            };
            if !b.is_awake_dynamic() {
                continue;
            }
            if b.linear_velocity.length_squared() < SLEEP_SPEED2
                && b.angular_velocity.length_squared() < 4.0 * SLEEP_SPEED2
            {
                b.rest_time += dt;
                if b.rest_time > SLEEP_TIME {
                    b.sleeping = true;
                    b.linear_velocity = Vec3::ZERO;
                    b.angular_velocity = Vec3::ZERO;
                }
            } else {
                b.rest_time = 0.0;
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

fn relative_velocity(a: &Body, b: &Body, ra: Vec3, rb: Vec3) -> Vec3 {
    (b.linear_velocity + b.angular_velocity.cross(rb))
        - (a.linear_velocity + a.angular_velocity.cross(ra))
}

/// Applies `p` to `b` and `-p` to `a`, at their contact arms.
fn apply(a: &mut Body, b: &mut Body, ra: Vec3, rb: Vec3, p: Vec3) {
    let (inv_ma, inv_mb) = (a.inverse_mass(), b.inverse_mass());
    a.linear_velocity -= p * inv_ma;
    let da = a.inverse_inertia(ra.cross(p));
    a.angular_velocity -= da;
    b.linear_velocity += p * inv_mb;
    let db = b.inverse_inertia(rb.cross(p));
    b.angular_velocity += db;
}

#[cfg(test)]
mod tests;
