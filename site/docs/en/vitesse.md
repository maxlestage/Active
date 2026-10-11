---
title: Vitesse
description: Serve Active pages with Vitesse: routes returning Html, islands, static files, 404 pages, a sitemap, async data, compression, tests, and why rendering per request is safe.
---

[Vitesse](https://maxlestage.github.io/Vitesse/) is a Rust web framework with the API of Express: `app.get`, `req.param`, middlewares, routers, its own HTTP/1.1 engine on tokio. Rendering an `active` page in a Vitesse handler is one line, `Html(Document::new(view).render())`; this page builds a complete small shop around it: pages with SEO tags, an island, static files, a 404 page, a sitemap, and data loaded asynchronously. This website itself is a Vitesse server, in `site/src/server`.

## Dependencies

The crate layout is the one from [Installation](../installation/#one-crate-two-targets): components in `src/lib.rs`, the server in `src/main.rs`. Vitesse brings its own runtime: there is no tokio to add.

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
vitesse = { version = "0.1", features = ["compress"] }

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
use vitesse::prelude::*;

/// Shared by every request.
struct Shop {
    /// Public address, used in canonical URLs and the sitemap.
    origin: String,
}

async fn home(req: Request) -> Html<String> {
    let shop = req.state::<Shop>();
    Html(my_site::home_page(&shop.origin))
}

async fn product(req: Request) -> Response {
    let shop = req.state::<Shop>();
    match my_site::find_product(req.param("slug").unwrap_or("")) {
        Some(product) => Html(my_site::product_page(&shop.origin, product)).into_response(),
        None => not_found(req).await,
    }
}

async fn not_found(req: Request) -> Response {
    let shop = req.state::<Shop>();
    res::status(404).html(my_site::not_found_page(&shop.origin))
}

async fn sitemap(req: Request) -> Response {
    let shop = req.state::<Shop>();
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    let paths = std::iter::once("/".to_string())
        .chain(my_site::PRODUCTS.iter().map(|p| format!("/products/{}", p.slug)));
    for path in paths {
        xml.push_str(&format!("<url><loc>{}{path}</loc></url>", shop.origin));
    }
    xml.push_str("</urlset>");
    res::send(xml).header("content-type", "application/xml; charset=utf-8")
}

async fn robots(req: Request) -> String {
    let shop = req.state::<Shop>();
    format!("User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n", shop.origin)
}

fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let origin = std::env::var("SITE_URL").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let mut app = App::new();
    app.state(Shop { origin });
    app.middleware(middleware::compress());
    app.get("/", home)
        .get("/products/:slug", product)
        .get("/sitemap.xml", sitemap)
        .get("/robots.txt", robots)
        .static_dir("/pkg", "pkg")
        .static_dir("/static", "static")
        .fallback(not_found);
    println!("http://localhost:{port}");
    app.run(port)
}
```

Build the bundle into `pkg/` ([Building the client](../client-build/)), put `app.css` in `static/`, and run `cargo run`.

What each part does:

- **`Html(...)`** sets `Content-Type: text/html; charset=utf-8`.
- **`static_dir`** serves the bundle and static files with the right content types, including `application/wasm`, and the `ETag` and `Last-Modified` headers browsers cache with.
- **`middleware::compress()`** compresses pages, the loader and the `.wasm` file with Brotli or gzip, depending on what the browser accepts (the `compress` feature of Vitesse).
- **`app.state(...)`** shares a value with every handler, which reads it with `req.state()`.
- **`PORT` and `SITE_URL`** come from the environment, which is how hosts such as [Heroku](../deploy-heroku/) configure an app. `app.run` listens on every interface, starts the runtime on every core and stops cleanly on Ctrl+C or SIGTERM.

## Errors and 404

Return the right status code with the page: search engines index a page answered with `200` even if it says "not found". In the example, unknown products and unknown URLs (`fallback`) both answer `404` with a page marked `noindex`.

## Async data

Real pages load data from a database or an API. Do all the waiting first, then build and render the view **without any `.await` in between**:

```rust
use active::prelude::*;
use vitesse::prelude::*;

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

async fn reviews(req: Request) -> Html<String> {
    let slug = req.param("slug").unwrap_or("").to_string();
    // 1. Await everything the page needs.
    let reviews = fetch_reviews(&slug).await;
    // 2. Build and render in one synchronous call.
    Html(reviews_page(&slug, &reviews))
}
```

The compiler enforces this rule. Vitesse runs handlers on several threads, so their futures must be `Send`. Views (which hold reference-counted closures) and states (which belong to the thread that created them) are not `Send`: a handler that keeps either across an `.await` does not compile. The fix is always the same: move the `.await` before the code that builds the view.

> **Tip:** The error says that the future "cannot be sent between threads safely", or that the function is not a `Handler`; in both cases it points at the view or the state kept across the `.await`.

## Why per-request rendering is safe

`active` keeps states in a store that belongs to the current thread. With the pattern above, each request builds its view and renders it in one synchronous call, on one thread, from start to finish: nothing from another request can run in between, and since views and states cannot cross an `.await`, nothing can leak from one request to another. At the end of each render, the states that the last two renders did not use are released, so a server can render pages concurrently on all its threads, for as long as it runs, while keeping only a handful of live states. Details are in [State, On the server](../state/#on-the-server).

## Embedding the bundle in the binary

To deploy a single file, embed the bundle with `include_bytes!` instead of serving `pkg/` from disk. Build the bundle before the server, since the files are read at compile time:

```rust
use vitesse::prelude::*;

static JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site.js"));
static WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site_bg.wasm"));

/// The routes of the embedded bundle, to mount in any app.
fn bundle() -> Router {
    let mut router = Router::new();
    router
        .get("/my_site.js", |_| async { res::send(JS).header("content-type", "text/javascript") })
        .get("/my_site_bg.wasm", |_| async { res::send(WASM).header("content-type", "application/wasm") });
    router
}
```

Mount them with `app.mount("/pkg", bundle())`. With a content hash in the URLs (see [Documents, Long-term caching](../documents/#long-term-caching)), you can also add a `Cache-Control: public, max-age=31536000, immutable` header.

## Testing

Vitesse's test client calls the app without a network, which makes page tests fast:

```rust
use vitesse::prelude::*;
use vitesse::test::TestClient;

#[tokio::test]
async fn home_has_a_title() {
    let mut app = App::new();
    app.get("/", |_| async { Html("<title>Home</title>") });
    let client = TestClient::new(app);
    let response = client.get("/").await;
    assert_eq!(response.status(), 200);
    assert!(response.text().contains("<title>Home</title>"));
}
```

This needs `tokio = { version = "1", features = ["macros", "rt"] }` in `[dev-dependencies]`. This website's tests, in `site/tests/server.rs`, check its SEO tags, redirects and caching this way.

## Other servers

Nothing in `active` is specific to Vitesse: `Document::render()` returns a `String`. In Actix Web, Rocket, Warp, Hyper or a hand-written server, send that string with the `text/html; charset=utf-8` content type. `examples/hello` does it with the standard library alone.
