---
title: Documents
description: Turn a view into a complete HTML page with Document: language, merged SEO tags, stylesheets, extra head HTML, the WebAssembly loader and page attributes.
---

A view describes the content of a page. `Document` wraps it into a complete HTML document: the doctype, the `<html>` element with its language, a `<head>` with the merged SEO tags, your stylesheets and the code that starts the WebAssembly bundle, then the `<body>`. Its `render()` returns the page as a `String`, ready to send from a server or to write to a file.

## A complete page

```rust
use active::prelude::*;

fn page() -> String {
    let body = el("main")
        .child(Seo::new().title("Contact").description("Write to us, we answer within a day."))
        .child(h1().text("Contact"));

    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
        .stylesheet("/static/app.css")
        .head(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml"/>"#)
        .client("/pkg/acme.js", "/pkg/acme_bg.wasm")
        .html_attr("class", "light")
        .body_attr("class", "page-contact")
        .render()
}

let html = page();
assert!(html.starts_with(r#"<!DOCTYPE html><html lang="en" class="light"><head><meta charset="utf-8"/>"#));
assert!(html.contains("<title>Contact · Acme</title>"));
assert!(html.ends_with(r#"<body class="page-contact"><main><h1>Contact</h1></main></body></html>"#));
```

## What render writes

The output is a single line of HTML. Laid out for reading, its structure is always:

```html
<!DOCTYPE html>
<html lang="en" class="light">                      <!-- lang, then html_attr -->
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Contact · Acme</title>                     <!-- merged Seo tags -->
  <meta name="description" content="…"/>
  …
  <style>active-island{display:contents}</style>
  <link rel="stylesheet" href="/static/app.css"/>   <!-- stylesheet, in call order -->
  <link rel="modulepreload" href="/pkg/acme.js"/>   <!-- client -->
  <link rel="preload" href="/pkg/acme_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/acme.js";init({module_or_path:"/pkg/acme_bg.wasm"});</script>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>   <!-- head, in call order -->
</head>
<body class="page-contact">                         <!-- body_attr -->
  <main><h1>Contact</h1></main>                     <!-- your view -->
</body>
</html>
```

The charset and viewport tags are always written (`.viewport(…)` changes the latter), so every page is UTF-8 and mobile-friendly. The small `active-island` style keeps [island](../islands/) wrappers out of the layout.

## The builder

| Method | Effect |
| --- | --- |
| `Document::new(view)` | a page whose `<body>` contains `view` (anything that converts into a `Node`) |
| `.lang(code)` | `<html lang="…">`, e.g. `"en"`, `"fr"` |
| `.viewport(content)` | replaces the viewport tag's `content` (default `width=device-width, initial-scale=1`), e.g. to add `viewport-fit=cover` |
| `.seo(seo)` | the base `Seo`, merged with every `Seo` found in the view; calling it again replaces it |
| `.stylesheet(href)` | a `<link rel="stylesheet">`; call it once per file |
| `.head(html)` | trusted HTML appended to the `<head>`: icons, fonts, analytics, `theme-color` |
| `.client(js, wasm)` | loads the WebAssembly bundle (see below) |
| `.manifest(href, &manifest)` | makes the page installable: manifest link, `theme-color`, home screen title and icon (see [PWA](../pwa/)) |
| `.service_worker(url)` | registers the offline service worker served at `url` (see [PWA](../pwa/)) |
| `.html_attr(name, value)` | an attribute on `<html>` |
| `.body_attr(name, value)` | an attribute on `<body>` |
| `.render()` | the whole page as a `String` |
| `.final_seo()` | the merged `Seo` of the page, without rendering (useful in tests) |

`Document` implements `Clone` and `Default`.

> **Warning:** `.head(...)` inserts HTML as is. Only pass markup you wrote yourself.

## SEO

`Document` collects every `Seo` in the view, including inside islands and in the current content of dynamic regions, and merges them over the base given to `.seo(...)`. The result is written once, so the page always has exactly one `<title>`. The rules are described in [SEO, Merging](../seo/#merging).

`final_seo()` returns that merged value, which makes SEO easy to test:

```rust
use active::prelude::*;

let doc = Document::new(div().child(Seo::new().title("Pricing")))
    .seo(Seo::new().title_template("%s · Acme"));
assert!(doc.final_seo().to_html().starts_with("<title>Pricing · Acme</title>"));
```

## Loading the browser bundle

`wasm-bindgen --target web --no-typescript` produces two files: a small JavaScript module (the loader) and the `.wasm` binary. `.client(js, wasm)` takes their URLs and writes three things into the `<head>`:

1. `<link rel="modulepreload">` for the loader;
2. `<link rel="preload" as="fetch" type="application/wasm">` for the binary, so it downloads **in parallel** with the HTML and the loader instead of after them;
3. one `<script type="module">` line that imports the loader and starts the module with the binary's URL.

When the module starts, its `#[wasm_bindgen(start)]` function runs: it [hydrates islands](../islands/) or [mounts an app](../client-app/). This line is the only JavaScript in the page besides the loader generated by `wasm-bindgen`.

Pages without any island do not need the bundle: leave `.client(...)` out and they ship no code at all.

The URLs are written into a JavaScript string safely (quotes, backslashes and `<` are escaped), so they can contain query strings such as `?v=…`.

### Long-term caching

Browsers can keep the bundle and stylesheets forever if their URL changes whenever their content does. A common way is a version query computed from the file's content, as this website does in `site/src/server/assets.rs`:

```rust
/// A short content hash (FNV-1a), enough to tell versions of a file apart.
fn version(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")[..10].to_string()
}

fn versioned(url: &str, bytes: &[u8]) -> String {
    format!("{url}?v={}", version(bytes))
}

let url = versioned("/pkg/acme_bg.wasm", b"\0asm");
assert!(url.starts_with("/pkg/acme_bg.wasm?v="));
```

Read the files once at startup, compute their versioned URLs, and pass those to `.stylesheet(...)` and `.client(...)`. Serve URLs that carry a `?v=` with `Cache-Control: public, max-age=31536000, immutable`, and the HTML itself with `no-cache`.

## Rendering

`render()` renders the body first, collecting its `Seo` values on the way, then writes the page around it: each dynamic region is evaluated once per render. A document can be rendered as many times as needed; on the server, the states its view uses are released automatically once later renders no longer need them (see [State, On the server](../state/#on-the-server)). `final_seo()`, when you call it yourself, evaluates the dynamic regions separately.

To send it, return it with the `text/html; charset=utf-8` content type. With Axum, wrapping it in `axum::response::Html` does that for you (see [Axum](../axum/)). For a static site, write it to an `index.html` file (see [Static sites](../static-site/)).

## Without Document

For full control over the page's markup (an existing template, another framework), render the parts yourself: `Node::render()` gives the body HTML and `Seo` gives the head tags with `to_html()`, `write_html()` or `format!("{seo}")`. You then lose the automatic merging of `Seo` values found in the view and the bundle loader, which you would write by hand.
