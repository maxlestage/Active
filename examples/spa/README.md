# spa

A client-side app (single-page app) written with `active`: a to-do list with two pages.
The browser renders everything; the server only sends one page with the SEO defaults and
the WebAssembly bundle.

- `src/app.rs`: the app — components, `use_state`, an `effect` saving to `localStorage`,
  lists with `children_dyn`, a form, and routing with `active::location()` and
  `active::query_param()` (the filter lives in the URL: `/?show=done`). Links to the
  same site change page without reloading, and each page's `Seo` updates the title and the
  meta tags.
- `src/lib.rs`: the page shell and the site's SEO, shared with the server.
- `src/main.rs`: a server written with the standard library, answering the shell for every
  path (any static host with a "fallback to index.html" rule works the same).

```sh
# once
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129

# from the repository root
cargo build -p spa --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --out-dir examples/spa/pkg target/wasm32-unknown-unknown/wasm-release/spa.wasm
cargo run -p spa
```

Open http://localhost:3000.
