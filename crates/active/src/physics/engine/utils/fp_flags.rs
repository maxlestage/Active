// Copyright 2020 Sébastien Crozet, under the Apache License 2.0 (see LICENSE in
// crates/active/src/physics/engine). Changed for Active.
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
