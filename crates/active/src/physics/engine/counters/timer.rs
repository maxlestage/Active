// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
use core::{
    fmt::{Display, Error, Formatter},
    time::Duration,
};

/// A timer.
#[derive(Copy, Clone, Debug, Default)]
pub struct Timer {
    time: Duration,
}

impl Timer {
    /// Creates a new timer initialized to zero and not started.
    pub fn new() -> Self {
        Timer {
            time: Duration::from_secs(0),
        }
    }

    /// Resets the timer to 0.
    pub fn reset(&mut self) {
        self.time = Duration::from_secs(0)
    }

    /// Start the timer.
    pub fn start(&mut self) {}

    /// Pause the timer.
    pub fn pause(&mut self) {}

    /// Resume the timer.
    pub fn resume(&mut self) {}

    /// The measured time between the last `.start()` and `.pause()` calls.
    pub fn time(&self) -> Duration {
        self.time
    }

    /// The measured time in milliseconds between the last `.start()` and `.pause()` calls.
    pub fn time_ms(&self) -> f64 {
        self.time.as_secs_f64() * 1000.0
    }
}

impl Display for Timer {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}ms", self.time_ms())
    }
}
