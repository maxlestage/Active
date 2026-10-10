# active

[![crates.io](https://img.shields.io/crates/v/active.svg)](https://crates.io/crates/active)
[![docs.rs](https://img.shields.io/docsrs/active)](https://docs.rs/active)
[![MIT](https://img.shields.io/crates/l/active.svg)](https://github.com/maxlestage/Active/blob/master/LICENSE)

**A simple, fast front-end library for Rust: components, state, routing and complete SEO
tags, rendered on the server and run in the browser as WebAssembly.**

You write components as plain Rust functions that return views, with state and effects
in the style of React hooks. Then ship them the way your project needs:

- **server-rendered pages with islands**: the server writes the HTML (fast: straight into
  a `String`, no virtual DOM) and only the interactive parts wake up in the browser;
- **static sites**: the same pages written to files at build time;
- **client-side apps**: the whole app runs in the browser, with a router and a `<head>`
  that follows the current page.

- **Views**: builder functions for every HTML tag (`div().class("card").child(...)`),
  fragments, raw HTML, dynamic text, attributes, classes and lists.
- **State**: `use_state` returns a `Copy` handle you move into closures; `effect` re-runs
  when the states it read change. Fine-grained: only the bound text, attribute or class is
  updated.
- **SEO**: `Seo` renders title, description, robots, canonical, hreflang, Open Graph,
  Twitter and JSON-LD. Put a `Seo` anywhere in the tree: the document merges them, the
  deepest wins, no duplicate tags.
- **Lists and conditions**: `children_dyn` / `fragment_dyn` rebuild only their part of the
  page when the states they read change.
- **Islands**: `island("counter", props, counter)` on the server, `hydrate(&[("counter",
  counter)])` in the browser. Static content is never re-rendered or downloaded twice.
- **Client-side apps**: `mount_to("#app", app())`, `location()` and `navigate()` for
  routing without reloads; each page's `Seo` updates the title and meta tags.
- **Small**: no macros, no `unsafe`. The server build depends on nothing; the `client`
  feature adds `wasm-bindgen` and `web-sys`.

## Install

```sh
cargo add active                      # server
cargo add active --features client    # browser bundle (wasm32)
```

## A page

```rust
use active::prelude::*;

fn card(title: &str, body: &str) -> Element {
    article().class("card").child(h2().text(title.to_string())).child(p().text(body.to_string()))
}

let page = main()
    .child(Seo::new().title("Contact").description("Write to us."))
    .child(h1().text("Contact"))
    .child(card("Email", "hello@acme.com"));

let html = Document::new(page)
    .lang("en")
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
    .render();

assert!(html.starts_with(r#"<!DOCTYPE html><html lang="en"><head>"#));
assert!(html.contains("<title>Contact · Acme</title>"));
assert!(html.contains(r#"<meta name="description" content="Write to us."/>"#));
assert!(html.contains(r#"<article class="card"><h2>Email</h2><p>hello@acme.com</p></article>"#));
```

`Document::new(body)` builds the whole HTML page: doctype, `<head>` with the merged SEO
tags, stylesheets, extra head HTML and the WebAssembly loader, then the `<body>`. A single
view renders on its own with `Node::render()`.

## State and islands

```rust
use active::prelude::*;

/// Rendered on the server, made interactive in the browser.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    div()
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().class_if("negative", move || count.get() < 0).text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
        .into()
}

let html = island("counter", "3", counter).render();
assert_eq!(
    html,
    concat!(
        r#"<active-island data-island="counter" data-props="3"><div>"#,
        r#"<button>−</button><span><!--#-->3</span><button>+</button>"#,
        r#"</div></active-island>"#,
    )
);
```

In the browser crate (built for `wasm32-unknown-unknown` with the `client` feature):

```rust,ignore
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

`hydrate` finds every `<active-island>` of the page, runs the component with its props and
binds it to the HTML that is already there: event listeners, dynamic text, attributes and
classes. No node is recreated. On the server, handlers and effects are never run, and the
state arena is emptied after each render, so a server can render pages forever without
growing.

Build the browser bundle with [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # same version as in Cargo.lock
cargo build --lib --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/release/my_app.wasm
```

and point the document at it: `Document::new(body).client("/pkg/my_app.js",
"/pkg/my_app_bg.wasm")`. The page then preloads the module and the `.wasm` file in
parallel and starts them with one generated line. You never write JavaScript.

## Client-side apps

```rust,ignore
use active::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app());
}

fn app() -> Node {
    let path = location(); // the router: back/forward and same-site links update it
    div()
        .child(Seo::new().site_name("Todos").title_template("%s · Todos"))
        .child(nav().child(a().href("/").text("Home")).child(a().href("/about").text("About")))
        .child(main().children_dyn(move || match path.get().as_str() {
            "/" => vec![home()],
            "/about" => vec![about()],
            _ => vec![not_found()],
        }))
        .into()
}

fn about() -> Node {
    section()
        .child(Seo::new().title("About")) // document.title becomes "About · Todos"
        .child(h1().text("About"))
        .into()
}
```

Forms are controlled with `attr_dyn("value", …)` and `on_input`; a complete to-do app
with routing, filters and `localStorage` is in
[`examples/spa`](https://github.com/maxlestage/Active/tree/master/examples/spa).

## SEO

```rust
use active::Seo;

let site = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com") // resolves relative canonical, image and alternates
    .image("/og.png")
    .twitter("@acme");

let head = site
    .clone()
    .title("Oslo chair")
    .description("Solid oak chair, delivered in 48 h.")
    .canonical("/products/oslo")
    .og_type("product")
    .alternate("fr", "/fr/products/oslo")
    .json_ld(r#"{"@context":"https://schema.org","@type":"Product","name":"Oslo chair"}"#)
    .to_html();

assert!(head.starts_with("<title>Oslo chair · Acme</title>"));
assert!(head.contains(r#"<link rel="canonical" href="https://acme.com/products/oslo"/>"#));
assert!(head.contains(r#"<link rel="alternate" hreflang="fr" href="https://acme.com/fr/products/oslo"/>"#));
assert!(head.contains(r#"<meta property="og:image" content="https://acme.com/og.png"/>"#));
assert!(head.contains(r#"<meta name="twitter:card" content="summary_large_image"/>"#));
```

| Method | Tags |
| --- | --- |
| `.title(title)` | `<title>`, `og:title` (without template) |
| `.title_template(template)` | `<title>`: the first `%s` is replaced by the title |
| `.site_name(name)` | `og:site_name`, and the title when none is set |
| `.description(text)` | `<meta name="description">`, `og:description` |
| `.canonical(url)` | `<link rel="canonical">`, `og:url` |
| `.image(url)` | `og:image`, `twitter:card` = `summary_large_image` |
| `.og_type(type)` | `og:type` (default `website`) |
| `.locale(locale)` | `og:locale` |
| `.twitter(handle)` | `twitter:site` |
| `.base_url(url)` | resolves relative `canonical`, `image` and `alternate` URLs |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang>` |
| `.json_ld(json)` | `<script type="application/ld+json">` (`<` escaped as `\u003c`) |
| `.meta_name(name, content)`, `.meta_property(property, content)` | any other `<meta>` |
| `.merge(&other)`, `.merged(&other)` | combine: values set in `other` win |
| `.to_html()`, `.write_html(&mut out)`, `Display` | render |

### Merging

A layout sets the site defaults, a page sets its title, a component deep in the page adds
its JSON-LD: `Document` collects every `Seo` of the tree, in order, and merges them. Values
set later (deeper) override earlier ones, alternates and extra meta tags are replaced by
key, JSON-LD blocks add up. The page always gets exactly one `<title>`.

```rust
use active::prelude::*;

let layout = |content: Element| div().child(Seo::new().title("Acme").description("Furniture.")).child(content);
let page = layout(main().child(Seo::new().title("Chairs")));

let html = Document::new(page).seo(Seo::new().title_template("%s · Acme")).render();
assert_eq!(html.matches("<title>").count(), 1);
assert!(html.contains("<title>Chairs · Acme</title>"));
assert!(html.contains(r#"content="Furniture.""#));
```

## Axum

```rust,ignore
use active::prelude::*;
use axum::{Router, response::Html, routing::get};

async fn home() -> Html<String> {
    let page = main().child(Seo::new().title("Home")).child(h1().text("Hello"));
    Html(Document::new(page).lang("en").render())
}

let app = Router::new().route("/", get(home));
```

Any server works the same way (Actix Web, Rocket, Hyper, a static site generator):
`Document::render()` returns a `String`.

## Links

- Guides in English, French and Spanish, examples and benchmarks:
  [github.com/maxlestage/Active](https://github.com/maxlestage/Active).
- Server page with an island: [`examples/hello`](https://github.com/maxlestage/Active/tree/master/examples/hello).
- Client-side app with routing: [`examples/spa`](https://github.com/maxlestage/Active/tree/master/examples/spa).
- The project website, built with `active`: [`site/`](https://github.com/maxlestage/Active/tree/master/site).

## License

[MIT](https://github.com/maxlestage/Active/blob/master/LICENSE)
