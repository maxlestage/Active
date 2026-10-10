//! A complete page without any web framework: a layout, a page, an island and the SEO
//! tags they declare, merged into one `<head>`.
//!
//! Run with `cargo run -p active --example page`.

use active::prelude::*;

/// Shared by every page.
fn layout(content: Element) -> Element {
    div()
        .class("app")
        .child(Seo::new().description("Solid oak furniture, delivered in 48 hours."))
        .child(header().child(a().href("/").text("Acme")))
        .child(content)
        .child(footer().text("© Acme"))
}

/// Interactive in the browser once the page is hydrated; plain HTML here.
fn quantity(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(1));
    div()
        .class("quantity")
        .child(
            button()
                .on_click(move |_| n.update(|n| *n = n.saturating_sub(1)))
                .text("−"),
        )
        .child(output().text_dyn(move || n.get().to_string()))
        .child(button().on_click(move |_| n.update(|n| *n += 1)).text("+"))
        .into()
}

fn product() -> Element {
    // `main` is also the name of this program's entry point, hence the full path.
    active::html::main()
        .child(
            Seo::new()
                .title("Oslo chair")
                .canonical("/products/oslo")
                .og_type("product")
                .json_ld(
                    r#"{"@context":"https://schema.org","@type":"Product","name":"Oslo chair"}"#,
                ),
        )
        .child(h1().text("Oslo chair"))
        .child(p().text("Solid oak, 4.5 kg."))
        .child(island("quantity", "1", quantity))
}

fn main() {
    let site = Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png");
    let html = Document::new(layout(product()))
        .lang("en")
        .seo(site)
        .stylesheet("/app.css")
        .client("/pkg/acme.js", "/pkg/acme_bg.wasm")
        .render();
    println!("{html}");
}
