---
title: El crate de Rust
description: El crate active es el gemelo en Rust del componente Seo. Genera las mismas etiquetas del head desde servidores Rust, generadores estáticos y WebAssembly.
---

No todas las páginas las renderiza React. `active` es un crate de Rust que produce las mismas etiquetas SEO que `<Seo>`, con las mismas opciones y las mismas reglas, para servidores web en Rust (Axum, Actix Web…), generadores de sitios estáticos, edge workers y WebAssembly.

- **Cero dependencias**, sin `unsafe`, sin macros.
- **La misma salida que React**, byte a byte, con dos pequeñas diferencias que se enumeran más abajo.
- **Escapado como en React** (`&`, `<`, `>`, `"`, `'`), con el JSON-LD protegido contra la inyección de `</script>`.
- Compila para `wasm32-unknown-unknown`. Requiere Rust 1.85 o posterior.

## Instalar

```sh
cargo add active
```

## Ejemplo rápido

```rust
use active::Seo;

let head = Seo::new()
    .title("Oslo chair")
    .description("Solid oak chair, delivered in 48 h.")
    .canonical("https://acme.com/products/oslo")
    .to_html();
```

Ahora `head` contiene, en una sola línea:

```html
<title>Oslo chair</title>
<meta name="description" content="Solid oak chair, delivered in 48 h."/>
<link rel="canonical" href="https://acme.com/products/oslo"/>
<meta property="og:title" content="Oslo chair"/>
<meta property="og:description" content="Solid oak chair, delivered in 48 h."/>
<meta property="og:type" content="website"/>
<meta property="og:url" content="https://acme.com/products/oslo"/>
<meta name="twitter:card" content="summary"/>
```

## Valores por defecto del sitio y páginas

`Seo` es un builder: cada setter lo recibe por valor y lo devuelve. Construye una sola vez los valores de todo tu sitio y después clónalos para cada página, igual que [`createSeo`](../site-defaults/) en React. Los setters de la página sobrescriben los valores por defecto.

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com") // resolves relative canonical, image and alternates
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us.")
        .canonical("/contact")
        .og_type("article")
        .alternate("en", "/contact")
        .alternate("fr", "/fr/contact")
        .alternate("x-default", "/contact")
        .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#)
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "The Acme logo")
        .to_html()
}
```

Todos los setters aceptan `impl Into<String>`, así que funcionan tanto los literales de cadena como `String` y `&String`. `noindex` y `nofollow` reciben un `bool`.

## Métodos

| Método | Etiquetas | Prop de React |
| --- | --- | --- |
| `Seo::new()` | Solo `og:type` (`website`) y `twitter:card` (`summary`) | |
| `.title(title)` | `<title>`, `og:title` (sin plantilla) | `title` |
| `.title_template(template)` | `<title>`: el primer `%s` se reemplaza por el título | `titleTemplate` |
| `.site_name(name)` | `og:site_name`; el título cuando no se define ninguno | `siteName` |
| `.description(text)` | `<meta name="description">`, `og:description` | `description` |
| `.canonical(url)` | `<link rel="canonical">`, `og:url` | `canonical` |
| `.image(url)` | `og:image`; `twitter:card` pasa a ser `summary_large_image` | `image` |
| `.og_type(type)` | `og:type`, `website` por defecto | `type` |
| `.locale(locale)` | `og:locale` | `locale` |
| `.twitter(handle)` | `twitter:site` | `twitter` |
| `.base_url(url)` | Prefijo para las URLs relativas de `canonical`, `image` y de las alternativas | `baseUrl` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` | `noindex`, `nofollow` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang>`, una por llamada, en el orden de las llamadas | `alternates` |
| `.json_ld(json)` | `<script type="application/ld+json">`, uno por llamada | `jsonLd` |
| `.meta_name(name, content)` | `<meta name content>` adicional | `meta` |
| `.meta_property(property, content)` | `<meta property content>` adicional | `meta` |

Las reglas son las del componente de React: el mismo orden de etiquetas, el mismo valor de respaldo para el título, los mismos valores de `robots`, la misma resolución de URLs. Una cadena vacía cuenta como "no definido". Llamar dos veces a `.alternate()` con el mismo idioma reemplaza la URL.

`json_ld` recibe JSON que ya serializaste, por ejemplo con serde_json:

```rust
let ld = serde_json::json!({
    "@context": "https://schema.org",
    "@type": "Product",
    "name": product.name,
});
let seo = SITE.clone().title(&product.name).json_ld(ld.to_string());
```

El JSON se inserta tal cual, salvo que cada `<` se convierte en `\u003c`: el mismo JSON, pero nunca puede cerrar la etiqueta `<script>`.

## Renderizado

Tres formas de obtener el HTML, que producen todas las mismas etiquetas sin espacios entre ellas:

```rust
// 1. A new String.
let head: String = seo.to_html();

// 2. Appended to a buffer you are already writing.
let mut page = String::from("<!doctype html><html><head><meta charset=\"utf-8\">");
seo.write_html(&mut page);
page.push_str("</head><body>…</body></html>");

// 3. Display: straight into format! or write!, with no intermediate String.
let page = format!("<!doctype html><html><head>{seo}</head><body>…</body></html>");
```

## Diferencias con React

- **`hreflang` va en minúsculas.** React escribe el atributo como `hrefLang`. Los nombres de atributos HTML no distinguen entre mayúsculas y minúsculas, así que ambos significan lo mismo.
- **El JSON-LD va al final del head.** React deja el `<script>` donde se renderiza `<Seo>`, en el `<body>`. El crate no tiene body, así que añade el script después de las demás etiquetas. Los buscadores aceptan ambas opciones.
- `json_ld` produce un script por llamada, mientras que la prop de React produce un script por valor (un objeto, o un array en un único script).

## Ejemplo con Axum

```rust
use std::sync::LazyLock;

use active::Seo;
use axum::{Router, response::Html, routing::get};

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
});

async fn page() -> Html<String> {
    let seo = SITE
        .clone()
        .title("Pricing")
        .description("Simple plans for teams of every size.")
        .canonical("/pricing");
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">{seo}</head><body><h1>Pricing</h1></body></html>"#
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/pricing", get(page));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Dependencias: `cargo add active axum` y `cargo add tokio --features full`. Actix Web, Rocket, Leptos o un motor de plantillas funcionan de la misma manera: el crate te da una cadena y tú la colocas en tu `<head>`. El repositorio también incluye un ejemplo sin framework: `cargo run --example axum_like` en `crates/active`.

## Generador de sitios estáticos

```rust
use std::fs;

fn main() -> std::io::Result<()> {
    let pages = [
        ("/", "Home", "Furniture made to last."),
        ("/about/", "About", "Who we are and how we work."),
    ];
    for (path, title, description) in pages {
        let seo = SITE.clone().title(title).description(description).canonical(path);
        let dir = format!("dist{path}");
        fs::create_dir_all(&dir)?;
        fs::write(
            format!("{dir}index.html"),
            format!("<!doctype html><html><head><meta charset=\"utf-8\">{seo}</head><body><h1>{title}</h1></body></html>"),
        )?;
    }
    Ok(())
}
```

## En el navegador con wasm-bindgen

Como no tiene dependencias, `active` se compila a WebAssembly tal cual. Envuélvelo en tu propio crate de wasm-bindgen para generar etiquetas del head en el navegador o en un edge worker. Es exactamente lo que hace el playground en vivo de la página de inicio de este sitio: cada pulsación de tecla llama al crate, compilado a WebAssembly.

```toml
[dependencies]
active = "0.1"
wasm-bindgen = "0.2"
```

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn seo_head(title: &str, description: &str, path: &str, origin: &str) -> String {
    active::Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url(origin)
        .title(title)
        .description(description)
        .canonical(path)
        .to_html()
}
```

Después cárgalo con active-react como cualquier otro módulo (consulta [cargar WebAssembly](../loading-wasm/)):

```tsx
import { useWasm, wasm } from "active-react";
import url from "./pkg/seo_wasm_bg.wasm?url";

const seoWasm = wasm(() => import("./pkg/seo_wasm.js"), { url });

export function HeadPreview({ title, description }: { title: string; description: string }) {
  const rust = useWasm(seoWasm);
  return <pre>{rust ? rust.seo_head(title, description, "/products/oak-chair", location.origin) : "…"}</pre>;
}
```

> **Nota:** En una app React no necesitas el crate para definir las etiquetas de tus propias páginas: `<Seo>` se encarga de ello. El crate es para el código Rust que produce HTML, o para mostrar etiquetas como texto, como hace el playground.

## Referencia de la API

La documentación completa de la API de Rust está en [docs.rs/active](https://docs.rs/active).
