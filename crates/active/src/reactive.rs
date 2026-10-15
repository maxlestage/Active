//! Fine-grained reactivity: [`use_state`], [`memo`] and [`effect`].
//!
//! A [`State`] is a small `Copy` handle you can move into any closure. Reading it inside an
//! [`effect`] subscribes the effect; writing it re-runs the effects that read it — and only
//! those: there is no virtual DOM and no re-render.
//!
//! Changes propagate without glitches: when a state changes, the [`memo`]s computed from it
//! are brought up to date before the effects that read them run, each memo only if one of
//! its inputs really changed, and each effect at most once per change. A state written inside
//! an effect notifies its readers once that effect has finished.
//!
//! Values live in a per-thread arena. On the server, a state is released at the end of the
//! second top-level render that does not read it, so a long-running server never accumulates
//! state, and a view can still be rendered as many times as needed. In the browser, states and
//! effects created while rendering a dynamic list are released when the list re-renders.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

/// An effect that ran this many times in one propagation is stopped: two effects are writing
/// what the other reads.
const MAX_RUNS: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    index: usize,
    version: u64,
}

/// Whether an effect (or a memo) may be out of date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
    /// Up to date.
    Clean,
    /// A memo it reads may have changed: the memo is refreshed before the effect runs, and
    /// the effect runs only if the memo's value changed.
    Check,
    /// A state it reads changed: it runs.
    Dirty,
}

struct StateSlot {
    version: u64,
    value: Option<Rc<RefCell<Box<dyn Any>>>>,
    subscribers: Vec<Key>,
    /// Server: the top-level render that last read it (or the one before its creation).
    used: u64,
    /// The memo computing this state, if any: brought up to date before each read.
    derived_by: Option<Key>,
}

struct EffectSlot {
    version: u64,
    run: Option<Rc<dyn Fn()>>,
    dependencies: Vec<Key>,
    running: bool,
    /// A memo: the state it computes.
    derived: Option<Key>,
    status: Status,
    /// Waiting in the run queue.
    queued: bool,
    /// Runs during the propagation `propagation` (see [`MAX_RUNS`]).
    runs: u32,
    propagation: u64,
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
    /// Effects marked by a change, in marking order, to run once the running effect is done.
    queue: RefCell<VecDeque<Key>>,
    /// The queue is being run: a change made meanwhile only marks and queues.
    propagating: Cell<bool>,
    /// Number of propagations started on this thread.
    propagations: Cell<u64>,
}

thread_local! {
    static RUNTIME: Runtime = Runtime::default();
}

/// Ends a propagation, even when an effect panicked.
struct Propagation<'a>(&'a Runtime);

impl Drop for Propagation<'_> {
    fn drop(&mut self) {
        self.0.propagating.set(false);
        // After a panic, what was left in the queue is forgotten.
        let left: Vec<Key> = self.0.queue.borrow_mut().drain(..).collect();
        if !left.is_empty() {
            let mut effects = self.0.effects.borrow_mut();
            for key in left {
                if let Some(slot) = effects
                    .get_mut(key.index)
                    .filter(|s| s.version == key.version)
                {
                    slot.queued = false;
                }
            }
        }
    }
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

    /// The readers of a state.
    fn subscribers(&self, key: Key) -> Vec<Key> {
        match self.states.borrow().get(key.index) {
            Some(slot) if slot.version == key.version => slot.subscribers.clone(),
            _ => Vec::new(),
        }
    }

    /// The state changed: its readers are marked, then run (unless a propagation is already
    /// running: it picks them up once the current effect is done).
    fn notify(&self, key: Key) {
        for effect in self.subscribers(key) {
            self.mark(effect, Status::Dirty);
        }
        self.propagate();
    }

    /// Marks `effect` dirty or to check, the readers of the memo it computes to check, and
    /// queues the effects (not the memos: they are refreshed when read).
    fn mark(&self, effect: Key, status: Status) {
        let (memo_of, queue) = {
            let mut effects = self.effects.borrow_mut();
            let Some(slot) = effects
                .get_mut(effect.index)
                .filter(|s| s.version == effect.version)
            else {
                return;
            };
            // The running effect wrote a state it reads: it does not re-trigger itself.
            if slot.running || slot.status >= status {
                return;
            }
            let was_clean = slot.status == Status::Clean;
            slot.status = status;
            let queue = slot.derived.is_none() && !slot.queued;
            if queue {
                slot.queued = true;
            }
            (if was_clean { slot.derived } else { None }, queue)
        };
        if queue {
            self.queue.borrow_mut().push_back(effect);
        }
        // Below a memo, values may change: checked when read.
        if let Some(state) = memo_of {
            for reader in self.subscribers(state) {
                self.mark(reader, Status::Check);
            }
        }
    }

    /// Runs the queued effects, each once the memos it reads are up to date. Effects queued
    /// meanwhile (by states written in an effect) run in turn.
    fn propagate(&self) {
        if self.propagating.replace(true) {
            return;
        }
        self.propagations.set(self.propagations.get() + 1);
        let _done = Propagation(self);
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(effect) = next else {
                break;
            };
            self.refresh(effect);
            if let Some(slot) = self
                .effects
                .borrow_mut()
                .get_mut(effect.index)
                .filter(|s| s.version == effect.version)
            {
                slot.queued = false;
            }
        }
    }

    fn status(&self, effect: Key) -> Option<Status> {
        self.effects
            .borrow()
            .get(effect.index)
            .filter(|s| s.version == effect.version)
            .map(|s| s.status)
    }

    /// Brings `effect` up to date: when a memo it reads may have changed, that memo is
    /// refreshed first (and marks `effect` dirty if its value changed); then `effect` runs if
    /// it is dirty.
    fn refresh(&self, effect: Key) {
        let (status, dependencies) = {
            let effects = self.effects.borrow();
            match effects
                .get(effect.index)
                .filter(|s| s.version == effect.version)
            {
                Some(slot) if !slot.running => (slot.status, slot.dependencies.clone()),
                _ => return,
            }
        };
        match status {
            Status::Clean => return,
            Status::Check => {
                for dep in dependencies {
                    let memo = self
                        .states
                        .borrow()
                        .get(dep.index)
                        .filter(|s| s.version == dep.version)
                        .and_then(|s| s.derived_by);
                    if let Some(memo) = memo {
                        self.refresh(memo);
                    }
                    if self.status(effect) == Some(Status::Dirty) {
                        break;
                    }
                }
                if self.status(effect) != Some(Status::Dirty) {
                    // No memo it reads changed.
                    if let Some(slot) = self
                        .effects
                        .borrow_mut()
                        .get_mut(effect.index)
                        .filter(|s| s.version == effect.version)
                    {
                        slot.status = Status::Clean;
                    }
                    return;
                }
            }
            Status::Dirty => {}
        }
        {
            let mut effects = self.effects.borrow_mut();
            let Some(slot) = effects
                .get_mut(effect.index)
                .filter(|s| s.version == effect.version)
            else {
                return;
            };
            let propagation = self.propagations.get();
            slot.runs = if slot.propagation == propagation {
                slot.runs + 1
            } else {
                1
            };
            slot.propagation = propagation;
            if slot.runs > MAX_RUNS {
                slot.status = Status::Clean;
                drop(effects);
                #[cfg(feature = "client")]
                web_sys::console::warn_1(
                    &"active: an effect ran 1000 times in one update and was stopped: two effects write what the other reads".into(),
                );
                return;
            }
        }
        self.run_effect(effect);
    }

    /// Brings the memo computing `key` up to date, if any, before a read.
    fn pull(&self, key: Key) {
        let memo = self
            .states
            .borrow()
            .get(key.index)
            .filter(|s| s.version == key.version)
            .and_then(|s| s.derived_by);
        if let Some(memo) = memo {
            self.refresh(memo);
        }
    }

    fn run_effect(&self, key: Key) {
        let run = match self
            .effects
            .borrow()
            .get(key.index)
            .filter(|s| s.version == key.version)
        {
            Some(slot) if !slot.running => slot.run.clone(),
            _ => return,
        };
        self.run_as(key, || {
            if let Some(run) = run {
                run();
            }
        });
    }

    /// Runs `f` as `effect`: the states it reads subscribe the effect, which is up to date
    /// afterwards. `None` while the effect is already running.
    fn run_as<R>(&self, key: Key, f: impl FnOnce() -> R) -> Option<R> {
        let dependencies = {
            let mut effects = self.effects.borrow_mut();
            let slot = effects
                .get_mut(key.index)
                .filter(|s| s.version == key.version)?;
            if slot.running {
                return None;
            }
            slot.running = true;
            slot.status = Status::Clean;
            std::mem::take(&mut slot.dependencies)
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
        let result = f();
        self.observer.set(previous);
        if let Some(slot) = self
            .effects
            .borrow_mut()
            .get_mut(key.index)
            .filter(|s| s.version == key.version)
        {
            slot.running = false;
        }
        Some(result)
    }

    /// Creates an effect slot (not run yet). `derived`: the state it computes, for a memo.
    #[cfg_attr(not(any(feature = "client", test)), allow(dead_code))]
    fn new_effect(&self, run: Option<Rc<dyn Fn()>>, derived: Option<Key>) -> Key {
        let version = self.next_version();
        let slot = EffectSlot {
            version,
            run,
            dependencies: Vec::new(),
            running: false,
            derived,
            status: Status::Clean,
            queued: false,
            runs: 0,
            propagation: 0,
        };
        let index = match self.free_effects.borrow_mut().pop() {
            Some(index) => {
                self.effects.borrow_mut()[index] = slot;
                index
            }
            None => {
                let mut effects = self.effects.borrow_mut();
                effects.push(slot);
                effects.len() - 1
            }
        };
        Key { index, version }
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
                slot.derived_by = None;
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
                        slot.derived_by = None;
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
                                slot.derived = None;
                                slot.status = Status::Clean;
                                slot.queued = false;
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
        let slot = StateSlot {
            version,
            value: Some(Rc::new(RefCell::new(Box::new(initial) as Box<dyn Any>))),
            subscribers: Vec::new(),
            used: rt.renders.get(),
            derived_by: None,
        };
        let index = match rt.free_states.borrow_mut().pop() {
            Some(index) => {
                rt.states.borrow_mut()[index] = slot;
                index
            }
            None => {
                let mut states = rt.states.borrow_mut();
                states.push(slot);
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

    /// Reads the value through a reference (subscribes the running effect). A [`memo`] is
    /// brought up to date first.
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let cell = RUNTIME.with(|rt| {
            rt.pull(self.key);
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

    /// Changes the value in place and re-runs the effects that read it (once the running
    /// effect, if any, has finished).
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
///
/// An effect runs once per change, after the [`memo`]s it reads are up to date. A state it
/// writes notifies its readers once the effect has finished; an effect that writes a state it
/// reads does not re-trigger itself.
#[cfg_attr(not(any(feature = "client", test)), allow(unused_variables))]
pub fn effect(f: impl Fn() + 'static) {
    #[cfg(any(feature = "client", test))]
    RUNTIME.with(|rt| {
        let key = rt.new_effect(Some(Rc::new(f)), None);
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
/// A memo is never read out of date: when one of its inputs changes, it is recomputed before
/// the effects that read it run (and only if an input really changed, memos in between
/// included).
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
    #[cfg(not(any(feature = "client", test)))]
    {
        use_state(untrack(&f))
    }
    #[cfg(any(feature = "client", test))]
    RUNTIME.with(|rt| {
        // The first value is computed as the memo itself, so that its reads subscribe it.
        let key = rt.new_effect(None, None);
        let initial = rt.run_as(key, &f).expect("a new effect is not running");
        let state = use_state(initial);
        let compute = move || {
            let next = f();
            untrack(|| {
                if state.with(|current| *current != next) {
                    state.set(next);
                }
            });
        };
        if let Some(slot) = rt.effects.borrow_mut().get_mut(key.index) {
            slot.run = Some(Rc::new(compute));
            slot.derived = Some(state.key);
        }
        if let Some(slot) = rt.states.borrow_mut().get_mut(state.key.index) {
            slot.derived_by = Some(key);
        }
        rt.own(Owned::Effect(key));
        state
    })
}

/// A handle on a [`watch`]: [`stop`](Watcher::stop) it to stop calling the callback before
/// its component is released. `Copy`, so it can be moved into the callback itself.
#[derive(Clone, Copy, Debug)]
pub struct Watcher {
    #[cfg_attr(not(any(feature = "client", test)), allow(dead_code))]
    key: Option<Key>,
}

impl Watcher {
    /// Stops the watcher: the callback is never called again. Stopping twice does nothing.
    pub fn stop(self) {
        if let Some(key) = self.key {
            RUNTIME.with(|rt| rt.dispose(vec![Owned::Effect(key)]));
        }
    }
}

/// Calls `callback(&new, &old)` each time the value computed by `source` changes
/// (`PartialEq`): Vue's `watch`. `source` reads [`State`]s and is followed like an
/// [`effect`]; `callback` is not (what it reads does not re-trigger it), and is **not** called
/// for the first value: use [`watch_immediate`] for that.
///
/// Use it to react to a precise change with the previous value at hand: save a draft when the
/// text changes, reload when an id changes, log a transition, reset a page number when the
/// filter changes. An [`effect`] re-runs for any change of anything it reads; a watcher
/// compares values and separates what it follows from what it does.
///
/// Like effects, watchers run only in the browser (crate feature `client`): on the server
/// `watch` does nothing. A watcher created while building a component is released with it.
///
/// ```
/// use active::{use_state, watch};
///
/// let query = use_state(String::from("rust"));
/// let page = use_state(3);
/// // A new search starts again from the first page.
/// watch(move || query.get(), move |_new, _old| page.set(1));
/// # #[cfg(feature = "client")] {
/// query.set("wasm".into());
/// assert_eq!(page.get(), 1);
/// # }
/// ```
pub fn watch<T: Clone + PartialEq + 'static>(
    source: impl Fn() -> T + 'static,
    callback: impl Fn(&T, &T) + 'static,
) -> Watcher {
    new_watcher(source, move |new, old: Option<&T>| {
        if let Some(old) = old {
            callback(new, old);
        }
    })
}

/// Like [`watch`], but also calls `callback(&value, None)` right away with the first value
/// (Vue's `{ immediate: true }`); later calls get `Some(&old)`. In the browser only, like
/// [`watch`].
///
/// ```
/// use active::{use_state, watch_immediate};
/// use std::{cell::RefCell, rc::Rc};
///
/// let id = use_state(1);
/// let loads = Rc::new(RefCell::new(Vec::new()));
/// let log = loads.clone();
/// watch_immediate(move || id.get(), move |id, _old| log.borrow_mut().push(*id));
/// # #[cfg(feature = "client")] {
/// id.set(2);
/// assert_eq!(*loads.borrow(), [1, 2]);
/// # }
/// ```
pub fn watch_immediate<T: Clone + PartialEq + 'static>(
    source: impl Fn() -> T + 'static,
    callback: impl Fn(&T, Option<&T>) + 'static,
) -> Watcher {
    new_watcher(source, callback)
}

#[cfg_attr(not(any(feature = "client", test)), allow(unused_variables))]
fn new_watcher<T: Clone + PartialEq + 'static>(
    source: impl Fn() -> T + 'static,
    callback: impl Fn(&T, Option<&T>) + 'static,
) -> Watcher {
    #[cfg(not(any(feature = "client", test)))]
    {
        Watcher { key: None }
    }
    #[cfg(any(feature = "client", test))]
    RUNTIME.with(|rt| {
        let previous: RefCell<Option<T>> = RefCell::new(None);
        let run = move || {
            let next = source();
            let old = previous.borrow_mut().take();
            if old.as_ref() != Some(&next) {
                untrack(|| callback(&next, old.as_ref()));
            }
            *previous.borrow_mut() = Some(next);
        };
        let key = rt.new_effect(Some(Rc::new(run)), None);
        rt.own(Owned::Effect(key));
        rt.run_effect(key);
        Watcher { key: Some(key) }
    })
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
    fn an_effect_never_sees_a_stale_memo() {
        // The diamond: the effect reads `n` directly and through `double`.
        let n = use_state(1);
        let double = memo(move || n.get() * 2);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        effect(move || {
            assert_eq!(double.get(), n.get() * 2, "stale memo");
            r.set(r.get() + 1);
        });
        n.set(5);
        n.set(6);
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn a_memo_is_recomputed_only_when_an_input_really_changed() {
        let n = use_state(1);
        let (parity_runs, tens_runs, effect_runs) = (
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(0)),
        );
        let p = parity_runs.clone();
        let parity = memo(move || {
            p.set(p.get() + 1);
            n.get() % 2
        });
        let t = tens_runs.clone();
        let tens = memo(move || {
            t.set(t.get() + 1);
            parity.get() * 10
        });
        let e = effect_runs.clone();
        effect(move || {
            tens.get();
            e.set(e.get() + 1);
        });
        n.set(3); // parity is recomputed, unchanged: nothing below it runs
        assert_eq!(
            (parity_runs.get(), tens_runs.get(), effect_runs.get()),
            (2, 1, 1)
        );
        n.set(4);
        assert_eq!(
            (
                parity_runs.get(),
                tens_runs.get(),
                effect_runs.get(),
                tens.get()
            ),
            (3, 2, 2, 0)
        );
    }

    #[test]
    fn a_state_written_in_an_effect_notifies_its_readers_once_the_effect_is_done() {
        let a = use_state(1);
        let b = use_state(0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        effect(move || {
            let value = a.get();
            l.borrow_mut().push("A");
            b.set(value * 2);
            l.borrow_mut().push("A done");
        });
        let l = log.clone();
        effect(move || {
            b.get();
            l.borrow_mut().push("B");
        });
        log.borrow_mut().clear();
        a.set(2);
        assert_eq!(*log.borrow(), ["A", "A done", "B"]);
        assert_eq!(b.get(), 4);
    }

    #[test]
    fn a_memo_nobody_reads_is_brought_up_to_date_when_read() {
        let n = use_state(1);
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        let next = memo(move || {
            r.set(r.get() + 1);
            n.get() + 1
        });
        n.set(5);
        n.set(6);
        assert_eq!(runs.get(), 1);
        assert_eq!((next.get(), runs.get()), (7, 2));
    }

    #[test]
    fn two_effects_writing_what_the_other_reads_are_stopped() {
        let a = use_state(0);
        let b = use_state(0);
        effect(move || b.set(a.get() + 1));
        effect(move || a.set(b.get() + 1));
        a.set(10);
        assert!(a.get() > 10);
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
    fn watch_calls_back_with_new_and_old_values_only_on_real_changes() {
        let n = use_state(1);
        let other = use_state(0);
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        watch(
            move || n.get() / 10,
            move |new, old| {
                // Reads in the callback are not followed.
                other.get();
                l.borrow_mut().push((*new, *old));
            },
        );
        assert!(log.borrow().is_empty(), "not called for the first value");
        n.set(5); // 5 / 10 is still 0
        other.set(1);
        n.set(12);
        n.set(31);
        assert_eq!(*log.borrow(), [(1, 0), (3, 1)]);
    }

    #[test]
    fn watch_immediate_starts_with_the_first_value() {
        let n = use_state("a");
        let log = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        watch_immediate(
            move || n.get(),
            move |new, old| l.borrow_mut().push((*new, old.copied())),
        );
        n.set("b");
        assert_eq!(*log.borrow(), [("a", None), ("b", Some("a"))]);
    }

    #[test]
    fn a_stopped_or_released_watcher_is_never_called_again() {
        let n = use_state(0);
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let watcher = watch(move || n.get(), move |_, _| c.set(c.get() + 1));
        n.set(1);
        watcher.stop();
        watcher.stop();
        n.set(2);
        assert_eq!(calls.get(), 1);

        let c = calls.clone();
        let (_, owner) = scope(|| watch(move || n.get(), move |_, _| c.set(c.get() + 10)));
        n.set(3);
        owner.dispose();
        n.set(4);
        assert_eq!(calls.get(), 11);
    }

    #[test]
    fn a_watcher_can_stop_itself_from_its_callback() {
        let n = use_state(0);
        let calls = Rc::new(Cell::new(0));
        let me: Rc<Cell<Option<Watcher>>> = Rc::default();
        let (c, m) = (calls.clone(), me.clone());
        let watcher = watch(
            move || n.get(),
            move |_, _| {
                c.set(c.get() + 1);
                if let Some(w) = m.get() {
                    w.stop();
                }
            },
        );
        me.set(Some(watcher));
        n.set(1);
        n.set(2);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn states_can_be_read_while_another_is_updated() {
        let a = use_state(2);
        let b = use_state(0);
        b.update(|v| *v = a.get() * 10);
        assert_eq!(b.get(), 20);
    }
}
