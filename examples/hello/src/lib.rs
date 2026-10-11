//! Shared by the server and the browser.
use active::prelude::*;
use active::pwa::Manifest;

/// The color of the app: the status bar, the splash screen and the icon.
pub const THEME: &str = "#6d4aff";

const STYLE: &str = "<style>body{font:18px system-ui;margin:3rem}.counter{display:flex;gap:1rem;align-items:center}.negative{color:crimson}</style>";

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

/// The web app manifest: the name, the colors and the icons of the installed app
/// (`static/`, drawn from `icon.svg`).
pub fn manifest() -> Manifest {
    Manifest::new("Hello")
        .short_name("Hello")
        .description("A server-rendered page with one WebAssembly island.")
        .start_url("/")
        .scope("/")
        .id("/")
        .theme_color(THEME)
        .background_color("#0b0b10")
        .lang("en")
        .icon("/static/icon-192.png", "192x192", "image/png")
        .icon("/static/icon-512.png", "512x512", "image/png")
        .maskable_icon("/static/icon-maskable-512.png", "512x512", "image/png")
        .icon("/static/icon.svg", "any", "image/svg+xml")
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
        .head(STYLE)
        // Installable, and kept on the device by the service worker (main.rs serves both).
        .manifest("/manifest.webmanifest", &manifest())
        .service_worker("/sw.js")
        .client("/pkg/hello.js", "/pkg/hello_bg.wasm")
        .render()
}

/// `/offline/`: what the service worker shows when a page is not saved on the device and
/// the network is down.
pub fn offline() -> String {
    let body = main()
        .child(
            Seo::new()
                .title("Offline")
                .description("This page is not saved on this device yet.")
                .noindex(true),
        )
        .child(h1().text("You are offline"))
        .child(
            p().text("This page is not saved on this device yet. ")
                .child(a().href("/").text("The counter"))
                .text(" is."),
        );
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Hello").title_template("%s · Hello"))
        .head(STYLE)
        .manifest("/manifest.webmanifest", &manifest())
        .render()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_is_installable() {
        let html = page();
        assert!(html.contains(r#"<link rel="manifest" href="/manifest.webmanifest"/>"#));
        assert!(html.contains(r##"<meta name="theme-color" content="#6d4aff"/>"##));
        assert!(html.contains(r#"<link rel="apple-touch-icon" href="/static/icon-512.png"/>"#));
        assert!(html.contains(r#"navigator.serviceWorker.register("/sw.js")"#));
        let json = manifest().to_json();
        assert!(json.starts_with(r#"{"name":"Hello","short_name":"Hello""#));
        assert!(json.contains(r#""start_url":"/","scope":"/","display":"standalone""#));
        assert!(json.contains(r#""src":"/static/icon-512.png""#));
        assert!(json.contains(r#""purpose":"maskable""#));
        let offline = offline();
        assert!(offline.contains("You are offline") && offline.contains(r#"href="/""#));
        assert!(offline.contains(r#"<meta name="robots" content="noindex,follow"/>"#));
    }
}
