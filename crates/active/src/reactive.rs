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
    Cleanup(Box<dyn FnOnce()>),
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
        } else {
            match item {
                // No owner: lives as long as the page (e.g. an island's event listeners).
                Owned::Value(value) => std::mem::forget(value),
                // Never disposed, so never run.
                Owned::Cleanup(f) => std::mem::forget(f),
                Owned::State(_) | Owned::Effect(_) => {}
            }
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
        // Cleanups first, last registered first: they can still read the scope's states.
        let (cleanups, items): (Vec<Owned>, Vec<Owned>) = items
            .into_iter()
            .partition(|item| matches!(item, Owned::Cleanup(_)));
        for cleanup in cleanups.into_iter().rev() {
            if let Owned::Cleanup(f) = cleanup {
                f();
            }
        }
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
                Owned::Cleanup(_) => {}
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
    /// Whether the state still exists: `false` once its owner was disposed (the dynamic
    /// region that created it re-rendered, or the page changed). Check it before writing
    /// from code that may outlive the view, such as the end of a network request.
    ///
    /// ```
    /// use active::{scope, use_state};
    ///
    /// let (count, owner) = scope(|| use_state(0));
    /// assert!(count.is_alive());
    /// owner.dispose();
    /// assert!(!count.is_alive());
    /// ```
    pub fn is_alive(self) -> bool {
        RUNTIME.with(|rt| {
            rt.states
                .borrow()
                .get(self.key.index)
                .is_some_and(|slot| slot.version == self.key.version)
        })
    }

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

/// A state computed from other states, updated only when its value really changes.
///
/// A closure such as `move || total(cart.get())` re-runs everything that reads it whenever
/// `cart` changes. A memo stores the result instead: what reads the memo re-runs only when
/// the new result differs (`PartialEq`) from the previous one. Use it when the inputs change
/// often but the result rarely does, for instance the list of names shown in a menu while
/// positions update every second.
///
/// ```
/// use active::{memo, use_state};
///
/// let scores = use_state(vec![3, 9, 4]);
/// let best = memo(move || scores.with(|s| s.iter().copied().max()));
/// assert_eq!(best.get(), Some(9));
/// ```
///
/// On the server, effects do not run: the memo keeps the value computed when it was created.
pub fn memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> State<T> {
    let state = use_state(untrack(&f));
    effect(move || {
        let next = f();
        untrack(|| {
            if state.with(|current| *current != next) {
                state.set(next);
            }
        });
    });
    state
}

/// Runs `f` when the component being built is released: the [`fragment_dyn`] region that
/// created it re-renders, the router shows another page, or its [`Scope`] is disposed. Stop
/// here what the component started outside the view: a timer, a socket, an animation loop, a
/// listener on `window`.
///
/// Cleanups run before the component's states are released, so they can still read them.
/// On the server, and outside any component, `f` never runs.
///
/// ```
/// use std::{cell::Cell, rc::Rc};
/// use active::{on_cleanup, scope};
///
/// let stopped = Rc::new(Cell::new(false));
/// let flag = stopped.clone();
/// let ((), owner) = scope(|| on_cleanup(move || flag.set(true)));
/// assert!(!stopped.get());
/// owner.dispose();
/// assert!(stopped.get());
/// ```
///
/// [`fragment_dyn`]: crate::fragment_dyn
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    RUNTIME.with(|rt| rt.own(Owned::Cleanup(Box::new(f))));
}

/// Keeps `value` alive as long as the current scope (used for DOM event closures).
#[allow(dead_code)]
pub(crate) fn keep_alive(value: Box<dyn Any>) {
    RUNTIME.with(|rt| rt.own(Owned::Value(value)));
}

/// Runs `f` without subscribing the running effect or dynamic region to the states `f`
/// reads.
///
/// A component reads its states inside closures (`text_dyn`, `children_dyn`…), so that only
/// that part of the page follows them. Wrap code that reads a state once, to build
/// something, in `untrack`: reading it directly inside a [`fragment_dyn`] would rebuild the
/// whole region, and recreate the component, every time the state changes.
///
/// ```
/// use active::{fragment_dyn, html::*, untrack, use_state};
///
/// let (page, step) = (use_state(1), use_state(10));
/// // Rebuilt when `page` changes; reads `step` once, without following it.
/// let view = div().child(fragment_dyn(move || {
///     let first = untrack(|| step.get());
///     vec![p().text(format!("Page {}, from {first}", page.get())).into()]
/// }));
/// ```
///
/// [`fragment_dyn`]: crate::fragment_dyn
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
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
    fn cleanups_run_once_at_dispose_and_can_read_their_states() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        let ((), owner) = scope(|| {
            let n = use_state(7);
            let first = l.clone();
            on_cleanup(move || first.borrow_mut().push(format!("first {}", n.get())));
            on_cleanup(move || l.borrow_mut().push("second".to_string()));
        });
        assert!(log.borrow().is_empty());
        owner.dispose();
        assert_eq!(*log.borrow(), ["second", "first 7"]);
    }

    #[test]
    fn a_memo_notifies_only_when_its_value_changes() {
        let n = use_state(1);
        let parity = memo(move || n.get() % 2);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        effect(move || {
            parity.get();
            r.set(r.get() + 1);
        });
        n.set(3); // still odd
        assert_eq!(runs.get(), 1);
        n.set(4);
        assert_eq!((parity.get(), runs.get()), (0, 2));
    }

    #[test]
    fn untracked_reads_do_not_subscribe_the_running_effect() {
        let (a, b) = (use_state(1), use_state(10));
        let runs = Rc::new(Cell::new(0));
        let counter = runs.clone();
        effect(move || {
            counter.set(counter.get() + 1);
            let _sum = a.get() + untrack(|| b.get());
        });
        b.set(20);
        assert_eq!(runs.get(), 1);
        a.set(2);
        assert_eq!(runs.get(), 2);
    }

    #[test]
    fn a_disposed_state_is_no_longer_alive_even_when_its_slot_is_reused() {
        let (old, owner) = scope(|| use_state(1));
        assert!(old.is_alive());
        owner.dispose();
        let reused = use_state(2);
        assert!(!old.is_alive());
        assert!(reused.is_alive());
    }

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
