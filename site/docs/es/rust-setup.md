---
title: Configuración de Rust
description: Instala Rust, el target wasm32 y wasm-pack o wasm-bindgen-cli, configura Cargo para binarios pequeños y rápidos y compila tu primer módulo.
---

Esta página prepara todo lo que necesitas para compilar Rust a WebAssembly para active-react. Solo la necesitas para la parte WASM: las funcionalidades SEO funcionan sin Rust.

## Instalar Rust

Rust se instala con **rustup**, el gestor oficial de la toolchain. En macOS y Linux:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

En Windows, descarga y ejecuta `rustup-init.exe` desde [rustup.rs](https://rustup.rs). Después abre una terminal nueva y comprueba:

```sh
rustc --version
cargo --version
```

Mantén Rust actualizado con `rustup update`.

## Añadir el target de WebAssembly

Por defecto, Rust compila para tu máquina. Añade una sola vez el target del navegador:

```sh
rustup target add wasm32-unknown-unknown
```

## Elegir una herramienta de build

Rust produce un archivo `.wasm`; después, **wasm-bindgen** genera el código de enlace (glue) JavaScript y los tipos TypeScript que te permiten pasar cadenas, arrays y más. Puedes usarlo de dos maneras.

### wasm-pack (lo más sencillo)

```sh
cargo install wasm-pack
```

wasm-pack ejecuta Cargo, descarga automáticamente la versión de wasm-bindgen adecuada para tu crate y optimiza el resultado. Un solo comando lo hace todo.

### wasm-bindgen-cli (más control)

```sh
cargo install wasm-bindgen-cli --version 0.2.129
```

La versión del CLI **debe coincidir exactamente** con la versión del crate `wasm-bindgen` de tu `Cargo.lock`. Fija el crate con `=` para que ambas no puedan desincronizarse nunca:

```toml
[dependencies]
wasm-bindgen = "=0.2.129"
```

Si difieren, el build se detiene con un error como este:

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary:

  rust Wasm file schema version: 0.2.129
     this binary schema version: 0.2.100
```

Corrígelo instalando el CLI correspondiente (`cargo install -f wasm-bindgen-cli --version 0.2.129`) o cambiando la versión del crate. Esta es la configuración que usan el ejemplo y los benchmarks del repositorio.

## Cargo.toml

Una biblioteca WebAssembly necesita `crate-type = ["cdylib"]`. Este es el archivo completo, con el perfil release validado por los [benchmarks](../benchmarks/):

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Qué hace cada ajuste del perfil:

| Ajuste | Efecto |
| --- | --- |
| `opt-level = 3` | Optimiza para velocidad. `"s"` y `"z"` optimizan para tamaño; los benchmarks los encontraron solo un poco más pequeños y algo más lentos |
| `lto = true` | Optimización en tiempo de enlazado en todos los crates: elimina el código no usado de las dependencias, resultado más pequeño y más rápido |
| `codegen-units = 1` | Compila el crate como una sola unidad para que el optimizador lo vea todo. Builds más lentos, mejor código |
| `panic = "abort"` | Sin la maquinaria de desenrollado de pila en el binario. Un panic provoca un trap y lanza un `RuntimeError` en JavaScript |
| `strip = true` | Elimina del binario los nombres de símbolos y la información de depuración |

Estos ajustes solo afectan a los builds `--release`, así que los builds de depuración siguen compilando rápido.

## Escribir funciones en Rust

Marca con `#[wasm_bindgen]` cada función que quieras llamar desde JavaScript:

```rust
use wasm_bindgen::prelude::*;

/// Numbers: i32, u32, f64… become JavaScript numbers.
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Strings: &str in, String out.
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello {name}, from Rust!")
}

/// Typed arrays: &[f64] receives a Float64Array.
#[wasm_bindgen]
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Vec<f64> comes back as a new Float64Array.
#[wasm_bindgen]
pub fn normalize(values: &[f64]) -> Vec<f64> {
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    if max <= 0.0 {
        return values.to_vec();
    }
    values.iter().map(|v| v / max).collect()
}
```

Cómo cruzan la frontera los tipos más comunes:

| Rust | TypeScript |
| --- | --- |
| `i32`, `u32`, `f32`, `f64` | `number` |
| `i64`, `u64` | `bigint` |
| `bool` | `boolean` |
| `&str`, `String` | `string` |
| `&[f64]`, `Vec<f64>` | `Float64Array` |
| `&[u8]`, `Vec<u8>` | `Uint8Array` |
| `Option<T>` | `T \| undefined` (los argumentos también aceptan `null`) |

Las cadenas y los arrays se copian entre la memoria de JavaScript y la de WebAssembly, lo que lleva tiempo: consulta la [guía de rendimiento](../performance/).

## Build

Con wasm-pack, desde la raíz de tu proyecto:

```sh
wasm-pack build hello --target web --out-dir ../src/pkg
```

`--out-dir` es relativo a la carpeta del crate. Con wasm-bindgen-cli, hacen falta dos comandos:

```sh
cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm
```

Usa `--target web` (recomendado): active-react lo inicializa por ti y puede precargar el binario. `--target bundler` también funciona si tu bundler admite la integración ESM de WebAssembly.

## Archivos generados

| Archivo | Contenido |
| --- | --- |
| `hello.js` | Glue JavaScript: tus funciones, más una exportación por defecto `init` |
| `hello_bg.wasm` | El Rust compilado |
| `hello.d.ts` | Los tipos TypeScript de tus funciones |
| `hello_bg.wasm.d.ts` | Los tipos de las exportaciones de bajo nivel (rara vez los necesitas) |
| `package.json`, `.gitignore` | Solo con wasm-pack |

Los tipos generados son precisos. Para las funciones anteriores:

```ts
export function add(a: number, b: number): number;
export function greet(name: string): string;
export function mean(values: Float64Array): number;
export function normalize(values: Float64Array): Float64Array;
```

> **Nota:** Los nombres de archivo usan el nombre del crate con los guiones convertidos en guiones bajos: un crate llamado `my-crate` produce `my_crate.js` y `my_crate_bg.wasm`.

## Estructura del proyecto

Una estructura que funciona bien con Vite:

```text
my-app/
├── hello/                 Rust crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── src/
│   ├── pkg/               generated, not committed
│   ├── App.tsx
│   ├── seo.ts             createSeo defaults
│   └── wasm.ts            wasm() declarations
├── index.html
└── package.json
```

Añade `hello/target` y `src/pkg` a `.gitignore` (wasm-pack ya escribe un `.gitignore` dentro de `src/pkg`).

## Scripts de npm

Encapsula el build en un script para que nadie tenga que recordar los flags:

```json
{
  "scripts": {
    "wasm": "wasm-pack build hello --target web --out-dir ../src/pkg",
    "dev": "npm run wasm && vite",
    "build": "npm run wasm && tsc -b && vite build"
  }
}
```

El mismo script con wasm-bindgen-cli, como en el [ejemplo vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg):

```json
{
  "scripts": {
    "wasm": "cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm"
  }
}
```

Después de cambiar el código Rust, vuelve a ejecutar `npm run wasm`: Vite detecta los archivos nuevos y recarga la página.

Siguiente: [carga el módulo en React](../loading-wasm/).
