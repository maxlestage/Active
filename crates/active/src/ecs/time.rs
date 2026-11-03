//! Time: how long the last frame took, how long the app has run, timers.

use std::time::Duration;

use super::world::Resource;

/// The time of the frame (`Res<Time>`): in `FixedUpdate`, the fixed step's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Time<T: Default = ()> {
    delta: Duration,
    elapsed: Duration,
    context: T,
}

impl<T: Default + 'static> Resource for Time<T> {}

impl<T: Default> Time<T> {
    /// Moves time forward.
    pub fn advance_by(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
    }

    /// How long the last frame took.
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// The same, in seconds.
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// The same, in seconds, as an `f64`.
    pub fn delta_secs_f64(&self) -> f64 {
        self.delta.as_secs_f64()
    }

    /// How long the app has run.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The same, in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// The same, in seconds, as an `f64`.
    pub fn elapsed_secs_f64(&self) -> f64 {
        self.elapsed.as_secs_f64()
    }

    /// The clock's own data (the step of [`Fixed`]).
    pub fn context(&self) -> &T {
        &self.context
    }

    /// The same, to change.
    pub fn context_mut(&mut self) -> &mut T {
        &mut self.context
    }
}

/// The fixed-step clock of `FixedUpdate` (64 times a second by default).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fixed {
    timestep: Duration,
    overstep: Duration,
}

impl Default for Fixed {
    fn default() -> Fixed {
        Fixed {
            timestep: Duration::from_micros(15_625),
            overstep: Duration::ZERO,
        }
    }
}

impl Time<Fixed> {
    /// A fixed clock running `hz` times a second.
    pub fn from_hz(hz: f64) -> Time<Fixed> {
        let mut time = Time::<Fixed>::default();
        time.set_timestep_hz(hz);
        time
    }

    /// The step.
    pub fn timestep(&self) -> Duration {
        self.context.timestep
    }

    /// Changes the step.
    pub fn set_timestep(&mut self, step: Duration) {
        assert!(!step.is_zero(), "a fixed step cannot be zero");
        self.context.timestep = step;
    }

    /// Changes the step to `hz` times a second.
    pub fn set_timestep_hz(&mut self, hz: f64) {
        assert!(hz > 0.0 && hz.is_finite(), "a fixed rate must be positive");
        self.set_timestep(Duration::from_secs_f64(1.0 / hz));
    }

    /// The time accumulated towards the next step.
    pub fn overstep(&self) -> Duration {
        self.context.overstep
    }

    /// The fraction of a step accumulated (to interpolate drawings between steps).
    pub fn overstep_fraction(&self) -> f32 {
        self.context.overstep.as_secs_f32() / self.context.timestep.as_secs_f32()
    }

    pub(crate) fn accumulate(&mut self, delta: Duration) {
        self.context.overstep += delta;
    }

    /// Takes one step, if enough time accumulated.
    pub(crate) fn expend(&mut self) -> bool {
        let step = self.context.timestep;
        if self.context.overstep >= step {
            self.context.overstep -= step;
            self.advance_by(step);
            true
        } else {
            false
        }
    }
}

/// How [`Time`] moves at each update: with the clock, or by a fixed duration (for tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeUpdateStrategy {
    /// With the real time (the browser's clock in a page).
    #[default]
    Automatic,
    /// By this duration at each update.
    ManualDuration(Duration),
}

impl Resource for TimeUpdateStrategy {}

/// The longest frame counted: after a pause (a hidden tab), time does not jump.
pub(crate) const MAX_DELTA: Duration = Duration::from_millis(250);

/// The clock, in seconds since some start.
pub(crate) fn now_secs() -> Option<f64> {
    #[cfg(all(target_arch = "wasm32", feature = "client"))]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() / 1000.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        Some(
            START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_secs_f64(),
        )
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "client")))]
    {
        None
    }
}

/// Whether a [`Timer`] stops or starts again when it finishes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimerMode {
    /// It finishes once.
    #[default]
    Once,
    /// It starts again.
    Repeating,
}

/// Counts time towards a duration: tick it with `time.delta()` each frame.
///
/// ```
/// use active::ecs::prelude::*;
/// use std::time::Duration;
///
/// let mut timer = Timer::from_seconds(1.0, TimerMode::Repeating);
/// timer.tick(Duration::from_millis(600));
/// assert!(!timer.just_finished());
/// timer.tick(Duration::from_millis(600));
/// assert!(timer.just_finished());
/// assert!((timer.elapsed_secs() - 0.2).abs() < 1e-6);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timer {
    duration: Duration,
    elapsed: Duration,
    mode: TimerMode,
    paused: bool,
    finished: bool,
    times_finished_this_tick: u32,
}

impl Timer {
    /// A timer of this duration.
    pub fn new(duration: Duration, mode: TimerMode) -> Timer {
        Timer {
            duration,
            mode,
            ..Timer::default()
        }
    }

    /// A timer of this many seconds.
    pub fn from_seconds(seconds: f32, mode: TimerMode) -> Timer {
        Timer::new(Duration::from_secs_f32(seconds.max(0.0)), mode)
    }

    /// Moves it forward.
    pub fn tick(&mut self, delta: Duration) -> &Self {
        self.times_finished_this_tick = 0;
        if self.paused || (self.finished && self.mode == TimerMode::Once) {
            if self.mode == TimerMode::Repeating {
                self.finished = false;
            }
            return self;
        }
        if self.mode == TimerMode::Repeating {
            self.finished = false;
        }
        self.elapsed += delta;
        if self.elapsed >= self.duration {
            match self.mode {
                TimerMode::Once => {
                    self.elapsed = self.duration;
                    self.times_finished_this_tick = 1;
                }
                TimerMode::Repeating if self.duration.is_zero() => {
                    self.elapsed = Duration::ZERO;
                    self.times_finished_this_tick = u32::MAX;
                }
                TimerMode::Repeating => {
                    let laps = self.elapsed.as_nanos() / self.duration.as_nanos();
                    self.times_finished_this_tick = u32::try_from(laps).unwrap_or(u32::MAX);
                    self.elapsed = Duration::from_nanos(
                        (self.elapsed.as_nanos() % self.duration.as_nanos()) as u64,
                    );
                }
            }
            self.finished = true;
        }
        self
    }

    /// Whether it has finished (a repeating timer: during the tick it did).
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether it finished during the last tick.
    pub fn just_finished(&self) -> bool {
        self.times_finished_this_tick > 0
    }

    /// How many times it finished during the last tick.
    pub fn times_finished_this_tick(&self) -> u32 {
        self.times_finished_this_tick
    }

    /// The time counted.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// The same, in seconds.
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// Its duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Changes its duration.
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// From 0 to 1.
    pub fn fraction(&self) -> f32 {
        if self.duration.is_zero() {
            1.0
        } else {
            self.elapsed.as_secs_f32() / self.duration.as_secs_f32()
        }
    }

    /// What is left, in seconds.
    pub fn remaining_secs(&self) -> f32 {
        self.duration.saturating_sub(self.elapsed).as_secs_f32()
    }

    /// Back to zero.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.finished = false;
        self.times_finished_this_tick = 0;
    }

    /// Stops counting.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Counts again.
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Whether it is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Its mode.
    pub fn mode(&self) -> TimerMode {
        self.mode
    }
}

/// True every `duration`: `.run_if(on_timer(Duration::from_secs(1)))`.
pub fn on_timer(duration: Duration) -> impl FnMut(super::Res<'_, Time>) -> bool + 'static {
    let mut timer = Timer::new(duration, TimerMode::Repeating);
    move |time: super::Res<'_, Time>| timer.tick(time.delta()).just_finished()
}
