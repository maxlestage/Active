---
title: Instalación
description: Instala Rust, añade el crate active, prepara un solo crate para el servidor y el navegador, e instala el target WebAssembly y la CLI de wasm-bindgen.
---

Esta página prepara todo lo que necesitas: la toolchain de Rust, el crate `active`, una estructura de proyecto que compila tanto para el servidor como para el navegador, y las dos herramientas que generan el bundle de WebAssembly.

## Requisitos

| Herramienta | Versión | Para qué |
| --- | --- | --- |
| Rust | 1.85 o posterior | `active` usa la edición 2024 |
| Target `wasm32-unknown-unknown` | cualquiera | compila el bundle del navegador |
| `wasm-bindgen-cli` | la misma versión que el crate `wasm-bindgen` de `Cargo.lock` | genera el cargador y el `.wasm` final |

Si solo renderizas HTML en un servidor o generas un sitio estático sin islas, basta con Rust.

## Instalar Rust

Instala [rustup](https://rustup.rs), que gestiona las versiones de Rust, y comprueba la versión:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

En Windows, descarga en su lugar `rustup-init.exe` desde [rustup.rs](https://rustup.rs). Más adelante, actualiza con `rustup update`.

## Añadir el crate

```sh
cargo add active
```

> **Nota:** Hasta que la primera publicación esté en crates.io, usa como dependencia el repositorio Git: `active = { git = "https://maxlestage.github.io/Active/" }` en `Cargo.toml`. Cargo encuentra el paquete `active` dentro del repositorio.

### Features de Cargo

| Feature | Por defecto | Qué añade |
| --- | --- | --- |
| `client` | desactivada | Runtime del navegador: `active::hydrate`, `active::mount`, listeners de eventos, efectos. Incorpora `wasm-bindgen` y `web-sys`. |

Sin `client`, `active` no tiene ninguna dependencia. Los manejadores de eventos y los efectos se aceptan (así los mismos componentes compilan en todas partes), pero nunca se ejecutan.

## Un crate, dos destinos

La estructura recomendada es un único crate cuya biblioteca contiene los componentes. La compilación nativa es tu servidor; la compilación a WebAssembly de esa misma biblioteca es el bundle del navegador. Así están organizados `examples/hello` y este sitio web.

```text
my-site/
├── Cargo.toml
└── src/
    ├── lib.rs    componentes, islas y el punto de entrada del navegador
    └── main.rs   el servidor (solo nativo)
```

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
# cdylib: el .wasm para el navegador. rlib: la biblioteca que usa src/main.rs.
crate-type = ["cdylib", "rlib"]

[dependencies]
active = "0.1"

# Dependencias solo del servidor: no compilan para wasm32.
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
axum = "0.8"
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread"] }

# Solo para el navegador: el runtime del cliente y el generador de bindings.
[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"
```

Cargo combina las dos líneas de `active`: en `wasm32` el crate recibe la feature `client`, y en el servidor no. Por eso tu código puede llamar a `active::hydrate` detrás de `#[cfg(target_arch = "wasm32")]`:

```rust
// src/lib.rs
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

/// Se ejecuta en el navegador cuando arranca el módulo WebAssembly.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

> **Advertencia:** Mantén los crates de servidor como `tokio` y `axum` en la tabla `cfg(not(target_arch = "wasm32"))`. Cargo compila todas las dependencias del paquete para el target que estás compilando, incluso las que solo usa `main.rs`, y los crates de red no compilan para `wasm32-unknown-unknown`.

## Instalar las herramientas de WebAssembly

Añade el target de compilación y la herramienta de línea de comandos `wasm-bindgen`:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

La versión de la CLI debe ser exactamente la del crate `wasm-bindgen` que usa tu proyecto; si no, `wasm-bindgen` se detiene con un error de versión de esquema. Fijar el crate con `=0.2.129` (como arriba) mantiene ambas versiones sincronizadas. Para ver qué versión tiene tu archivo de bloqueo:

```sh
cargo tree -i wasm-bindgen --target wasm32-unknown-unknown
```

## Añadir los perfiles de compilación

Los perfiles van en el `Cargo.toml` raíz de tu proyecto (la raíz del workspace, si tienes uno). Estos son los ajustes que usa este repositorio: una compilación rápida para el servidor y una compilación pequeña para el navegador.

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

# Bundle del navegador: cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

[Rendimiento](../performance/) explica cada ajuste y muestra los tamaños que producen.

## Comprobar la instalación

Renderiza una vista en una prueba. Si pasa, el crate está bien instalado:

```rust
#[test]
fn renders() {
    use active::prelude::*;
    let html = Node::from(p().class("hello").text("Hello")).render();
    assert_eq!(html, r#"<p class="hello">Hello</p>"#);
}
```

Después compila una vez el bundle del navegador para comprobar las herramientas de WebAssembly:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

Cargo reemplaza `-` por `_` en los nombres de archivo, así que el paquete `my-site` produce `my_site.wasm`. El [Inicio rápido](../quick-start/) continúa desde aquí, y [Compilar el cliente](../client-build/) explica la compilación en detalle.

## Configurar el editor

Sirve cualquier editor con [rust-analyzer](https://rust-analyzer.github.io). Por defecto analiza la compilación nativa, así que el código detrás de `#[cfg(target_arch = "wasm32")]` aparece en gris. Para trabajar en código que solo se ejecuta en el navegador, apunta rust-analyzer al target de WebAssembly, por ejemplo en el `settings.json` de VS Code:

```text
"rust-analyzer.cargo.target": "wasm32-unknown-unknown"
```

Vuelve a cambiarlo cuando trabajes en el servidor.
