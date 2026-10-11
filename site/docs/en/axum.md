---
title: Axum
description: Serve active pages with Axum 0.8: routes returning Html, islands, static files, 404 pages, a sitemap, async data, and why rendering per request is safe.
---

[Axum](https://docs.rs/axum) is a popular Rust web framework built on Tokio. Rendering an `active` page in an Axum handler is one line, `Html(Document::new(view).render())`; this page builds a complete small shop around it: pages with SEO tags, an island, static files, a 404 page, a sitemap, and data loaded asynchronously. This website itself is an Axum server, in `site/src/server`.

## Dependencies

The crate layout is the one from [Installation](../installation/#one-crate-two-targets): components in `src/lib.rs`, the server in `src/main.rs`.

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
active = "0.1"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
axum = "0.8"
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread"] }
tower-http = { version = "0.7", features = ["compression-br", "compression-gzip", "fs"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"
```

## The pages

`src/lib.rs` holds the components, the island and one function per page. Each page function returns the finished HTML:

```rust
use active::prelude::*;

pub struct Product {
    pub slug: &'static str,
    pub name: &'static str,
    pub price: u32,
}

pub static PRODUCTS: [Product; 2] = [
    Product { slug: "oak-chair", name: "Oak chair", price: 249 },
    Product { slug: "pine-table", name: "Pine table", price: 590 },
];

pub fn find_product(slug: &str) -> Option<&'static Product> {
    PRODUCTS.iter().find(|p| p.slug == slug)
}

/// Island: a quantity picker, interactive in the browser.
pub fn quantity(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(1));
    div()
        .class("quantity")
        .child(button().on_click(move |_| n.update(|n| *n = n.saturating_sub(1).max(1))).text("−"))
        .child(output().text_dyn(move || n.get().to_string()))
        .child(button().on_click(move |_| n.update(|n| *n += 1)).text("+"))
        .into()
}

pub const ISLANDS: [(&str, active::IslandFn); 1] = [("quantity", quantity)];

/// Shared by every page. `origin` is the public address, e.g. "https://acme.com".
fn document(origin: &str, content: Element) -> String {
    let body = div()
        .child(header().child(a().href("/").text("Acme")))
        .child(main().child(content))
        .child(footer().text("© Acme"));
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url(origin.to_string()))
        .stylesheet("/static/app.css")
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}

pub fn home_page(origin: &str) -> String {
    let content = section()
        .child(Seo::new().title("Furniture").description("Solid wood furniture.").canonical("/"))
        .child(h1().text("Our furniture"))
        .child(ul().children(PRODUCTS.iter().map(|p| {
            li().child(a().href(format!("/products/{}", p.slug)).text(p.name))
        })));
    document(origin, content)
}

pub fn product_page(origin: &str, product: &Product) -> String {
    let content = article()
        .child(
            Seo::new()
                .title(product.name)
                .description(format!("{}, {} €, delivered in 48 hours.", product.name, product.price))
                .canonical(format!("/products/{}", product.slug))
                .og_type("product"),
        )
        .child(h1().text(product.name))
        .child(p().text(format!("{} €", product.price)))
        .child(island("quantity", "1", quantity));
    document(origin, content)
}

pub fn not_found_page(origin: &str) -> String {
    let content = section()
        .child(Seo::new().title("Page not found").noindex(true))
        .child(h1().text("Page not found"))
        .child(a().href("/").text("Back to the shop"));
    document(origin, content)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&ISLANDS);
}
```

## The server

`src/main.rs` maps URLs to page functions and serves the files:

```rust
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

/// Shared by every request.
struct App {
    /// Public address, used in canonical URLs and the sitemap.
    origin: String,
}

type Shared = State<Arc<App>>;

async fn home(State(app): Shared) -> Html<String> {
    Html(my_site::home_page(&app.origin))
}

async fn product(State(app): Shared, Path(slug): Path<String>) -> Response {
    match my_site::find_product(&slug) {
        Some(product) => Html(my_site::product_page(&app.origin, product)).into_response(),
        None => not_found(State(app)).await,
    }
}

async fn not_found(State(app): Shared) -> Response {
    (StatusCode::NOT_FOUND, Html(my_site::not_found_page(&app.origin))).into_response()
}

async fn sitemap(State(app): Shared) -> impl IntoResponse {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    let paths = std::iter::once("/".to_string())
        .chain(my_site::PRODUCTS.iter().map(|p| format!("/products/{}", p.slug)));
    for path in paths {
        xml.push_str(&format!("<url><loc>{}{path}</loc></url>", app.origin));
    }
    xml.push_str("</urlset>");
    ([(header::CONTENT_TYPE, "application/xml; charset=utf-8")], xml)
}

async fn robots(State(app): Shared) -> impl IntoResponse {
    let body = format!("User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n", app.origin);
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let origin = std::env::var("SITE_URL").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let app = Router::new()
        .route("/", get(home))
        .route("/products/{slug}", get(product))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
        .nest_service("/pkg", ServeDir::new("pkg"))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .with_state(Arc::new(App { origin }));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("http://localhost:{port}");
    axum::serve(listener, app).await.unwrap();
}
```

Build the bundle into `pkg/` ([Building the client](../client-build/)), put `app.css` in `static/`, and run `cargo run`.

What each part does:

- **`Html(...)`** sets `Content-Type: text/html; charset=utf-8`.
- **`ServeDir`** serves the bundle and static files with the right content types, including `application/wasm`.
- **`CompressionLayer`** compresses pages, the loader and the `.wasm` file with Brotli or gzip, depending on what the browser accepts.
- **`PORT` and `SITE_URL`** come from the environment, which is how hosts such as [Heroku](../deploy-heroku/) configure an app.

## Errors and 404

Return the right status code with the page: search engines index a page answered with `200` even if it says "not found". In the example, unknown products and unknown URLs (`fallback`) both answer `404` with a page marked `noindex`.

## Async data

Real pages load data from a database or an API. Do all the waiting first, then build and render the view **without any `.await` in between**:

```rust
use active::prelude::*;
use axum::{extract::Path, response::Html};

/// Stands for a database query or an HTTP call.
async fn fetch_reviews(slug: &str) -> Vec<String> {
    vec![format!("Great {slug}, five stars.")]
}

fn reviews_page(slug: &str, reviews: &[String]) -> String {
    let body = section()
        .child(Seo::new().title(format!("Reviews of {slug}")))
        .child(ul().children(reviews.iter().map(|r| li().text(r.clone()))));
    Document::new(body).render()
}

async fn reviews(Path(slug): Path<String>) -> Html<String> {
    // 1. Await everything the page needs.
    let reviews = fetch_reviews(&slug).await;
    // 2. Build and render in one synchronous call.
    Html(reviews_page(&slug, &reviews))
}
```

The compiler enforces this rule. Axum runs handlers on several threads, so their futures must be `Send`. Views (which hold reference-counted closures) and states (which belong to the thread that created them) are not `Send`: a handler that keeps either across an `.await` does not compile. The fix is always the same: move the `.await` before the code that builds the view.

> **Tip:** Axum reports this as a long "the trait bound `…: Handler<_, _>` is not satisfied" error. Put `#[axum::debug_handler]` on the handler (it needs axum's `macros` feature) to get the real cause instead: "future cannot be sent between threads safely", pointing at the value kept across the `.await`.

## Why per-request rendering is safe

`active` keeps states in a store that belongs to the current thread. With the pattern above, each request builds its view and renders it in one synchronous call, on one thread, from start to finish: nothing from another request can run in between, and since views and states cannot cross an `.await`, nothing can leak from one request to another. At the end of each render, the states that the last two renders did not use are released, so a server can render pages concurrently on all its threads, for as long as it runs, while keeping only a handful of live states. Details are in [State, On the server](../state/#on-the-server).

## Embedding the bundle in the binary

To deploy a single file, embed the bundle with `include_bytes!` instead of serving `pkg/` from disk. Build the bundle before the server, since the files are read at compile time:

```rust
use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

static JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site.js"));
static WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site_bg.wasm"));

/// Routes for the embedded bundle, usable in a router with any state type.
fn bundle_routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/pkg/my_site.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript")], JS).into_response() }))
        .route("/pkg/my_site_bg.wasm", get(|| async { ([(header::CONTENT_TYPE, "application/wasm")], WASM).into_response() }))
}
```

Merge them into your router with `.merge(bundle_routes())` (before `.with_state(...)`). With a content hash in the URLs (see [Documents, Long-term caching](../documents/#long-term-caching)), you can also add a `Cache-Control: public, max-age=31536000, immutable` header.

## Testing

Axum routers can be called without a network, which makes page tests fast:

```rust
use axum::{Router, body::Body, http::{Request, StatusCode}, response::Html, routing::get};
use tower::ServiceExt;

#[tokio::test]
async fn home_has_a_title() {
    let app = Router::new().route("/", get(|| async { Html(String::from("<title>Home</title>")) }));
    let response = app.oneshot(Request::get("/").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1_000_000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("<title>Home</title>"));
}
```

This needs `tower = { version = "0.5", features = ["util"] }` in `[dev-dependencies]`. This website's tests, in `site/tests/server.rs`, check its SEO tags, redirects and caching this way.

## Other servers

Nothing in `active` is specific to Axum: `Document::render()` returns a `String`. In Actix Web, Rocket, Warp, Hyper or a hand-written server, send that string with the `text/html; charset=utf-8` content type. `examples/hello` does it with the standard library alone.
