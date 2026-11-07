//! Systems: plain functions whose parameters say what they use (`Query`, `Res`, `ResMut`,
//! `Commands`, `Local`…), run by the schedules of the [`App`](super::App).

use std::any::type_name;
use std::cell::{Ref as CellRef, RefMut};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::query::{Query, QueryData, QueryFilter, Single, Ticks};
use super::world::{Bundle, Command, Component, Entity, Resource, World};

/// What a system knows about itself while it runs.
#[derive(Clone, Debug, Default)]
pub struct SystemMeta {
    /// Its name (the function's path).
    pub name: &'static str,
    /// What changed since it last ran.
    pub ticks: Ticks,
}

/// Something a system asks for: the world gives it each time the system runs.
pub trait SystemParam: Sized {
    /// What the parameter keeps from one run to the next (`Local`'s value).
    type State: 'static;
    /// The parameter with the lifetimes of one run.
    type Item<'w, 's>: SystemParam<State = Self::State>;
    /// The state, when the system first runs.
    fn init_state(world: &mut World) -> Self::State;
    /// Whether the system can run (`Single` needs exactly one entity).
    fn validate(state: &Self::State, world: &World, meta: &SystemMeta) -> bool {
        let _ = (state, world, meta);
        true
    }
    /// The parameter, for one run.
    fn get_param<'w, 's>(
        state: &'s mut Self::State,
        world: &'w World,
        meta: &SystemMeta,
    ) -> Self::Item<'w, 's>;
}

/// A parameter with the lifetimes of one run.
pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;

impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Query<'_, '_, D, F> {
    type State = ();
    type Item<'w, 's> = Query<'w, 's, D, F>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>(
        (): &'s mut (),
        world: &'w World,
        meta: &SystemMeta,
    ) -> Query<'w, 's, D, F> {
        Query::new(world, meta.ticks)
    }
}

impl<D: QueryData + 'static, F: QueryFilter + 'static> SystemParam for Single<'_, D, F> {
    type State = ();
    type Item<'w, 's> = Single<'w, D, F>;
    fn init_state(_: &mut World) {}
    fn validate((): &(), world: &World, meta: &SystemMeta) -> bool {
        Query::<D, F>::new(world, meta.ticks).count() == 1
    }
    fn get_param<'w, 's>((): &'s mut (), world: &'w World, meta: &SystemMeta) -> Single<'w, D, F> {
        let item = Query::<D, F>::new(world, meta.ticks)
            .single()
            .unwrap_or_else(|e| panic!("{}: {e}", meta.name));
        Single {
            item,
            _filter: PhantomData,
        }
    }
}

impl SystemParam for &World {
    type State = ();
    type Item<'w, 's> = &'w World;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>((): &'s mut (), world: &'w World, _: &SystemMeta) -> &'w World {
        world
    }
}

/// A resource to read: `Res<Time>`.
pub struct Res<'w, T: Resource> {
    value: CellRef<'w, T>,
    added: u32,
    changed: u32,
    ticks: Ticks,
}

/// A resource to change: `ResMut<Score>`.
pub struct ResMut<'w, T: Resource> {
    value: RefMut<'w, T>,
    added: u32,
    changed: &'w std::cell::Cell<u32>,
    ticks: Ticks,
}

impl<T: Resource> Res<'_, T> {
    /// Whether it was added since the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_newer(self.added)
    }
    /// Whether it was added or changed since the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_newer(self.changed)
    }
}

impl<T: Resource> ResMut<'_, T> {
    /// Whether it was added since the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_newer(self.added)
    }
    /// Whether it was added or changed since the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_newer(self.changed.get())
    }
    /// Changes it without marking it changed.
    pub fn bypass_change_detection(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Resource> Deref for Res<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: Resource> Deref for ResMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: Resource> DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.changed.set(self.ticks.this_run);
        &mut self.value
    }
}

impl<T: Resource + fmt::Debug> fmt::Debug for Res<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Resource + fmt::Debug> fmt::Debug for ResMut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

fn res<'w, T: Resource>(world: &'w World, meta: &SystemMeta) -> Option<Res<'w, T>> {
    let (added, changed) = world.resource_ticks::<T>()?;
    let (added, changed) = (added.get(), changed.get());
    // Not `World::get_resource`: reading must not mark the resource changed.
    world.get_resource::<T>().map(|value| Res {
        value,
        added,
        changed,
        ticks: meta.ticks,
    })
}

fn res_mut<'w, T: Resource>(world: &'w World, meta: &SystemMeta) -> Option<ResMut<'w, T>> {
    let (added, changed) = world.resource_ticks::<T>()?;
    let added = added.get();
    let before = changed.get();
    let value = world.get_resource_mut::<T>()?;
    // Asking for it does not change it: only writing through `DerefMut` does.
    changed.set(before);
    Some(ResMut {
        value,
        added,
        changed,
        ticks: meta.ticks,
    })
}

#[cold]
fn missing_resource<T>(meta: &SystemMeta) -> ! {
    panic!(
        "the system `{}` needs the resource `{}`, which does not exist: add it with \
         `app.init_resource::<{1}>()` or `insert_resource`, or ask for `Option<Res<{1}>>`",
        meta.name,
        type_name::<T>()
    )
}

impl<T: Resource> SystemParam for Res<'_, T> {
    type State = ();
    type Item<'w, 's> = Res<'w, T>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>((): &'s mut (), world: &'w World, meta: &SystemMeta) -> Res<'w, T> {
        res(world, meta).unwrap_or_else(|| missing_resource::<T>(meta))
    }
}

impl<T: Resource> SystemParam for ResMut<'_, T> {
    type State = ();
    type Item<'w, 's> = ResMut<'w, T>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>((): &'s mut (), world: &'w World, meta: &SystemMeta) -> ResMut<'w, T> {
        res_mut(world, meta).unwrap_or_else(|| missing_resource::<T>(meta))
    }
}

impl<T: Resource> SystemParam for Option<Res<'_, T>> {
    type State = ();
    type Item<'w, 's> = Option<Res<'w, T>>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>(
        (): &'s mut (),
        world: &'w World,
        meta: &SystemMeta,
    ) -> Option<Res<'w, T>> {
        res(world, meta)
    }
}

impl<T: Resource> SystemParam for Option<ResMut<'_, T>> {
    type State = ();
    type Item<'w, 's> = Option<ResMut<'w, T>>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>(
        (): &'s mut (),
        world: &'w World,
        meta: &SystemMeta,
    ) -> Option<ResMut<'w, T>> {
        res_mut(world, meta)
    }
}

/// A value of the system itself, kept from one run to the next: `mut count: Local<u32>`.
pub struct Local<'s, T: Default + 'static>(&'s mut T);

impl<T: Default + 'static> SystemParam for Local<'_, T> {
    type State = T;
    type Item<'w, 's> = Local<'s, T>;
    fn init_state(_: &mut World) -> T {
        T::default()
    }
    fn get_param<'w, 's>(state: &'s mut T, _: &'w World, _: &SystemMeta) -> Local<'s, T> {
        Local(state)
    }
}

impl<T: Default + 'static> Deref for Local<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: Default + 'static> DerefMut for Local<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}

// ----- Commands -------------------------------------------------------------------------

/// Changes to the world made after the system: spawning, inserting, despawning, adding
/// resources, triggering events.
///
/// ```
/// use active::ecs::prelude::*;
///
/// struct Ball;
/// active::component!(Ball);
///
/// fn setup(mut commands: Commands) {
///     commands.spawn((Ball, Transform::from_xyz(0.0, 1.0, 0.0)));
/// }
///
/// let mut app = App::new();
/// app.add_systems(Startup, setup);
/// app.update();
/// assert_eq!(app.world().query_filtered::<Entity, With<Ball>>().iter(app.world()).count(), 1);
/// ```
pub struct Commands<'w, 's> {
    world: &'w World,
    _state: PhantomData<&'s ()>,
}

impl SystemParam for Commands<'_, '_> {
    type State = ();
    type Item<'w, 's> = Commands<'w, 's>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>((): &'s mut (), world: &'w World, _: &SystemMeta) -> Commands<'w, 's> {
        Commands {
            world,
            _state: PhantomData,
        }
    }
}

impl<'w> Commands<'w, '_> {
    /// Commands on this world, outside a system (applied by [`World::flush`]).
    pub fn new(world: &'w World) -> Self {
        Commands {
            world,
            _state: PhantomData,
        }
    }

    /// Queues a change.
    pub fn queue(&mut self, command: impl FnOnce(&mut World) + 'static) {
        self.world
            .commands
            .borrow_mut()
            .push(Box::new(command) as Command);
    }

    /// A new entity with these components; its id is known at once.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        let entity = self.world.reserve();
        self.queue(move |world| {
            world.activate(entity);
            world.insert(entity, bundle);
        });
        EntityCommands {
            commands: self.reborrow(),
            entity,
        }
    }

    /// A new entity without components.
    pub fn spawn_empty(&mut self) -> EntityCommands<'_> {
        self.spawn(())
    }

    /// Commands on an existing entity.
    pub fn entity(&mut self, entity: Entity) -> EntityCommands<'_> {
        EntityCommands {
            commands: self.reborrow(),
            entity,
        }
    }

    /// Adds or replaces a resource.
    pub fn insert_resource<R: Resource>(&mut self, value: R) {
        self.queue(move |world| world.insert_resource(value));
    }

    /// Adds the default value of a resource, unless it is there.
    pub fn init_resource<R: Resource + Default>(&mut self) {
        self.queue(|world| world.init_resource::<R>());
    }

    /// Removes a resource.
    pub fn remove_resource<R: Resource>(&mut self) {
        self.queue(|world| {
            world.remove_resource::<R>();
        });
    }

    /// Runs the observers of the event (`commands.trigger(Explosion { .. })`).
    pub fn trigger<E: super::Event>(&mut self, event: E) {
        self.queue(move |world| world.trigger(event));
    }

    fn reborrow(&mut self) -> Commands<'w, '_> {
        Commands {
            world: self.world,
            _state: PhantomData,
        }
    }
}

/// Commands on one entity (from `commands.spawn` or `commands.entity`).
pub struct EntityCommands<'a> {
    commands: Commands<'a, 'a>,
    entity: Entity,
}

impl EntityCommands<'_> {
    /// The entity.
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds components (replacing those it has).
    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        let entity = self.entity;
        self.commands.queue(move |world| {
            if world.contains(entity) {
                world.insert(entity, bundle);
            }
        });
        self
    }

    /// Removes a component.
    pub fn remove<T: Component>(&mut self) -> &mut Self {
        let entity = self.entity;
        self.commands.queue(move |world| {
            world.remove::<T>(entity);
        });
        self
    }

    /// Removes the entity, with its children.
    pub fn despawn(&mut self) {
        let entity = self.entity;
        self.commands.queue(move |world| {
            world.despawn(entity);
        });
    }

    /// Spawns children of this entity: `.with_children(|parent| { parent.spawn(..); })`.
    pub fn with_children(&mut self, f: impl FnOnce(&mut ChildSpawner<'_>)) -> &mut Self {
        let mut spawner = ChildSpawner {
            commands: self.commands.reborrow(),
            parent: self.entity,
        };
        f(&mut spawner);
        self
    }

    /// Makes `child` a child of this entity.
    pub fn add_child(&mut self, child: Entity) -> &mut Self {
        let parent = self.entity;
        self.commands.queue(move |world| {
            if world.contains(child) && world.contains(parent) {
                world.insert(child, super::ChildOf(parent));
            }
        });
        self
    }

    /// The commands, to do something else.
    pub fn commands(&mut self) -> Commands<'_, '_> {
        self.commands.reborrow()
    }
}

/// Spawns the children of an entity, in [`EntityCommands::with_children`].
pub struct ChildSpawner<'a> {
    commands: Commands<'a, 'a>,
    parent: Entity,
}

impl ChildSpawner<'_> {
    /// A new child with these components.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityCommands<'_> {
        let parent = self.parent;
        let mut child = self.commands.spawn(bundle);
        child.insert(super::ChildOf(parent));
        child
    }

    /// The parent.
    pub fn target_entity(&self) -> Entity {
        self.parent
    }
}

// ----- Function systems -----------------------------------------------------------------

/// A system the schedules can run.
pub trait System: 'static {
    /// What it returns (`bool` for run conditions).
    type Out;
    /// Its name.
    fn name(&self) -> &'static str;
    /// Runs it, then applies its commands; `None` when its parameters could not be had
    /// (`Single` without exactly one entity).
    fn run(&mut self, world: &mut World) -> Option<Self::Out>;
}

/// A function whose parameters are all [`SystemParam`]s.
pub trait SystemParamFunction<Marker>: 'static {
    /// Its parameters, as a tuple.
    type Param: SystemParam;
    /// What it returns.
    type Out;
    /// Calls it.
    fn run(&mut self, param: SystemParamItem<'_, '_, Self::Param>) -> Self::Out;
}

/// Turns a function into a [`System`].
pub trait IntoSystem<Out, Marker>: Sized {
    /// The system.
    type System: System<Out = Out>;
    /// Makes it.
    fn into_system(self) -> Self::System;
}

/// A function made into a system.
pub struct FunctionSystem<Marker, F: SystemParamFunction<Marker>> {
    func: F,
    state: Option<<F::Param as SystemParam>::State>,
    meta: SystemMeta,
    last_run: u32,
    _marker: PhantomData<fn() -> Marker>,
}

#[doc(hidden)]
pub struct IsFunctionSystem;

impl<Marker: 'static, F: SystemParamFunction<Marker>> IntoSystem<F::Out, (IsFunctionSystem, Marker)>
    for F
{
    type System = FunctionSystem<Marker, F>;
    fn into_system(self) -> Self::System {
        FunctionSystem {
            func: self,
            state: None,
            meta: SystemMeta {
                name: type_name::<F>(),
                ticks: Ticks::default(),
            },
            last_run: 0,
            _marker: PhantomData,
        }
    }
}

impl<Marker: 'static, F: SystemParamFunction<Marker>> System for FunctionSystem<Marker, F> {
    type Out = F::Out;

    fn name(&self) -> &'static str {
        self.meta.name
    }

    fn run(&mut self, world: &mut World) -> Option<F::Out> {
        if self.state.is_none() {
            self.state = Some(F::Param::init_state(world));
        }
        let this_run = world.increment_tick();
        self.meta.ticks = Ticks {
            last_run: self.last_run,
            this_run,
        };
        let state = self.state.as_mut().expect("initialized above");
        let out = if F::Param::validate(state, world, &self.meta) {
            let param = F::Param::get_param(state, world, &self.meta);
            Some(self.func.run(param))
        } else {
            None
        };
        self.last_run = this_run;
        world.flush();
        out
    }
}

/// A function taking the whole world, run alone: `fn(world: &mut World)`.
pub struct ExclusiveSystem<F> {
    func: F,
    name: &'static str,
}

#[doc(hidden)]
pub struct IsExclusiveSystem;

impl<Out: 'static, F: FnMut(&mut World) -> Out + 'static> IntoSystem<Out, IsExclusiveSystem> for F {
    type System = ExclusiveSystem<F>;
    fn into_system(self) -> Self::System {
        ExclusiveSystem {
            func: self,
            name: type_name::<F>(),
        }
    }
}

impl<Out: 'static, F: FnMut(&mut World) -> Out + 'static> System for ExclusiveSystem<F> {
    type Out = Out;
    fn name(&self) -> &'static str {
        self.name
    }
    fn run(&mut self, world: &mut World) -> Option<Out> {
        world.increment_tick();
        let out = (self.func)(world);
        world.flush();
        Some(out)
    }
}

macro_rules! param_tuple {
    ($($p:ident),*) => {
        #[allow(non_snake_case, unused_variables, clippy::unused_unit)]
        impl<$($p: SystemParam),*> SystemParam for ($($p,)*) {
            type State = ($($p::State,)*);
            type Item<'w, 's> = ($($p::Item<'w, 's>,)*);
            fn init_state(world: &mut World) -> Self::State {
                ($($p::init_state(world),)*)
            }
            fn validate(state: &Self::State, world: &World, meta: &SystemMeta) -> bool {
                let ($($p,)*) = state;
                true $(&& $p::validate($p, world, meta))*
            }
            fn get_param<'w, 's>(state: &'s mut Self::State, world: &'w World, meta: &SystemMeta) -> Self::Item<'w, 's> {
                let ($($p,)*) = state;
                ($($p::get_param($p, world, meta),)*)
            }
        }

        #[allow(non_snake_case)]
        impl<Out, Func, $($p: SystemParam),*> SystemParamFunction<fn($($p,)*) -> Out> for Func
        where
            Func: 'static,
            for<'a> &'a mut Func: FnMut($($p),*) -> Out + FnMut($(SystemParamItem<'_, '_, $p>),*) -> Out,
        {
            type Param = ($($p,)*);
            type Out = Out;
            fn run(&mut self, param: SystemParamItem<'_, '_, ($($p,)*)>) -> Out {
                // Calls through `&mut Func`, whose `FnMut` takes the items of this run.
                #[allow(clippy::too_many_arguments)]
                fn call<Out, $($p),*>(mut f: impl FnMut($($p),*) -> Out, $($p: $p),*) -> Out {
                    f($($p),*)
                }
                let ($($p,)*) = param;
                call(self, $($p),*)
            }
        }
    };
}

param_tuple!();
param_tuple!(P0);
param_tuple!(P0, P1);
param_tuple!(P0, P1, P2);
param_tuple!(P0, P1, P2, P3);
param_tuple!(P0, P1, P2, P3, P4);
param_tuple!(P0, P1, P2, P3, P4, P5);
param_tuple!(P0, P1, P2, P3, P4, P5, P6);
param_tuple!(P0, P1, P2, P3, P4, P5, P6, P7);
param_tuple!(P0, P1, P2, P3, P4, P5, P6, P7, P8);
param_tuple!(P0, P1, P2, P3, P4, P5, P6, P7, P8, P9);
param_tuple!(P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
param_tuple!(P0, P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);

// ----- Schedule configs -----------------------------------------------------------------

pub(crate) type BoxedSystem = Box<dyn System<Out = ()>>;
pub(crate) type BoxedCondition = Box<dyn System<Out = bool>>;

/// Systems to add to a schedule, with their run conditions.
pub struct ScheduleConfigs {
    pub(crate) entries: Vec<Entry>,
    pub(crate) conditions: Vec<BoxedCondition>,
}

pub(crate) enum Entry {
    System(BoxedSystem),
    Group(ScheduleConfigs),
}

impl ScheduleConfigs {
    /// Runs the systems, in order, when the conditions hold.
    pub(crate) fn run(&mut self, world: &mut World) {
        for condition in &mut self.conditions {
            if condition.run(world) != Some(true) {
                return;
            }
        }
        for entry in &mut self.entries {
            match entry {
                Entry::System(system) => {
                    system.run(world);
                }
                Entry::Group(group) => group.run(world),
            }
        }
    }
}

/// What `add_systems` takes: a system, a tuple of them, with `.chain()` and `.run_if(..)`.
///
/// The systems of a schedule run one after the other, in the order they were added, so that
/// a frame always gives the same result: `.chain()` is always kept.
pub trait IntoScheduleConfigs<Marker>: Sized {
    /// The configs.
    fn into_configs(self) -> ScheduleConfigs;

    /// Runs the systems in order (they always are).
    fn chain(self) -> ScheduleConfigs {
        self.into_configs()
    }

    /// Runs the systems only when `condition` returns `true` (checked once for them all).
    fn run_if<M>(self, condition: impl IntoSystem<bool, M>) -> ScheduleConfigs {
        let mut configs = self.into_configs();
        if configs.conditions.is_empty() {
            configs.conditions.push(Box::new(condition.into_system()));
            configs
        } else {
            ScheduleConfigs {
                entries: vec![Entry::Group(configs)],
                conditions: vec![Box::new(condition.into_system())],
            }
        }
    }
}

impl IntoScheduleConfigs<()> for ScheduleConfigs {
    fn into_configs(self) -> ScheduleConfigs {
        self
    }
}

impl<Marker, F: IntoSystem<(), Marker>> IntoScheduleConfigs<(IsFunctionSystem, Marker)> for F {
    fn into_configs(self) -> ScheduleConfigs {
        ScheduleConfigs {
            entries: vec![Entry::System(Box::new(self.into_system()))],
            conditions: Vec::new(),
        }
    }
}

#[doc(hidden)]
pub struct IsTuple;

macro_rules! config_tuple {
    ($(($m:ident, $s:ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($m, $s: IntoScheduleConfigs<$m>),*> IntoScheduleConfigs<(IsTuple, $($m,)*)> for ($($s,)*) {
            fn into_configs(self) -> ScheduleConfigs {
                let ($($s,)*) = self;
                ScheduleConfigs { entries: vec![$(Entry::Group($s.into_configs())),*], conditions: Vec::new() }
            }
        }
    };
}

config_tuple!((M0, S0));
config_tuple!((M0, S0), (M1, S1));
config_tuple!((M0, S0), (M1, S1), (M2, S2));
config_tuple!((M0, S0), (M1, S1), (M2, S2), (M3, S3));
config_tuple!((M0, S0), (M1, S1), (M2, S2), (M3, S3), (M4, S4));
config_tuple!((M0, S0), (M1, S1), (M2, S2), (M3, S3), (M4, S4), (M5, S5));
config_tuple!(
    (M0, S0),
    (M1, S1),
    (M2, S2),
    (M3, S3),
    (M4, S4),
    (M5, S5),
    (M6, S6)
);
config_tuple!(
    (M0, S0),
    (M1, S1),
    (M2, S2),
    (M3, S3),
    (M4, S4),
    (M5, S5),
    (M6, S6),
    (M7, S7)
);
config_tuple!(
    (M0, S0),
    (M1, S1),
    (M2, S2),
    (M3, S3),
    (M4, S4),
    (M5, S5),
    (M6, S6),
    (M7, S7),
    (M8, S8)
);
config_tuple!(
    (M0, S0),
    (M1, S1),
    (M2, S2),
    (M3, S3),
    (M4, S4),
    (M5, S5),
    (M6, S6),
    (M7, S7),
    (M8, S8),
    (M9, S9)
);

// ----- Run conditions -------------------------------------------------------------------

/// True when the resource exists: `.run_if(resource_exists::<Score>)`.
pub fn resource_exists<T: Resource>(res: Option<Res<'_, T>>) -> bool {
    res.is_some()
}

/// True when the resource was added or changed since the condition last ran.
pub fn resource_changed<T: Resource>(res: Option<Res<'_, T>>) -> bool {
    res.is_some_and(|r| r.is_changed())
}

/// True the first time only.
pub fn run_once(mut done: Local<'_, bool>) -> bool {
    !std::mem::replace(&mut *done, true)
}

/// True when a message of this type was written since the condition last ran.
pub fn on_message<M: super::Message>(mut reader: super::MessageReader<'_, '_, M>) -> bool {
    reader.read().count() > 0
}

/// The opposite of a condition: `.run_if(not(resource_exists::<Paused>))`.
pub fn not<M, C: IntoSystem<bool, M>>(condition: C) -> impl FnMut(&mut World) -> bool + 'static {
    let mut condition = condition.into_system();
    move |world: &mut World| condition.run(world) != Some(true)
}
