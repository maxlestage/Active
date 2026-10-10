//! Shared by the server and the browser.
use active::prelude::*;

/// An island: rendered on the server, made interactive in the browser.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    div()
        .class("counter")
        .child(
            button()
                .attr("aria-label", "minus")
                .on_click(move |_| count.update(|n| *n -= 1))
                .text("−"),
        )
        .child(
            span()
                .class_if("negative", move || count.get() < 0)
                .text_dyn(move || count.get().to_string()),
        )
        .child(
            button()
                .attr("aria-label", "plus")
                .on_click(move |_| count.update(|n| *n += 1))
                .text("+"),
        )
        .into()
}

/// The whole page, with its SEO tags. Static parts ship no code to the browser.
pub fn page() -> String {
    let site = Seo::new().site_name("Hello").title_template("%s · Hello");
    let body = main()
        .child(
            Seo::new()
                .title("Counter")
                .description("A server-rendered page with one WebAssembly island."),
        )
        .child(h1().text("Hello from Rust"))
        .child(p().text("Everything below is HTML from the server. The counter is an island:"))
        .child(island("counter", "3", counter));
    Document::new(body)
        .lang("en")
        .seo(site)
        .head("<style>body{font:18px system-ui;margin:3rem}.counter{display:flex;gap:1rem;align-items:center}.negative{color:crimson}</style>")
        .client("/pkg/hello.js", "/pkg/hello_bg.wasm")
        .render()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
