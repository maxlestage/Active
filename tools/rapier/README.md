# Rapier, inside Active

This directory is a copy of [Rapier](https://rapier.rs), the physics engine of
[Dimforge](https://dimforge.com) (rapier3d {VERSION}, Copyright 2020 Sébastien Crozet),
which runs `active::physics`. It is compiled with Active's `physics` feature, on by default.

**Its license is the Apache License 2.0** ([LICENSE](LICENSE)), not Active's: the code of
this directory, and of `crates/active/tests/rapier`, can be used, changed and shared under
those terms. The rest of Active stays under Active's own license.

## Changes

The files were copied from the rapier3d crate published on crates.io and changed by
`tools/vendor_rapier.py`, so that they compile as the module `active::physics::rapier`:

- Rapier's Cargo features are resolved for Active's build (3D, `f32`, `std`: Rapier's
  defaults): the code of the other builds (2D, `f64`, serde, rayon, debug rendering, finite
  elements…) is removed.
- `crate::` paths point to `crate::physics::rapier::`, and the examples and tests use
  `active::physics::rapier::` instead of `rapier3d::`.
- `lib.rs` became `mod.rs`: no `#![no_std]`, the external crates (`parry3d`, `nalgebra`,
  `num-traits`) are named at Active's root, and the module allows `unsafe` code, which
  Active denies everywhere else.
- Each file starts with a notice of where it comes from.

Nothing else is changed. To move to another version of Rapier:
`python3 tools/vendor_rapier.py VERSION && cargo fmt`, then check the dependencies of
`crates/active/Cargo.toml` against those of the new rapier3d.
