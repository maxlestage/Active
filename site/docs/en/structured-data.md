---
title: Structured data
description: Add JSON-LD structured data to your pages with Seo::json_ld: organizations, breadcrumbs, articles, products and FAQs, built with serde_json or by hand.
---

Structured data tells search engines what a page is about in a machine-readable form: this page is a product with a price, this one an article by an author, these links are a breadcrumb trail. Google uses it for rich results (stars, prices, breadcrumbs, FAQ entries). The standard format is [JSON-LD](https://json-ld.org) with the [schema.org](https://schema.org) vocabulary, in a `<script type="application/ld+json">` tag.

`active` adds it through the same `Seo` builder as the other tags.

## Seo::json_ld

`.json_ld(json)` takes a string of JSON you have already serialized and adds one `<script type="application/ld+json">` tag per call, after the other head tags:

```rust
use active::Seo;

let seo = Seo::new().json_ld(
    r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}"#,
);
assert!(seo.to_html().ends_with(
    r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}</script>"#
));
```

The JSON is written as is, with one change: every `<` becomes `\u003c`. That is still the same JSON value, but a string such as `</script>` in your data can never close the tag early and inject HTML. An empty string adds nothing.

## Building the JSON with serde_json

The safest way to produce the JSON is [`serde_json`](https://docs.rs/serde_json), which escapes every value correctly:

```toml
[dependencies]
serde_json = "1"
```

```rust
use active::Seo;
use serde_json::json;

fn product_seo(name: &str, price: &str, image: &str) -> Seo {
    let data = json!({
        "@context": "https://schema.org",
        "@type": "Product",
        "name": name,
        "image": image,
        "offers": {
            "@type": "Offer",
            "price": price,
            "priceCurrency": "EUR",
            "availability": "https://schema.org/InStock"
        }
    });
    Seo::new().title(name.to_string()).og_type("product").json_ld(data.to_string())
}

let html = product_seo("Oak chair", "249.00", "https://acme.com/oak.jpg").to_html();
assert!(html.contains(r#""@type":"Product""#));
```

`json!` builds a `serde_json::Value`, and `to_string()` serializes it without whitespace. With your own types, derive `serde::Serialize` and call `serde_json::to_string(&value)`.

## Without a dependency

For a few fields, `format!` works, provided every text value is escaped as a JSON string:

```rust
use active::Seo;

/// `s` as a JSON string literal, quotes included.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

let title = r#"Say "hello" to Rust"#;
let seo = Seo::new().json_ld(format!(
    r#"{{"@context":"https://schema.org","@type":"Article","headline":{}}}"#,
    json_string(title)
));
assert!(seo.to_html().contains(r#""headline":"Say \"hello\" to Rust""#));
```

Remember to double the braces (`{{` and `}}`) of the JSON itself inside `format!`.

## Common types

Every example below is a JSON-LD object; pass it to `.json_ld(...)` as shown above.

### Organization and website

Once per site, typically in the layout or the home page:

```text
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "Acme",
  "url": "https://acme.com",
  "logo": "https://acme.com/logo.png",
  "sameAs": ["https://acme.example"]
}
```

### Breadcrumbs

Shows the page's position in the site in search results. This website adds one to every documentation page:

```text
{
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    { "@type": "ListItem", "position": 1, "name": "Docs", "item": "https://acme.com/docs/" },
    { "@type": "ListItem", "position": 2, "name": "SEO", "item": "https://acme.com/docs/seo/" }
  ]
}
```

### Article

For blog posts and news. Pair it with `.og_type("article")`:

```text
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Shipping a Rust website",
  "datePublished": "2026-03-01",
  "author": { "@type": "Person", "name": "Ada" },
  "image": "https://acme.com/posts/rust-site.png"
}
```

### FAQ

```text
{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "mainEntity": [
    {
      "@type": "Question",
      "name": "Do you ship abroad?",
      "acceptedAnswer": { "@type": "Answer", "text": "Yes, to the whole European Union." }
    }
  ]
}
```

The JSON is shown on several lines here for reading; `serde_json` produces a single line, and both are valid.

## Several blocks on one page

Each `json_ld` call adds its own `<script>` tag, and when [`Seo` values merge](../seo/#merging), JSON-LD blocks are **added up** rather than replaced. A layout can declare the organization while each page adds its breadcrumbs:

```rust
use active::prelude::*;

let organization = r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#;
let breadcrumbs = r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#;

let page = div()
    .child(Seo::new().json_ld(organization))
    .child(article().child(Seo::new().title("Chairs").json_ld(breadcrumbs)));

let html = Document::new(page).render();
assert_eq!(html.matches(r#"<script type="application/ld+json">"#).count(), 2);
```

> **Warning:** Because blocks add up, a block declared by both the layout and a page appears twice. Declare each site-wide block in one place only.

## Check your markup

- Test a public page with Google's [Rich Results Test](https://search.google.com/test/rich-results) or the [Schema Markup Validator](https://validator.schema.org).
- In your own tests, render the page and look at the merged tags with `Document::final_seo()`:

```rust
use active::prelude::*;

let page = main().child(Seo::new().json_ld(r#"{"@type":"Thing"}"#));
let seo = Document::new(page).final_seo();
assert!(seo.to_html().contains(r#"{"@type":"Thing"}"#));
```
