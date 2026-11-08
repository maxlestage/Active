// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Structure for combining the various physics components to perform an actual simulation.

pub use collision_pipeline::CollisionPipeline;
pub use event_handler::ActiveEvents;
pub use event_handler::ChannelEventCollector;
pub use event_handler::EventHandler;
pub use physics_hooks::ActiveHooks;
pub use physics_hooks::{
    ContactModificationContext, ModifiableContacts, ModifiableManifold, PairFilterContext,
    PhysicsHooks,
};
pub use physics_pipeline::{PhysicsPipeline, Quarantine};
pub use physics_world::PhysicsWorld;
pub use query_pipeline::{QueryFilter, QueryFilterFlags, QueryPipeline, QueryPipelineMut};

mod collision_pipeline;
mod event_handler;
mod physics_hooks;
mod physics_pipeline;
mod physics_world;
mod query_pipeline;
mod user_changes;
