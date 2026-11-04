//! Games and simulations the way Bevy writes them (its API as of 0.20), written for
//! Active: an entity component system, an `App` with plugins and schedules, and 3D drawn
//! by [`active::three`](crate::three) in a canvas.
//!
//! ```
//! use active::ecs::prelude::*;
//!
//! struct Spinning(f32);
//! active::component!(Spinning);
//!
//! fn setup(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//! ) {
//!     commands.spawn((
//!         Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
//!         MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
//!         Transform::from_xyz(0.0, 0.5, 0.0),
//!         Spinning(1.0),
//!     ));
//!     commands.spawn((PointLight::default(), Transform::from_xyz(4.0, 8.0, 4.0)));
//!     commands.spawn((Camera3d::default(), Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y)));
//! }
//!
//! fn spin(time: Res<Time>, mut query: Query<(&mut Transform, &Spinning)>) {
//!     for (mut transform, spinning) in &mut query {
//!         transform.rotate_y(spinning.0 * time.delta_secs());
//!     }
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(DefaultPlugins).add_systems(Startup, setup).add_systems(Update, spin);
//! // In a page: `app.run()`, or `app.run_in(canvas)` from a component.
//! app.update();
//! let frame = RenderFrame::extract(app.world(), Vec2::new(800.0, 600.0)).unwrap();
//! assert_eq!(frame.mesh_count(), 1);
//! ```
//!
//! What differs from Bevy: components, resources, messages, events and states are declared
//! with `impl Component for T {}` (or the [`component!`](crate::component) macro) rather
//! than derived; the systems of a schedule run one after the other, in the order they were
//! added; shadows, textures, spot lights and the 2D renderer are not there. Everything but
//! drawing works on the server and in tests too.

// The lifetimes of `SystemParam::get_param` are written out, like the trait's.
#![allow(clippy::needless_lifetimes)]

mod app;
mod input;
mod message;
mod pbr;
mod physics;
mod query;
mod system;
mod time;
mod transform;
#[cfg(feature = "client")]
mod web;
mod window;
mod world;

pub use crate::physics::{Attraction, Collider, Shape};
pub use app::{
    App, AppExit, DefaultPlugins, First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate,
    FixedUpdate, InputPlugin, Label, Last, MinimalPlugins, NextState, OnEnter, OnExit, Plugin,
    PluginGroup, PluginGroupBuilder, Plugins, PostStartup, PostUpdate, PreStartup, PreUpdate,
    ScheduleLabel, Startup, State, States, TimePlugin, TransformPlugin, Update, in_state,
    state_changed,
};
pub use input::{
    AccumulatedMouseMotion, AccumulatedMouseScroll, ButtonInput, KeyCode, MouseButton,
    input_just_pressed, input_pressed,
};
pub use message::{
    Event, FunctionObserver, IntoObserverSystem, Message, MessageReader, MessageWriter, Messages,
    ObserverSystem, On,
};
pub use pbr::{
    AlphaMode, AmbientLight, Assets, Camera, Camera3d, Circle, ClearColor, ClearColorConfig, Color,
    ComputedCameraValues, Cone, Cuboid, Cylinder, DirectionalLight, Exposure, Face,
    GlobalAmbientLight, Handle, InfinitePlane3d, LinearRgba, Mesh, Mesh3d, MeshMaterial3d,
    OrthographicProjection, PbrPlugin, PerspectiveProjection, Plane3d, PlaneMeshBuilder,
    PointLight, Projection, Ray3d, Rectangle, RenderFrame, ScalingMode, Sphere, SphereMeshBuilder,
    StandardMaterial, Tonemapping, Torus, ViewportConversionError, light_consts, palettes,
};
pub use physics::{
    AngularDamping, AngularVelocity, ColliderDensity, CollisionEnded, CollisionStarted,
    ExternalForce, ExternalImpulse, ExternalTorque, Friction, Gravity, GravityScale, LinearDamping,
    LinearVelocity, Mass, PhysicsPlugin, PhysicsSettings, PhysicsState, Restitution, RigidBody,
    RollingFriction, Sleeping, step_physics,
};
pub use query::{
    Added, Changed, Has, Mut, Or, Query, QueryData, QueryEntityError, QueryFilter, QueryIter,
    QuerySingleError, QueryState, Ref, Single, Ticks, With, Without,
};
pub use system::{
    ChildSpawner, Commands, EntityCommands, ExclusiveSystem, FunctionSystem, IntoScheduleConfigs,
    IntoSystem, Local, Res, ResMut, ScheduleConfigs, System, SystemMeta, SystemParam,
    SystemParamFunction, SystemParamItem, not, on_message, resource_changed, resource_exists,
    run_once,
};
pub use time::{Fixed, Time, TimeUpdateStrategy, Timer, TimerMode, on_timer};
pub use transform::{
    ChildOf, Children, GlobalTransform, InheritedVisibility, Transform, Visibility,
    propagate_transforms, propagate_visibility,
};
#[cfg(feature = "client")]
pub use web::Running;
pub use window::{PrimaryWindow, Window, WindowPlugin, WindowResolution};
pub use world::{Bundle, Component, Entity, EntityWorldMut, Resource, World};

/// A name for an entity, for debugging: `Name::new("Player")`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    /// A name.
    pub fn new(name: impl Into<String>) -> Name {
        Name(name.into())
    }

    /// The name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Component for Name {}

/// `T::default()`, for `..default()` in struct literals.
pub fn default<T: Default>() -> T {
    T::default()
}

/// Everything at once: `use active::ecs::prelude::*;`.
pub mod prelude {
    pub use super::*;
    pub use crate::three::math::{Mat4, Quat, Vec2, Vec3};
}

#[cfg(test)]
mod tests;
