//! Code shown in the home page tabs, highlighted once.

use std::sync::OnceLock;

use super::highlight::highlight;

const SNIPPETS: [&str; 4] = [
    r#"use active::prelude::*;

// A component is a function that returns a view.
fn product_card(name: &str, price: u32) -> Element {
    article()
        .class("card")
        .child(h2().text(name.to_string()))
        .child(p().class("price").text(format!("{price} €")))
}

// Rendered on the server, straight to HTML.
let html = Node::from(product_card("Oak chair", 249)).render();"#,
    r#"use active::{Document, Seo};

// Site defaults, then page values: every Seo merges, the deepest wins.
let site = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com")
    .image("/og.png");

let page = main()
    .child(Seo::new().title("Oak chair").canonical("/products/oak-chair"))
    .child(h1().text("Oak chair"));

let html = Document::new(page).lang("en").seo(site).render();"#,
    r#"use active::prelude::*;

// The same function renders on the server and hydrates in the browser.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

// Browser bundle (wasm32): wake up the islands.
#[wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}"#,
    r#"use axum::{response::Html, routing::get, Router};

async fn home() -> Html<String> {
    let body = main()
        .child(h1().text("Hello"))
        .child(island("counter", "0", counter));
    Html(Document::new(body)
        .seo(Seo::new().title("Home"))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render())
}

let app = Router::new().route("/", get(home));"#,
];

/// Highlighted HTML of tab `index`.
pub fn html(index: usize) -> String {
    static HTML: OnceLock<Vec<String>> = OnceLock::new();
    let all = HTML.get_or_init(|| {
        SNIPPETS
            .iter()
            .map(|code| highlight(code, "rust"))
            .collect()
    });
    all.get(index)
        .map(|code| format!("<pre><code>{code}</code></pre>"))
        .unwrap_or_default()
}
