# active

[![crates.io](https://img.shields.io/crates/v/active.svg)](https://crates.io/crates/active)
[![docs.rs](https://img.shields.io/docsrs/active)](https://docs.rs/active)
[![MIT](https://img.shields.io/crates/l/active.svg)](https://github.com/maxlestage/Active/blob/master/LICENSE)

**SEO `<head>` tags for Rust: title, description, robots, canonical, hreflang, Open Graph,
Twitter and JSON-LD, in one builder.**

`active` is the Rust twin of the `<Seo>` component from
[active-react](https://github.com/maxlestage/Active). Same options, same rules, same
output, byte for byte: a page rendered by a Rust server (Axum, Actix Web…), a static site
generator or WebAssembly gets exactly the tags React 19 renders for `<Seo>`.

- Zero dependencies, no `unsafe`, no macros, no allocation beyond the output `String`.
- Escaped like React (`&`, `<`, `>`, `"`, `'`), JSON-LD safe against `</script>` injection.
- Relative URLs resolved against your site's base URL.
- Works on `wasm32-unknown-unknown`.

## Install

```sh
cargo add active
```

## Usage

```rust
use active::Seo;

let head = Seo::new()
    .title("Oslo chair")
    .description("Solid oak chair, delivered in 48 h.")
    .canonical("https://acme.com/products/oslo")
    .to_html();

assert_eq!(
    head,
    concat!(
        r#"<title>Oslo chair</title>"#,
        r#"<meta name="description" content="Solid oak chair, delivered in 48 h."/>"#,
        r#"<link rel="canonical" href="https://acme.com/products/oslo"/>"#,
        r#"<meta property="og:title" content="Oslo chair"/>"#,
        r#"<meta property="og:description" content="Solid oak chair, delivered in 48 h."/>"#,
        r#"<meta property="og:type" content="website"/>"#,
        r#"<meta property="og:url" content="https://acme.com/products/oslo"/>"#,
        r#"<meta name="twitter:card" content="summary"/>"#,
    )
);
```

`Seo` also implements `Display`, so it drops straight into `format!` without an
intermediate `String`, and `write_html(&mut String)` appends to a buffer you already have.

## Site defaults

Set the site-wide values once and clone them for each page, like
`createSeo(defaults)` in active-react. Page setters override the defaults.

```rust
use std::sync::LazyLock;
use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com") // resolves relative canonical, image and alternates
        .image("/og.png")
        .locale("fr_FR")
        .twitter("@acme")
});

let html = SITE
    .clone()
    .title("Contact")
    .description("Write to us.")
    .canonical("/contact")
    .og_type("article")
    .alternate("en", "/en/contact")
    .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#)
    .meta_name("theme-color", "#000")
    .meta_property("og:image:alt", "Logo")
    .to_html();

assert!(html.starts_with("<title>Contact · Acme</title>"));
assert!(html.contains(r#"<link rel="canonical" href="https://acme.com/contact"/>"#));
assert!(html.contains(r#"<meta property="og:title" content="Contact"/>"#));

// No title: the site name is used as is, without the template.
assert!(SITE.to_html().starts_with("<title>Acme</title>"));
```

Output, on a single line in reality (line breaks added here for reading):

```html
<title>Contact · Acme</title>
<meta name="description" content="Write to us."/>
<link rel="canonical" href="https://acme.com/contact"/>
<link rel="alternate" hreflang="en" href="https://acme.com/en/contact"/>
<meta property="og:title" content="Contact"/>
<meta property="og:description" content="Write to us."/>
<meta property="og:type" content="article"/>
<meta property="og:url" content="https://acme.com/contact"/>
<meta property="og:image" content="https://acme.com/og.png"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="fr_FR"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
<meta name="theme-color" content="#000"/>
<meta property="og:image:alt" content="Logo"/>
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Organization","name":"Acme"}</script>
```

## Methods

Every setter takes `impl Into<String>` (`bool` for `noindex` / `nofollow`) and returns the
updated `Seo`.

| Method | Tags | active-react prop |
| --- | --- | --- |
| `Seo::new()`, `Seo::default()` | `og:type` = `website` and `twitter:card` = `summary` only | |
| `.title(title)` | `<title>`, `og:title` (without template) | `title` |
| `.title_template(template)` | `<title>`: the first `%s` is replaced by the title | `titleTemplate` |
| `.site_name(name)` | `og:site_name`, title when none is set | `siteName` |
| `.description(text)` | `<meta name="description">`, `og:description` | `description` |
| `.canonical(url)` | `<link rel="canonical">`, `og:url` | `canonical` |
| `.image(url)` | `og:image`, `twitter:card` = `summary_large_image` | `image` |
| `.og_type(type)` | `og:type` (default `website`) | `type` |
| `.locale(locale)` | `og:locale` | `locale` |
| `.twitter(handle)` | `twitter:site` | `twitter` |
| `.base_url(url)` | resolves relative `canonical`, `image` and `alternate` URLs | `baseUrl` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` | `noindex`, `nofollow` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang>`, in call order (same `lang` again: replaced) | `alternates` |
| `.json_ld(json)` | `<script type="application/ld+json">`, once per call | `jsonLd` |
| `.meta_name(name, content)` | extra `<meta name content>` | `meta` |
| `.meta_property(property, content)` | extra `<meta property content>` | `meta` |
| `.to_html()` | renders to a new `String` | |
| `.write_html(&mut out)` | appends to an existing `String` | |
| `Display` | `format!("{seo}")`, `write!(f, "{seo}")` | |

Rules, identical to active-react:

- Tags come in the order shown above; a tag whose value is missing or empty is left out.
  `og:type` and `twitter:card` are always present.
- `<meta name="robots">` only appears when `noindex` or `nofollow` is set:
  `noindex,follow`, `index,nofollow` or `noindex,nofollow`.
- URL resolution: URLs with a scheme (`https:`, `mailto:`…) or starting with `//` are kept;
  otherwise `base_url` (minus one trailing `/`) is prepended, with a `/` in between if needed.
- `json_ld` takes JSON you already serialized (with `serde_json::to_string`, for example).
  It is inserted as is, except that `<` becomes `\u003c`, which is the same JSON but can never
  close the `<script>` tag.

## Axum

```rust,ignore
use std::sync::LazyLock;
use active::Seo;
use axum::{Router, response::Html, routing::get};

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com")
});

async fn contact() -> Html<String> {
    let seo = SITE.clone().title("Contact").description("Write to us.").canonical("/contact");
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">{seo}</head><body><h1>Contact</h1></body></html>"#
    ))
}

let app = Router::new().route("/contact", get(contact));
```

Actix Web, Rocket, Warp, Leptos, a templating engine or a static site generator work the
same way: `active` returns a `String`, put it in your `<head>`. Run
`cargo run --example axum_like` for a complete page built without any framework.

## WebAssembly

`active` has no dependencies and only uses `String` and `std::fmt`, so it builds for
`wasm32-unknown-unknown` (and WASI) as is: render your head tags in an edge worker, or expose
them to JavaScript from your own `wasm-bindgen` crate.

```rust,ignore
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn seo_head(title: &str, description: &str) -> String {
    active::Seo::new().title(title).description(description).to_html()
}
```

## Differences from active-react

- The alternate attribute is printed `hreflang`; React prints `hrefLang`. Both are the same
  attribute (HTML attribute names are case-insensitive).
- React leaves the JSON-LD `<script>` where `<Seo>` is rendered, in the body. `active` appends
  it to the end of the head tags, which search engines accept just as well.
- `json_ld` produces one `<script>` per call; active-react produces one per `jsonLd` value
  (an object, or an array in one script).

## Links

- JavaScript library: [active-react](https://github.com/maxlestage/Active), `<Seo>` and
  Rust/WASM loading for React 19.
- API reference: [docs.rs/active](https://docs.rs/active).
- Guides: the documentation site, with a Rust crate page, sources in
  [`site/`](https://github.com/maxlestage/Active/tree/master/site).

## License

[MIT](https://github.com/maxlestage/Active/blob/master/LICENSE)
