//! Physics for the entities, the way Avian and Rapier write it for Bevy: every entity with
//! a mesh (or a `Collider`) gets a body, with the mass of its volume, pulled by gravity,
//! pushed by its forces and colliding with the others.

use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

use super::app::{App, Plugin, Update};
use super::message::{Message, Messages};
use super::pbr::{Assets, Mesh, Mesh3d};
use super::query::Has;
use super::time::Time;
use super::transform::{ChildOf, GlobalTransform, Transform};
use super::world::{Component, Entity, Resource, World};
use crate::physics::{Attraction, Body, BodyId, BodyKind, Collider, CollisionEvent, PhysicsWorld};
use crate::three::math::{Mat4, Quat, Vec3};

/// How an entity moves. Without it: a mesh with a volume is `Dynamic`, a flat one
/// (`Plane3d`, `Circle`, `Rectangle`) is `Static` (ground), and a child of another entity is
/// `Kinematic` (it follows its parent and pushes what it meets).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RigidBody {
    /// Moved by forces, gravity and collisions.
    #[default]
    Dynamic,
    /// Never moves; infinitely heavy.
    Static,
    /// Moved by your code only (its `Transform`), pushing what it meets.
    Kinematic,
}

impl Component for RigidBody {}

impl From<RigidBody> for BodyKind {
    fn from(r: RigidBody) -> BodyKind {
        match r {
            RigidBody::Dynamic => BodyKind::Dynamic,
            RigidBody::Static => BodyKind::Static,
            RigidBody::Kinematic => BodyKind::Kinematic,
        }
    }
}

impl From<BodyKind> for RigidBody {
    fn from(k: BodyKind) -> RigidBody {
        match k {
            BodyKind::Dynamic => RigidBody::Dynamic,
            BodyKind::Static => RigidBody::Static,
            BodyKind::Kinematic => RigidBody::Kinematic,
        }
    }
}

/// The shape it collides as, when it is not its mesh's (or it has no mesh).
impl Component for Collider {}

macro_rules! scalar_component {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name(pub f32);

        impl Component for $name {}

        impl Deref for $name {
            type Target = f32;
            fn deref(&self) -> &f32 {
                &self.0
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut f32 {
                &mut self.0
            }
        }
    };
}

macro_rules! vector_component {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq)]
        pub struct $name(pub Vec3);

        impl Component for $name {}

        impl Deref for $name {
            type Target = Vec3;
            fn deref(&self) -> &Vec3 {
                &self.0
            }
        }

        impl DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Vec3 {
                &mut self.0
            }
        }
    };
}

scalar_component!(
    /// Its mass, in kg: computed from its volume and density (1 kg/m³ by default), and
    /// kept up to date when its scale or density changes; set it, or insert it, to choose
    /// a mass of your own, which then stays.
    Mass
);
scalar_component!(
    /// Its mass per volume, in kg/m³ (1 by default; 1000 is water): its mass follows.
    ColliderDensity
);
scalar_component!(
    /// How much it bounces, from 0 to 1 (0.2 by default).
    Restitution
);
scalar_component!(
    /// How much it grips (0.5 by default; 0: ice).
    Friction
);
scalar_component!(
    /// How much it resists rolling and spinning on what it touches: the rolling
    /// resistance coefficient (0.01 by default, a rubber ball on concrete; 0 rolls forever).
    RollingFriction
);
scalar_component!(
    /// How much gravity pulls it (1 by default; 0: it floats).
    GravityScale
);
scalar_component!(
    /// How fast it slows down by itself, per second, as in thick air or water (0 by
    /// default, as in a vacuum).
    LinearDamping
);
scalar_component!(
    /// How fast its rotation slows down by itself, per second (0 by default).
    AngularDamping
);
vector_component!(
    /// Its velocity, in m/s: read it, or set it to throw the entity.
    LinearVelocity
);
vector_component!(
    /// Its rotation speed, in rad/s, around the axis it points along.
    AngularVelocity
);
vector_component!(
    /// A kick, in N·s, applied once at the next step (then back to zero).
    ExternalImpulse
);

/// A force pushing it, in newtons: kept from step to step (a thruster), or applied once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExternalForce {
    /// The force.
    pub force: Vec3,
    /// Kept after each step (`true`), or applied once.
    pub persistent: bool,
}

impl ExternalForce {
    /// A force kept from step to step.
    pub fn new(force: Vec3) -> ExternalForce {
        ExternalForce {
            force,
            persistent: true,
        }
    }

    /// The same, kept (`true`) or applied once.
    pub fn with_persistence(mut self, persistent: bool) -> ExternalForce {
        self.persistent = persistent;
        self
    }
}

impl Component for ExternalForce {}

/// A torque turning it, in N·m: kept from step to step, or applied once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExternalTorque {
    /// The torque.
    pub torque: Vec3,
    /// Kept after each step (`true`), or applied once.
    pub persistent: bool,
}

impl ExternalTorque {
    /// A torque kept from step to step.
    pub fn new(torque: Vec3) -> ExternalTorque {
        ExternalTorque {
            torque,
            persistent: true,
        }
    }
}

impl Component for ExternalTorque {}

/// Marks a body that sleeps: still for a while, it costs nothing until something touches
/// it (read only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sleeping;

impl Component for Sleeping {}

/// The acceleration of gravity (9.81 m/s² down): a resource.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gravity(pub Vec3);

impl Default for Gravity {
    fn default() -> Gravity {
        Gravity(Vec3::new(0.0, -9.81, 0.0))
    }
}

impl Resource for Gravity {}

/// Bodies attract each other while this resource exists: `app.insert_resource(Attraction::new(1.0))`.
impl Resource for Attraction {}

/// How the physics runs: a resource.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicsSettings {
    /// Stopped (`true`) or running.
    pub paused: bool,
    /// Substeps per frame: more is steadier (4).
    pub substeps: u32,
    /// Solver passes per substep: more makes stacks stiffer (6).
    pub iterations: u32,
    /// The longest frame simulated at once: a slow frame slows the simulation down instead
    /// of letting objects go through each other (1/30 s).
    pub max_step: f32,
}

impl Default for PhysicsSettings {
    fn default() -> PhysicsSettings {
        PhysicsSettings {
            paused: false,
            substeps: 4,
            iterations: 6,
            max_step: 1.0 / 30.0,
        }
    }
}

impl Resource for PhysicsSettings {}

/// Two entities started touching: a message (`MessageReader<CollisionStarted>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionStarted(pub Entity, pub Entity);

/// Two entities stopped touching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionEnded(pub Entity, pub Entity);

impl Message for CollisionStarted {}
impl Message for CollisionEnded {}

struct Tracked {
    body: BodyId,
    /// The pose last read or written, in the frame the entity is placed in (its
    /// `Transform` for a root, the world for a child).
    pose: (Vec3, Quat),
    velocity: (Vec3, Vec3),
    mass: f32,
    /// Whether its mass was chosen by the code (a `Mass` inserted or changed): it then
    /// stays when the scale or the density changes.
    chosen_mass: bool,
    scale: Vec3,
    collider: Collider,
}

/// The bodies of the entities: a resource of the plugin.
#[derive(Default)]
pub struct PhysicsState {
    world: PhysicsWorld,
    tracked: HashMap<Entity, Tracked>,
}

impl Resource for PhysicsState {}

impl PhysicsState {
    /// The physics world (its bodies, contacts and events).
    pub fn world(&self) -> &PhysicsWorld {
        &self.world
    }

    /// The body of an entity.
    pub fn body(&self, entity: Entity) -> Option<&Body> {
        self.world.body(self.tracked.get(&entity)?.body)
    }

    /// The entities touching this one now.
    pub fn touching(&self, entity: Entity) -> Vec<Entity> {
        let Some(id) = self.tracked.get(&entity).map(|t| t.body) else {
            return Vec::new();
        };
        let of = |b: BodyId| {
            self.tracked
                .iter()
                .find(|(_, t)| t.body == b)
                .map(|(e, _)| *e)
        };
        self.world
            .contacts()
            .filter_map(|(a, b)| {
                if a == id {
                    of(b)
                } else if b == id {
                    of(a)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Gives every mesh a body and moves them, each frame, before the systems of `Update`
/// added after it. In `DefaultPlugins`.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gravity>()
            .init_resource::<PhysicsSettings>()
            .init_resource::<PhysicsState>()
            .add_message::<CollisionStarted>()
            .add_message::<CollisionEnded>()
            .add_systems(Update, step_physics);
    }
}

/// An entity that has a body, as found this frame.
struct Found {
    entity: Entity,
    collider: Collider,
    flat: bool,
    child: bool,
    /// Where it is, in the frame its `Transform` is in for a root, in the world for a child.
    pose: (Vec3, Quat),
    scale: Vec3,
}

fn find(world: &World) -> Vec<Found> {
    let meshes = world.get_resource::<Assets<Mesh>>();
    let query = world.query::<(
        Entity,
        Option<&Mesh3d>,
        Option<&Collider>,
        &Transform,
        &GlobalTransform,
        Has<ChildOf>,
    )>();
    let mut found = Vec::new();
    for (entity, mesh, collider, transform, global, child) in query.iter(world) {
        let (collider, flat) = match (collider, mesh) {
            (Some(c), _) => (*c, false),
            (None, Some(m)) => {
                let Some(asset) = meshes.as_ref().and_then(|a| a.get(m.0)) else {
                    continue;
                };
                match asset.collider() {
                    Some(c) => (*c, asset.is_flat()),
                    None => (crate::three::physics::bounding_box(asset.geometry()), false),
                }
            }
            (None, None) => continue,
        };
        let (pose, scale) = if child {
            let (t, r, s) = global.to_matrix().decompose();
            ((t, r), s)
        } else {
            ((transform.translation, transform.rotation), transform.scale)
        };
        found.push(Found {
            entity,
            collider,
            flat,
            child,
            pose,
            scale,
        });
    }
    found
}

fn component<T: Component + Copy>(world: &World, entity: Entity) -> Option<T> {
    world.get::<T>(entity).map(|c| *c)
}

/// Moves the bodies by one frame, and the entities with them.
pub fn step_physics(world: &mut World) {
    let settings = world
        .get_resource::<PhysicsSettings>()
        .map(|s| *s)
        .unwrap_or_default();
    let dt = world
        .get_resource::<Time>()
        .map_or(1.0 / 60.0, |t| t.delta_secs())
        .min(settings.max_step);
    if settings.paused || dt <= 0.0 {
        return;
    }
    let Some(mut state) = world.remove_resource::<PhysicsState>() else {
        return;
    };
    state.world.gravity = world
        .get_resource::<Gravity>()
        .map_or(Gravity::default().0, |g| g.0);
    state.world.substeps = settings.substeps;
    state.world.iterations = settings.iterations;
    state.world.attraction = world.get_resource::<Attraction>().map(|a| *a);

    let found = find(world);
    let mut present = HashSet::new();
    let mut to_insert: Vec<(Entity, RigidBody, f32, bool)> = Vec::new();
    let mut once: Vec<Entity> = Vec::new();
    for f in &found {
        present.insert(f.entity);
        let e = f.entity;
        let default_kind = if f.child {
            RigidBody::Kinematic
        } else if f.flat {
            RigidBody::Static
        } else {
            RigidBody::Dynamic
        };
        let kind: BodyKind = component::<RigidBody>(world, e)
            .unwrap_or(default_kind)
            .into();
        let tracked = match state.tracked.get_mut(&e) {
            Some(t) => t,
            None => {
                let mut body = Body::new(kind, f.collider.scaled(f.scale))
                    .at(f.pose.0)
                    .turned(f.pose.1);
                if let Some(d) = component::<ColliderDensity>(world, e) {
                    body.set_density(d.0);
                }
                if let Some(m) = component::<Mass>(world, e) {
                    body.set_mass(m.0);
                }
                let velocity = (
                    component::<LinearVelocity>(world, e).map_or(Vec3::ZERO, |v| v.0),
                    component::<AngularVelocity>(world, e).map_or(Vec3::ZERO, |v| v.0),
                );
                body.linear_velocity = velocity.0;
                body.angular_velocity = velocity.1;
                let mass = body.mass();
                let id = state.world.add(body);
                to_insert.push((e, kind.into(), mass, world.has::<RigidBody>(e)));
                state.tracked.insert(
                    e,
                    Tracked {
                        body: id,
                        pose: f.pose,
                        velocity,
                        mass,
                        chosen_mass: world.has::<Mass>(e),
                        scale: f.scale,
                        collider: f.collider,
                    },
                );
                state.tracked.get_mut(&e).expect("just inserted")
            }
        };
        let Some(body) = state.world.body_mut(tracked.body) else {
            continue;
        };
        body.kind = kind;
        // A `Mass` that differs from the last one written: the code chose it.
        let chosen = component::<Mass>(world, e)
            .map(|m| m.0)
            .filter(|&m| m != tracked.mass);
        if chosen.is_some() {
            tracked.chosen_mass = true;
        }
        if tracked.collider != f.collider
            || !crate::three::physics::same_scale(tracked.scale, f.scale)
        {
            // Same density, new volume: the mass follows.
            body.set_collider(f.collider.scaled(f.scale));
            tracked.collider = f.collider;
            tracked.scale = f.scale;
        }
        if let Some(d) = component::<ColliderDensity>(world, e) {
            if (body.density() - d.0).abs() > 1e-6 * d.0.abs().max(1.0) {
                body.set_density(d.0);
            }
        }
        if tracked.chosen_mass {
            let m = chosen.unwrap_or(tracked.mass);
            if body.mass() != m {
                body.set_mass(m);
            }
        }
        tracked.mass = body.mass();
        if let Some(r) = component::<Restitution>(world, e) {
            body.restitution = r.0;
        }
        if let Some(r) = component::<Friction>(world, e) {
            body.friction = r.0;
        }
        if let Some(r) = component::<RollingFriction>(world, e) {
            body.rolling_friction = r.0;
        }
        if let Some(r) = component::<GravityScale>(world, e) {
            body.gravity_scale = r.0;
        }
        if let Some(r) = component::<LinearDamping>(world, e) {
            body.linear_damping = r.0;
        }
        if let Some(r) = component::<AngularDamping>(world, e) {
            body.angular_damping = r.0;
        }
        let moved = f.pose != tracked.pose;
        match kind {
            BodyKind::Kinematic => {
                // Reaches where the code put it during the step, pushing what is there.
                if moved {
                    let target = f.pose.0 + f.pose.1 * body.collider().center_of_mass();
                    body.linear_velocity = (target - body.center_of_mass()) / dt;
                    body.angular_velocity =
                        crate::three::physics::turn_rate(body.rotation, f.pose.1, dt);
                } else {
                    body.linear_velocity = Vec3::ZERO;
                    body.angular_velocity = Vec3::ZERO;
                }
                tracked.pose = f.pose;
            }
            _ if moved => {
                // Moved by the code: it is there now.
                body.position = f.pose.0;
                body.rotation = f.pose.1;
                body.wake_up();
                tracked.pose = f.pose;
            }
            _ => {}
        }
        if kind == BodyKind::Dynamic {
            let velocity = (
                component::<LinearVelocity>(world, e).map_or(tracked.velocity.0, |v| v.0),
                component::<AngularVelocity>(world, e).map_or(tracked.velocity.1, |v| v.0),
            );
            if velocity != tracked.velocity {
                // Set by the code.
                body.linear_velocity = velocity.0;
                body.angular_velocity = velocity.1;
                body.wake_up();
                tracked.velocity = velocity;
            }
            if let Some(force) = component::<ExternalForce>(world, e) {
                if force.force != Vec3::ZERO {
                    body.apply_force(force.force);
                }
                if !force.persistent {
                    once.push(e);
                }
            }
            if let Some(torque) = component::<ExternalTorque>(world, e) {
                if torque.torque != Vec3::ZERO {
                    body.apply_torque(torque.torque);
                }
                if !torque.persistent {
                    once.push(e);
                }
            }
            if let Some(impulse) = component::<ExternalImpulse>(world, e) {
                if impulse.0 != Vec3::ZERO {
                    body.apply_impulse(impulse.0);
                    once.push(e);
                }
            }
        }
    }
    let gone: Vec<Entity> = state
        .tracked
        .keys()
        .filter(|e| !present.contains(e))
        .copied()
        .collect();
    for e in gone {
        if let Some(t) = state.tracked.remove(&e) {
            state.world.remove(t.body);
        }
    }

    state.world.step(dt);

    // The entities follow their bodies.
    let mut sleeping = Vec::new();
    for (&e, t) in state.tracked.iter_mut() {
        let Some(body) = state.world.body(t.body) else {
            continue;
        };
        if body.kind != BodyKind::Dynamic {
            continue;
        }
        sleeping.push((e, body.is_sleeping()));
        // The mass it has now (its scale or density may have changed it).
        if component::<Mass>(world, e).is_some_and(|m| m.0 != t.mass) {
            if let Some(mut m) = world.get_mut::<Mass>(e) {
                m.0 = t.mass;
            }
        }
        let velocity = (body.linear_velocity, body.angular_velocity);
        t.velocity = velocity;
        if component::<LinearVelocity>(world, e).is_some_and(|v| v.0 != velocity.0) {
            if let Some(mut v) = world.get_mut::<LinearVelocity>(e) {
                v.0 = velocity.0;
            }
        }
        if component::<AngularVelocity>(world, e).is_some_and(|v| v.0 != velocity.1) {
            if let Some(mut v) = world.get_mut::<AngularVelocity>(e) {
                v.0 = velocity.1;
            }
        }
        let pose = (body.position, body.rotation);
        let local = match world.get::<ChildOf>(e).map(|c| c.0) {
            // A dynamic child: back into its parent's space.
            Some(parent) => {
                let parent = world
                    .get::<GlobalTransform>(parent)
                    .map_or(Mat4::IDENTITY, |g| g.to_matrix());
                let (t, r, _) =
                    (parent.inverse() * Mat4::compose(pose.0, pose.1, t.scale)).decompose();
                (t, r)
            }
            None => pose,
        };
        if pose != t.pose || world.get::<ChildOf>(e).is_some() {
            if let Some(mut transform) = world.get_mut::<Transform>(e) {
                if (transform.translation, transform.rotation) != local {
                    transform.translation = local.0;
                    transform.rotation = local.1;
                }
            }
            t.pose = if world.get::<ChildOf>(e).is_some() {
                pose
            } else {
                local
            };
        }
    }
    for e in once {
        if let Some(mut f) = world.get_mut::<ExternalForce>(e) {
            if !f.persistent {
                f.force = Vec3::ZERO;
            }
        }
        if let Some(mut t) = world.get_mut::<ExternalTorque>(e) {
            if !t.persistent {
                t.torque = Vec3::ZERO;
            }
        }
        if let Some(mut i) = world.get_mut::<ExternalImpulse>(e) {
            i.0 = Vec3::ZERO;
        }
    }
    let entity_of = |b: BodyId| {
        state
            .tracked
            .iter()
            .find(|(_, t)| t.body == b)
            .map(|(e, _)| *e)
    };
    let events: Vec<CollisionEvent> = state.world.events().to_vec();
    let (mut started, mut ended) = (Vec::new(), Vec::new());
    for event in events {
        match event {
            CollisionEvent::Started(a, b) => {
                if let (Some(a), Some(b)) = (entity_of(a), entity_of(b)) {
                    started.push(CollisionStarted(a, b));
                }
            }
            CollisionEvent::Ended(a, b) => {
                if let (Some(a), Some(b)) = (entity_of(a), entity_of(b)) {
                    ended.push(CollisionEnded(a, b));
                }
            }
        }
    }
    world.insert_resource(state);
    if let Some(mut messages) = world.get_resource_mut::<Messages<CollisionStarted>>() {
        for m in started {
            messages.write(m);
        }
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<CollisionEnded>>() {
        for m in ended {
            messages.write(m);
        }
    }
    // The components the entities did not have yet, and the sleep marks.
    for (e, kind, mass, has_kind) in to_insert {
        if !world.contains(e) {
            continue;
        }
        if !has_kind {
            world.insert(e, kind);
        }
        if !world.has::<Mass>(e) {
            world.insert(e, Mass(mass));
        }
        let state = world.resource::<PhysicsState>();
        let velocity = state.body(e).map_or((Vec3::ZERO, Vec3::ZERO), |b| {
            (b.linear_velocity, b.angular_velocity)
        });
        drop(state);
        if !world.has::<LinearVelocity>(e) {
            world.insert(e, LinearVelocity(velocity.0));
        }
        if !world.has::<AngularVelocity>(e) {
            world.insert(e, AngularVelocity(velocity.1));
        }
    }
    for (e, asleep) in sleeping {
        match (asleep, world.has::<Sleeping>(e)) {
            (true, false) => world.insert(e, Sleeping),
            (false, true) => {
                world.remove::<Sleeping>(e);
            }
            _ => {}
        }
    }
}
