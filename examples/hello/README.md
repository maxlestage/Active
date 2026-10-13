# hello

The smallest `active` app: one page rendered by a Rust server, with a counter island
hydrated by WebAssembly. One crate is used twice:

- `src/lib.rs`: the components and the page, shared by both sides;
- `src/main.rs`: a server written with the standard library only (no framework);
- the same `lib.rs`, compiled for `wasm32-unknown-unknown` with the `client` feature of
  `active`, hydrates the island in the browser (`start()`).

```sh
# once
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129

# from the repository root: the browser bundle, then the server
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello
```

Open http://localhost:3000. View the page source: the counter is already there, in the
HTML, at `3`. Click it: the WebAssembly module now drives it.

## Installable

The page is also a Progressive Web App, with `active::pwa` (see the
[guide](https://active-c1883a966650.herokuapp.com/en/docs/pwa/)):

- `manifest()` in `lib.rs` is the web app manifest, served at `/manifest.webmanifest`: the
  name, the colors and the icons of `static/` (drawn from `icon.svg`);
- `worker()` in `main.rs` is the service worker, served at `/sw.js`: at installation it
  saves the page, the bundle and the icons, and its version is a hash of them
  (`active::pwa::version`), so a change deploys a new worker;
- `offline()` is the page shown without network for anything not saved;
- the page calls `.manifest(...)` and `.service_worker(...)`, which write the tags and
  register the worker.

Install it from the browser's menu, or stop the server and reload: the counter is still
there, and still works.
