---
title: Documentos
description: Convierte una vista en una página HTML completa con Document: idioma, etiquetas SEO fusionadas, hojas de estilo, HTML extra, cargador WebAssembly y atributos.
---

Una vista describe el contenido de una página. `Document` la envuelve en un documento HTML completo: el doctype, el elemento `<html>` con su idioma, un `<head>` con las etiquetas SEO fusionadas, tus hojas de estilo y el código que arranca el bundle de WebAssembly, y después el `<body>`. Su `render()` devuelve la página como un `String`, lista para enviarla desde un servidor o escribirla en un archivo.

## Una página completa

```rust
use active::prelude::*;

fn page() -> String {
    let body = el("main")
        .child(Seo::new().title("Contact").description("Write to us, we answer within a day."))
        .child(h1().text("Contact"));

    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
        .stylesheet("/static/app.css")
        .head(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml"/>"#)
        .client("/pkg/acme.js", "/pkg/acme_bg.wasm")
        .html_attr("class", "light")
        .body_attr("class", "page-contact")
        .render()
}

let html = page();
assert!(html.starts_with(r#"<!DOCTYPE html><html lang="en" class="light"><head><meta charset="utf-8"/>"#));
assert!(html.contains("<title>Contact · Acme</title>"));
assert!(html.ends_with(r#"<body class="page-contact"><main><h1>Contact</h1></main></body></html>"#));
```

## Qué escribe render

La salida es una sola línea de HTML. Formateada para leerla, su estructura es siempre esta:

```html
<!DOCTYPE html>
<html lang="en" class="light">                      <!-- lang, then html_attr -->
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Contact · Acme</title>                     <!-- merged Seo tags -->
  <meta name="description" content="…"/>
  …
  <style>active-island{display:contents}</style>
  <link rel="stylesheet" href="/static/app.css"/>   <!-- stylesheet, in call order -->
  <link rel="modulepreload" href="/pkg/acme.js"/>   <!-- client -->
  <link rel="preload" href="/pkg/acme_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/acme.js";init({module_or_path:"/pkg/acme_bg.wasm"});</script>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>   <!-- head, in call order -->
</head>
<body class="page-contact">                         <!-- body_attr -->
  <main><h1>Contact</h1></main>                     <!-- your view -->
</body>
</html>
```

Las etiquetas charset y viewport se escriben siempre (`.viewport(…)` cambia la segunda), así que todas las páginas están en UTF-8 y se adaptan a móviles. El pequeño estilo `active-island` evita que los contenedores de las [islas](../islands/) afecten al diseño.

## El builder

| Método | Efecto |
| --- | --- |
| `Document::new(view)` | una página cuyo `<body>` contiene `view` (cualquier cosa que se convierta en un `Node`) |
| `.lang(code)` | `<html lang="…">`, p. ej. `"en"`, `"fr"` |
| `.viewport(content)` | sustituye el `content` de la etiqueta viewport (por defecto `width=device-width, initial-scale=1`), por ejemplo para añadir `viewport-fit=cover` |
| `.seo(seo)` | el `Seo` base, fusionado con cada `Seo` que se encuentre en la vista; llamarlo de nuevo lo reemplaza |
| `.stylesheet(href)` | un `<link rel="stylesheet">`; llámalo una vez por archivo |
| `.head(html)` | HTML de confianza añadido al `<head>`: iconos, fuentes, analítica, `theme-color` |
| `.client(js, wasm)` | carga el bundle de WebAssembly (ver más abajo) |
| `.manifest(href, &manifest)` | hace la página instalable: enlace al manifiesto, `theme-color`, título e icono de la pantalla de inicio (ver [PWA](../pwa/)) |
| `.service_worker(url)` | registra el service worker sin conexión servido en `url` (ver [PWA](../pwa/)) |
| `.html_attr(name, value)` | un atributo en `<html>` |
| `.body_attr(name, value)` | un atributo en `<body>` |
| `.render()` | la página completa como `String` |
| `.final_seo()` | el `Seo` fusionado de la página, sin renderizarla (útil en pruebas) |

`Document` implementa `Clone` y `Default`.

> **Advertencia:** `.head(...)` inserta el HTML tal cual. Pasa solo marcado que hayas escrito tú mismo.

## SEO

`Document` recoge cada `Seo` de la vista, incluidos los que están dentro de islas y en el contenido actual de las regiones dinámicas, y los fusiona sobre la base pasada a `.seo(...)`. El resultado se escribe una sola vez, así que la página siempre tiene exactamente un `<title>`. Las reglas se describen en [SEO, Fusión](../seo/#fusion).

`final_seo()` devuelve ese valor fusionado, lo que facilita probar el SEO:

```rust
use active::prelude::*;

let doc = Document::new(div().child(Seo::new().title("Pricing")))
    .seo(Seo::new().title_template("%s · Acme"));
assert!(doc.final_seo().to_html().starts_with("<title>Pricing · Acme</title>"));
```

## Cargar el bundle del navegador

`wasm-bindgen --target web --no-typescript` produce dos archivos: un pequeño módulo JavaScript (el cargador) y el binario `.wasm`. `.client(js, wasm)` recibe sus URL y escribe tres cosas en el `<head>`:

1. `<link rel="modulepreload">` para el cargador;
2. `<link rel="preload" as="fetch" type="application/wasm">` para el binario, de modo que se descargue **en paralelo** con el HTML y el cargador, y no después;
3. una línea `<script type="module">` que importa el cargador y arranca el módulo con la URL del binario.

Cuando el módulo arranca, se ejecuta su función `#[wasm_bindgen(start)]`: [hidrata las islas](../islands/) o [monta una aplicación](../client-app/). Esta línea es el único JavaScript de la página, aparte del cargador generado por `wasm-bindgen`.

Las páginas sin ninguna isla no necesitan el bundle: omite `.client(...)` y no enviarán nada de código.

Las URL se escriben de forma segura dentro de una cadena JavaScript (las comillas, las barras invertidas y `<` se escapan), así que pueden contener parámetros de consulta como `?v=…`.

### Caché a largo plazo

Los navegadores pueden guardar el bundle y las hojas de estilo para siempre si su URL cambia cada vez que cambia su contenido. Una forma habitual es un parámetro de versión calculado a partir del contenido del archivo, como hace este sitio en [`site/src/server/assets.rs`](https://github.com/maxlestage/Active/blob/master/site/src/server/assets.rs):

```rust
/// Un hash corto del contenido (FNV-1a), suficiente para distinguir versiones de un archivo.
fn version(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")[..10].to_string()
}

fn versioned(url: &str, bytes: &[u8]) -> String {
    format!("{url}?v={}", version(bytes))
}

let url = versioned("/pkg/acme_bg.wasm", b"\0asm");
assert!(url.starts_with("/pkg/acme_bg.wasm?v="));
```

Lee los archivos una vez al arrancar, calcula sus URL versionadas y pásalas a `.stylesheet(...)` y `.client(...)`. Sirve las URL que llevan `?v=` con `Cache-Control: public, max-age=31536000, immutable`, y el propio HTML con `no-cache`.

## Renderizado

`render()` renderiza primero el body, recogiendo sus valores `Seo` por el camino, y después escribe la página a su alrededor: cada región dinámica se evalúa una vez por renderizado. Un documento se puede renderizar tantas veces como haga falta; en el servidor, los estados que usa su vista se liberan automáticamente en cuanto los renderizados posteriores ya no los necesitan (consulta [Estado, En el servidor](../state/#en-el-servidor)). `final_seo()`, cuando lo llamas tú, evalúa las regiones dinámicas por separado.

Para enviarlo, devuélvelo con el tipo de contenido `text/html; charset=utf-8`. Con Axum, envolverlo en `axum::response::Html` lo hace por ti (consulta [Axum](../axum/)). Para un sitio estático, escríbelo en un archivo `index.html` (consulta [Sitios estáticos](../static-site/)).

## Sin Document

Para tener control total sobre el marcado de la página (una plantilla existente, otro framework), renderiza las partes tú mismo: `Node::render()` da el HTML del body y `Seo` da las etiquetas del head con `to_html()`, `write_html()` o `format!("{seo}")`. Pierdes entonces la fusión automática de los valores `Seo` encontrados en la vista y el cargador del bundle, que tendrías que escribir a mano.
