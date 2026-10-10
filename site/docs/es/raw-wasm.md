---
title: WebAssembly puro
description: Distribuye pequeñas funciones numéricas de Rust como un simple archivo .wasm con exportaciones extern C, sin glue JavaScript, y cárgalas con wasm(url).
---

wasm-bindgen es práctico, pero añade glue JavaScript y helpers de memoria a cada módulo. Para cálculo numérico puro puedes prescindir de él: exporta funciones simples desde Rust y carga directamente el archivo `.wasm`. En los benchmarks, un módulo con dos funciones pesaba **144 bytes**, frente a los 15,87 kB de la versión con wasm-bindgen (que además manejaba una cadena).

## Cuándo usarlo

Los módulos puros encajan muy bien cuando todas las funciones reciben y devuelven solo números: matemáticas, física, checksums, interpolación, conversión de colores, puntuaciones.

En cuanto necesites cadenas, arrays u objetos, usa [wasm-bindgen](../rust-setup/) en su lugar (consulta las [limitaciones](#limitaciones)).

## Escribir el código Rust

Crea un crate de biblioteca sin dependencias:

```toml
[package]
name = "math"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Exporta las funciones con `#[unsafe(no_mangle)]` y `extern "C"`:

```rust
// src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn fib(n: u32) -> u32 {
    if n < 2 { n } else { fib(n - 1).wrapping_add(fib(n - 2)) }
}
```

- `#[unsafe(no_mangle)]` conserva el nombre de la función tal cual, para que JavaScript pueda encontrarla. Es la sintaxis de la edición 2024 de Rust; las ediciones anteriores escriben `#[no_mangle]`.
- `extern "C"` le da a la función una convención de llamada estable que WebAssembly entiende.

## Build

```sh
cargo build --release --target wasm32-unknown-unknown
```

El binario está en `target/wasm32-unknown-unknown/release/math.wasm`. No hace falta ninguna otra herramienta. Cópialo donde tu app sirve los archivos estáticos, por ejemplo en la carpeta `public/` de Vite:

```sh
cp target/wasm32-unknown-unknown/release/math.wasm ../public/math.wasm
```

## Cargarlo

Pasa la URL del archivo a `wasm()`, con el tipo de sus exportaciones:

```ts
// src/wasm.ts
import { wasm } from "active-react";

export const math = wasm<{ add(a: number, b: number): number; fib(n: number): number }>("/math.wasm");
```

Después úsalo como cualquier otro módulo:

```tsx
import { useWasm } from "active-react";
import { math } from "./wasm";

export function Fibonacci({ n }: { n: number }) {
  const m = useWasm(math);
  return <p>fib({n}) = {m ? m.fib(n) : "…"}</p>;
}
```

Como la URL es la propia fuente, los módulos puros siempre se precargan durante el renderizado en servidor. También puedes dejar que Vite añada un hash al nombre del archivo en lugar de usar `public/`:

```ts
import mathUrl from "./math.wasm?url";

export const math = wasm<{ add(a: number, b: number): number }>(mathUrl);
```

> **Nota:** Sin argumento de tipo, las exportaciones se tipan como `WebAssembly.Exports`. Escribe la interfaz a mano: es tu contrato con el código Rust.

## El tipo MIME

active-react descarga el archivo y lo compila **mientras se descarga** con `WebAssembly.instantiateStreaming`, que es la vía más rápida. Los navegadores solo lo permiten cuando el servidor envía la cabecera `Content-Type: application/wasm`.

Si la cabecera falta o es distinta, active-react recurre a descargar primero el archivo completo (`arrayBuffer()`) y después compilarlo. Tu módulo sigue funcionando, solo que un poco más tarde. Los servidores de desarrollo y de previsualización de Vite y la mayoría de los hostings estáticos envían la cabecera correcta; comprueba la tuya en la pestaña de red del navegador y consulta las [preguntas frecuentes](../faq/) para corregirla.

Una respuesta que no sea 2xx es un error, por ejemplo `wasm: /math.wasm responded 404`, que se lanza a tu error boundary como cualquier [error de carga](../loading-wasm/#manejo-de-errores).

## Pasar importaciones

Un módulo puro también puede llamar a JavaScript. Declara las funciones en Rust, dentro de un módulo de importación llamado `env`:

```rust
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn now() -> f64;
    fn report(ms: f64);
}

#[unsafe(no_mangle)]
pub extern "C" fn timed_fib(n: u32) -> u32 {
    let start = unsafe { now() };
    let result = fib(n);
    unsafe { report(now() - start) };
    result
}
```

Después proporciónalas con la opción `imports`:

```ts
export const math = wasm<{ timed_fib(n: number): number }>("/math.wasm", {
  imports: {
    env: {
      now: () => performance.now(),
      report: (ms: number) => console.log(`fib took ${ms} ms`),
    },
  },
});
```

> **Consejo:** Las versiones recientes de Rust se niegan a enlazar funciones declaradas pero no definidas. El atributo `#[link(wasm_import_module = "env")]` las marca explícitamente como importaciones.

## Números en la frontera

Las funciones WebAssembly solo intercambian números, y conviene conocer algunas conversiones:

| Rust | JavaScript |
| --- | --- |
| `i32`, `f32`, `f64` | `number` |
| `u32` | `number`, pero leído **con signo**: los valores por encima de 2 147 483 647 vuelven negativos. Usa `result >>> 0` |
| `i64`, `u64` | `bigint` |
| `bool` | `number`: `1` para true, `0` para false |
| `usize`, punteros | `number` (32 bits en `wasm32`) |

## Limitaciones

- **Solo números.** Las cadenas, los arrays y los structs viven en la memoria lineal del módulo. Para pasarlos, tienes que exportar un asignador de memoria, copiar bytes en `memory.buffer` desde JavaScript, pasar un puntero y una longitud y, después, liberar la memoria. wasm-bindgen genera exactamente este código por ti, y correctamente.
- **Los tipos se escriben a mano.** Nada comprueba que tu interfaz TypeScript coincida con las firmas de Rust.
- **Sin mensajes de panic.** Un panic provoca un trap y lanza un `RuntimeError` genérico en JavaScript, sin el mensaje de Rust.

Si alguna de estas limitaciones te estorba, cambia a [wasm-bindgen](../rust-setup/). El glue JavaScript solo añade 0,93 kB gzip para tres funciones.
