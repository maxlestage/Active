---
title: Guía de rendimiento
description: Lo que enseñan los benchmarks sobre cargar, llamar y dimensionar Rust/WebAssembly en React, y por qué el SEO con active-react no cuesta casi nada.
---

Cada decisión de diseño de active-react se basa en mediciones. Esta página convierte los [benchmarks](../benchmarks/) en consejos prácticos: qué hace que un módulo WebAssembly se cargue rápido, se llame rápido y sea pequeño, y cuánto cuesta la parte SEO.

## Carga: la red decide

Los benchmarks midieron el tiempo desde la navegación hasta el primer resultado de Rust en pantalla, con todas las formas de cargar un módulo en una página renderizada en el cliente:

| Escenario (renderizado en el cliente) | localhost | RTT de 100 ms |
| --- | ---: | ---: |
| active-react, `.wasm` puro | 28,8 ms | 331,5 ms |
| react-wasm 1.0.1, `.wasm` puro | 28,5 ms | 327,1 ms |
| `useEffect` escrito a mano, `.wasm` puro | 25,8 ms | 328,9 ms |
| active-react + wasm-bindgen | 23,5 ms | 328,4 ms |
| `useEffect` escrito a mano + wasm-bindgen | 24,6 ms | 332,9 ms |

Con una latencia realista, todos rondan los **330 ms**. El código de carga no importa: el tiempo se va en los viajes de ida y vuelta por la red (el HTML, después el JavaScript, después el archivo `.wasm`) y en la compilación.

Así que hay dos cosas que marcan una diferencia real:

1. **Empezar la descarga antes.** Con renderizado en servidor, active-react emite un preload para el binario, que entonces se descarga en paralelo con el JavaScript. Misma página, mismo módulo: **224,2 ms con preload frente a 330,0 ms sin él, un 32 % más rápido**.
2. **Enviar un binario más pequeño.** Menos bytes que descargar y compilar. Consulta [tamaño del binario](#tamano-del-binario) más abajo.

> **Consejo:** Pasa a `wasm()` la opción `url` y renderiza en el servidor o en tiempo de build. Obtienes el preload sin esfuerzo. Consulta [cargar WebAssembly](../loading-wasm/#preload).

## Llamadas: agrupa el trabajo

Llamar a WebAssembly es rápido, pero no gratis. Medido en Chromium:

| Medida | JS puro | wasm puro | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)`, por llamada | 0,55 ns | 4,05 ns | 4,81 ns |
| `fib(30)`, recursivo | 11,4 ms | 2,1 ms | 1,9 ms |
| `greet(str) -> String`, por llamada | 4,7 ns | — | 553,0 ns |

Tres lecciones:

- **Rust es unas 5 veces más rápido en trabajo real.** `fib(30)` tarda 2,1 ms en WebAssembly frente a 11,4 ms en JavaScript.
- **Cada llamada cuesta unos 4 ns**, frente a 0,6 ns para una función JavaScript. Diez millones de llamadas diminutas desperdician tiempo cruzando la frontera.
- **Las cadenas son caras: unos 550 ns por llamada.** wasm-bindgen copia y vuelve a codificar cada cadena en ambas direcciones.

La regla: **envía grandes bloques de trabajo, no muchas llamadas pequeñas.** Pasa un typed array completo de una vez en lugar de llamar a Rust para cada elemento:

```rust
use wasm_bindgen::prelude::*;

// One call processes the whole dataset.
#[wasm_bindgen]
pub fn smooth(values: &[f64], window: usize) -> Vec<f64> {
    let window = window.max(1);
    values
        .windows(window)
        .map(|w| w.iter().sum::<f64>() / window as f64)
        .collect()
}
```

```ts
// Good: one call, one copy in, one copy out.
const smoothed = rust.smooth(new Float64Array(samples), 5);

// Slow: one boundary crossing per item (scale being a tiny Rust function).
const scaled = samples.map((x) => rust.scale(x, 2));
```

Reserva las cadenas para los extremos (un título, un resultado que mostrar), no para los bucles internos.

## Cuándo vale la pena Rust

Buenos candidatos: bucles intensivos sobre números, parsing, compresión, procesamiento de imagen y audio, simulaciones, criptografía, búsqueda en grandes volúmenes de datos en memoria. Una tarea que tarda milisegundos en JavaScript y se ejecuta en una sola llamada es el caso ideal.

Malos candidatos: funciones pequeñas llamadas muy a menudo desde JavaScript, formateo de cadenas, trabajo con el DOM (WebAssembly no puede tocar el DOM sin pasar por JavaScript).

## Tamaño del binario

| Elemento | Minificado | Gzip |
| --- | ---: | ---: |
| active-react `wasm` + `useWasm` | 1,03 kB | 0,60 kB |
| react-wasm 1.0.1 `useWasm` | 5,27 kB | 2,07 kB |
| Glue JS de wasm-bindgen (3 funciones) | 1,89 kB | 0,93 kB |
| Binario `extern "C"` puro (add + fib) | 144 B | 139 B |
| Binario wasm-bindgen (add + fib + greet) | 15,87 kB | 6,76 kB |

El binario de wasm-bindgen es unas 100 veces más grande que el puro, y casi todo procede de la función `greet`: `format!` y la maquinaria de `String` (asignador de memoria, manejo de UTF-8, formateo) pesan unos 15 kB. Las funciones puramente numéricas se compilan en unas pocas decenas de bytes.

Para mantener los binarios pequeños:

- Evita `format!`, `println!` y `to_string()` en el código que se envía al navegador cuando puedas devolver números en su lugar.
- Vigila las dependencias pesadas: `serde_json`, `regex` o `chrono` añaden mucho cada una.
- Para módulos solo numéricos, prescinde por completo de wasm-bindgen: consulta [WebAssembly puro](../raw-wasm/).

## Perfiles de Cargo

El mismo crate, medido en tamaño y velocidad (`lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` en todas las filas; `fib(30)` medido en Node):

| Perfil | wasm puro | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15,87 kB | 6,76 kB | 3,76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14,46 kB | 6,37 kB | 3,89 ms |
| `opt-level = "s"` | 144 B | 14,71 kB | 6,51 kB | 4,17 ms |
| `opt-level = "z"` | 137 B | 14,72 kB | 6,52 kB | 4,32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13,58 kB | 6,19 kB | 3,88 ms |

## Recomendaciones

- **Usa por defecto `opt-level = 3`** con `lto`, `codegen-units = 1`, `panic = "abort"` y `strip`. Es el perfil más rápido, y la diferencia de tamaño con los demás es pequeña.
- **Si cada kilobyte cuenta**, usa `opt-level = "z"` más `wasm-opt -Oz` (de binaryen). Es aproximadamente un 8 % más pequeño en gzip (6,19 kB frente a 6,76 kB), con una velocidad similar en estas mediciones.
- `opt-level = "s"` o `"z"`, sin nada más, son más lentos y apenas más pequeños: por sí solos no merecen la pena.
- **Precarga** el binario con renderizado en servidor y la opción `url`.
- **Agrupa** las llamadas y evita las cadenas en las rutas críticas.

## El SEO no cuesta casi nada

La parte SEO se midió frente a etiquetas escritas a mano (React 19 sin biblioteca) y las alternativas populares, en la misma página:

- **Renderizado en servidor:** 42,4 µs por página, frente a 39,7 µs con etiquetas escritas a mano, es decir, +7 %. Es **1,5× más rápido que react-helmet** (66,5 µs). Con `renderWithHead` y una plantilla HTML: 44,1 µs.
- **Actualizaciones del head al navegar:** una mediana de **0,3 ms** entre el renderizado de una nueva página y un `<head>` actualizado, lo mismo que con etiquetas escritas a mano.
- **Tamaño del bundle:** 0,74 kB gzip, frente a entre 6,07 y 6,76 kB para @unhead/react, react-helmet-async y react-helmet.

La razón es sencilla: React 19 hace el trabajo. `<Seo>` renderiza etiquetas y nada más: ni contexto, ni efecto, ni segunda pasada por el árbol.
