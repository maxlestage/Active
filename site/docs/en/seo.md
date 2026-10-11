---
title: SEO tags
description: Describe every page for search engines and social networks with Seo: title, description, canonical, hreflang, robots, Open Graph, Twitter and JSON-LD.
---

Search engines and social networks read the `<head>` of your pages: the title and description shown in results, the canonical address, the language versions, the preview image of a shared link. `active::Seo` writes all of these tags from one builder, and a page can contain several `Seo` values that are [merged](#merging) into a single, duplicate-free `<head>`.

## A complete example

```rust
use active::Seo;

let head = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com")
    .title("Oak chair")
    .description("Solid oak chair, built to last and delivered in 48 hours.")
    .canonical("/products/oak-chair")
    .image("/images/oak-chair.jpg")
    .og_type("product")
    .locale("en_US")
    .twitter("@acme")
    .alternate("en", "/products/oak-chair")
    .alternate("fr", "/fr/produits/chaise-chene")
    .to_html();

assert!(head.starts_with("<title>Oak chair · Acme</title>"));
assert!(head.contains(r#"<link rel="canonical" href="https://acme.com/products/oak-chair"/>"#));
```

The output, one tag per line here for reading (the real output has no line breaks):

```html
<title>Oak chair · Acme</title>
<meta name="description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<link rel="canonical" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="en" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="fr" href="https://acme.com/fr/produits/chaise-chene"/>
<meta property="og:title" content="Oak chair"/>
<meta property="og:description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<meta property="og:type" content="product"/>
<meta property="og:url" content="https://acme.com/products/oak-chair"/>
<meta property="og:image" content="https://acme.com/images/oak-chair.jpg"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
```

Inside a page, you rarely call `to_html()` yourself: put the `Seo` in the view and let [`Document`](../documents/) render it in the `<head>`.

## The builder

`Seo::new()` (or `Seo::default()`) starts empty. Every setter takes the value by `impl Into<String>` (a `bool` for `noindex` and `nofollow`) and returns the updated `Seo`, so calls chain.

| Method | Tags it produces |
| --- | --- |
| `.title(title)` | `<title>` and `og:title` |
| `.title_template(template)` | `<title>` only: the first `%s` is replaced by the title, e.g. `"%s · Acme"` |
| `.site_name(name)` | `og:site_name`, and the title when no title is set |
| `.description(text)` | `<meta name="description">` and `og:description` |
| `.canonical(url)` | `<link rel="canonical">` and `og:url` |
| `.image(url)` | `og:image`; also switches `twitter:card` to `summary_large_image` |
| `.og_type(type)` | `og:type`: `website` when not set, `article`, `product`… |
| `.locale(locale)` | `og:locale`, e.g. `en_US` |
| `.twitter(handle)` | `twitter:site`, e.g. `@acme` |
| `.base_url(origin)` | no tag: resolves relative `canonical`, `image` and `alternate` URLs |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang="…">`, one per call |
| `.json_ld(json)` | `<script type="application/ld+json">`, one per call (see [Structured data](../structured-data/)) |
| `.meta_name(name, content)` | any other `<meta name="…" content="…">` |
| `.meta_property(property, content)` | any other `<meta property="…" content="…">` |

To render: `.to_html()` returns a new `String`, `.write_html(&mut out)` appends to an existing one, and `Seo` implements `Display`, so `format!("<head>{seo}</head>")` works too.

## Rendering rules

These rules make the output predictable:

- **Fixed order.** Tags always come in the order shown in the example above, then the extra `meta_name`/`meta_property` tags in call order, then JSON-LD.
- **Empty means unset.** A value that is missing or an empty string produces no tag. `og:type` and `twitter:card` are always present.
- **Title and template.** With a title and a template, `<title>` is the template with `%s` replaced; `og:title` is the bare title, which reads better in link previews. Without a title, the site name is used as is, without the template. An explicitly empty title (`.title("")`) renders no `<title>` at all.
- **Robots.** The robots tag appears only when `noindex` or `nofollow` is `true`: `noindex,follow`, `index,nofollow` or `noindex,nofollow`. Pages are indexable by default, so there is nothing to write for normal pages.
- **URLs.** With `base_url("https://acme.com")`, a relative URL such as `/contact` or `contact` becomes `https://acme.com/contact`. URLs with a scheme (`https:`, `mailto:`…) or starting with `//` are kept as they are. One trailing `/` on the base is ignored.
- **Alternates.** Calling `.alternate` again with the same language replaces its URL in place. An empty URL is skipped.
- **Escaping.** Every value is HTML-escaped (`&`, `<`, `>`, `"`, `'`). In JSON-LD, `<` is written `\u003c` so the data can never close its `<script>` tag.

## Site defaults

Most values are the same on every page: site name, title template, base URL, default share image, locale, Twitter handle. Build them once and clone them, or give them to the document:

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us, we answer within a day.")
        .canonical("/contact")
        .to_html()
}
```

When the address of the site is only known at runtime (a `SITE_URL` setting, the `Host` header of the request), build the defaults in a function that takes the origin, as this website does in `site/src/server/pages.rs`.

## SEO in views

A `Seo` converts into a `Node`, so it can be a child anywhere in a view. It renders nothing in the body: [`Document`](../documents/) collects it and writes the tags in the `<head>`.

```rust
use active::prelude::*;

fn product_page(name: &str, slug: &str) -> Element {
    article()
        .child(
            Seo::new()
                .title(name.to_string())
                .description(format!("{name}: solid oak, delivered in 48 hours."))
                .canonical(format!("/products/{slug}"))
                .og_type("product"),
        )
        .child(h1().text(name.to_string()))
}

let html = Document::new(product_page("Oak chair", "oak-chair"))
    .lang("en")
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
    .render();

assert!(html.contains("<title>Oak chair · Acme</title>"));
assert!(html.contains(r#"<meta property="og:type" content="product"/>"#));
```

This keeps a page's description next to its content: the component that knows the product also describes it.

## Merging

A page can hold several `Seo` values: site defaults given to `Document::seo`, a layout's description, the page's title, a component's JSON-LD. `Document` merges them into one set of tags:

1. It starts from the `Seo` given to `Document::seo` (empty if none).
2. It walks the body in document order (the order the tags appear in the HTML) and applies each `Seo` it finds on top of the previous result.
3. A value set in a later `Seo` replaces the earlier one. Values a later `Seo` does not set are kept.
4. Alternates are replaced by language, extra meta tags by name or property, and JSON-LD blocks are added up.

In a typical tree the layout comes first and the page is nested inside it, so **the deepest, most specific value wins**:

```rust
use active::prelude::*;

fn layout(content: Element) -> Element {
    div()
        .child(Seo::new().description("Solid oak furniture.").noindex(true))
        .child(content)
}

let page = layout(section().child(Seo::new().title("Chairs").noindex(false)));
let html = Document::new(page)
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
    .render();

assert_eq!(html.matches("<title>").count(), 1);
assert!(html.contains("<title>Chairs · Acme</title>"));
assert!(html.contains(r#"content="Solid oak furniture.""#));
assert!(!html.contains("robots")); // the page's noindex(false) wins
```

Where the merge looks:

- every element's children, fragments and islands;
- the current content of dynamic children (`children_dyn`, `fragment_dyn`), so a page chosen by a router describes itself like any other;
- not inside raw HTML strings (`.html(…)`), which are plain text to `active`.

> **Tip:** Because "later wins", place a layout's `Seo` before the content it wraps, as above. A `Seo` added after the content would override the page's values.

You can also merge by hand: `a.merge(&b)` applies `b` on top of `a`, and `a.merged(&b)` returns a new value. `Document::final_seo()` returns the merged result of a whole page, which is handy in tests.

### In the browser

- **Islands** never touch the `<head>`: the server already wrote it.
- **Client-side apps** keep it up to date. When a view is mounted with `active::mount_to` or `active::mount`, its `Seo` values are merged and written to the head; each time a dynamic region is rebuilt (for example when the router shows another page), the region's `Seo` values are merged over those of its ancestors and the head is updated. Put the site defaults in a `Seo` in the root view. See [Client-side apps, Titles and meta tags](../client-app/#titles-and-meta-tags).
- `active::update_head(&seo)` writes a `Seo` into the head yourself, for example from an event handler. A `Seo` with neither a title nor a site name empties the page title.

## Languages and hreflang

For a site in several languages, give every version the same list of alternates, plus `x-default` for visitors whose language you do not offer:

```rust
use active::Seo;

fn alternates(seo: Seo, path: &str) -> Seo {
    ["en", "fr", "es"]
        .into_iter()
        .fold(seo, |seo, lang| seo.alternate(lang, format!("/{lang}{path}")))
        .alternate("x-default", format!("/en{path}"))
}

let seo = alternates(Seo::new().base_url("https://acme.com").title("Contact"), "/contact/");
assert!(seo.to_html().contains(r#"hreflang="x-default" href="https://acme.com/en/contact/""#));
```

Set `Document::lang` to the page's language as well, and `locale` to the matching Open Graph locale (`en_US`, `fr_FR`…).

## Pages that should not be indexed

Error pages, search results and private areas should stay out of search results:

```rust
use active::prelude::*;

fn not_found() -> Element {
    section()
        .child(Seo::new().title("Page not found").noindex(true))
        .child(h1().text("Page not found"))
}
```

Also return the right HTTP status (404) from your server; see [Vitesse](../vitesse/#errors-and-404).

## Using Seo without views

`Seo` does not need the rest of the library. It works with any template engine, web framework or static generator, on the server or in WebAssembly:

```rust
use active::Seo;

let seo = Seo::new().title("Hello").description("A page.");
let page = format!(
    r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>{seo}</head><body>…</body></html>"#
);
assert!(page.contains("<title>Hello</title>"));
```

## Checklist

- One clear title per page, around 50 to 60 characters, and a description of 120 to 160 characters.
- A canonical URL on every page, absolute (set `base_url`).
- A share image (`image`) of 1200 × 630 pixels for good link previews.
- `hreflang` alternates on every language version, including `x-default`.
- `noindex` on error, search and private pages.
- A `sitemap.xml` and a `robots.txt` (see [Vitesse](../vitesse/) and [Static sites](../static-site/#sitemap-and-robots-txt)).
- [Structured data](../structured-data/) where it helps: organization, breadcrumbs, articles, products.
