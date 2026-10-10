//! A request handler in the style of Axum or Actix Web, without the framework: site
//! defaults built once, cloned per page, and the tags dropped into the page `<head>`.
//!
//! Run with `cargo run --example axum_like`.

use std::sync::LazyLock;

use active::Seo;

/// Site-wide values, built once (like `createSeo(defaults)` in active-react).
static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
        .locale("fr_FR")
        .twitter("@acme")
});

/// What `async fn contact() -> Html<String>` would return in Axum.
fn contact() -> String {
    let seo = SITE
        .clone()
        .title("Contact")
        .description("Write to us.")
        .canonical("/contact")
        .og_type("article")
        .alternate("en", "/en/contact")
        .alternate("fr", "/contact")
        .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}"#)
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "Acme logo");

    // `Seo` implements `Display`: the tags are written straight into the page.
    format!(
        r#"<!doctype html><html lang="fr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">{seo}</head><body><h1>Contact</h1><p>Write to us.</p></body></html>"#
    )
}

fn main() {
    println!("{}", contact());
}
