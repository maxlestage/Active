// Rapier 0.36.1 (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied
// into Active and changed (features resolved, paths moved). See README.md and
// LICENSE in crates/active/src/physics/rapier.
//! Floating-point control flags for exception handling.

/// This is an RAII structure that disables floating point exceptions while
/// it is alive, so that operations which generate NaNs and infinite values
/// intentionally will not trip an exception when debugging problematic
/// code that is generating NaNs and infinite values erroneously.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DisableFloatingPointExceptionsFlags {}

impl DisableFloatingPointExceptionsFlags {
    #[allow(dead_code)]
    /// Disables floating point exceptions as long as this object is not dropped.
    pub fn disable_floating_point_exceptions() -> Self {
        Self {}
    }
}
