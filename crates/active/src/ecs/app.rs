//! The app: plugins, schedules, states, and the update of each frame.

use std::any::{TypeId, type_name};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU8;
use std::ops::Deref;
use std::time::Duration;

use super::input::{
    AccumulatedMouseMotion, AccumulatedMouseScroll, ButtonInput, KeyCode, MouseButton, PendingInput,
};
use super::message::{IntoObserverSystem, Message, Messages};
use super::system::{IntoScheduleConfigs, Res, ScheduleConfigs};
use super::time::{Fixed, MAX_DELTA, Time, TimeUpdateStrategy, now_secs};
use super::transform::{propagate_transforms, propagate_visibility};
use super::window::{PrimaryWindow, Window};
use super::world::{Resource, World};

/// The key of a schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label {
    type_id: TypeId,
    value: u64,
}

/// A schedule's name: `Startup`, `Update`, `OnEnter(GameState::Playing)`…
pub trait ScheduleLabel: 'static {
    /// Its key.
    fn label(&self) -> Label;
}

macro_rules! schedules {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
            pub struct $name;

            impl ScheduleLabel for $name {
                fn label(&self) -> Label {
                    Label { type_id: TypeId::of::<$name>(), value: 0 }
                }
            }
        )*
    };
}

schedules!(
    /// Once, before `Startup`.
    PreStartup,
    /// Once, before the first frame: spawn the scene here.
    Startup,
    /// Once, after `Startup`.
    PostStartup,
    /// First in each frame (messages are updated here).
    First,
    /// Before `Update` (input is read here).
    PreUpdate,
    /// The fixed step, first.
    FixedFirst,
    /// The fixed step, before `FixedUpdate`.
    FixedPreUpdate,
    /// The fixed step (64 times a second by default): physics, gameplay.
    FixedUpdate,
    /// The fixed step, after `FixedUpdate`.
    FixedPostUpdate,
    /// The fixed step, last.
    FixedLast,
    /// Each frame: most systems go here.
    Update,
    /// After `Update` (transforms and visibility are computed here).
    PostUpdate,
    /// Last in each frame.
    Last,
);

const STARTUP: [fn() -> Label; 3] = [
    || PreStartup.label(),
    || Startup.label(),
    || PostStartup.label(),
];
const FIXED: [fn() -> Label; 5] = [
    || FixedFirst.label(),
    || FixedPreUpdate.label(),
    || FixedUpdate.label(),
    || FixedPostUpdate.label(),
    || FixedLast.label(),
];

/// The systems of one schedule, run in the order they were added.
#[derive(Default)]
struct Schedule {
    configs: Vec<ScheduleConfigs>,
}

impl Schedule {
    fn run(&mut self, world: &mut World) {
        for configs in &mut self.configs {
            configs.run(world);
        }
    }
}

/// How the app ended.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppExit {
    /// Normally.
    #[default]
    Success,
    /// With an error code.
    Error(NonZeroU8),
}

impl AppExit {
    /// An error with this code (1 if 0).
    pub fn from_code(code: u8) -> AppExit {
        NonZeroU8::new(code).map_or(AppExit::Success, AppExit::Error)
    }
}

impl Message for AppExit {}

type Transition = Box<dyn FnMut(&mut World) -> Vec<Label>>;

/// An app: its world, its schedules and the plugins that filled them.
///
/// ```
/// use active::ecs::prelude::*;
///
/// struct Counter(u32);
/// active::resource!(Counter);
///
/// fn count(mut counter: ResMut<Counter>) {
///     counter.0 += 1;
/// }
///
/// let mut app = App::new();
/// app.insert_resource(Counter(0)).add_systems(Update, count);
/// app.update();
/// app.update();
/// assert_eq!(app.world().resource::<Counter>().0, 2);
/// ```
pub struct App {
    world: World,
    schedules: HashMap<Label, Schedule>,
    plugins: HashSet<TypeId>,
    message_updates: Vec<fn(&mut World)>,
    transitions: Vec<Transition>,
    started: bool,
    last_time: Option<f64>,
}

impl Default for App {
    fn default() -> App {
        App::new()
    }
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("world", &self.world)
            .field("schedules", &self.schedules.len())
            .finish()
    }
}

impl App {
    /// An app without plugins (add `DefaultPlugins` for 3D in a page, `MinimalPlugins` for
    /// time only).
    pub fn new() -> App {
        let mut app = App {
            world: World::new(),
            schedules: HashMap::new(),
            plugins: HashSet::new(),
            message_updates: Vec::new(),
            transitions: Vec::new(),
            started: false,
            last_time: None,
        };
        app.add_message::<AppExit>();
        app
    }

    /// The world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The world, to change.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Adds plugins: a plugin, a group (`DefaultPlugins`) or a tuple of them.
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        plugins.add_to_app(self);
        self
    }

    /// Whether a plugin of this type was added.
    pub fn is_plugin_added<P: Plugin>(&self) -> bool {
        self.plugins.contains(&TypeId::of::<P>())
    }

    fn add_plugin(&mut self, plugin: &dyn Plugin, id: TypeId) {
        if plugin.is_unique() && !self.plugins.insert(id) {
            panic!("the plugin `{}` was added twice", plugin.name());
        }
        plugin.build(self);
    }

    /// Adds systems to a schedule: `app.add_systems(Update, (movement, collisions).chain())`.
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<M>,
    ) -> &mut Self {
        self.schedules
            .entry(schedule.label())
            .or_default()
            .configs
            .push(systems.into_configs());
        self
    }

    /// Adds an observer, run when its event is triggered.
    pub fn add_observer<E: super::Event, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, M>,
    ) -> &mut Self {
        self.world.add_observer(observer);
        self
    }

    /// Adds or replaces a resource.
    pub fn insert_resource<R: Resource>(&mut self, value: R) -> &mut Self {
        self.world.insert_resource(value);
        self
    }

    /// Adds the default value of a resource, unless it is there.
    pub fn init_resource<R: Resource + Default>(&mut self) -> &mut Self {
        self.world.init_resource::<R>();
        self
    }

    /// Registers a message type: its `Messages<M>` and their update each frame.
    pub fn add_message<M: Message>(&mut self) -> &mut Self {
        if !self.world.contains_resource::<Messages<M>>() {
            self.world.init_resource::<Messages<M>>();
            self.message_updates
                .push(|world| world.resource_mut::<Messages<M>>().update());
        }
        self
    }

    /// Adds a state, starting at its default value.
    pub fn init_state<S: States + Default>(&mut self) -> &mut Self {
        self.insert_state(S::default())
    }

    /// Adds a state, starting at `state`: `OnEnter(state)` runs before the first frame.
    pub fn insert_state<S: States>(&mut self, state: S) -> &mut Self {
        if self.world.contains_resource::<State<S>>() {
            return self;
        }
        self.world.insert_resource(State(state.clone()));
        self.world.insert_resource(NextState::<S>::Unchanged);
        let mut entered = false;
        self.transitions.push(Box::new(move |world: &mut World| {
            if !entered {
                entered = true;
                let current = world.resource::<State<S>>().0.clone();
                return vec![OnEnter(current).label()];
            }
            let next = std::mem::replace(
                &mut *world.resource_mut::<NextState<S>>(),
                NextState::Unchanged,
            );
            let NextState::Pending(next) = next else {
                return Vec::new();
            };
            let current = world.resource::<State<S>>().0.clone();
            if current == next {
                return Vec::new();
            }
            world.insert_resource(State(next.clone()));
            vec![OnExit(current).label(), OnEnter(next).label()]
        }));
        self
    }

    fn run_schedule(&mut self, label: Label) {
        // Taken out while it runs: its systems change the world, not the schedules.
        if let Some(mut schedule) = self.schedules.remove(&label) {
            schedule.run(&mut self.world);
            let added = self.schedules.insert(label, schedule);
            if let Some(added) = added {
                // Systems added to it while it ran come after.
                self.schedules
                    .get_mut(&label)
                    .expect("just inserted")
                    .configs
                    .extend(added.configs);
            }
        }
    }

    /// Runs one schedule now.
    pub fn run_schedule_now(&mut self, schedule: impl ScheduleLabel) {
        self.run_schedule(schedule.label());
    }

    /// Runs one frame: the startup schedules the first time, then `First`, `PreUpdate`, the
    /// state transitions, the fixed steps, `Update`, `PostUpdate` and `Last`.
    pub fn update(&mut self) {
        if !self.started {
            self.started = true;
            for label in STARTUP {
                self.run_schedule(label());
            }
        }
        let delta = self.frame_delta();
        if self.world.contains_resource::<Time>() {
            self.world.resource_mut::<Time>().advance_by(delta);
        }
        for update in self.message_updates.clone() {
            update(&mut self.world);
        }
        self.run_schedule(First.label());
        self.run_schedule(PreUpdate.label());
        self.run_transitions();
        self.run_fixed(delta);
        self.run_schedule(Update.label());
        self.run_schedule(PostUpdate.label());
        self.run_schedule(Last.label());
        self.world.flush();
    }

    fn frame_delta(&mut self) -> Duration {
        match self
            .world
            .get_resource::<TimeUpdateStrategy>()
            .map(|s| *s)
            .unwrap_or_default()
        {
            TimeUpdateStrategy::ManualDuration(delta) => delta,
            TimeUpdateStrategy::Automatic => {
                let now = now_secs();
                let delta = match (self.last_time, now) {
                    (Some(before), Some(now)) => (now - before).max(0.0),
                    _ => 0.0,
                };
                self.last_time = now;
                Duration::from_secs_f64(delta).min(MAX_DELTA)
            }
        }
    }

    fn run_transitions(&mut self) {
        let mut transitions = std::mem::take(&mut self.transitions);
        for transition in &mut transitions {
            for label in transition(&mut self.world) {
                self.run_schedule(label);
            }
        }
        transitions.append(&mut self.transitions);
        self.transitions = transitions;
    }

    fn run_fixed(&mut self, delta: Duration) {
        if !self.world.contains_resource::<Time<Fixed>>() {
            return;
        }
        self.world.resource_mut::<Time<Fixed>>().accumulate(delta);
        // Never more than 16 steps a frame: a slow frame must not make the next slower.
        for _ in 0..16 {
            if !self.world.resource_mut::<Time<Fixed>>().expend() {
                break;
            }
            // Inside the fixed step, `Res<Time>` is the fixed clock.
            let fixed = *self.world.resource::<Time<Fixed>>();
            let main = std::mem::replace(&mut *self.world.resource_mut::<Time>(), {
                let mut t = Time::default();
                t.advance_by(fixed.elapsed());
                t.advance_by_delta(fixed.delta());
                t
            });
            for label in FIXED {
                self.run_schedule(label());
            }
            *self.world.resource_mut::<Time>() = main;
        }
    }

    /// Whether a system asked the app to stop (by writing an `AppExit` message).
    pub fn should_exit(&self) -> Option<AppExit> {
        let messages = self.world.get_resource::<Messages<AppExit>>()?;
        let mut exits = messages.iter().copied();
        let first = exits.next()?;
        Some(exits.find(|e| *e != AppExit::Success).unwrap_or(first))
    }

    /// Runs the app until it exits. In a page (the `client` feature), it draws in the canvas
    /// named by `Window::canvas` (a CSS selector), or in a new canvas added to the page, and
    /// returns at once.
    pub fn run(&mut self) -> AppExit {
        #[cfg(all(target_arch = "wasm32", feature = "client"))]
        {
            let app = std::mem::take(self);
            if let Err(error) = super::web::run_in_page(app) {
                web_sys::console::error_1(&error.into());
                return AppExit::from_code(1);
            }
            AppExit::Success
        }
        #[cfg(not(all(target_arch = "wasm32", feature = "client")))]
        loop {
            self.update();
            if let Some(exit) = self.should_exit() {
                return exit;
            }
        }
    }
}

impl Time {
    /// Sets the last delta without moving the elapsed time.
    fn advance_by_delta(&mut self, delta: Duration) {
        let elapsed = self.elapsed();
        *self = Time::default();
        self.advance_by(elapsed.saturating_sub(delta));
        self.advance_by(delta);
    }
}

// ----- Plugins --------------------------------------------------------------------------

/// A part of an app: it adds systems, resources and other plugins in `build`.
///
/// A function `fn(&mut App)` is a plugin too.
pub trait Plugin: 'static {
    /// Adds what the plugin brings.
    fn build(&self, app: &mut App);

    /// Its name.
    fn name(&self) -> &str {
        type_name::<Self>()
    }

    /// Whether adding it twice is a mistake (true).
    fn is_unique(&self) -> bool {
        true
    }
}

impl<F: Fn(&mut App) + 'static> Plugin for F {
    fn build(&self, app: &mut App) {
        self(app);
    }
    fn is_unique(&self) -> bool {
        false
    }
}

/// What `add_plugins` takes.
pub trait Plugins<Marker> {
    /// Adds them.
    fn add_to_app(self, app: &mut App);
}

#[doc(hidden)]
pub struct IsPlugin;
#[doc(hidden)]
pub struct IsPluginGroup;
#[doc(hidden)]
pub struct IsPluginTuple;

impl<P: Plugin> Plugins<IsPlugin> for P {
    fn add_to_app(self, app: &mut App) {
        app.add_plugin(&self, TypeId::of::<P>());
    }
}

impl<G: PluginGroup> Plugins<IsPluginGroup> for G {
    fn add_to_app(self, app: &mut App) {
        for (id, plugin) in self.build().plugins {
            app.add_plugin(plugin.as_ref(), id);
        }
    }
}

macro_rules! plugin_tuple {
    ($(($m:ident, $p:ident)),*) => {
        #[allow(non_snake_case)]
        impl<$($m, $p: Plugins<$m>),*> Plugins<(IsPluginTuple, $($m,)*)> for ($($p,)*) {
            fn add_to_app(self, app: &mut App) {
                let ($($p,)*) = self;
                $($p.add_to_app(app);)*
            }
        }
    };
}

plugin_tuple!((M0, P0));
plugin_tuple!((M0, P0), (M1, P1));
plugin_tuple!((M0, P0), (M1, P1), (M2, P2));
plugin_tuple!((M0, P0), (M1, P1), (M2, P2), (M3, P3));
plugin_tuple!((M0, P0), (M1, P1), (M2, P2), (M3, P3), (M4, P4));
plugin_tuple!((M0, P0), (M1, P1), (M2, P2), (M3, P3), (M4, P4), (M5, P5));
plugin_tuple!(
    (M0, P0),
    (M1, P1),
    (M2, P2),
    (M3, P3),
    (M4, P4),
    (M5, P5),
    (M6, P6)
);
plugin_tuple!(
    (M0, P0),
    (M1, P1),
    (M2, P2),
    (M3, P3),
    (M4, P4),
    (M5, P5),
    (M6, P6),
    (M7, P7)
);

/// Plugins added together, which can be changed first: `DefaultPlugins.set(WindowPlugin {
/// .. })`.
pub trait PluginGroup: Sized + 'static {
    /// Its plugins.
    fn build(self) -> PluginGroupBuilder;

    /// Replaces its plugin of this type.
    fn set<P: Plugin>(self, plugin: P) -> PluginGroupBuilder {
        self.build().set(plugin)
    }

    /// Leaves out its plugin of this type.
    fn disable<P: Plugin>(self) -> PluginGroupBuilder {
        self.build().disable::<P>()
    }
}

/// The plugins of a group.
pub struct PluginGroupBuilder {
    plugins: Vec<(TypeId, Box<dyn Plugin>)>,
}

impl PluginGroupBuilder {
    /// An empty group.
    pub fn new() -> PluginGroupBuilder {
        PluginGroupBuilder {
            plugins: Vec::new(),
        }
    }

    /// Adds a plugin at the end.
    #[allow(clippy::should_implement_trait)] // the usual name
    pub fn add<P: Plugin>(mut self, plugin: P) -> PluginGroupBuilder {
        self.plugins.push((TypeId::of::<P>(), Box::new(plugin)));
        self
    }

    /// Replaces the plugin of this type (or adds it).
    pub fn set<P: Plugin>(mut self, plugin: P) -> PluginGroupBuilder {
        match self
            .plugins
            .iter_mut()
            .find(|(id, _)| *id == TypeId::of::<P>())
        {
            Some(slot) => slot.1 = Box::new(plugin),
            None => self.plugins.push((TypeId::of::<P>(), Box::new(plugin))),
        }
        self
    }

    /// Leaves out the plugin of this type.
    pub fn disable<P: Plugin>(mut self) -> PluginGroupBuilder {
        self.plugins.retain(|(id, _)| *id != TypeId::of::<P>());
        self
    }
}

impl Default for PluginGroupBuilder {
    fn default() -> Self {
        PluginGroupBuilder::new()
    }
}

impl PluginGroup for PluginGroupBuilder {
    fn build(self) -> PluginGroupBuilder {
        self
    }
}

/// Everything a 3D app in a page needs: time, input, transforms, the window, assets,
/// meshes, materials, lights, cameras and physics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        let plugins = PluginGroupBuilder::new()
            .add(TimePlugin)
            .add(InputPlugin)
            .add(TransformPlugin)
            .add(super::window::WindowPlugin::default())
            .add(super::pbr::PbrPlugin);
        // With the `physics` feature, on by default.
        #[cfg(feature = "physics")]
        let plugins = plugins.add(super::physics::PhysicsPlugin);
        plugins
    }
}

/// Time only, for apps without a page (servers, tests).
#[derive(Clone, Copy, Debug, Default)]
pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::new().add(TimePlugin)
    }
}

/// `Time`, `Time<Fixed>` and `FixedUpdate`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<Time<Fixed>>()
            .init_resource::<TimeUpdateStrategy>();
    }
}

/// The keyboard and the mouse.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .init_resource::<PendingInput>()
            .add_systems(PreUpdate, read_input);
    }
}

/// Applies what the page sent since the last frame.
fn read_input(world: &mut World) {
    let pending = std::mem::take(&mut *world.resource_mut::<PendingInput>());
    {
        let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        if pending.blurred {
            keys.release_all();
        }
        for (key, down) in pending.keys {
            if down {
                keys.press(key)
            } else {
                keys.release(key)
            }
        }
    }
    {
        let mut buttons = world.resource_mut::<ButtonInput<MouseButton>>();
        buttons.clear();
        if pending.blurred {
            buttons.release_all();
        }
        for (button, down) in pending.buttons {
            if down {
                buttons.press(button)
            } else {
                buttons.release(button)
            }
        }
    }
    world.resource_mut::<AccumulatedMouseMotion>().delta = pending.motion;
    world.resource_mut::<AccumulatedMouseScroll>().delta = pending.scroll;
    if let Some(cursor) = pending.cursor {
        for mut window in world
            .query_filtered::<&mut Window, super::With<PrimaryWindow>>()
            .iter(world)
        {
            window.cursor_position = cursor;
        }
    }
}

/// `GlobalTransform` and `InheritedVisibility`, computed in `PostUpdate`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (propagate_transforms, propagate_visibility));
        // The scene spawned in `Startup` is in place for the first frame's `Update`.
        app.add_systems(PostStartup, (propagate_transforms, propagate_visibility));
    }
}

// ----- States ---------------------------------------------------------------------------

/// The values of a state of the app (a menu, playing, paused): `impl States for GameState
/// {}` on an enum that is `Clone + Eq + Hash + Debug`.
///
/// ```
/// use active::ecs::prelude::*;
///
/// #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
/// enum GameState {
///     #[default]
///     Menu,
///     Playing,
/// }
/// active::states!(GameState);
///
/// fn start(mut next: ResMut<NextState<GameState>>) {
///     next.set(GameState::Playing);
/// }
///
/// let mut app = App::new();
/// app.init_state::<GameState>().add_systems(Update, start.run_if(in_state(GameState::Menu)));
/// app.update();
/// app.update();
/// assert_eq!(*app.world().resource::<State<GameState>>().get(), GameState::Playing);
/// ```
pub trait States: Clone + PartialEq + Eq + Hash + fmt::Debug + 'static {}

/// Implements [`States`] for types.
#[macro_export]
macro_rules! states {
    ($($t:ty),+ $(,)?) => { $(impl $crate::ecs::States for $t {})+ };
}

/// The current value of a state: `Res<State<GameState>>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State<S: States>(S);

impl<S: States> Resource for State<S> {}

impl<S: States> State<S> {
    /// The value.
    pub fn get(&self) -> &S {
        &self.0
    }
}

impl<S: States> Deref for State<S> {
    type Target = S;
    fn deref(&self) -> &S {
        &self.0
    }
}

/// The value a state takes before the next frame's `Update`: `ResMut<NextState<S>>`, then
/// `.set(..)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NextState<S: States> {
    /// No change.
    #[default]
    Unchanged,
    /// Changes to this value.
    Pending(S),
}

impl<S: States> Resource for NextState<S> {}

impl<S: States> NextState<S> {
    /// Changes the state (`OnExit` then `OnEnter` run before the next `Update`).
    pub fn set(&mut self, state: S) {
        *self = NextState::Pending(state);
    }

    /// Cancels the change.
    pub fn reset(&mut self) {
        *self = NextState::Unchanged;
    }
}

/// The schedule run when a state takes this value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnEnter<S: States>(pub S);

/// The schedule run when a state leaves this value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OnExit<S: States>(pub S);

fn hash_of(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl<S: States> ScheduleLabel for OnEnter<S> {
    fn label(&self) -> Label {
        Label {
            type_id: TypeId::of::<OnEnter<S>>(),
            value: hash_of(&self.0),
        }
    }
}

impl<S: States> ScheduleLabel for OnExit<S> {
    fn label(&self) -> Label {
        Label {
            type_id: TypeId::of::<OnExit<S>>(),
            value: hash_of(&self.0),
        }
    }
}

/// True while the state has this value: `.run_if(in_state(GameState::Playing))`.
pub fn in_state<S: States>(state: S) -> impl FnMut(Option<Res<'_, State<S>>>) -> bool + 'static {
    move |current: Option<Res<'_, State<S>>>| current.is_some_and(|c| c.0 == state)
}

/// True when the state changed since the condition last ran.
pub fn state_changed<S: States>(current: Option<Res<'_, State<S>>>) -> bool {
    current.is_some_and(|c| c.is_changed())
}
