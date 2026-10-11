---
title: Inicio rápido
description: Ejecuta en cinco minutos una página renderizada en el servidor con una isla de contador en WebAssembly: primero con el ejemplo incluido, luego en tu proyecto.
---

Esta página pone una página real en tu pantalla: HTML renderizado por un servidor Rust, con etiquetas SEO completas, y un contador interactivo compilado a WebAssembly. Supone que ya tienes instaladas las herramientas de [Instalación](../installation/) (Rust, el target `wasm32-unknown-unknown` y `wasm-bindgen-cli`).

## Ejecutar el ejemplo

El repositorio contiene una aplicación mínima, `examples/hello`. Clónalo, compila el bundle del navegador e inicia el servidor:

```sh
git clone https://maxlestage.github.io/Active/
cd Active
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello
```

Abre [http://localhost:3000](http://localhost:3000) y haz clic en los botones. El servidor del ejemplo solo usa la biblioteca estándar, así que compila rápido.

## Crear tu propio proyecto

La misma aplicación, en un crate nuevo, con un servidor [Axum](../axum/).

### 1. Crear el crate

```sh
cargo new my-site
cd my-site
```

### 2. Declarar las dependencias

Reemplaza `Cargo.toml` por:

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
tower-http = { version = "0.7", features = ["fs"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

> **Nota:** Mientras el crate no esté publicado en crates.io, reemplaza los dos valores `active = …` por `{ git = "https://maxlestage.github.io/Active/" }` (añadiendo `features = ["client"]` al segundo).

### 3. Escribir la página

Crea `src/lib.rs`. Lo comparten el servidor y el navegador:

```rust
use active::prelude::*;

/// Una isla: renderizada en el servidor, interactiva en el navegador.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    div()
        .class("counter")
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
        .into()
}

/// La página completa. Todo, excepto el contador, es HTML simple.
pub fn page() -> String {
    let body = main()
        .child(Seo::new().title("Home").description("My first page with active."))
        .child(h1().text("Hello from Rust"))
        .child(p().text("This paragraph is plain HTML. The counter below is an island:"))
        .child(island("counter", "0", counter));
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("My site").title_template("%s · My site"))
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}

/// Punto de entrada del navegador: se ejecuta una vez cargado el módulo WebAssembly.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

### 4. Escribir el servidor

Reemplaza `src/main.rs`:

```rust
use axum::{Router, response::Html, routing::get};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { Html(my_site::page()) }))
        .nest_service("/pkg", ServeDir::new("pkg"));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
```

### 5. Compilar el bundle del navegador

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

Esto escribe `pkg/my_site.js` (el cargador) y `pkg/my_site_bg.wasm` (tu código). Vuelve a ejecutar estos dos comandos cada vez que cambies código que se ejecuta en el navegador.

### 6. Ejecutarlo

```sh
cargo run
```

Abre [http://localhost:3000](http://localhost:3000). El contador funciona, y el código fuente de la página contiene todo su texto.

## Qué acaba de pasar

Abre el código fuente de la página en tu navegador (normalmente `Ctrl+U` o `⌘+⌥+U`). Acortado e indentado para leerlo mejor, se ve así:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Home · My site</title>
  <meta name="description" content="My first page with active."/>
  <meta property="og:title" content="Home"/>
  <meta property="og:description" content="My first page with active."/>
  <meta property="og:type" content="website"/>
  <meta property="og:site_name" content="My site"/>
  <meta name="twitter:card" content="summary"/>
  <style>active-island{display:contents}</style>
  <link rel="modulepreload" href="/pkg/my_site.js"/>
  <link rel="preload" href="/pkg/my_site_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/my_site.js";init({module_or_path:"/pkg/my_site_bg.wasm"});</script>
</head>
<body>
  <main>
    <h1>Hello from Rust</h1>
    <p>This paragraph is plain HTML. The counter below is an island:</p>
    <active-island data-island="counter" data-props="0">
      <div class="counter"><button>−</button><span><!--#-->0</span><button>+</button></div>
    </active-island>
  </main>
</body>
</html>
```

Paso a paso:

1. **El servidor lo renderiza todo.** `Document::render()` escribe el doctype, el `<head>` y el body en un único `String`. El `Seo` colocado en el body y el que se pasa a `Document::seo` se fusionan en un solo conjunto de etiquetas (consulta [SEO](../seo/#fusion)).
2. **La isla queda marcada.** `island("counter", "0", counter)` renderizó el HTML del contador dentro de un elemento `<active-island>` que registra su nombre y sus props. El comentario `<!--#-->` marca el texto dinámico para que el navegador pueda encontrarlo.
3. **El bundle se carga en paralelo.** Los dos enlaces `preload` empiezan a descargar el cargador y el archivo `.wasm` mientras se analiza la página. El script de módulo de una línea arranca el módulo WebAssembly.
4. **La isla despierta.** `start()` llama a `active::hydrate`, que encuentra el elemento `<active-island data-island="counter">`, vuelve a ejecutar `counter("0")` en el navegador y conecta los listeners de clic y el texto dinámico al HTML que ya está ahí. No se vuelve a crear nada.

El encabezado y el párrafo nunca ejecutan código en el navegador.

## Próximos pasos

- Construye vistas con [componentes](../components/) y luego añade [estado](../state/) y [eventos](../events/).
- Describe cada página para los buscadores con [SEO](../seo/).
- Descubre cómo se hidratan las [islas](../islands/), o entrega los mismos componentes como un [sitio estático](../static-site/).
- Pon el sitio en línea con [Desplegar en Heroku](../deploy-heroku/).
