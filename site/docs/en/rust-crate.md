---
title: The Rust crate
description: The active crate is the Rust twin of the Seo component. It generates the same head tags from Rust servers, static site generators and WebAssembly.
---

Not every page is rendered by React. `active` is a Rust crate that produces the same SEO tags as `<Seo>`, with the same options and the same rules, for Rust web servers (Axum, Actix Web…), static site generators, edge workers and WebAssembly.

- **Zero dependencies**, no `unsafe`, no macros.
- **Same output as React**, byte for byte, with two small differences listed below.
- **Escaped like React** (`&`, `<`, `>`, `"`, `'`), with JSON-LD protected against `</script>` injection.
- Builds for `wasm32-unknown-unknown`. Requires Rust 1.85 or newer.

## Install

```sh
cargo add active
```

## Quick example

```rust
use active::Seo;

let head = Seo::new()
    .title("Oslo chair")
    .description("Solid oak chair, delivered in 48 h.")
    .canonical("https://acme.com/products/oslo")
    .to_html();
```

`head` now contains, on a single line:

```html
<title>Oslo chair</title>
<meta name="description" content="Solid oak chair, delivered in 48 h."/>
<link rel="canonical" href="https://acme.com/products/oslo"/>
<meta property="og:title" content="Oslo chair"/>
<meta property="og:description" content="Solid oak chair, delivered in 48 h."/>
<meta property="og:type" content="website"/>
<meta property="og:url" content="https://acme.com/products/oslo"/>
<meta name="twitter:card" content="summary"/>
```

## Site defaults and pages

`Seo` is a builder: each setter takes it by value and returns it. Build your site-wide values once, then clone them for each page, just like [`createSeo`](../site-defaults/) in React. Page setters override the defaults.

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com") // resolves relative canonical, image and alternates
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us.")
        .canonical("/contact")
        .og_type("article")
        .alternate("en", "/contact")
        .alternate("fr", "/fr/contact")
        .alternate("x-default", "/contact")
        .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#)
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "The Acme logo")
        .to_html()
}
```

Every setter accepts `impl Into<String>`, so string literals, `String` and `&String` all work. `noindex` and `nofollow` take a `bool`.

## Methods

| Method | Tags | React prop |
| --- | --- | --- |
| `Seo::new()` | Only `og:type` (`website`) and `twitter:card` (`summary`) | |
| `.title(title)` | `<title>`, `og:title` (without template) | `title` |
| `.title_template(template)` | `<title>`: the first `%s` is replaced by the title | `titleTemplate` |
| `.site_name(name)` | `og:site_name`; the title when none is set | `siteName` |
| `.description(text)` | `<meta name="description">`, `og:description` | `description` |
| `.canonical(url)` | `<link rel="canonical">`, `og:url` | `canonical` |
| `.image(url)` | `og:image`; `twitter:card` becomes `summary_large_image` | `image` |
| `.og_type(type)` | `og:type`, `website` by default | `type` |
| `.locale(locale)` | `og:locale` | `locale` |
| `.twitter(handle)` | `twitter:site` | `twitter` |
| `.base_url(url)` | Prefix for relative `canonical`, `image` and alternate URLs | `baseUrl` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` | `noindex`, `nofollow` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang>`, once per call, in call order | `alternates` |
| `.json_ld(json)` | `<script type="application/ld+json">`, once per call | `jsonLd` |
| `.meta_name(name, content)` | Extra `<meta name content>` | `meta` |
| `.meta_property(property, content)` | Extra `<meta property content>` | `meta` |

The rules are the ones of the React component: same tag order, same title fallback, same `robots` values, same URL resolution. An empty string counts as "not set". Calling `.alternate()` twice with the same language replaces the URL.

`json_ld` takes JSON you already serialized, for example with serde_json:

```rust
let ld = serde_json::json!({
    "@context": "https://schema.org",
    "@type": "Product",
    "name": product.name,
});
let seo = SITE.clone().title(&product.name).json_ld(ld.to_string());
```

The JSON is inserted as is, except that every `<` becomes `\u003c`: the same JSON, but it can never close the `<script>` tag.

## Rendering

Three ways to get the HTML, all producing the same tags with no whitespace between them:

```rust
// 1. A new String.
let head: String = seo.to_html();

// 2. Appended to a buffer you are already writing.
let mut page = String::from("<!doctype html><html><head><meta charset=\"utf-8\">");
seo.write_html(&mut page);
page.push_str("</head><body>…</body></html>");

// 3. Display: straight into format! or write!, with no intermediate String.
let page = format!("<!doctype html><html><head>{seo}</head><body>…</body></html>");
```

## Differences from React

- **`hreflang` is lowercase.** React prints the attribute as `hrefLang`. HTML attribute names are case-insensitive, so both mean the same thing.
- **JSON-LD comes last in the head.** React leaves the `<script>` where `<Seo>` is rendered, in the `<body>`. The crate has no body, so it appends the script after the other tags. Search engines accept both.
- `json_ld` produces one script per call, while the React prop produces one script per value (an object, or an array in a single script).

## Axum example

```rust
use std::sync::LazyLock;

use active::Seo;
use axum::{Router, response::Html, routing::get};

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
});

async fn page() -> Html<String> {
    let seo = SITE
        .clone()
        .title("Pricing")
        .description("Simple plans for teams of every size.")
        .canonical("/pricing");
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">{seo}</head><body><h1>Pricing</h1></body></html>"#
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/pricing", get(page));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Dependencies: `cargo add active axum` and `cargo add tokio --features full`. Actix Web, Rocket, Leptos or a template engine work the same way: the crate gives you a string, you put it in your `<head>`. The repository also has a framework-free example: `cargo run --example axum_like` in `crates/active`.

## Static site generator

```rust
use std::fs;

fn main() -> std::io::Result<()> {
    let pages = [
        ("/", "Home", "Furniture made to last."),
        ("/about/", "About", "Who we are and how we work."),
    ];
    for (path, title, description) in pages {
        let seo = SITE.clone().title(title).description(description).canonical(path);
        let dir = format!("dist{path}");
        fs::create_dir_all(&dir)?;
        fs::write(
            format!("{dir}index.html"),
            format!("<!doctype html><html><head><meta charset=\"utf-8\">{seo}</head><body><h1>{title}</h1></body></html>"),
        )?;
    }
    Ok(())
}
```

## In the browser with wasm-bindgen

Because it has no dependencies, `active` compiles to WebAssembly as is. Wrap it in your own wasm-bindgen crate to generate head tags in the browser or in an edge worker. This is exactly what the live playground on this site's home page does: every keystroke calls the crate, compiled to WebAssembly.

```toml
[dependencies]
active = "0.1"
wasm-bindgen = "0.2"
```

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn seo_head(title: &str, description: &str, path: &str, origin: &str) -> String {
    active::Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url(origin)
        .title(title)
        .description(description)
        .canonical(path)
        .to_html()
}
```

Then load it with active-react like any other module (see [loading WebAssembly](../loading-wasm/)):

```tsx
import { useWasm, wasm } from "active-react";
import url from "./pkg/seo_wasm_bg.wasm?url";

const seoWasm = wasm(() => import("./pkg/seo_wasm.js"), { url });

export function HeadPreview({ title, description }: { title: string; description: string }) {
  const rust = useWasm(seoWasm);
  return <pre>{rust ? rust.seo_head(title, description, "/products/oak-chair", location.origin) : "…"}</pre>;
}
```

> **Note:** In a React app you don't need the crate to set your own page's tags: `<Seo>` does it. The crate is for Rust code that produces HTML, or for showing tags as text like the playground.

## API reference

The full Rust API documentation is on [docs.rs/active](https://docs.rs/active).
