---
title: Compilar el cliente
description: Compila tus componentes a WebAssembly con cargo y wasm-bindgen, sirve el bundle, cárgalo desde la página y automatiza la compilación en CI y Docker.
---

La parte de navegador de un sitio `active` es tu crate de biblioteca compilado a WebAssembly. Dos comandos lo generan: `cargo` compila el crate para el target `wasm32-unknown-unknown` y después `wasm-bindgen` produce el archivo `.wasm` final y un pequeño cargador JavaScript. Esta página cubre ambos comandos, cómo servir el resultado y cómo automatizar la compilación.

## Qué se genera

```text
src/lib.rs ──cargo build --target wasm32-unknown-unknown──▶ target/…/my_site.wasm
                                                                 │
                       wasm-bindgen --target web --no-typescript ▼
                                       pkg/my_site.js         el cargador (un módulo JavaScript)
                                       pkg/my_site_bg.wasm    tu código, servido a los navegadores
```

El cargador es código generado: nunca lo editas. La página lo arranca con la línea que escribe [`Document::client`](../documents/#cargar-el-bundle-del-navegador).

## Requisitos

- El target: `rustup target add wasm32-unknown-unknown`.
- La CLI, **exactamente** en la versión del crate `wasm-bindgen` de tu `Cargo.lock`: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
- Un crate preparado para dos destinos, como en [Instalación](../installation/#un-crate-dos-destinos): `crate-type = ["cdylib", "rlib"]`, `active` con la feature `client` y `wasm-bindgen` en `[target.'cfg(target_arch = "wasm32")'.dependencies]`, y una función `#[wasm_bindgen(start)]`.

## Comandos de compilación

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

- `--lib` compila solo la biblioteca. Tu servidor (`src/main.rs`) no se compila para el navegador.
- `--profile wasm-release` usa el perfil optimizado para tamaño de [Instalación](../installation/#anadir-los-perfiles-de-compilacion). Sin él, usa `--release` y la ruta `…/release/my_site.wasm`.
- El archivo `.wasm` lleva el nombre del paquete, con `-` reemplazado por `_`: el paquete `my-site` da `my_site.wasm`.
- `--target web` produce un cargador en forma de módulo ES estándar, que es lo que espera `Document::client`.
- `--no-typescript` evita generar las declaraciones de TypeScript (`.d.ts`), que un proyecto Rust no necesita.

En un [workspace de Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html), selecciona el paquete con `-p` y usa el directorio `target/` del workspace, como hace este repositorio con su ejemplo:

```sh
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
```

### Compilaciones más rápidas durante el desarrollo

Una compilación de depuración es más rápida y da mejores mensajes de pánico, a costa de un archivo mucho más grande:

```sh
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/debug/my_site.wasm
```

Un pequeño script te ahorra escribir. Guárdalo como `build.sh` en la raíz del proyecto, hazlo ejecutable con `chmod +x build.sh` y ejecuta `./build.sh && cargo run` después de cada cambio:

```sh
#!/bin/sh
set -e
PROFILE=${PROFILE:-wasm-release}
DIR=$PROFILE
[ "$PROFILE" = dev ] && DIR=debug # the dev profile writes to target/…/debug
cargo build --lib --target wasm32-unknown-unknown --profile "$PROFILE"
wasm-bindgen --target web --no-typescript --out-dir pkg \
  "target/wasm32-unknown-unknown/$DIR/my_site.wasm"
```

`PROFILE=dev ./build.sh` hace una compilación de depuración.

Añade `pkg` a tu `.gitignore`: es un resultado de la compilación.

## Servir el bundle

Sirve la carpeta `pkg/` en una URL, normalmente `/pkg/`, con estos tipos de contenido:

| Archivo | Tipo de contenido |
| --- | --- |
| `.js` | `text/javascript` |
| `.wasm` | `application/wasm` |

Con el tipo correcto, el navegador compila el módulo mientras lo descarga. Con uno incorrecto, el cargador muestra una advertencia en la consola y recurre a un método más lento. El `static_dir` de [Vitesse](../vitesse/) y la mayoría de los hostings estáticos definen ambos tipos automáticamente.

Comprime ambos archivos: WebAssembly se comprime bien con Brotli o gzip (consulta [Rendimiento](../performance/#tamanos)).

## Cargarlo desde la página

Pasa las dos URL al documento:

```rust
use active::prelude::*;

fn page(body: Element) -> String {
    Document::new(body)
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}
```

La página precarga ambos archivos y arranca el módulo. Después se ejecuta tu función `#[wasm_bindgen(start)]`, normalmente `active::hydrate(&ISLANDS)` para las [islas](../islands/) o `active::mount_to("#app", app())` para una [aplicación del lado del cliente](../client-app/).

## En integración continua

Este repositorio compila su sitio en cada push con GitHub Actions (`.github/workflows/ci.yml`). Los pasos relevantes:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown
- uses: Swatinem/rust-cache@v2
- name: Install wasm-bindgen-cli
  run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
- name: Build the site
  run: |
    cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
    wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
    cargo build -p active-site --bin active-site --release
```

También pasa el linter al código del navegador, que la compilación normal no comprueba:

```sh
cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings
```

Para Docker, el `Dockerfile` ejecuta los mismos comandos en una etapa de compilación; consulta [Desplegar en Heroku](../deploy-heroku/#como-se-despliega-el-repositorio).

## Solución de problemas

### wasm-bindgen informa de una discrepancia de esquema o de versión

La CLI y el crate `wasm-bindgen` no coinciden. Averigua la versión del crate con `cargo tree -i wasm-bindgen --target wasm32-unknown-unknown` e instala la misma versión de la CLI con `cargo install wasm-bindgen-cli --version <version> --locked`. Fijar el crate con `wasm-bindgen = "=0.2.129"` evita sorpresas tras un `cargo update`.

### Errores sobre tokio, mio o sockets al compilar para wasm32

Se está compilando una dependencia del servidor para el navegador. Muévela a `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` y asegúrate de pasar `--lib`.

### No se encuentra `hydrate`, `mount_to` o `location`

Solo existen con la feature `client`. Comprueba que `active = { …, features = ["client"] }` esté en las dependencias de `wasm32`, y que el código que las llama esté detrás de `#[cfg(target_arch = "wasm32")]`.

### `can't find crate for core` o `std`

El target no está instalado: `rustup target add wasm32-unknown-unknown`. Si tienes varias toolchains, instálalo para la que usa el proyecto.

### La página carga pero nada reacciona

- Abre la consola de desarrollador del navegador y busca errores o advertencias.
- Comprueba que `/pkg/my_site.js` y `/pkg/my_site_bg.wasm` se cargan (pestaña Red) y que sus URL coinciden con las que se pasan a `Document::client`.
- Comprueba que la función `start` tiene `#[wasm_bindgen(start)]` y llama a `hydrate` con los nombres de isla correctos.
- Después de cambiar el código del navegador, vuelve a compilar el bundle: `cargo run` por sí solo recompila únicamente el servidor.
- Una advertencia `active: re-rendering island (… mismatch)` significa que el HTML del servidor y la vista del navegador difieren; consulta [Discrepancias de hidratación](../islands/#discrepancias-de-hidratacion).
