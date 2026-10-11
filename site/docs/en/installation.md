---
title: Installation
description: Install Rust, add the active crate, set up one crate for the server and the browser, and install the WebAssembly target and the wasm-bindgen CLI.
---

This page sets up everything you need: the Rust toolchain, the `active` crate, a project layout that compiles both for the server and for the browser, and the two tools that build the WebAssembly bundle.

## Requirements

| Tool | Version | Why |
| --- | --- | --- |
| Rust | 1.85 or newer | `active` uses the 2024 edition |
| `wasm32-unknown-unknown` target | any | compiles the browser bundle |
| `wasm-bindgen-cli` | same version as the `wasm-bindgen` crate in `Cargo.lock` | generates the loader and the final `.wasm` |

If you only render HTML on a server or generate a static site without islands, Rust alone is enough.

## Install Rust

Install [rustup](https://rustup.rs), which manages Rust versions, then check the version:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

On Windows, download `rustup-init.exe` from [rustup.rs](https://rustup.rs) instead. Update later with `rustup update`.

## Add the crate

```sh
cargo add active --git https://github.com/maxlestage/Active
```

### Feature flags

| Feature | Default | What it adds |
| --- | --- | --- |
| `client` | off | Browser runtime: `active::hydrate`, `active::mount`, event listeners, effects. Pulls in `wasm-bindgen` and `web-sys`. |

Without `client`, `active` has no dependencies at all. Event handlers and effects are accepted (so the same components compile everywhere) but never run.

## One crate, two targets

The recommended layout is a single crate whose library holds the components. The native build is your server; the WebAssembly build of the same library is the browser bundle. This is how `examples/hello` and this website are organized.

```text
my-site/
├── Cargo.toml
└── src/
    ├── lib.rs    components, islands and the browser entry point
    └── main.rs   the server (native only)
```

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
# cdylib: the .wasm for the browser. rlib: the library used by src/main.rs.
crate-type = ["cdylib", "rlib"]

[dependencies]
active = { git = "https://github.com/maxlestage/Active" }

# Server-only dependencies: they do not compile for wasm32.
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
vitesse = { git = "https://github.com/maxlestage/Vitesse" }

# Browser-only: the client runtime and the bindings generator.
[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { git = "https://github.com/maxlestage/Active", features = ["client"] }
wasm-bindgen = "=0.2.129"
```

The two `active` lines are merged by Cargo: on `wasm32` the crate gets the `client` feature, on the server it does not. Your code can therefore call `active::hydrate` behind `#[cfg(target_arch = "wasm32")]`:

```rust
// src/lib.rs
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

/// Runs in the browser when the WebAssembly module starts.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

> **Warning:** Keep server crates such as `vitesse` under the `cfg(not(target_arch = "wasm32"))` table. Cargo compiles every dependency of the package for the target you build, even ones only `main.rs` uses, and networking crates do not build for `wasm32-unknown-unknown`.

## Install the WebAssembly tools

Add the compilation target and the `wasm-bindgen` command-line tool:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

The CLI version must be exactly the version of the `wasm-bindgen` crate your project uses, otherwise `wasm-bindgen` stops with a schema version error. Pinning the crate with `=0.2.129` (as above) keeps both in sync. To see which version your lock file has:

```sh
cargo tree -i wasm-bindgen --target wasm32-unknown-unknown
```

## Add the build profiles

Profiles live in the root `Cargo.toml` of your project (the workspace root if you have one). These are the settings used by this repository: a fast server build and a small browser build.

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

# Browser bundle: cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

[Performance](../performance/) explains each setting and shows the sizes they produce.

## Check the setup

Render a view in a test. If this passes, the crate is installed correctly:

```rust
#[test]
fn renders() {
    use active::prelude::*;
    let html = Node::from(p().class("hello").text("Hello")).render();
    assert_eq!(html, r#"<p class="hello">Hello</p>"#);
}
```

Then build the browser bundle once to check the WebAssembly tools:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

Cargo replaces `-` with `_` in file names, so the package `my-site` produces `my_site.wasm`. The [Quick start](../quick-start/) continues from here, and [Building the client](../client-build/) covers the build in detail.

## Editor setup

Any editor with [rust-analyzer](https://rust-analyzer.github.io) works. By default it checks the native build, so code behind `#[cfg(target_arch = "wasm32")]` appears greyed out. To work on browser-only code, point rust-analyzer at the WebAssembly target, for example in VS Code's `settings.json`:

```text
"rust-analyzer.cargo.target": "wasm32-unknown-unknown"
```

Switch it back when you work on the server.
