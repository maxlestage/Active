---
title: Quick start
description: Run a server-rendered page with a WebAssembly counter island in five minutes, first with the bundled example, then in a new project of your own.
---

This page gets a real page on your screen: HTML rendered by a Rust server, with complete SEO tags, and one interactive counter compiled to WebAssembly. It assumes the tools from [Installation](../installation/) are installed (Rust, the `wasm32-unknown-unknown` target and `wasm-bindgen-cli`).

## Run the example

The repository contains a minimal app, `examples/hello`. Clone it, build the browser bundle, start the server:

```sh
git clone https://github.com/maxlestage/Active
cd Active
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello
```

Open [http://localhost:3000](http://localhost:3000) and click the buttons. The example's server uses only the standard library, so it builds quickly.

## Create your own project

The same app, in a new crate, with a [Vitesse](../vitesse/) server.

### 1. Create the crate

```sh
cargo new my-site
cd my-site
```

### 2. Declare the dependencies

Replace `Cargo.toml` with:

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
active = { git = "https://github.com/maxlestage/Active" }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
vitesse = { git = "https://github.com/maxlestage/Vitesse" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { git = "https://github.com/maxlestage/Active", features = ["client"] }
wasm-bindgen = "=0.2.129"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

### 3. Write the page

Create `src/lib.rs`. It is shared by the server and the browser:

```rust
use active::prelude::*;

/// An island: rendered on the server, interactive in the browser.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    div()
        .class("counter")
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
        .into()
}

/// The whole page. Everything except the counter is plain HTML.
pub fn page() -> String {
    let body = main()
        .child(Seo::new().title("Home").description("My first page with active."))
        .child(h1().text("Hello from Rust"))
        .child(p().text("This paragraph is plain HTML. The counter below is an island:"))
        .child(island("counter", "0", counter));
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("My site").title_template("%s · My site"))
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}

/// Browser entry point: runs once the WebAssembly module has loaded.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

### 4. Write the server

Replace `src/main.rs`:

```rust
use vitesse::prelude::*;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    app.get("/", |_| async { Html(my_site::page()) })
        .static_dir("/pkg", "pkg");
    println!("http://localhost:3000");
    app.run(3000)
}
```

### 5. Build the browser bundle

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

This writes `pkg/my_site.js` (the loader) and `pkg/my_site_bg.wasm` (your code). Run these two commands again whenever you change code that runs in the browser.

### 6. Run it

```sh
cargo run
```

Open [http://localhost:3000](http://localhost:3000). The counter works, and the page source contains the whole text of the page.

## What just happened

Open the page source in your browser (usually `Ctrl+U` or `⌘+⌥+U`). Shortened and indented for reading, it looks like this:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Home · My site</title>
  <meta name="description" content="My first page with active."/>
  <meta property="og:title" content="Home"/>
  <meta property="og:description" content="My first page with active."/>
  <meta property="og:type" content="website"/>
  <meta property="og:site_name" content="My site"/>
  <meta name="twitter:card" content="summary"/>
  <style>active-island{display:contents}</style>
  <link rel="modulepreload" href="/pkg/my_site.js"/>
  <link rel="preload" href="/pkg/my_site_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/my_site.js";init({module_or_path:"/pkg/my_site_bg.wasm"});</script>
</head>
<body>
  <main>
    <h1>Hello from Rust</h1>
    <p>This paragraph is plain HTML. The counter below is an island:</p>
    <active-island data-island="counter" data-props="0">
      <div class="counter"><button>−</button><span><!--#-->0</span><button>+</button></div>
    </active-island>
  </main>
</body>
</html>
```

Step by step:

1. **The server renders everything.** `Document::render()` writes the doctype, the `<head>` and the body into one `String`. The `Seo` placed in the body and the one given to `Document::seo` are merged into a single set of tags (see [SEO](../seo/#merging)).
2. **The island is marked.** `island("counter", "0", counter)` rendered the counter's HTML inside an `<active-island>` element that records its name and props. The `<!--#-->` comment marks the dynamic text so the browser can find it.
3. **The bundle loads in parallel.** The two `preload` links start downloading the loader and the `.wasm` file while the page is parsed. The one-line module script starts the WebAssembly module.
4. **The island wakes up.** `start()` calls `active::hydrate`, which finds the `<active-island data-island="counter">` element, runs `counter("0")` again in the browser and attaches the click listeners and the dynamic text to the HTML that is already there. Nothing is re-created.

The heading and paragraph never run any code in the browser.

## Next steps

- Build views with [components](../components/), then add [state](../state/) and [events](../events/).
- Describe every page for search engines with [SEO](../seo/).
- Learn how [islands](../islands/) hydrate, or ship the same components as a [static site](../static-site/).
- Put the site online with [Deploy to Heroku](../deploy-heroku/).
