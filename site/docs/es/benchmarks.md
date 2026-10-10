---
title: Benchmarks
description: Cómo se compara active-react con react-helmet, react-head, @unhead/react y react-wasm en tamaño, velocidad de SSR, navegador y carga de WASM.
---

Cada decisión de diseño de active-react se ha contrastado con mediciones. Esta página presenta el método, todos los resultados y cómo reproducirlos en tu máquina. Para las conclusiones prácticas, lee la [guía de rendimiento](../performance/).

## Metodología

**Entorno:** Intel(R) Xeon(R) Processor @ 2,80 GHz (4 núcleos), Node v22.22.0, Chromium 141.0.7390.37, React 19.3.0.

**La misma página para cada biblioteca**, compilada en modo producción: un `<title>` con plantilla, una descripción, una URL canónica, 6 etiquetas Open Graph, 2 etiquetas de Twitter, un `Product` en JSON-LD y un body de 30 elementos.

**Bibliotecas de SEO comparadas:** React 19 con etiquetas escritas a mano y sin biblioteca (la referencia), active-react, react-helmet 6.1.0, react-helmet-async 3.0.0, @unhead/react 3.4.2 y react-head 3.4.2.

- **Tamaño:** solo los imports que cada página usa realmente, empaquetados y minificados con esbuild, sin contar React ni React DOM. Comprimidos con gzip (nivel 9) y Brotli (calidad 11).
- **Renderizado en servidor:** el documento HTML completo (head y body). Cada biblioteca se ejecuta en **su propio proceso de Node** para que el estado del JIT nunca se filtre de una a otra. Tras un calentamiento, los renderizados se cronometran por lotes durante unos 2,5 segundos; el resultado es la mediana de 3 procesos.
- **Navegador:** Chromium headless, builds de producción, una página nueva por biblioteca. Después de las mediciones se audita el `<head>` final: sin duplicados y con los valores correctos.

**WASM:** dos crates de Rust con las mismas funciones, uno con exportaciones `extern "C"` simples (puro) y otro con wasm-bindgen, compilados con cada perfil de Cargo.

- **Carga:** tiempo desde el inicio de la navegación hasta que el primer resultado de Rust se renderiza en pantalla, en localhost y con **100 ms de latencia de ida y vuelta emulada mediante el Chrome DevTools Protocol** (realista para redes móviles). Mediana de 15 cargas en localhost y de 7 con latencia.
- **Costo de las llamadas:** 10 millones de llamadas a `add` y 300 000 llamadas a `greet` por variante, cada variante en su propio bucle para que los puntos de llamada sigan siendo monomórficos; mediana de 7 ejecuciones tras el calentamiento.

## Resultados de SEO

### Tamaño del bundle del cliente

JavaScript añadido al bundle por los imports realmente usados (minificado, sin contar React ni React DOM).

| Biblioteca | min | gzip | brotli |
| --- | ---: | ---: | ---: |
| React 19 nativo (sin biblioteca) | 0 | 0 | 0 |
| **active-react** | 1,53 kB | 0,74 kB | 0,62 kB |
| react-helmet-async 3.0.0 | 17,30 kB | 6,29 kB | 5,59 kB |
| @unhead/react 3.4.2 | 15,17 kB | 6,07 kB | 5,55 kB |
| react-head 3.4.2 | 3,20 kB | 1,35 kB | 1,21 kB |
| react-helmet 6.1.0 | 18,46 kB | 6,76 kB | 5,99 kB |

### Renderizado en servidor (Node)

Tiempo para producir el documento HTML completo (head y body), mediana de 3 procesos de unos 2,5 s cada uno.

| Biblioteca | µs / página | páginas / s | frente a React nativo |
| --- | ---: | ---: | ---: |
| React 19 nativo (sin biblioteca) | 39,7 | 25 199 | 0 % |
| **active-react** | 42,4 | 23 565 | +7 % |
| **active-react** (`renderWithHead`, plantilla HTML) | 44,1 | 22 665 | +11 % |
| @unhead/react 3.4.2 | 44,9 | 22 260 | +13 % |
| react-head 3.4.2 | 60,7 | 16 474 | +53 % |
| react-helmet-async 3.0.0 | 64,9 | 15 405 | +64 % |
| react-helmet 6.1.0 | 66,5 | 15 031 | +68 % |

### Navegador (Chromium headless)

- **Head listo:** desde el inicio de la navegación hasta que el título, la descripción y la canónica son correctos (mediana de 15 cargas).
- **Latencia:** entre el renderizado de una nueva página (cambio de ruta) y un `<head>` actualizado (300 navegaciones).
- **Ráfaga:** 1000 cambios de ruta seguidos, hasta que el `<head>` final es correcto.

| Biblioteca | Head listo | Latencia mediana | Latencia p95 | Ráfaga (nav/s) | `<head>` final |
| --- | ---: | ---: | ---: | ---: | ---: |
| React 19 nativo (sin biblioteca) | 20,9 ms | 0,3 ms | 0,5 ms | 2267 | Correcto |
| **active-react** | 23,8 ms | 0,3 ms | 0,6 ms | 2144 | Correcto |
| react-helmet-async 3.0.0 | 22,7 ms | 0,3 ms | 0,5 ms | 2402 | Correcto |
| @unhead/react 3.4.2 | 24,2 ms | 0,4 ms | 0,7 ms | 4753 | Correcto |
| react-head 3.4.2 | 23,5 ms | 1,4 ms | 5,7 ms | 1682 | Correcto |
| react-helmet 6.1.0 | 31,1 ms | 16,7 ms | 18,6 ms | 3362 | Correcto |

> **Nota:** Los enfoques basados en el hoisting nativo de React 19 (nativo, active-react, react-helmet-async 3, react-head) dejan en su sitio el `<title>` estático de `index.html`. React inserta su propio título **antes** que él, así que `document.title` es correcto. Elimina el `<title>` estático de `index.html` si quieres un único elemento.

## Resultados de WASM

### Perfiles de Cargo

El mismo crate, con `lto = true`, `codegen-units = 1`, `panic = "abort"` y `strip = true`. `fib(30)` recursivo medido en Node (V8).

| Perfil | wasm puro | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15,87 kB | 6,76 kB | 3,76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14,46 kB | 6,37 kB | 3,89 ms |
| `opt-level = "s"` | 144 B | 14,71 kB | 6,51 kB | 4,17 ms |
| `opt-level = "z"` | 137 B | 14,72 kB | 6,52 kB | 4,32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13,58 kB | 6,19 kB | 3,88 ms |

### Tamaños

| Elemento | min | gzip |
| --- | ---: | ---: |
| **active-react** `wasm` + `useWasm` | 1,03 kB | 0,60 kB |
| react-wasm 1.0.1 `useWasm` | 5,27 kB | 2,07 kB |
| Glue JS de wasm-bindgen (3 funciones) | 1,89 kB | 0,93 kB |
| Binario `extern "C"` puro (add + fib) | 144 B | 139 B |
| Binario wasm-bindgen (add + fib + greet) | 15,87 kB | 6,76 kB |

### Tiempo hasta el primer resultado WASM

Desde el inicio de la navegación hasta que se renderiza el resultado de una llamada a Rust (mediana). "RTT de 100 ms" emula una latencia de red realista (móvil, 4G) mediante el Chrome DevTools Protocol.

| Escenario | localhost | RTT de 100 ms |
| --- | ---: | ---: |
| active-react `wasm("/x.wasm")` (puro) | 28,8 ms | 331,5 ms |
| react-wasm 1.0.1 (puro) | 28,5 ms | 327,1 ms |
| `useEffect` escrito a mano (puro) | 25,8 ms | 328,9 ms |
| active-react + wasm-bindgen | 23,5 ms | 328,4 ms |
| `useEffect` escrito a mano + wasm-bindgen | 24,6 ms | 332,9 ms |
| SSR + hidratación, **con** preload automático (`url`) | 20,2 ms | 224,2 ms |
| SSR + hidratación, sin preload | 24,5 ms | 330,0 ms |

### Costo de las llamadas de JS a WASM (Chromium)

| Medida | JS puro | wasm puro | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)` por llamada | 0,55 ns | 4,05 ns | 4,81 ns |
| `fib(30)` recursivo | 11,4 ms | 2,1 ms | 1,9 ms |
| `greet(str) -> String` por llamada | 4,7 ns | — | 553,0 ns |

## Reproducir

Requisitos previos:

- Node.js 22 o posterior;
- Rust con el target `wasm32-unknown-unknown`;
- `wasm-bindgen-cli` 0.2.129: `cargo install wasm-bindgen-cli --version 0.2.129`;
- Chromium para Playwright: define `PLAYWRIGHT_BROWSERS_PATH`, o ejecuta `npx playwright install chromium`.

Después, desde la raíz del repositorio:

```sh
cd bench
npm run setup   # builds the library and installs the benchmark dependencies
npm run bench   # everything, about 5 minutes
```

Ejecuta solo una de las dos mitades con `node run.mjs seo` o `node run.mjs wasm`. El script reescribe `bench/RESULTS.md` y guarda los datos en bruto en `bench/results/`. La página escrita con cada biblioteca está en `bench/seo/libs/`, y las páginas WASM en `bench/wasm/pages/`.

## Salvedades

- **Las cifras de SSR varían en torno a ±5 % entre ejecuciones.** Las diferencias de unos pocos puntos porcentuales (active-react frente a @unhead/react, por ejemplo) están dentro del ruido; las grandes diferencias (frente a react-helmet) no lo están.
- **`performance.now()` tiene una precisión reducida a 0,1 ms en Chromium** sin aislamiento de origen cruzado (cross-origin isolation), así que las duraciones muy cortas, como las latencias de head de 0,3 ms, son aproximadas.
- **Las columnas de `fib(30)` difieren entre tablas** porque se midieron en motores y configuraciones distintos: Node para los perfiles de Cargo, Chromium para el costo de las llamadas.
- **Una máquina, una página.** Las cifras absolutas dependen de tu hardware y de tus páginas; lo que importa son las comparaciones entre bibliotecas.
- **La prueba de "ráfaga"** mide el rendimiento bruto de 1000 cambios de ruta consecutivos, algo que los usuarios reales nunca hacen. La latencia por navegación es la cifra más significativa para las apps reales.
