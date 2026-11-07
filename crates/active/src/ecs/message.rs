//! Messages, read by systems in later frames, and events, which run their observers at
//! once.

use std::any::{Any, TypeId};
use std::cell::{Ref as CellRef, RefCell, RefMut};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;

use super::system::{SystemMeta, SystemParam, SystemParamItem};
use super::world::{Resource, World};

/// A value systems write and others read, buffered for two frames:
/// `impl Message for Collision {}`. Register it with
/// `app.add_message::<Collision>()`.
pub trait Message: 'static {}

/// Implements [`Message`] for types.
#[macro_export]
macro_rules! message {
    ($($t:ty),+ $(,)?) => { $(impl $crate::ecs::Message for $t {})+ };
}

/// The messages of one type: a resource, made by `app.add_message`.
pub struct Messages<M: Message> {
    /// `(id, message, frame)`.
    queue: Vec<(u64, M, u64)>,
    next_id: u64,
    frame: u64,
}

impl<M: Message> Resource for Messages<M> {}

impl<M: Message> Default for Messages<M> {
    fn default() -> Self {
        Messages {
            queue: Vec::new(),
            next_id: 0,
            frame: 0,
        }
    }
}

impl<M: Message> Messages<M> {
    /// Adds a message.
    pub fn write(&mut self, message: M) {
        self.queue.push((self.next_id, message, self.frame));
        self.next_id += 1;
    }

    /// Next frame: the messages of two frames ago go.
    pub fn update(&mut self) {
        self.frame += 1;
        let frame = self.frame;
        self.queue.retain(|(_, _, written)| frame - written < 2);
    }

    /// The messages kept, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &M> {
        self.queue.iter().map(|(_, m, _)| m)
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Removes them all.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

/// Writes messages, in a system: `mut writer: MessageWriter<Collision>`.
pub struct MessageWriter<'w, M: Message> {
    messages: RefMut<'w, Messages<M>>,
}

impl<M: Message> MessageWriter<'_, M> {
    /// Writes one.
    pub fn write(&mut self, message: M) {
        self.messages.write(message);
    }

    /// Writes them all.
    pub fn write_batch(&mut self, messages: impl IntoIterator<Item = M>) {
        for message in messages {
            self.messages.write(message);
        }
    }
}

#[cold]
fn unregistered<M>(meta: &SystemMeta) -> ! {
    panic!(
        "the system `{}` uses the message `{}`, which is not registered: add `app.add_message::<{1}>()`",
        meta.name,
        std::any::type_name::<M>()
    )
}

impl<M: Message> SystemParam for MessageWriter<'_, M> {
    type State = ();
    type Item<'w, 's> = MessageWriter<'w, M>;
    fn init_state(_: &mut World) {}
    fn get_param<'w, 's>(
        (): &'s mut (),
        world: &'w World,
        meta: &SystemMeta,
    ) -> MessageWriter<'w, M> {
        let messages = world
            .get_resource_mut::<Messages<M>>()
            .unwrap_or_else(|| unregistered::<M>(meta));
        MessageWriter { messages }
    }
}

/// Reads the messages written since this system last read them:
/// `mut reader: MessageReader<Collision>`, then `for message in reader.read()`.
pub struct MessageReader<'w, 's, M: Message> {
    messages: CellRef<'w, Messages<M>>,
    cursor: &'s mut u64,
}

impl<M: Message> MessageReader<'_, '_, M> {
    /// The new messages (each reader sees each message once).
    pub fn read(&mut self) -> impl Iterator<Item = &M> {
        let from = std::mem::replace(self.cursor, self.messages.next_id);
        self.messages
            .queue
            .iter()
            .filter(move |(id, _, _)| *id >= from)
            .map(|(_, m, _)| m)
    }

    /// How many new messages there are.
    pub fn len(&self) -> usize {
        self.messages
            .queue
            .iter()
            .filter(|(id, _, _)| *id >= *self.cursor)
            .count()
    }

    /// Whether there is none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Marks them all read.
    pub fn clear(&mut self) {
        *self.cursor = self.messages.next_id;
    }
}

impl<M: Message> SystemParam for MessageReader<'_, '_, M> {
    type State = u64;
    type Item<'w, 's> = MessageReader<'w, 's, M>;
    fn init_state(_: &mut World) -> u64 {
        0
    }
    fn get_param<'w, 's>(
        cursor: &'s mut u64,
        world: &'w World,
        meta: &SystemMeta,
    ) -> MessageReader<'w, 's, M> {
        let messages = world
            .get_resource::<Messages<M>>()
            .unwrap_or_else(|| unregistered::<M>(meta));
        MessageReader { messages, cursor }
    }
}

// ----- Events and observers -------------------------------------------------------------

/// Something that happens and is handled at once by its observers:
/// `impl Event for Explosion {}`, then `commands.trigger(Explosion
/// { .. })` and `app.add_observer(|explosion: On<Explosion>| ..)`.
pub trait Event: 'static {}

/// Implements [`Event`] for types.
#[macro_export]
macro_rules! event {
    ($($t:ty),+ $(,)?) => { $(impl $crate::ecs::Event for $t {})+ };
}

/// The event an observer handles, its first parameter: use it like a `&E`.
pub struct On<'e, E: Event> {
    event: &'e E,
}

impl<'e, E: Event> On<'e, E> {
    /// The event.
    pub fn event(&self) -> &'e E {
        self.event
    }
}

impl<E: Event> Deref for On<'_, E> {
    type Target = E;
    fn deref(&self) -> &E {
        self.event
    }
}

/// An observer, run when its event is triggered.
pub trait ObserverSystem<E: Event>: 'static {
    /// Runs it, then applies its commands.
    fn run(&mut self, event: &E, world: &mut World);
}

/// Turns a function `fn(On<E>, params…)` into an observer.
pub trait IntoObserverSystem<E: Event, Marker>: Sized {
    /// The observer.
    type System: ObserverSystem<E>;
    /// Makes it.
    fn into_observer(self) -> Self::System;
}

/// A function made into an observer.
pub struct FunctionObserver<E: Event, P: SystemParam, F> {
    func: F,
    state: Option<P::State>,
    meta: SystemMeta,
    last_run: u32,
    _marker: PhantomData<fn() -> E>,
}

macro_rules! observer_function {
    ($($p:ident),*) => {
        #[allow(non_snake_case)]
        impl<E: Event, Func, $($p: SystemParam + 'static),*> IntoObserverSystem<E, fn(E, $($p,)*)> for Func
        where
            Func: 'static,
            for<'a> &'a mut Func: FnMut(On<'_, E>, $($p),*) + FnMut(On<'_, E>, $(SystemParamItem<'_, '_, $p>),*),
        {
            type System = FunctionObserver<E, ($($p,)*), Func>;
            fn into_observer(self) -> Self::System {
                FunctionObserver {
                    func: self,
                    state: None,
                    meta: SystemMeta { name: std::any::type_name::<Func>(), ticks: Default::default() },
                    last_run: 0,
                    _marker: PhantomData,
                }
            }
        }

        #[allow(non_snake_case)]
        impl<E: Event, Func, $($p: SystemParam + 'static),*> ObserverSystem<E> for FunctionObserver<E, ($($p,)*), Func>
        where
            Func: 'static,
            for<'a> &'a mut Func: FnMut(On<'_, E>, $($p),*) + FnMut(On<'_, E>, $(SystemParamItem<'_, '_, $p>),*),
        {
            fn run(&mut self, event: &E, world: &mut World) {
                #[allow(clippy::too_many_arguments)]
                fn call<E: Event, $($p),*>(mut f: impl FnMut(On<'_, E>, $($p),*), on: On<'_, E>, $($p: $p),*) {
                    f(on, $($p),*)
                }
                if self.state.is_none() {
                    self.state = Some(<($($p,)*) as SystemParam>::init_state(world));
                }
                let this_run = world.increment_tick();
                self.meta.ticks = super::query::Ticks { last_run: self.last_run, this_run };
                let state = self.state.as_mut().expect("initialized above");
                if <($($p,)*) as SystemParam>::validate(state, world, &self.meta) {
                    let ($($p,)*) = <($($p,)*) as SystemParam>::get_param(state, world, &self.meta);
                    call(&mut self.func, On { event }, $($p),*);
                }
                self.last_run = this_run;
                world.flush();
            }
        }
    };
}

observer_function!();
observer_function!(P0);
observer_function!(P0, P1);
observer_function!(P0, P1, P2);
observer_function!(P0, P1, P2, P3);
observer_function!(P0, P1, P2, P3, P4);
observer_function!(P0, P1, P2, P3, P4, P5);
observer_function!(P0, P1, P2, P3, P4, P5, P6);

/// The observers of the world, by event type (each a `Vec<Box<dyn ObserverSystem<E>>>`).
#[derive(Default)]
pub(crate) struct Observers {
    by_event: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}

type ObserverList<E> = Vec<Box<dyn ObserverSystem<E>>>;

impl World {
    /// Adds an observer: a function whose first parameter is `On<E>`.
    pub fn add_observer<E: Event, M>(&mut self, observer: impl IntoObserverSystem<E, M>) {
        let mut by_event = self.observers.by_event.borrow_mut();
        let list = by_event
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(ObserverList::<E>::new()));
        if let Some(list) = list.downcast_mut::<ObserverList<E>>() {
            list.push(Box::new(observer.into_observer()));
        }
    }

    /// Runs the observers of the event now, in the order they were added.
    pub fn trigger<E: Event>(&mut self, event: E) {
        let taken = self
            .observers
            .by_event
            .borrow_mut()
            .remove(&TypeId::of::<E>());
        let Some(mut list) = taken.and_then(|l| l.downcast::<ObserverList<E>>().ok()) else {
            return;
        };
        for observer in list.iter_mut() {
            observer.run(&event, self);
        }
        // Put them back, before the ones added while they ran.
        let mut by_event = self.observers.by_event.borrow_mut();
        let slot = by_event
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(ObserverList::<E>::new()));
        if let Some(added) = slot.downcast_mut::<ObserverList<E>>() {
            list.append(added);
            *added = *list;
        }
    }
}
