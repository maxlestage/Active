[![Active: front-end, SEO de verdad, todo en Rust. Una isla contador escrita en Rust](https://raw.githubusercontent.com/maxlestage/Active/master/site/static/og.png)](https://maxlestage.github.io/Active/)

# Active

[English](README.md) · [Français](README.fr.md) · **Español**

**Una biblioteca front-end simple y rápida para Rust: componentes, estado, enrutamiento y etiquetas SEO completas, renderizados en el servidor y ejecutados en el navegador como WebAssembly.**

```rust
use active::prelude::*;

fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

fn page() -> String {
    let body = div()
        .child(Seo::new().title("Home").description("My site, in Rust."))
        .child(h1().text("Hello"))
        .child(island("counter", "0", counter)); // the only part sent to the browser
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}
```

El servidor envía HTML terminado con todas las etiquetas SEO. En el navegador, la misma función `counter`, compilada a WebAssembly, toma el control del botón que ya está en pantalla. Sin JavaScript que escribir, sin DOM virtual, sin un segundo renderizado del contenido estático.

Los mismos componentes también sirven para **aplicaciones del lado del cliente**: `active::mount_to("#app", app())` renderiza toda la aplicación en el navegador, `active::location()` y `active::navigate()` enrutan sin recargar, y el `Seo` de cada página actualiza el título y las metaetiquetas ([`examples/spa`](examples/spa)). O escríbelos en archivos para un **sitio estático**.

📚 **Documentación en inglés, francés y español**: el sitio web de [`site/`](site), hecho a su vez con `active`. Ejecútalo con `cargo run -p active-site` o despliega tu propia copia en Heroku desde un teléfono (más abajo). Las páginas también se pueden leer en GitHub, en [`site/docs/`](site/docs).

## Por qué

- **Simple.** Los componentes son funciones de Rust que devuelven una vista: las ideas de los hooks de React, sin el JavaScript. El estado es `use_state`, un handle `Copy` que mueves dentro de los closures. No hay macros que aprender ni plugin de compilación.
- **SEO primero.** El servidor siempre renderiza la página completa. `Seo` cubre título, descripción, robots, canonical, hreflang, Open Graph, Twitter y JSON-LD; pon uno en un layout, otro en una página y otro en un componente, y se fusionan sin ninguna etiqueta duplicada.
- **Rápido.** El renderizado en el servidor escribe directamente en un `String`. En el navegador solo se hidratan las islas, con vínculos de grano fino: un cambio de estado actualiza exactamente el texto, el atributo o la clase que depende de él. Consulta los [benchmarks](#benchmarks).
- **Ligero.** La compilación para el servidor no tiene dependencias. El bundle del navegador usa `wasm-bindgen` y `web-sys`, nada más.
- **Instalable.** `active::pwa` escribe el manifiesto de la app web y un service worker sin conexión, y `Document::manifest` / `Document::service_worker` añaden sus etiquetas: tu sitio se instala en la pantalla de inicio y funciona sin red, sin escribir JavaScript. El sitio web del proyecto es una PWA ([guía](site/docs/es/pwa.md)).

## Benchmarks

Medido frente a los principales frameworks de Rust con las mismas páginas, todo en Rust ([método y datos en bruto](bench/RESULTS.md)):

| | Renderizado en el servidor de una página de producto | WebAssembly + código de enlace para un contador (gzip) | Interactivo con 100 ms de latencia |
| --- | ---: | ---: | ---: |
| **active** | **5.6 µs** | **36.8 kB** | **224 ms** |
| Leptos 0.8 + leptos_meta | 13.4 µs | 60.9 kB | 227 ms |
| Leptos 0.8, `<head>` escrito a mano | 5.2 µs | — | — |
| Sycamore 0.9 | 33.0 µs | 45.8 kB | 227 ms |
| Yew 0.23 | 17.2 µs | 96.3 kB | 228 ms |
| Dioxus 0.7 | 24.2 µs | 170.4 kB | 339 ms |
| maud 0.27 (solo plantillas) | 1.4 µs | — | — |

El bundle más pequeño de todos, y un renderizado en el servidor 2.4 veces más rápido que el del otro framework que fusiona las etiquetas SEO de los componentes (Leptos con leptos_meta). Intel Xeon a 2.8 GHz, Rust 1.97, Chromium 141; ejecútalo tú mismo con `cd bench && cargo run --release -p runner`.

## Instalación

```sh
cargo add active --git https://github.com/maxlestage/Active                      # server
cargo add active --git https://github.com/maxlestage/Active --features client    # browser bundle, for wasm32-unknown-unknown
```

Dos aplicaciones completas: [`examples/hello`](examples/hello), una página de servidor con una isla (un solo crate para ambos lados), y [`examples/spa`](examples/spa), una aplicación de tareas del lado del cliente con enrutamiento. Ejecuta la primera:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello   # http://localhost:3000
```

## Repositorio

| Carpeta | Contenido |
| --- | --- |
| [`crates/active`](crates/active) | El crate `active`: vistas, estado, SEO, documentos, islas, hidratación |
| [`examples/hello`](examples/hello) | La aplicación más pequeña: una página y una isla de contador, con un servidor que solo usa std |
| [`examples/spa`](examples/spa) | Una aplicación del lado del cliente: lista de tareas, enrutamiento, formularios, `localStorage` |
| [`site`](site) | Sitio web de presentación y documentación (en/fr/es): servidor Vitesse, animaciones en WebAssembly, búsqueda y área de pruebas interactiva, instalable y sin conexión (PWA) |
| [`bench`](bench) | Benchmarks frente a Leptos, Dioxus, Yew, Sycamore, maud y HTML escrito a mano |

## Publicar el sitio en GitHub Pages (gratis)

1. En GitHub, abre **Settings → Pages** y cambia **Source** a **GitHub Actions** (una sola vez; la app de GitHub o el sitio móvil sirven).
2. Eso es todo. Tras cada CI en verde, que fusiona la rama en `master`, el workflow [`pages.yml`](.github/workflows/pages.yml) exporta el sitio a archivos estáticos con `active-site export` y lo publica en `https://maxlestage.github.io/Active/`. Para publicar enseguida: **Actions → Pages → Run workflow**.

Guía: [`site/docs/es/deploy-pages.md`](site/docs/es/deploy-pages.md).

## Desplegar el sitio en Heroku desde un teléfono

1. Abre `https://heroku.com/deploy?template=https://github.com/maxlestage/Active` en el navegador de tu teléfono, elige un nombre y toca **Deploy app**. Heroku lee [`app.json`](app.json) y construye el sitio con el buildpack [`emk/rust`](https://elements.heroku.com/buildpacks/emk/heroku-buildpack-rust) en la pila `heroku-24`: primero el servidor (ver [`RustConfig`](RustConfig)) y después el bundle de WebAssembly ([`heroku-build.sh`](heroku-build.sh)), todo a partir de las fuentes en Rust; [`Procfile`](Procfile) lo arranca. El [`Dockerfile`](Dockerfile) (ver [`heroku.yml`](heroku.yml)) sigue disponible para las aplicaciones en la pila de contenedores.
2. Para volver a desplegar con cada cambio: en la pestaña **Deploy** de la aplicación, conecta el repositorio y activa **Enable Automatic Deploys** desde `master`.

Heroku no tiene plan gratuito: un dyno Eco cuesta unos 5 USD al mes. Guía paso a paso: [`site/docs/es/deploy-heroku.md`](site/docs/es/deploy-heroku.md).

## Publicar en crates.io desde un teléfono

1. Crea un token en [crates.io/settings/tokens](https://crates.io/settings/tokens) y añádelo como secreto del repositorio con el nombre `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Cambia `version` en [`crates/active/Cargo.toml`](crates/active/Cargo.toml) (icono del lápiz en GitHub) y haz commit en `master`.

Cuando la CI pasa, el workflow [`release.yml`](.github/workflows/release.yml) crea la etiqueta de la nueva versión (`v0.1.0` para `0.1.0`), crea la GitHub Release con notas generadas a partir de los commits, ejecuta las pruebas y publica `active`. Una GitHub Release publicada a mano también funciona: la versión se toma entonces de su etiqueta. Guía: [`site/docs/es/publishing.md`](site/docs/es/publishing.md).

## Desarrollo

```sh
cargo test --workspace                     # library, site and examples
cargo run -p active --example page         # print a complete page
cargo run -p active-site                   # the website on http://localhost:3000
```

El bundle del navegador del sitio web se compila como el del ejemplo anterior (`-p active-site`, con la salida en `site/pkg`); consulta [`heroku-build.sh`](heroku-build.sh) o el [`Dockerfile`](Dockerfile). Cada rama que pasa la CI se fusiona automáticamente en `master`, salvo las ramas llamadas `wip/…` ([`ci.yml`](.github/workflows/ci.yml)); después `master` se publica en GitHub Pages ([`pages.yml`](.github/workflows/pages.yml)).

## Licencia

Active es software propietario: © 2026 Maxime Nathan Lestage, todos los derechos reservados. No es software de código abierto. El código fuente se publica en crates.io y GitHub para que pueda leerse, auditarse y compilarse, y puedes usarlo libremente como dependencia de tus propios programas, también comerciales. Modificarlo o redistribuir su código fuente requiere el permiso escrito del autor. Condiciones completas: [LICENSE](LICENSE).

## Créditos

Diseñado y desarrollado por Maxime Nathan Lestage, dentro de una misma familia de proyectos Rust: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) y [Solidify](https://maxlestage.github.io/Solidify/).
