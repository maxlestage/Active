---
title: Axum
description: Sirve páginas de active con Axum 0.8: rutas que devuelven Html, islas, archivos estáticos, 404, sitemap, datos asíncronos y renderizado seguro por petición.
---

[Axum](https://docs.rs/axum) es un framework web de Rust muy popular, construido sobre Tokio. Renderizar una página de `active` en un manejador de Axum ocupa una línea, `Html(Document::new(view).render())`; esta página construye a su alrededor una pequeña tienda completa: páginas con etiquetas SEO, una isla, archivos estáticos, una página 404, un sitemap y datos cargados de forma asíncrona. Este mismo sitio web es un servidor Axum, en `site/src/server`.

## Dependencias

La estructura del crate es la de [Instalación](../installation/#un-crate-dos-destinos): los componentes en `src/lib.rs` y el servidor en `src/main.rs`.

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

## Las páginas

`src/lib.rs` contiene los componentes, la isla y una función por página. Cada función de página devuelve el HTML terminado:

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

/// Isla: un selector de cantidad, interactivo en el navegador.
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

/// Compartido por todas las páginas. `origin` es la dirección pública, p. ej. "https://acme.com".
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

## El servidor

`src/main.rs` asocia las URL con las funciones de página y sirve los archivos:

```rust
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

/// Compartido por todas las peticiones.
struct App {
    /// Dirección pública, usada en las URL canónicas y en el sitemap.
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

Compila el bundle en `pkg/` ([Compilar el cliente](../client-build/)), pon `app.css` en `static/` y ejecuta `cargo run`.

Qué hace cada parte:

- **`Html(...)`** define `Content-Type: text/html; charset=utf-8`.
- **`ServeDir`** sirve el bundle y los archivos estáticos con los tipos de contenido correctos, incluido `application/wasm`.
- **`CompressionLayer`** comprime las páginas, el cargador y el archivo `.wasm` con Brotli o gzip, según lo que acepte el navegador.
- **`PORT` y `SITE_URL`** vienen del entorno, que es como los hostings como [Heroku](../deploy-heroku/) configuran una aplicación.

## Errores y 404

Devuelve el código de estado correcto junto con la página: los buscadores indexan una página respondida con `200` aunque diga "no encontrada". En el ejemplo, tanto los productos desconocidos como las URL desconocidas (`fallback`) responden `404` con una página marcada como `noindex`.

## Datos asíncronos

Las páginas reales cargan datos de una base de datos o de una API. Haz primero toda la espera y después construye y renderiza la vista **sin ningún `.await` entre medias**:

```rust
use active::prelude::*;
use axum::{extract::Path, response::Html};

/// Representa una consulta a una base de datos o una llamada HTTP.
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
    // 1. Espera todo lo que necesita la página.
    let reviews = fetch_reviews(&slug).await;
    // 2. Construye y renderiza en una sola llamada síncrona.
    Html(reviews_page(&slug, &reviews))
}
```

El compilador impone esta regla. Axum ejecuta los manejadores en varios hilos, así que sus futures deben ser `Send`. Las vistas (que contienen closures con conteo de referencias) y los estados (que pertenecen al hilo que los creó) no son `Send`: un manejador que conserve cualquiera de ellos a través de un `.await` no compila. La solución es siempre la misma: mueve el `.await` antes del código que construye la vista.

> **Consejo:** Axum lo notifica con un largo error "the trait bound `…: Handler<_, _>` is not satisfied". Pon `#[axum::debug_handler]` en el manejador (requiere la feature `macros` de axum) para obtener en su lugar la causa real: "future cannot be sent between threads safely", que señala el valor conservado a través del `.await`.

## Por qué es seguro renderizar en cada petición

`active` guarda los estados en un almacén que pertenece al hilo actual. Con el patrón anterior, cada petición construye su vista y la renderiza en una sola llamada síncrona, en un solo hilo, de principio a fin: nada de otra petición puede ejecutarse entre medias y, como las vistas y los estados no pueden cruzar un `.await`, nada puede filtrarse de una petición a otra. Al final de cada renderizado se liberan los estados que no usaron los dos últimos renderizados, así que un servidor puede renderizar páginas de forma concurrente en todos sus hilos, durante todo el tiempo que esté en marcha, manteniendo solo un puñado de estados vivos. Los detalles están en [Estado, En el servidor](../state/#en-el-servidor).

## Incrustar el bundle en el binario

Para desplegar un único archivo, incrusta el bundle con `include_bytes!` en lugar de servir `pkg/` desde el disco. Compila el bundle antes que el servidor, ya que los archivos se leen en tiempo de compilación:

```rust
use axum::Router;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;

static JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site.js"));
static WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site_bg.wasm"));

/// Rutas para el bundle incrustado, utilizables en un enrutador con cualquier tipo de estado.
fn bundle_routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/pkg/my_site.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript")], JS).into_response() }))
        .route("/pkg/my_site_bg.wasm", get(|| async { ([(header::CONTENT_TYPE, "application/wasm")], WASM).into_response() }))
}
```

Combínalas con tu enrutador mediante `.merge(bundle_routes())` (antes de `.with_state(...)`). Con un hash del contenido en las URL (consulta [Documentos, Caché a largo plazo](../documents/#cache-a-largo-plazo)), también puedes añadir una cabecera `Cache-Control: public, max-age=31536000, immutable`.

## Pruebas

Los enrutadores de Axum se pueden llamar sin red, lo que hace que las pruebas de páginas sean rápidas:

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

Esto requiere `tower = { version = "0.5", features = ["util"] }` en `[dev-dependencies]`. Las pruebas de este sitio, en `site/tests/server.rs`, comprueban así sus etiquetas SEO, sus redirecciones y su caché.

## Otros servidores

Nada en `active` es específico de Axum: `Document::render()` devuelve un `String`. En Actix Web, Rocket, Warp, Hyper o un servidor escrito a mano, envía esa cadena con el tipo de contenido `text/html; charset=utf-8`. `examples/hello` lo hace solo con la biblioteca estándar.
