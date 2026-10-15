//! Development helpers: how many states and effects are alive (a leak shows as a number
//! that keeps growing), and effects that count and log their runs (Vue's devtools
//! timeline, Solid's `DEV` hooks), to see what re-runs when a state changes.
//!
//! ```
//! use active::{debug, use_state};
//!
//! let before = debug::stats();
//! let count = use_state(0);
//! debug::effect("count label", move || {
//!     let _ = count.get();
//! });
//! let after = debug::stats();
//! assert_eq!(after.states, before.states + 1);
//! # #[cfg(feature = "client")]
//! # {
//! count.set(1);
//! assert_eq!(debug::runs("count label"), 2);
//! # }
//! ```

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// A snapshot of the reactive runtime of this thread, from [`stats`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Stats {
    /// States alive (memos included).
    pub states: usize,
    /// Effects and memos alive.
    pub effects: usize,
    /// Effect runs since the thread started (browser and tests: effects never run on the
    /// server).
    pub effect_runs: u64,
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} states, {} effects, {} effect runs",
            self.states, self.effects, self.effect_runs
        )
    }
}

/// The current numbers of states and effects of this thread. Check it after closing a
/// dialog or leaving a page a few times: numbers that keep growing mean something is never
/// released (a state created outside any component in a region rebuilt often, for example).
pub fn stats() -> Stats {
    let (effects, effect_runs) = crate::reactive::effect_counts();
    Stats {
        states: crate::reactive::live_states(),
        effects,
        effect_runs,
    }
}

thread_local! {
    static RUNS: RefCell<HashMap<&'static str, u64>> = RefCell::new(HashMap::new());
    static LOG: Cell<bool> = const { Cell::new(false) };
}

/// Like [`effect`](crate::effect), counting its runs under `name` (read them with [`runs`])
/// and, after [`log`]`(true)`, writing each run to the console (the terminal on a native
/// target): `active: effect "cart total" ran (3)`.
pub fn effect(name: &'static str, f: impl Fn() + 'static) {
    crate::reactive::effect(move || {
        let n = RUNS.with(|r| {
            let mut r = r.borrow_mut();
            let n = r.entry(name).or_insert(0);
            *n += 1;
            *n
        });
        if LOG.with(Cell::get) {
            write_log(&format!("active: effect {name:?} ran ({n})"));
        }
        f();
    });
}

/// How many times the effects created by [`effect`] under `name` ran.
pub fn runs(name: &str) -> u64 {
    RUNS.with(|r| r.borrow().get(name).copied().unwrap_or(0))
}

/// Turns the logging of [`effect`] runs on or off.
pub fn log(on: bool) {
    LOG.with(|l| l.set(on));
}

/// Writes [`stats`] to the console (the terminal on a native target), prefixed by `label`.
pub fn log_stats(label: &str) {
    write_log(&format!("active: {label}: {}", stats()));
}

fn write_log(line: &str) {
    #[cfg(feature = "client")]
    web_sys::console::log_1(&line.into());
    #[cfg(not(feature = "client"))]
    eprintln!("{line}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::{scope, use_state};

    #[test]
    fn stats_count_what_is_alive_and_named_effects_count_their_runs() {
        let start = stats();
        let (state, owner) = scope(|| {
            let s = use_state(1);
            effect("test effect", move || {
                let _ = s.get();
            });
            s
        });
        let during = stats();
        assert_eq!(during.states, start.states + 1);
        assert_eq!(during.effects, start.effects + 1);
        assert_eq!(runs("test effect"), 1);
        log(true);
        state.set(2);
        log(false);
        assert_eq!(runs("test effect"), 2);
        assert!(stats().effect_runs >= start.effect_runs + 2);
        owner.dispose();
        let end = stats();
        assert_eq!((end.states, end.effects), (start.states, start.effects));
        assert_eq!(runs("unknown"), 0);
        assert!(end.to_string().contains("states"));
    }
}
