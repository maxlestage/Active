//! Queries: the entities that have some components, and those components.

use std::cell::{Cell, Ref as CellRef, RefMut};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::world::{Column, Component, Entity, World};

/// The ticks of the running system: what changed since it last ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ticks {
    /// When the system last ran (0: never).
    pub last_run: u32,
    /// This run.
    pub this_run: u32,
}

impl Ticks {
    pub(crate) fn is_newer(self, tick: u32) -> bool {
        tick > self.last_run
    }
}

/// A component to read, from a query of `&T`: use it like a `&T`.
pub struct Ref<'w, T> {
    value: CellRef<'w, T>,
    added: u32,
    changed: u32,
    ticks: Ticks,
}

impl<T> Ref<'_, T> {
    /// Whether it was added since the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_newer(self.added)
    }

    /// Whether it was added or changed since the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_newer(self.changed)
    }
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Ref<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

/// A component to change, from a query of `&mut T`: use it like a `&mut T`. Changing it
/// marks it changed (for `Changed<T>`).
pub struct Mut<'w, T> {
    value: RefMut<'w, T>,
    added: u32,
    changed: &'w Cell<u32>,
    ticks: Ticks,
}

impl<T> Mut<'_, T> {
    /// Whether it was added since the system last ran.
    pub fn is_added(&self) -> bool {
        self.ticks.is_newer(self.added)
    }

    /// Whether it was added or changed since the system last ran.
    pub fn is_changed(&self) -> bool {
        self.ticks.is_newer(self.changed.get())
    }

    /// Marks it changed without changing it.
    pub fn set_changed(&mut self) {
        self.changed.set(self.ticks.this_run);
    }

    /// Changes it without marking it changed.
    pub fn bypass_change_detection(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> Deref for Mut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.changed.set(self.ticks.this_run);
        &mut self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for Mut<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

fn borrow_conflict<T>() -> ! {
    panic!(
        "the component `{}` of an entity is used twice at the same time: two queries of one system reach it \
         (separate them with `With`/`Without`) or an item of the previous iteration is still alive",
        std::any::type_name::<T>()
    )
}

/// What a query gives for each entity: `&T`, `&mut T`, `Entity`, `Option<D>`, `Has<T>`, or a
/// tuple of them.
pub trait QueryData {
    /// The item of one entity.
    type Item<'w>;
    /// What is looked up once per query.
    #[doc(hidden)]
    type Fetch<'w>: Copy;
    #[doc(hidden)]
    fn init(world: &World) -> Self::Fetch<'_>;
    /// Whether the entity at this index has what the item needs.
    #[doc(hidden)]
    fn matches(fetch: Self::Fetch<'_>, index: usize) -> bool;
    /// The item (only called when `matches`).
    #[doc(hidden)]
    fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, ticks: Ticks) -> Self::Item<'w>;
}

impl<T: Component> QueryData for &T {
    type Item<'w> = Ref<'w, T>;
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn matches(fetch: Self::Fetch<'_>, index: usize) -> bool {
        fetch.is_some_and(|c| c.get(index).is_some())
    }
    fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, ticks: Ticks) -> Ref<'w, T> {
        let slot = fetch
            .and_then(|c| c.get(entity.index() as usize))
            .expect("checked by matches");
        let value = slot
            .value
            .try_borrow()
            .unwrap_or_else(|_| borrow_conflict::<T>());
        Ref {
            value,
            added: slot.added.get(),
            changed: slot.changed.get(),
            ticks,
        }
    }
}

impl<T: Component> QueryData for &mut T {
    type Item<'w> = Mut<'w, T>;
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn matches(fetch: Self::Fetch<'_>, index: usize) -> bool {
        fetch.is_some_and(|c| c.get(index).is_some())
    }
    fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, ticks: Ticks) -> Mut<'w, T> {
        let slot = fetch
            .and_then(|c| c.get(entity.index() as usize))
            .expect("checked by matches");
        let value = slot
            .value
            .try_borrow_mut()
            .unwrap_or_else(|_| borrow_conflict::<T>());
        Mut {
            value,
            added: slot.added.get(),
            changed: &slot.changed,
            ticks,
        }
    }
}

impl QueryData for Entity {
    type Item<'w> = Entity;
    type Fetch<'w> = ();
    fn init(_: &World) {}
    fn matches((): (), _: usize) -> bool {
        true
    }
    fn fetch<'w>(_: Self::Fetch<'w>, entity: Entity, _: Ticks) -> Self::Item<'w> {
        entity
    }
}

impl<D: QueryData> QueryData for Option<D> {
    type Item<'w> = Option<D::Item<'w>>;
    type Fetch<'w> = D::Fetch<'w>;
    fn init(world: &World) -> Self::Fetch<'_> {
        D::init(world)
    }
    fn matches(_: Self::Fetch<'_>, _: usize) -> bool {
        true
    }
    fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, ticks: Ticks) -> Self::Item<'w> {
        D::matches(fetch, entity.index() as usize).then(|| D::fetch(fetch, entity, ticks))
    }
}

/// Whether the entity has a `T`, in a query: `Query<(Entity, Has<Player>)>`.
pub struct Has<T>(PhantomData<T>);

impl<T: Component> QueryData for Has<T> {
    type Item<'w> = bool;
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn matches(_: Self::Fetch<'_>, _: usize) -> bool {
        true
    }
    fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, _: Ticks) -> Self::Item<'w> {
        fetch.is_some_and(|c| c.get(entity.index() as usize).is_some())
    }
}

macro_rules! data_tuple {
    ($($d:ident),*) => {
        #[allow(non_snake_case, unused_variables, clippy::unused_unit)]
        impl<$($d: QueryData),*> QueryData for ($($d,)*) {
            type Item<'w> = ($($d::Item<'w>,)*);
            type Fetch<'w> = ($($d::Fetch<'w>,)*);
            fn init(world: &World) -> Self::Fetch<'_> {
                ($($d::init(world),)*)
            }
            fn matches(fetch: Self::Fetch<'_>, index: usize) -> bool {
                let ($($d,)*) = fetch;
                true $(&& $d::matches($d, index))*
            }
            fn fetch<'w>(fetch: Self::Fetch<'w>, entity: Entity, ticks: Ticks) -> Self::Item<'w> {
                let ($($d,)*) = fetch;
                ($($d::fetch($d, entity, ticks),)*)
            }
        }
    };
}

data_tuple!();
data_tuple!(D0);
data_tuple!(D0, D1);
data_tuple!(D0, D1, D2);
data_tuple!(D0, D1, D2, D3);
data_tuple!(D0, D1, D2, D3, D4);
data_tuple!(D0, D1, D2, D3, D4, D5);
data_tuple!(D0, D1, D2, D3, D4, D5, D6);
data_tuple!(D0, D1, D2, D3, D4, D5, D6, D7);

/// Which entities a query keeps: `With<T>`, `Without<T>`, `Changed<T>`, `Added<T>`,
/// `Or<(..)>`, or a tuple of them (all must hold).
pub trait QueryFilter {
    #[doc(hidden)]
    type Fetch<'w>: Copy;
    #[doc(hidden)]
    fn init(world: &World) -> Self::Fetch<'_>;
    #[doc(hidden)]
    fn keeps(fetch: Self::Fetch<'_>, index: usize, ticks: Ticks) -> bool;
}

/// Keeps the entities that have a `T` (without reading it).
pub struct With<T>(PhantomData<T>);
/// Keeps the entities that have no `T`.
pub struct Without<T>(PhantomData<T>);
/// Keeps the entities whose `T` was added or changed since the system last ran.
pub struct Changed<T>(PhantomData<T>);
/// Keeps the entities whose `T` was added since the system last ran.
pub struct Added<T>(PhantomData<T>);
/// Keeps the entities that pass one of the filters: `Or<(With<A>, With<B>)>`.
pub struct Or<F>(PhantomData<F>);

impl<T: Component> QueryFilter for With<T> {
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn keeps(fetch: Self::Fetch<'_>, index: usize, _: Ticks) -> bool {
        fetch.is_some_and(|c| c.get(index).is_some())
    }
}

impl<T: Component> QueryFilter for Without<T> {
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn keeps(fetch: Self::Fetch<'_>, index: usize, _: Ticks) -> bool {
        !fetch.is_some_and(|c| c.get(index).is_some())
    }
}

impl<T: Component> QueryFilter for Changed<T> {
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn keeps(fetch: Self::Fetch<'_>, index: usize, ticks: Ticks) -> bool {
        fetch
            .and_then(|c| c.get(index))
            .is_some_and(|s| ticks.is_newer(s.changed.get()))
    }
}

impl<T: Component> QueryFilter for Added<T> {
    type Fetch<'w> = Option<&'w Column<T>>;
    fn init(world: &World) -> Self::Fetch<'_> {
        world.column::<T>()
    }
    fn keeps(fetch: Self::Fetch<'_>, index: usize, ticks: Ticks) -> bool {
        fetch
            .and_then(|c| c.get(index))
            .is_some_and(|s| ticks.is_newer(s.added.get()))
    }
}

macro_rules! filter_tuple {
    ($($f:ident),*) => {
        #[allow(non_snake_case, unused_variables, clippy::unused_unit)]
        impl<$($f: QueryFilter),*> QueryFilter for ($($f,)*) {
            type Fetch<'w> = ($($f::Fetch<'w>,)*);
            fn init(world: &World) -> Self::Fetch<'_> {
                ($($f::init(world),)*)
            }
            fn keeps(fetch: Self::Fetch<'_>, index: usize, ticks: Ticks) -> bool {
                let ($($f,)*) = fetch;
                true $(&& $f::keeps($f, index, ticks))*
            }
        }

        #[allow(non_snake_case, unused_variables, clippy::unused_unit)]
        impl<$($f: QueryFilter),*> QueryFilter for Or<($($f,)*)> {
            type Fetch<'w> = ($($f::Fetch<'w>,)*);
            fn init(world: &World) -> Self::Fetch<'_> {
                ($($f::init(world),)*)
            }
            fn keeps(fetch: Self::Fetch<'_>, index: usize, ticks: Ticks) -> bool {
                let ($($f,)*) = fetch;
                false $(|| $f::keeps($f, index, ticks))*
            }
        }
    };
}

filter_tuple!();
filter_tuple!(F0);
filter_tuple!(F0, F1);
filter_tuple!(F0, F1, F2);
filter_tuple!(F0, F1, F2, F3);
filter_tuple!(F0, F1, F2, F3, F4);
filter_tuple!(F0, F1, F2, F3, F4, F5);

/// Why [`Query::get`] found nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryEntityError {
    /// The entity does not exist.
    NoSuchEntity(Entity),
    /// It exists, but does not match the query.
    QueryDoesNotMatch(Entity),
}

impl fmt::Display for QueryEntityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryEntityError::NoSuchEntity(e) => write!(f, "the entity {e} does not exist"),
            QueryEntityError::QueryDoesNotMatch(e) => {
                write!(f, "the entity {e} does not match the query")
            }
        }
    }
}

impl std::error::Error for QueryEntityError {}

/// Why [`Query::single`] found nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuerySingleError {
    /// No entity matches.
    NoEntities,
    /// Several do.
    MultipleEntities,
}

impl fmt::Display for QuerySingleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            QuerySingleError::NoEntities => "no entity matches the query",
            QuerySingleError::MultipleEntities => "several entities match the query",
        })
    }
}

impl std::error::Error for QuerySingleError {}

/// The entities that have the components `D` and pass the filter `F`, in a system:
/// `Query<&mut Transform, With<Player>>`.
///
/// ```
/// use active::ecs::prelude::*;
///
/// struct Position(f32);
/// struct Velocity(f32);
/// active::component!(Position, Velocity);
///
/// fn movement(mut query: Query<(&mut Position, &Velocity)>) {
///     for (mut position, velocity) in &mut query {
///         position.0 += velocity.0;
///     }
/// }
///
/// let mut app = App::new();
/// app.add_systems(Update, movement);
/// let ball = app.world_mut().spawn((Position(0.0), Velocity(2.0))).id();
/// app.update();
/// assert_eq!(app.world().get::<Position>(ball).unwrap().0, 2.0);
/// ```
pub struct Query<'w, 's, D: QueryData, F: QueryFilter = ()> {
    world: &'w World,
    ticks: Ticks,
    _marker: QueryMarker<'s, D, F>,
}

impl<'w, D: QueryData, F: QueryFilter> Query<'w, '_, D, F> {
    pub(crate) fn new(world: &'w World, ticks: Ticks) -> Self {
        Query {
            world,
            ticks,
            _marker: PhantomData,
        }
    }

    /// The items, one per matching entity.
    pub fn iter(&self) -> QueryIter<'w, D, F> {
        QueryIter {
            world: self.world,
            data: D::init(self.world),
            filter: F::init(self.world),
            ticks: self.ticks,
            index: 0,
            end: self.world.entity_capacity(),
        }
    }

    /// The items, to change.
    pub fn iter_mut(&mut self) -> QueryIter<'w, D, F> {
        self.iter()
    }

    /// The item of one entity.
    pub fn get(&self, entity: Entity) -> Result<D::Item<'w>, QueryEntityError> {
        if !self.world.contains(entity) {
            return Err(QueryEntityError::NoSuchEntity(entity));
        }
        let (data, filter) = (D::init(self.world), F::init(self.world));
        let index = entity.index() as usize;
        if D::matches(data, index) && F::keeps(filter, index, self.ticks) {
            Ok(D::fetch(data, entity, self.ticks))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch(entity))
        }
    }

    /// The item of one entity, to change.
    pub fn get_mut(&mut self, entity: Entity) -> Result<D::Item<'w>, QueryEntityError> {
        self.get(entity)
    }

    /// The item of the only matching entity.
    pub fn single(&self) -> Result<D::Item<'w>, QuerySingleError> {
        let mut entities = self.entities();
        match (entities.next(), entities.next()) {
            (Some(entity), None) => Ok(D::fetch(D::init(self.world), entity, self.ticks)),
            (None, _) => Err(QuerySingleError::NoEntities),
            (Some(_), Some(_)) => Err(QuerySingleError::MultipleEntities),
        }
    }

    /// The same, to change.
    pub fn single_mut(&mut self) -> Result<D::Item<'w>, QuerySingleError> {
        self.single()
    }

    /// Whether no entity matches.
    pub fn is_empty(&self) -> bool {
        self.entities().next().is_none()
    }

    /// Whether the entity matches.
    pub fn contains(&self, entity: Entity) -> bool {
        self.world.contains(entity) && {
            let index = entity.index() as usize;
            D::matches(D::init(self.world), index)
                && F::keeps(F::init(self.world), index, self.ticks)
        }
    }

    /// How many entities match (without borrowing their components).
    pub fn count(&self) -> usize {
        self.entities().count()
    }

    /// The matching entities, without their components.
    fn entities(&self) -> impl Iterator<Item = Entity> {
        let (world, ticks) = (self.world, self.ticks);
        let (data, filter) = (D::init(world), F::init(world));
        (0..world.entity_capacity()).filter_map(move |index| {
            let entity = world.entity_at(index)?;
            (D::matches(data, index) && F::keeps(filter, index, ticks)).then_some(entity)
        })
    }
}

type QueryMarker<'s, D, F> = PhantomData<(&'s (), fn() -> (D, F))>;

/// The items of a query.
pub struct QueryIter<'w, D: QueryData, F: QueryFilter> {
    world: &'w World,
    data: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    ticks: Ticks,
    index: usize,
    end: usize,
}

impl<'w, D: QueryData, F: QueryFilter> Iterator for QueryIter<'w, D, F> {
    type Item = D::Item<'w>;

    fn next(&mut self) -> Option<D::Item<'w>> {
        while self.index < self.end {
            let index = self.index;
            self.index += 1;
            let Some(entity) = self.world.entity_at(index) else {
                continue;
            };
            if D::matches(self.data, index) && F::keeps(self.filter, index, self.ticks) {
                return Some(D::fetch(self.data, entity, self.ticks));
            }
        }
        None
    }
}

impl<'w, D: QueryData, F: QueryFilter> IntoIterator for &Query<'w, '_, D, F> {
    type Item = D::Item<'w>;
    type IntoIter = QueryIter<'w, D, F>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, D: QueryData, F: QueryFilter> IntoIterator for &mut Query<'w, '_, D, F> {
    type Item = D::Item<'w>;
    type IntoIter = QueryIter<'w, D, F>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A query made outside systems, by [`World::query`].
pub struct QueryState<D: QueryData, F: QueryFilter = ()> {
    _marker: PhantomData<fn() -> (D, F)>,
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    pub(crate) fn new() -> Self {
        QueryState {
            _marker: PhantomData,
        }
    }

    /// The query on this world.
    pub fn query<'w>(&self, world: &'w World) -> Query<'w, 'static, D, F> {
        Query::new(
            world,
            Ticks {
                last_run: 0,
                this_run: world.tick(),
            },
        )
    }

    /// Its items.
    pub fn iter<'w>(&self, world: &'w World) -> QueryIter<'w, D, F> {
        self.query(world).iter()
    }

    /// Its items, to change.
    pub fn iter_mut<'w>(&self, world: &'w mut World) -> QueryIter<'w, D, F> {
        self.query(world).iter()
    }

    /// The item of one entity.
    pub fn get<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<D::Item<'w>, QueryEntityError> {
        self.query(world).get(entity)
    }

    /// The item of the only matching entity.
    pub fn single<'w>(&self, world: &'w World) -> Result<D::Item<'w>, QuerySingleError> {
        self.query(world).single()
    }
}

/// The item of the one entity a query matches, as a system parameter: the system is
/// skipped unless exactly one entity matches (`Single<&mut Transform, With<Player>>`).
pub struct Single<'w, D: QueryData, F: QueryFilter = ()> {
    pub(crate) item: D::Item<'w>,
    pub(crate) _filter: PhantomData<fn() -> F>,
}

impl<'w, D: QueryData, F: QueryFilter> Single<'w, D, F> {
    /// The item itself.
    pub fn into_inner(self) -> D::Item<'w> {
        self.item
    }
}

impl<'w, D: QueryData, F: QueryFilter> Deref for Single<'w, D, F> {
    type Target = D::Item<'w>;
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl<D: QueryData, F: QueryFilter> DerefMut for Single<'_, D, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}
