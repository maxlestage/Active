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
