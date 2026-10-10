//! Fine-grained reactivity: [`use_state`] and [`effect`].
//!
//! A [`State`] is a small `Copy` handle you can move into any closure. Reading it inside an
//! [`effect`] subscribes the effect; writing it re-runs the effects that read it — and only those: there is no virtual DOM and no re-render.
//!
//! Values live in a per-thread arena. On the server, a state is released at the end of the
//! second top-level render that does not read it, so a long-running server never accumulates
//! state, and a view can still be rendered as many times as needed. In the browser, states and
//! effects created while rendering a dynamic list are released when the list re-renders.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    index: usize,
    version: u64,
}

struct StateSlot {
    version: u64,
    value: Option<Rc<RefCell<Box<dyn Any>>>>,
    subscribers: Vec<Key>,
    /// Server: the top-level render that last read it (or the one before its creation).
    used: u64,
}

struct EffectSlot {
    version: u64,
    run: Option<Rc<dyn Fn()>>,
    dependencies: Vec<Key>,
    running: bool,
}

enum Owned {
    State(Key),
    #[cfg_attr(not(any(feature = "client", test)), allow(dead_code))]
    Effect(Key),
    Value(Box<dyn Any>),
}

#[derive(Default)]
struct Runtime {
    states: RefCell<Vec<StateSlot>>,
    free_states: RefCell<Vec<usize>>,
    effects: RefCell<Vec<EffectSlot>>,
    free_effects: RefCell<Vec<usize>>,
    observer: Cell<Option<Key>>,
    owners: RefCell<Vec<Vec<Owned>>>,
    versions: Cell<u64>,
    render_depth: Cell<u32>,
    /// Server: number of top-level renders started on this thread.
    renders: Cell<u64>,
}

thread_local! {
    static RUNTIME: Runtime = Runtime::default();
}

impl Runtime {
    fn next_version(&self) -> u64 {
        let v = self.versions.get() + 1;
        self.versions.set(v);
        v
    }

    fn own(&self, item: Owned) {
        if let Some(owner) = self.owners.borrow_mut().last_mut() {
            owner.push(item);
        } else if let Owned::Value(value) = item {
            // No owner: lives as long as the page (e.g. an island's event listeners).
            std::mem::forget(value);
        }
    }

    fn cell(&self, key: Key) -> Rc<RefCell<Box<dyn Any>>> {
        let mut states = self.states.borrow_mut();
        match states.get_mut(key.index) {
            Some(slot) if slot.version == key.version => {
                slot.used = self.renders.get();
                slot.value.clone().expect("live state")
            }
            _ => panic!(
                "active: this state was disposed (its owner re-rendered or the server render finished)"
            ),
        }
    }

    /// Subscribes the running effect, if any, to `key`.
    fn track(&self, key: Key) {
        let Some(effect) = self.observer.get() else {
            return;
        };
        let mut states = self.states.borrow_mut();
        let Some(slot) = states
            .get_mut(key.index)
            .filter(|s| s.version == key.version)
        else {
            return;
        };
        let subs = &mut slot.subscribers;
        if !subs.contains(&effect) {
            subs.push(effect);
            drop(states);
            let mut effects = self.effects.borrow_mut();
            if let Some(slot) = effects
                .get_mut(effect.index)
                .filter(|s| s.version == effect.version)
            {
                slot.dependencies.push(key);
            }
        }
    }

    fn notify(&self, key: Key) {
        let subscribers = match self.states.borrow().get(key.index) {
            Some(slot) if slot.version == key.version => slot.subscribers.clone(),
            _ => return,
        };
        for effect in subscribers {
            self.run_effect(effect);
        }
    }

    fn run_effect(&self, key: Key) {
        let (run, dependencies) = {
            let mut effects = self.effects.borrow_mut();
            let Some(slot) = effects
                .get_mut(key.index)
                .filter(|s| s.version == key.version)
            else {
                return;
            };
            if slot.running {
                return; // an effect writing a state it reads would loop forever
            }
            slot.running = true;
            (slot.run.clone(), std::mem::take(&mut slot.dependencies))
        };
        // Dependencies are collected again on every run.
        {
            let mut states = self.states.borrow_mut();
            for dep in dependencies {
                if let Some(slot) = states
                    .get_mut(dep.index)
                    .filter(|s| s.version == dep.version)
                {
                    slot.subscribers.retain(|e| *e != key);
                }
            }
        }
        let previous = self.observer.replace(Some(key));
        if let Some(run) = run {
            run();
        }
        self.observer.set(previous);
        if let Some(slot) = self
            .effects
            .borrow_mut()
            .get_mut(key.index)
            .filter(|s| s.version == key.version)
        {
            slot.running = false;
        }
    }

    /// Server: releases the states that neither this top-level render nor the previous one
    /// read (or created). Views rendered again keep theirs; finished requests free theirs.
    fn release_unused(&self) {
        let now = self.renders.get();
        let mut states = self.states.borrow_mut();
        let mut free = self.free_states.borrow_mut();
        for (index, slot) in states.iter_mut().enumerate() {
            if slot.version != 0 && slot.used + 1 < now {
                slot.version = 0;
                slot.value = None;
                slot.subscribers.clear();
                free.push(index);
            }
        }
    }

    fn dispose(&self, items: Vec<Owned>) {
        for item in items {
            match item {
                Owned::State(key) => {
                    let mut states = self.states.borrow_mut();
                    if let Some(slot) = states
                        .get_mut(key.index)
                        .filter(|s| s.version == key.version)
                    {
                        slot.version = 0;
                        slot.value = None;
                        slot.subscribers.clear();
                        self.free_states.borrow_mut().push(key.index);
                    }
                }
                Owned::Effect(key) => {
                    let dependencies = {
                        let mut effects = self.effects.borrow_mut();
                        match effects
                            .get_mut(key.index)
                            .filter(|s| s.version == key.version)
                        {
                            Some(slot) => {
                                slot.version = 0;
                                slot.run = None;
                                self.free_effects.borrow_mut().push(key.index);
                                std::mem::take(&mut slot.dependencies)
                            }
                            None => continue,
                        }
                    };
                    let mut states = self.states.borrow_mut();
                    for dep in dependencies {
                        if let Some(slot) = states
                            .get_mut(dep.index)
                            .filter(|s| s.version == dep.version)
                        {
                            slot.subscribers.retain(|e| *e != key);
                        }
                    }
                }
                Owned::Value(value) => drop(value),
            }
        }
    }
}

/// A reactive value. `Copy`, so it can be moved into as many closures as needed.
///
/// ```
/// use active::use_state;
///
/// let count = use_state(1);
/// count.update(|n| *n += 1);
/// assert_eq!(count.get(), 2);
/// ```
pub struct State<T> {
    key: Key,
    _type: PhantomData<fn() -> T>,
    // The value lives in this thread's arena: the handle must not be sent to another thread
    // (or held across an `.await` in a multi-threaded server).
    _thread: PhantomData<*const ()>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for State<T> {}

impl<T> std::fmt::Debug for State<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("index", &self.key.index)
            .finish()
    }
}

/// Creates a reactive value, owned by the component being built: it is released with it.
pub fn use_state<T: 'static>(initial: T) -> State<T> {
    new_state(initial, true)
}

/// A state that belongs to no component and lives as long as the page (router location).
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub(crate) fn root_state<T: 'static>(initial: T) -> State<T> {
    new_state(initial, false)
}

fn new_state<T: 'static>(initial: T, owned: bool) -> State<T> {
    RUNTIME.with(|rt| {
        let version = rt.next_version();
        let value = Some(Rc::new(RefCell::new(Box::new(initial) as Box<dyn Any>)));
        let index = match rt.free_states.borrow_mut().pop() {
            Some(index) => {
                rt.states.borrow_mut()[index] = StateSlot {
                    version,
                    value,
                    subscribers: Vec::new(),
                    used: rt.renders.get(),
                };
                index
            }
            None => {
                let mut states = rt.states.borrow_mut();
                states.push(StateSlot {
                    version,
                    value,
                    subscribers: Vec::new(),
                    used: rt.renders.get(),
                });
                states.len() - 1
            }
        };
        let key = Key { index, version };
        if owned {
            rt.own(Owned::State(key));
        }
        State {
            key,
            _type: PhantomData,
            _thread: PhantomData,
        }
    })
}

impl<T: 'static> State<T> {
    /// Reads the value through a reference (subscribes the running effect).
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let cell = RUNTIME.with(|rt| {
            let cell = rt.cell(self.key);
            rt.track(self.key);
            cell
        });
        let value = cell.try_borrow().unwrap_or_else(|_| {
            panic!("active: a state was read inside its own `update`; read it before updating")
        });
        f(value.downcast_ref::<T>().expect("state type"))
    }

    /// A clone of the value (subscribes the running effect).
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// Replaces the value and re-runs the effects that read it.
    pub fn set(self, value: T) {
        self.update(|v| *v = value);
    }

    /// Changes the value in place and re-runs the effects that read it.
    pub fn update(self, f: impl FnOnce(&mut T)) {
        let cell = RUNTIME.with(|rt| rt.cell(self.key));
        {
            let mut value = cell.try_borrow_mut().unwrap_or_else(|_| {
                panic!("active: a state was updated while being read or updated")
            });
            f(value.downcast_mut::<T>().expect("state type"));
        }
        RUNTIME.with(|rt| rt.notify(self.key));
    }
}

/// Runs `f` now, then again whenever a [`State`] it read changes.
///
/// Effects only run in the browser (crate feature `client`): on the server, a view is
/// rendered once and `effect` does nothing, so effects may freely touch the DOM.
#[cfg_attr(not(any(feature = "client", test)), allow(unused_variables))]
pub fn effect(f: impl Fn() + 'static) {
    #[cfg(any(feature = "client", test))]
    RUNTIME.with(|rt| {
        let version = rt.next_version();
        let slot = EffectSlot {
            version,
            run: Some(Rc::new(f)),
            dependencies: Vec::new(),
            running: false,
        };
        let index = match rt.free_effects.borrow_mut().pop() {
            Some(index) => {
                rt.effects.borrow_mut()[index] = slot;
                index
            }
            None => {
                let mut effects = rt.effects.borrow_mut();
                effects.push(slot);
                effects.len() - 1
            }
        };
        let key = Key { index, version };
        rt.own(Owned::Effect(key));
        rt.run_effect(key);
    });
}

/// Keeps `value` alive as long as the current scope (used for DOM event closures).
#[allow(dead_code)]
pub(crate) fn keep_alive(value: Box<dyn Any>) {
    RUNTIME.with(|rt| rt.own(Owned::Value(value)));
}

/// Runs `f` without subscribing the running effect to the states `f` reads (building a
/// list row must not make the whole list depend on the row's own states).
#[cfg_attr(not(feature = "client"), allow(dead_code))]
pub(crate) fn untracked<R>(f: impl FnOnce() -> R) -> R {
    let previous = RUNTIME.with(|rt| rt.observer.replace(None));
    let result = f();
    RUNTIME.with(|rt| rt.observer.set(previous));
    result
}

/// Everything created while running [`scope`]: dispose it to release states, effects and
/// event closures.
#[must_use = "a scope frees its states and effects only when disposed"]
pub struct Scope(Vec<Owned>);

impl Scope {
    /// Releases the states, effects and closures created in this scope.
    pub fn dispose(self) {
        RUNTIME.with(|rt| rt.dispose(self.0));
    }
}

/// Runs `f`, collecting the states and effects it creates into a [`Scope`].
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Scope) {
    RUNTIME.with(|rt| rt.owners.borrow_mut().push(Vec::new()));
    let result = f();
    let items = RUNTIME.with(|rt| rt.owners.borrow_mut().pop().unwrap_or_default());
    (result, Scope(items))
}

/// Number of states alive on this thread (tests).
#[cfg(test)]
pub(crate) fn live_states() -> usize {
    RUNTIME.with(|rt| rt.states.borrow().iter().filter(|s| s.version != 0).count())
}

/// Called around renders. Server: the outermost one releases the states that are no longer
/// used (see [`Runtime::release_unused`]).
pub(crate) fn render_guard<R>(f: impl FnOnce() -> R) -> R {
    RUNTIME.with(|rt| {
        let depth = rt.render_depth.get();
        if depth == 0 {
            rt.renders.set(rt.renders.get() + 1);
        }
        rt.render_depth.set(depth + 1);
    });
    let result = f();
    RUNTIME.with(|rt| {
        let depth = rt.render_depth.get() - 1;
        rt.render_depth.set(depth);
        if depth == 0 && !cfg!(feature = "client") && rt.owners.borrow().is_empty() {
            rt.release_unused();
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_rerun_when_their_states_change() {
        let count = use_state(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        effect(move || log.borrow_mut().push(count.get()));
        count.set(1);
        count.update(|n| *n += 1);
        assert_eq!(*seen.borrow(), [0, 1, 2]);
    }

    #[test]
    #[should_panic(expected = "inside its own `update`")]
    fn reading_a_state_inside_its_own_update_explains_the_mistake() {
        let n = use_state(1);
        n.update(|v| *v += n.get());
    }

    #[test]
    fn effects_track_dependencies_dynamically() {
        let flag = use_state(true);
        let a = use_state("a");
        let b = use_state("b");
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        effect(move || {
            r.set(r.get() + 1);
            let _ = if flag.get() { a.get() } else { b.get() };
        });
        b.set("b2"); // not read yet
        assert_eq!(runs.get(), 1);
        flag.set(false);
        a.set("a2"); // no longer read
        b.set("b3");
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn an_effect_writing_what_it_reads_does_not_loop() {
        let n = use_state(0);
        effect(move || {
            if n.get() < 100 {
                n.update(|v| *v += 1);
            }
        });
        assert_eq!(n.get(), 1);
    }

    #[test]
    fn disposing_a_scope_frees_its_states_and_stops_its_effects() {
        let source = use_state(0);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        let (inner, scope) = scope(|| {
            effect(move || {
                source.get();
                r.set(r.get() + 1);
            });
            use_state(5)
        });
        assert_eq!(inner.get(), 5);
        scope.dispose();
        source.set(1);
        assert_eq!(runs.get(), 1);
        assert!(std::panic::catch_unwind(|| inner.get()).is_err());
        // The slot is reused without confusing old handles with new ones.
        let reused = use_state(9);
        assert_eq!(reused.get(), 9);
        assert!(std::panic::catch_unwind(|| inner.get()).is_err());
    }

    #[test]
    fn states_can_be_read_while_another_is_updated() {
        let a = use_state(2);
        let b = use_state(0);
        b.update(|v| *v = a.get() * 10);
        assert_eq!(b.get(), 20);
    }
}
