---
title: Building the client
description: Compile your components to WebAssembly with cargo and wasm-bindgen, serve the bundle, load it from the page, and automate the build in CI and Docker.
---

The browser part of an `active` site is your library crate compiled to WebAssembly. Two commands build it: `cargo` compiles the crate for the `wasm32-unknown-unknown` target, then `wasm-bindgen` produces the final `.wasm` file and a small JavaScript loader. This page covers both commands, serving the result, and automating the build.

## What gets built

```text
src/lib.rs ──cargo build --target wasm32-unknown-unknown──▶ target/…/my_site.wasm
                                                                 │
                       wasm-bindgen --target web --no-typescript ▼
                                       pkg/my_site.js         the loader (a JavaScript module)
                                       pkg/my_site_bg.wasm    your code, served to browsers
```

The loader is generated: you never edit it. The page starts it with the one line written by [`Document::client`](../documents/#loading-the-browser-bundle).

## Requirements

- The target: `rustup target add wasm32-unknown-unknown`.
- The CLI, at **exactly** the version of the `wasm-bindgen` crate in your `Cargo.lock`: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
- A crate set up for two targets, as in [Installation](../installation/#one-crate-two-targets): `crate-type = ["cdylib", "rlib"]`, `active` with the `client` feature and `wasm-bindgen` under `[target.'cfg(target_arch = "wasm32")'.dependencies]`, and a `#[wasm_bindgen(start)]` function.

## Build commands

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

- `--lib` builds only the library. Your server (`src/main.rs`) is not compiled for the browser.
- `--profile wasm-release` uses the size-optimized profile from [Installation](../installation/#add-the-build-profiles). Without it, use `--release` and the path `…/release/my_site.wasm`.
- The `.wasm` file is named after the package, with `-` replaced by `_`: the package `my-site` gives `my_site.wasm`.
- `--target web` produces a standard ES module loader, which is what `Document::client` expects.
- `--no-typescript` skips the TypeScript declaration files (`.d.ts`), which a Rust project has no use for.

In a [Cargo workspace](https://doc.rust-lang.org/cargo/reference/workspaces.html), select the package with `-p` and use the workspace's `target/` directory, as this repository does for its example:

```sh
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
```

### Faster builds while developing

A debug build compiles faster and gives better panic messages, at the cost of a much larger file:

```sh
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/debug/my_site.wasm
```

A small script saves typing. Save it as `build.sh` at the root of the project, make it executable with `chmod +x build.sh`, and run `./build.sh && cargo run` after each change:

```sh
#!/bin/sh
set -e
PROFILE=${PROFILE:-wasm-release}
DIR=$PROFILE
[ "$PROFILE" = dev ] && DIR=debug # the dev profile writes to target/…/debug
cargo build --lib --target wasm32-unknown-unknown --profile "$PROFILE"
wasm-bindgen --target web --no-typescript --out-dir pkg \
  "target/wasm32-unknown-unknown/$DIR/my_site.wasm"
```

`PROFILE=dev ./build.sh` makes a debug build.

Add `pkg` to your `.gitignore`: it is a build output.

## Serving the bundle

Serve the `pkg/` folder at a URL, usually `/pkg/`, with these content types:

| File | Content type |
| --- | --- |
| `.js` | `text/javascript` |
| `.wasm` | `application/wasm` |

With the right type, the browser compiles the module while it downloads. With a wrong one, the loader prints a warning in the console and falls back to a slower path. [Vitesse](../vitesse/)'s `static_dir` and most static hosts set both types automatically.

Compress both files: WebAssembly compresses well with Brotli or gzip (see [Performance](../performance/#sizes)).

## Loading it from the page

Pass the two URLs to the document:

```rust
use active::prelude::*;

fn page(body: Element) -> String {
    Document::new(body)
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}
```

The page preloads both files and starts the module. Your `#[wasm_bindgen(start)]` function then runs, typically `active::hydrate(&ISLANDS)` for [islands](../islands/) or `active::mount_to("#app", app())` for a [client-side app](../client-app/).

## In continuous integration

This repository builds its site on every push with GitHub Actions (`.github/workflows/ci.yml`). The relevant steps:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown
- uses: Swatinem/rust-cache@v2
- name: Install wasm-bindgen-cli
  run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
- name: Build the site
  run: |
    cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
    wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
    cargo build -p active-site --bin active-site --release
```

It also lints the browser code, which the regular build does not check:

```sh
cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings
```

On Heroku, `heroku-build.sh` runs all three once the `emk/rust` buildpack has installed Rust; for Docker, the `Dockerfile` runs them in a build stage. See [Deploy to Heroku](../deploy-heroku/#how-the-repository-is-deployed).

## Troubleshooting

### wasm-bindgen reports a schema or version mismatch

The CLI and the `wasm-bindgen` crate differ. Find the crate's version with `cargo tree -i wasm-bindgen --target wasm32-unknown-unknown` and install the same CLI version with `cargo install wasm-bindgen-cli --version <version> --locked`. Pinning the crate with `wasm-bindgen = "=0.2.129"` avoids surprises after `cargo update`.

### Errors about tokio, mio or sockets when building for wasm32

A server dependency is compiled for the browser. Move it under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, and make sure you pass `--lib`.

### `hydrate`, `mount_to` or `location` cannot be found

They exist only with the `client` feature. Check that `active = { …, features = ["client"] }` is under the `wasm32` dependencies, and that the code calling them is behind `#[cfg(target_arch = "wasm32")]`.

### `can't find crate for core` or `std`

The target is not installed: `rustup target add wasm32-unknown-unknown`. With several toolchains, install it for the one the project uses.

### The page loads but nothing reacts

- Open the browser's developer console and look for errors or warnings.
- Check that `/pkg/my_site.js` and `/pkg/my_site_bg.wasm` load (Network tab) and that their URLs match those given to `Document::client`.
- Check that the `start` function has `#[wasm_bindgen(start)]` and calls `hydrate` with the right island names.
- After changing browser code, rebuild the bundle: `cargo run` alone rebuilds only the server.
- A warning `active: re-rendering island (… mismatch)` means the server HTML and the browser view differ; see [Hydration mismatches](../islands/#hydration-mismatches).
