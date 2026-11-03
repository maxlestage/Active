//! The world: entities, their components and the resources.
//!
//! Each component of each entity sits in its own `RefCell`, so systems share the world
//! (`&World`) and still change components: two queries reaching the same component of the
//! same entity at the same time is the only conflict, found when it happens. Structural
//! changes (spawning, inserting, despawning) go through [`Commands`](super::Commands),
//! applied after each system.

use std::any::{Any, TypeId, type_name};
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::fmt;

/// An entity: a generational index, cheap to copy and to compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// An entity that never exists, for fields set later.
    pub const PLACEHOLDER: Entity = Entity {
        index: u32::MAX,
        generation: u32::MAX,
    };

    /// Its index (reused after it is despawned).
    pub fn index(self) -> u32 {
        self.index
    }

    /// How many times its index was used before.
    pub fn generation(self) -> u32 {
        self.generation
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Data attached to entities (Bevy's `#[derive(Component)]`): write `impl Component for
/// Player {}`, or [`component!`](crate::component)`(Player, Speed)`.
///
/// The hooks run when the component is added or replaced, and when it is removed; the
/// built-in components use them to add the components they require (a `Transform` brings
/// its `GlobalTransform`) and to keep `Children` up to date.
pub trait Component: 'static {
    /// After the component is inserted (also when it replaces a value).
    fn on_insert(world: &mut World, entity: Entity) {
        let _ = (world, entity);
    }

    /// Before the component is removed or replaced, while it is still there.
    fn on_remove(world: &mut World, entity: Entity) {
        let _ = (world, entity);
    }
}

/// Data of the world itself, one value per type (Bevy's `#[derive(Resource)]`): write
/// `impl Resource for Score {}`, or [`resource!`](crate::resource)`(Score)`.
pub trait Resource: 'static {}

/// Implements [`Component`] for types (Bevy's `#[derive(Component)]`).
///
/// ```
/// use active::ecs::prelude::*;
///
/// struct Player;
/// struct Speed(f32);
/// active::component!(Player, Speed);
/// ```
#[macro_export]
macro_rules! component {
    ($($t:ty),+ $(,)?) => { $(impl $crate::ecs::Component for $t {})+ };
}

/// Implements [`Resource`] for types (Bevy's `#[derive(Resource)]`).
#[macro_export]
macro_rules! resource {
    ($($t:ty),+ $(,)?) => { $(impl $crate::ecs::Resource for $t {})+ };
}

/// When a value was added and last changed, for `Added`, `Changed` and `is_changed`.
#[derive(Debug)]
pub(crate) struct Slot<T> {
    pub(crate) value: RefCell<T>,
    pub(crate) added: Cell<u32>,
    pub(crate) changed: Cell<u32>,
}

impl<T> Slot<T> {
    fn new(value: T, tick: u32) -> Slot<T> {
        Slot {
            value: RefCell::new(value),
            added: Cell::new(tick),
            changed: Cell::new(tick),
        }
    }
}

/// The values of one component type, by entity index (what queries look up).
#[doc(hidden)]
pub struct Column<T> {
    pub(crate) slots: Vec<Option<Slot<T>>>,
}

impl<T> Column<T> {
    pub(crate) fn get(&self, index: usize) -> Option<&Slot<T>> {
        self.slots.get(index).and_then(Option::as_ref)
    }
}

/// What the world needs to do with a column without knowing its type.
trait AnyColumn {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains(&self, index: usize) -> bool;
    fn remove(&mut self, index: usize);
    /// The component's `on_remove` hook.
    fn on_remove(&self) -> fn(&mut World, Entity);
    fn name(&self) -> &'static str;
}

impl<T: Component> AnyColumn for Column<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn contains(&self, index: usize) -> bool {
        self.get(index).is_some()
    }
    fn remove(&mut self, index: usize) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = None;
        }
    }
    fn on_remove(&self) -> fn(&mut World, Entity) {
        T::on_remove
    }
    fn name(&self) -> &'static str {
        type_name::<T>()
    }
}

#[derive(Clone, Copy, Debug)]
struct Meta {
    generation: u32,
    alive: bool,
}

struct ResourceSlot {
    value: RefCell<Box<dyn Any>>,
    added: Cell<u32>,
    changed: Cell<u32>,
}

/// A component's hook.
type Hook = fn(&mut World, Entity);

/// A change to the world, made later (by [`Commands`](super::Commands)).
pub(crate) type Command = Box<dyn FnOnce(&mut World)>;

/// The entities, their components and the resources.
pub struct World {
    meta: RefCell<Vec<Meta>>,
    free: RefCell<Vec<u32>>,
    columns: HashMap<TypeId, Box<dyn AnyColumn>>,
    resources: HashMap<TypeId, ResourceSlot>,
    pub(crate) commands: RefCell<Vec<Command>>,
    tick: Cell<u32>,
    pub(crate) observers: super::message::Observers,
}

impl Default for World {
    fn default() -> World {
        World::new()
    }
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut components: Vec<_> = self.columns.values().map(|c| c.name()).collect();
        components.sort_unstable();
        f.debug_struct("World")
            .field("entities", &self.entity_count())
            .field("components", &components)
            .field("resources", &self.resources.len())
            .finish()
    }
}

impl World {
    /// An empty world.
    pub fn new() -> World {
        World {
            meta: RefCell::new(Vec::new()),
            free: RefCell::new(Vec::new()),
            columns: HashMap::new(),
            resources: HashMap::new(),
            commands: RefCell::new(Vec::new()),
            tick: Cell::new(1),
            observers: Default::default(),
        }
    }

    // ----- Ticks ------------------------------------------------------------------------

    /// The current change tick.
    pub(crate) fn tick(&self) -> u32 {
        self.tick.get()
    }

    /// Moves to the next change tick (each system run has its own).
    pub(crate) fn increment_tick(&self) -> u32 {
        let next = self.tick.get().wrapping_add(1).max(1);
        self.tick.set(next);
        next
    }

    // ----- Entities ---------------------------------------------------------------------

    /// Takes an entity id now, to spawn it later (what `commands.spawn` returns).
    pub(crate) fn reserve(&self) -> Entity {
        if let Some(index) = self.free.borrow_mut().pop() {
            let generation = self.meta.borrow()[index as usize].generation;
            return Entity { index, generation };
        }
        let mut meta = self.meta.borrow_mut();
        let index = u32::try_from(meta.len()).expect("too many entities");
        meta.push(Meta {
            generation: 0,
            alive: false,
        });
        Entity {
            index,
            generation: 0,
        }
    }

    /// Makes a reserved entity alive.
    pub(crate) fn activate(&mut self, entity: Entity) {
        let meta = &mut self.meta.get_mut()[entity.index as usize];
        if meta.generation == entity.generation {
            meta.alive = true;
        }
    }

    /// A new entity with the components of `bundle` (a component, or a tuple of them).
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorldMut<'_> {
        let entity = self.reserve();
        self.activate(entity);
        self.insert(entity, bundle);
        EntityWorldMut {
            world: self,
            entity,
        }
    }

    /// A new entity without components.
    pub fn spawn_empty(&mut self) -> EntityWorldMut<'_> {
        self.spawn(())
    }

    /// Whether the entity exists.
    pub fn contains(&self, entity: Entity) -> bool {
        self.meta
            .borrow()
            .get(entity.index as usize)
            .is_some_and(|m| m.alive && m.generation == entity.generation)
    }

    /// The live entity at this index, if any.
    pub(crate) fn entity_at(&self, index: usize) -> Option<Entity> {
        let meta = self.meta.borrow();
        let m = meta.get(index)?;
        m.alive.then_some(Entity {
            index: index as u32,
            generation: m.generation,
        })
    }

    /// One more than the largest entity index.
    pub(crate) fn entity_capacity(&self) -> usize {
        self.meta.borrow().len()
    }

    /// How many entities exist.
    pub fn entity_count(&self) -> usize {
        self.meta.borrow().iter().filter(|m| m.alive).count()
    }

    /// The entity, to read or change its components; `None` if it does not exist.
    pub fn get_entity_mut(&mut self, entity: Entity) -> Option<EntityWorldMut<'_>> {
        self.contains(entity).then_some(EntityWorldMut {
            world: self,
            entity,
        })
    }

    /// The entity, to read or change its components.
    ///
    /// # Panics
    ///
    /// If it does not exist.
    pub fn entity_mut(&mut self, entity: Entity) -> EntityWorldMut<'_> {
        assert!(self.contains(entity), "entity {entity} does not exist");
        EntityWorldMut {
            world: self,
            entity,
        }
    }

    /// Removes the entity, its components and its children; `false` if it did not exist.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.contains(entity) {
            return false;
        }
        if let Some(children) = self.get::<super::Children>(entity).map(|c| c.to_vec()) {
            for child in children {
                self.despawn(child);
            }
        }
        let index = entity.index as usize;
        let hooks: Vec<(TypeId, Hook)> = self
            .columns
            .iter()
            .filter(|(_, c)| c.contains(index))
            .map(|(id, c)| (*id, c.on_remove()))
            .collect();
        for (id, hook) in hooks {
            hook(self, entity);
            if let Some(column) = self.columns.get_mut(&id) {
                column.remove(index);
            }
        }
        let meta = &mut self.meta.get_mut()[index];
        meta.alive = false;
        meta.generation = meta.generation.wrapping_add(1);
        self.free.get_mut().push(entity.index);
        true
    }

    // ----- Components -------------------------------------------------------------------

    pub(crate) fn column<T: Component>(&self) -> Option<&Column<T>> {
        self.columns
            .get(&TypeId::of::<T>())
            .and_then(|c| c.as_any().downcast_ref())
    }

    /// Adds the components of `bundle` to the entity (replacing those it has).
    ///
    /// # Panics
    ///
    /// If the entity does not exist.
    pub fn insert(&mut self, entity: Entity, bundle: impl Bundle) {
        assert!(
            self.contains(entity),
            "cannot insert components into {entity}: it does not exist"
        );
        let mut hooks = Vec::new();
        bundle.insert_into(self, entity, &mut hooks);
        for hook in hooks {
            if self.contains(entity) {
                hook(self, entity);
            }
        }
    }

    /// Puts one component in place, and tells which hook to run after.
    pub(crate) fn insert_one<T: Component>(&mut self, entity: Entity, value: T) {
        let index = entity.index as usize;
        if self.has::<T>(entity) {
            T::on_remove(self, entity);
        }
        let tick = self.increment_tick();
        let column = self
            .columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Column::<T> { slots: Vec::new() }))
            .as_any_mut()
            .downcast_mut::<Column<T>>()
            .expect("a column has the type of its key");
        if column.slots.len() <= index {
            column.slots.resize_with(index + 1, || None);
        }
        match &mut column.slots[index] {
            Some(slot) => {
                *slot.value.get_mut() = value;
                slot.changed.set(tick);
            }
            empty => *empty = Some(Slot::new(value, tick)),
        }
    }

    /// Removes a component from the entity and gives it back.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.has::<T>(entity) || !self.contains(entity) {
            return None;
        }
        T::on_remove(self, entity);
        let column = self
            .columns
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<Column<T>>()?;
        column
            .slots
            .get_mut(entity.index as usize)?
            .take()
            .map(|slot| slot.value.into_inner())
    }

    /// Whether the entity has the component.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.contains(entity)
            && self
                .column::<T>()
                .is_some_and(|c| c.get(entity.index as usize).is_some())
    }

    /// The entity's component, to read.
    ///
    /// # Panics
    ///
    /// If a system is changing it at the same time.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        if !self.contains(entity) {
            return None;
        }
        let slot = self.column::<T>()?.get(entity.index as usize)?;
        Some(slot.value.try_borrow().unwrap_or_else(|_| conflict::<T>()))
    }

    /// The entity's component, to change (marks it changed).
    pub fn get_mut<T: Component>(&self, entity: Entity) -> Option<RefMut<'_, T>> {
        if !self.contains(entity) {
            return None;
        }
        let slot = self.column::<T>()?.get(entity.index as usize)?;
        // A new tick: systems that ran at the current one see the change.
        slot.changed.set(self.increment_tick());
        Some(
            slot.value
                .try_borrow_mut()
                .unwrap_or_else(|_| conflict::<T>()),
        )
    }

    // ----- Resources --------------------------------------------------------------------

    /// Adds a resource, or replaces it.
    pub fn insert_resource<R: Resource>(&mut self, value: R) {
        let tick = self.increment_tick();
        match self.resources.get_mut(&TypeId::of::<R>()) {
            Some(slot) => {
                *slot.value.get_mut() = Box::new(value);
                slot.changed.set(tick);
            }
            None => {
                self.resources.insert(
                    TypeId::of::<R>(),
                    ResourceSlot {
                        value: RefCell::new(Box::new(value)),
                        added: Cell::new(tick),
                        changed: Cell::new(tick),
                    },
                );
            }
        }
    }

    /// Adds the default value of a resource, unless it is there already.
    pub fn init_resource<R: Resource + Default>(&mut self) {
        if !self.contains_resource::<R>() {
            self.insert_resource(R::default());
        }
    }

    /// Whether the resource is there.
    pub fn contains_resource<R: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<R>())
    }

    /// Removes a resource and gives it back.
    pub fn remove_resource<R: Resource>(&mut self) -> Option<R> {
        let slot = self.resources.remove(&TypeId::of::<R>())?;
        slot.value.into_inner().downcast::<R>().ok().map(|b| *b)
    }

    /// A resource, to read.
    ///
    /// # Panics
    ///
    /// If it is missing (`init_resource`, `insert_resource`), or being changed.
    pub fn resource<R: Resource>(&self) -> Ref<'_, R> {
        self.get_resource().unwrap_or_else(|| missing::<R>())
    }

    /// A resource, to change.
    ///
    /// # Panics
    ///
    /// If it is missing, or borrowed elsewhere.
    pub fn resource_mut<R: Resource>(&self) -> RefMut<'_, R> {
        self.get_resource_mut().unwrap_or_else(|| missing::<R>())
    }

    /// A resource, if it is there.
    pub fn get_resource<R: Resource>(&self) -> Option<Ref<'_, R>> {
        let slot = self.resources.get(&TypeId::of::<R>())?;
        let value = slot
            .value
            .try_borrow()
            .unwrap_or_else(|_| resource_conflict::<R>());
        Some(Ref::map(value, |b| {
            b.downcast_ref::<R>()
                .expect("a resource has the type of its key")
        }))
    }

    /// A resource to change (marked changed), if it is there.
    pub fn get_resource_mut<R: Resource>(&self) -> Option<RefMut<'_, R>> {
        let slot = self.resources.get(&TypeId::of::<R>())?;
        // A new tick: systems that ran at the current one see the change.
        slot.changed.set(self.increment_tick());
        let value = slot
            .value
            .try_borrow_mut()
            .unwrap_or_else(|_| resource_conflict::<R>());
        Some(RefMut::map(value, |b| {
            b.downcast_mut::<R>()
                .expect("a resource has the type of its key")
        }))
    }

    /// When the resource was added and last changed.
    pub(crate) fn resource_ticks<R: Resource>(&self) -> Option<(&Cell<u32>, &Cell<u32>)> {
        self.resources
            .get(&TypeId::of::<R>())
            .map(|s| (&s.added, &s.changed))
    }

    // ----- Commands ---------------------------------------------------------------------

    /// Applies the changes queued by `Commands`, in order (with the ones they queue).
    pub fn flush(&mut self) {
        loop {
            let commands = std::mem::take(self.commands.get_mut());
            if commands.is_empty() {
                break;
            }
            // Changes made now are newer than the systems that asked for them.
            self.increment_tick();
            for command in commands {
                command(self);
            }
        }
    }

    /// Queries the world directly, outside systems (every component counts as changed).
    ///
    /// ```
    /// use active::ecs::prelude::*;
    ///
    /// struct Name(&'static str);
    /// active::component!(Name);
    ///
    /// let mut world = World::new();
    /// world.spawn(Name("Ada"));
    /// let names: Vec<_> = world.query::<&Name>().iter(&world).map(|n| n.0).collect();
    /// assert_eq!(names, ["Ada"]);
    /// ```
    pub fn query<D: super::QueryData>(&self) -> super::QueryState<D, ()> {
        super::QueryState::new()
    }

    /// The same, with a filter (`With<T>`, `Without<T>`…).
    pub fn query_filtered<D: super::QueryData, F: super::QueryFilter>(
        &self,
    ) -> super::QueryState<D, F> {
        super::QueryState::new()
    }
}

#[cold]
fn conflict<T>() -> ! {
    panic!(
        "the component `{}` of an entity is used twice at the same time (two queries or a query and \
         `World::get_mut`): keep the queries apart with `With`/`Without`, or end the first borrow sooner",
        type_name::<T>()
    )
}

#[cold]
fn missing<R>() -> ! {
    panic!(
        "the resource `{}` does not exist: add it with `app.init_resource::<{0}>()` or `insert_resource`, \
         or ask for `Option<Res<{0}>>`",
        type_name::<R>()
    )
}

#[cold]
fn resource_conflict<R>() -> ! {
    panic!(
        "the resource `{}` is used twice at the same time (`Res` and `ResMut` in one system?)",
        type_name::<R>()
    )
}

/// An entity of the world, to change directly.
pub struct EntityWorldMut<'w> {
    world: &'w mut World,
    entity: Entity,
}

impl EntityWorldMut<'_> {
    /// Its id.
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Adds components.
    pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
        self.world.insert(self.entity, bundle);
        self
    }

    /// Removes a component.
    pub fn remove<T: Component>(&mut self) -> &mut Self {
        self.world.remove::<T>(self.entity);
        self
    }

    /// One of its components.
    pub fn get<T: Component>(&self) -> Option<Ref<'_, T>> {
        self.world.get(self.entity)
    }

    /// One of its components, to change.
    pub fn get_mut<T: Component>(&mut self) -> Option<RefMut<'_, T>> {
        self.world.get_mut(self.entity)
    }

    /// Whether it has the component.
    pub fn contains<T: Component>(&self) -> bool {
        self.world.has::<T>(self.entity)
    }

    /// Removes it, with its children.
    pub fn despawn(self) {
        self.world.despawn(self.entity);
    }

    /// The world.
    pub fn world(&mut self) -> &mut World {
        self.world
    }
}

/// Components inserted together: a component, or a tuple of bundles (nested as needed).
pub trait Bundle: 'static {
    /// Puts the components in place, and collects the hooks to run once they all are.
    #[doc(hidden)]
    fn insert_into(
        self,
        world: &mut World,
        entity: Entity,
        hooks: &mut Vec<fn(&mut World, Entity)>,
    );
}

impl<C: Component> Bundle for C {
    fn insert_into(
        self,
        world: &mut World,
        entity: Entity,
        hooks: &mut Vec<fn(&mut World, Entity)>,
    ) {
        world.insert_one(entity, self);
        hooks.push(C::on_insert);
    }
}

macro_rules! bundle_tuple {
    ($($b:ident),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<$($b: Bundle),*> Bundle for ($($b,)*) {
            fn insert_into(self, world: &mut World, entity: Entity, hooks: &mut Vec<fn(&mut World, Entity)>) {
                let ($($b,)*) = self;
                $($b.insert_into(world, entity, hooks);)*
            }
        }
    };
}

bundle_tuple!();
bundle_tuple!(B0);
bundle_tuple!(B0, B1);
bundle_tuple!(B0, B1, B2);
bundle_tuple!(B0, B1, B2, B3);
bundle_tuple!(B0, B1, B2, B3, B4);
bundle_tuple!(B0, B1, B2, B3, B4, B5);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6, B7);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6, B7, B8);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6, B7, B8, B9);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6, B7, B8, B9, B10);
bundle_tuple!(B0, B1, B2, B3, B4, B5, B6, B7, B8, B9, B10, B11);

/// Inserts `T::default()` unless the entity has a `T` (the required components of Bevy).
pub(crate) fn require<T: Component + Default>(world: &mut World, entity: Entity) {
    if !world.has::<T>(entity) {
        world.insert(entity, T::default());
    }
}
