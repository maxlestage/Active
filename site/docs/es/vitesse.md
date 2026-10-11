---
title: Vitesse
description: Sirve páginas de Active con Vitesse: rutas que devuelven Html, islas, archivos estáticos, 404, sitemap, datos asíncronos, compresión, pruebas y renderizado seguro por petición.
---

[Vitesse](https://maxlestage.github.io/Vitesse/) es un framework web de Rust con la API de Express: `app.get`, `req.param`, middlewares, enrutadores, su propio motor HTTP/1.1 sobre tokio. Renderizar una página de `active` en un manejador de Vitesse ocupa una línea, `Html(Document::new(view).render())`; esta página construye a su alrededor una pequeña tienda completa: páginas con etiquetas SEO, una isla, archivos estáticos, una página 404, un sitemap y datos cargados de forma asíncrona. Este mismo sitio web es un servidor Vitesse, en `site/src/server`.

## Dependencias

La estructura del crate es la de [Instalación](../installation/#un-crate-dos-destinos): los componentes en `src/lib.rs` y el servidor en `src/main.rs`. Vitesse trae su propio runtime: no hay que añadir tokio.

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
active = { git = "https://github.com/maxlestage/Active" }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
vitesse = { git = "https://github.com/maxlestage/Vitesse", features = ["compress"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { git = "https://github.com/maxlestage/Active", features = ["client"] }
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
use vitesse::prelude::*;

/// Compartido por todas las peticiones.
struct Shop {
    /// Dirección pública, usada en las URL canónicas y en el sitemap.
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

Compila el bundle en `pkg/` ([Compilar el cliente](../client-build/)), pon `app.css` en `static/` y ejecuta `cargo run`.

Qué hace cada parte:

- **`Html(...)`** define `Content-Type: text/html; charset=utf-8`.
- **`static_dir`** sirve el bundle y los archivos estáticos con los tipos de contenido correctos, incluido `application/wasm`, y las cabeceras `ETag` y `Last-Modified` con las que los navegadores los guardan en caché.
- **`middleware::compress()`** comprime las páginas, el cargador y el archivo `.wasm` con Brotli o gzip, según lo que acepte el navegador (la feature `compress` de Vitesse).
- **`app.state(...)`** comparte un valor con todos los manejadores, que lo leen con `req.state()`.
- **`PORT` y `SITE_URL`** vienen del entorno, que es como los hostings como [Heroku](../deploy-heroku/) configuran una aplicación. `app.run` escucha en todas las interfaces, arranca el runtime en todos los núcleos y se detiene limpiamente con Ctrl+C o SIGTERM.

## Errores y 404

Devuelve el código de estado correcto junto con la página: los buscadores indexan una página respondida con `200` aunque diga "no encontrada". En el ejemplo, tanto los productos desconocidos como las URL desconocidas (`fallback`) responden `404` con una página marcada como `noindex`.

## Datos asíncronos

Las páginas reales cargan datos de una base de datos o de una API. Haz primero toda la espera y después construye y renderiza la vista **sin ningún `.await` entre medias**:

```rust
use active::prelude::*;
use vitesse::prelude::*;

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

async fn reviews(req: Request) -> Html<String> {
    let slug = req.param("slug").unwrap_or("").to_string();
    // 1. Espera todo lo que necesita la página.
    let reviews = fetch_reviews(&slug).await;
    // 2. Construye y renderiza en una sola llamada síncrona.
    Html(reviews_page(&slug, &reviews))
}
```

El compilador impone esta regla. Vitesse ejecuta los manejadores en varios hilos, así que sus futures deben ser `Send`. Las vistas (que contienen closures con conteo de referencias) y los estados (que pertenecen al hilo que los creó) no son `Send`: un manejador que conserve cualquiera de ellos a través de un `.await` no compila. La solución es siempre la misma: mueve el `.await` antes del código que construye la vista.

> **Consejo:** El error dice que el future "cannot be sent between threads safely", o que la función no es un `Handler`; en ambos casos señala la vista o el estado conservado a través del `.await`.

## Por qué es seguro renderizar en cada petición

`active` guarda los estados en un almacén que pertenece al hilo actual. Con el patrón anterior, cada petición construye su vista y la renderiza en una sola llamada síncrona, en un solo hilo, de principio a fin: nada de otra petición puede ejecutarse entre medias y, como las vistas y los estados no pueden cruzar un `.await`, nada puede filtrarse de una petición a otra. Al final de cada renderizado se liberan los estados que no usaron los dos últimos renderizados, así que un servidor puede renderizar páginas de forma concurrente en todos sus hilos, durante todo el tiempo que esté en marcha, manteniendo solo un puñado de estados vivos. Los detalles están en [Estado, En el servidor](../state/#en-el-servidor).

## Incrustar el bundle en el binario

Para desplegar un único archivo, incrusta el bundle con `include_bytes!` en lugar de servir `pkg/` desde el disco. Compila el bundle antes que el servidor, ya que los archivos se leen en tiempo de compilación:

```rust
use vitesse::prelude::*;

static JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site.js"));
static WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site_bg.wasm"));

/// Las rutas del bundle incrustado, para montar en cualquier aplicación.
fn bundle() -> Router {
    let mut router = Router::new();
    router
        .get("/my_site.js", |_| async { res::send(JS).header("content-type", "text/javascript") })
        .get("/my_site_bg.wasm", |_| async { res::send(WASM).header("content-type", "application/wasm") });
    router
}
```

Móntalas con `app.mount("/pkg", bundle())`. Con un hash del contenido en las URL (consulta [Documentos, Caché a largo plazo](../documents/#cache-a-largo-plazo)), también puedes añadir una cabecera `Cache-Control: public, max-age=31536000, immutable`.

## Pruebas

El cliente de pruebas de Vitesse llama a la aplicación sin red, lo que hace que las pruebas de páginas sean rápidas:

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

Esto requiere `tokio = { version = "1", features = ["macros", "rt"] }` en `[dev-dependencies]`. Las pruebas de este sitio, en `site/tests/server.rs`, comprueban así sus etiquetas SEO, sus redirecciones y su caché.

## Otros servidores

Nada en `active` es específico de Vitesse: `Document::render()` devuelve un `String`. En Actix Web, Rocket, Warp, Hyper o un servidor escrito a mano, envía esa cadena con el tipo de contenido `text/html; charset=utf-8`. `examples/hello` lo hace solo con la biblioteca estándar.
