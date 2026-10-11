---
title: Benchmarks
description: Active frente a Leptos, Dioxus, Yew, Sycamore, maud y HTML escrito a mano: renderizado en el servidor, tamaño del WebAssembly y tiempo hasta la interactividad.
---

`active` se comparó con los principales frameworks web de Rust en tres preguntas: con qué rapidez renderiza el servidor una página completa, cuánto WebAssembly descarga el navegador para un pequeño componente interactivo y cuánto tarda ese componente en poder usarse. Todo es Rust: las páginas, los servidores y el arnés de pruebas que controla Chromium. La suite completa está en `bench/`.

## Resumen

| | Active | El mejor de los demás |
| --- | ---: | ---: |
| Renderizado en el servidor de una página de producto | 5.58 µs | 5.19 µs (Leptos, `<head>` escrito a mano), 13.44 µs con leptos_meta |
| WebAssembly + código de enlace para un contador (gzip) | **36.8 kB** | 45.8 kB (Sycamore) |
| Interactivo, con 100 ms de latencia de red | **224.2 ms** | 226.7 ms (Sycamore) |

- **El bundle más pequeño.** La isla del contador envía 36.8 kB con gzip en total: un 20 % menos que Sycamore, un 40 % menos que Leptos y 4.6 veces menos que Dioxus.
- **Renderizado rápido en el servidor con SEO real.** Entre los frameworks que fusionan en el `<head>` las etiquetas SEO de los componentes (el `Document` de `active`, Leptos con leptos_meta), `active` es 2.4 veces más rápido. Solo Leptos escribiendo todo el `<head>` a mano en una única plantilla es ligeramente más rápido (un 7 %).
- **Interactivo en cuanto se ejecuta el bundle.** Los cuatro frameworks con hidratación quedan a pocos milisegundos unos de otros; lo que domina es la descarga, así que un bundle más pequeño ayuda en redes lentas.

> **Nota:** estas cifras provienen de una sola máquina (Intel Xeon a 2.8 GHz, 4 núcleos, Rust 1.97, Chromium 141). Compara los frameworks entre sí, no con tu propio hardware.

## Renderizado en el servidor

Cada implementación renderiza el mismo documento completo para la página de producto `i`, con un `i` distinto en cada iteración: `<html lang="en">`, un `<head>` con 11 etiquetas SEO (título con plantilla, descripción, canonical, Open Graph, Twitter) y un script JSON-LD, y un `<body>` con un `<h1>` y una lista de 30 elementos. Cada salida se comprueba etiqueta por etiqueta antes de medir. Cada implementación se ejecuta en su propio proceso, se calienta durante 1 s y después renderiza durante 3 s en lotes de unos 10 ms; la tabla muestra la mediana de tres ejecuciones.

| Implementación | µs / página | Páginas / s | Frente a escrito a mano |
| --- | ---: | ---: | ---: |
| maud 0.27 | 1.37 | 729 288 | ×0.68 |
| Escrito a mano (`push_str`/`write!`) | 2.01 | 497 191 | ×1.00 |
| Leptos 0.8, `<head>` escrito a mano | 5.19 | 192 686 | ×2.58 |
| **active** | **5.58** | **179 083** | **×2.78** |
| Leptos 0.8 + leptos_meta | 13.44 | 74 406 | ×6.68 |
| Yew 0.23 | 17.17 | 58 257 | ×8.53 |
| Dioxus 0.7 | 24.22 | 41 281 | ×12.04 |
| Sycamore 0.9 | 33.01 | 30 298 | ×16.41 |

maud y la página escrita a mano son plantillas: solo escriben cadenas. `active` construye un árbol de componentes, recoge el `Seo` de cada componente y los fusiona en el head, que es el trabajo que necesita un sitio real. Escribe directamente en un único `String`, sin DOM virtual, y la mayoría de los elementos no necesitan ninguna asignación de memoria aparte de su texto.

## Tamaño del WebAssembly

El mismo contador en todos los frameworks: un botón que incrementa el número que tiene al lado, renderizado por el servidor y retomado por el navegador. Compilado para `wasm32-unknown-unknown` con el perfil optimizado para tamaño (`opt-level = "s"`, `lto`, `panic = "abort"`) y después con `wasm-bindgen --target web --no-typescript`, sin `wasm-opt`. El total incluye el código de enlace JavaScript generado por wasm-bindgen.

| Framework | Modo en el navegador | .wasm | .wasm gzip | Enlace gzip | Total gzip |
| --- | --- | ---: | ---: | ---: | ---: |
| **active** | hidratación de islas | 81.6 kB | 32.4 kB | 4.3 kB | **36.8 kB** |
| Sycamore | hidratación | 105.8 kB | 41.1 kB | 4.7 kB | 45.8 kB |
| Leptos | hidratación | 144.9 kB | 54.8 kB | 6.2 kB | 60.9 kB |
| Yew | hidratación | 241.8 kB | 90.0 kB | 6.3 kB | 96.3 kB |
| Dioxus | renderizado en el cliente | 426.9 kB | 155.4 kB | 15.0 kB | 170.4 kB |

> **Consejo:** el contador del benchmark registra los tiempos con un número de coma flotante, lo que arrastra el formateo de flotantes de Rust a cada bundle. Sin él, el ejemplo `hello` ocupa unos 22 kB con gzip. Consulta [rendimiento](../performance/) para ver el tamaño de cada tipo de aplicación y cómo reducir aún más un bundle.

## Tiempo hasta la interactividad

Chromium sin interfaz (headless) carga cada página de contador 21 veces por perfil de red, con la caché HTTP desactivada, alternando entre frameworks. Cada aplicación escribe `performance.now()` en la página cuando arranca su WebAssembly y cuando el botón pasa a ser interactivo; el arnés lee esas marcas mediante el protocolo DevTools y, al final, hace clic de verdad en el botón (el número debe pasar de 3 a 4).

| Framework | Localhost: mediana (mín–máx) | Latencia de 100 ms: mediana (mín–máx) | Arranque → interactivo |
| --- | ---: | ---: | ---: |
| Yew | 25.3 ms (21–38) | 227.8 ms (225–239) | 0.8 ms |
| **active** | 25.5 ms (21–42) | **224.2 ms (221–232)** | 0.6 ms |
| Leptos | 25.5 ms (22–38) | 227.2 ms (222–239) | 0.5 ms |
| Sycamore | 25.7 ms (21–33) | 226.7 ms (222–392) | 0.5 ms |
| Dioxus | 41.3 ms (32–58) | 339.3 ms (332–351) | 1.0 ms |

Los rangos de los frameworks con hidratación se solapan: considéralos empatados. Dioxus renderiza en el cliente (su hidratación necesita su propio servidor full-stack) y su código de enlace importa archivos adicionales, lo que cuesta un viaje de ida y vuelta más por la red.

## Ejecútalo tú mismo

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cd bench
cargo run --release -p runner                    # everything, about 3 minutes after the first build
cargo run --release -p runner -- ssr             # one part: ssr, wasm, browser or report
BENCH_PRINT=1 cargo run --release -p ssr-active  # print the page an implementation renders
```

El runner encuentra Chromium en el `PATH` (o define `CHROME=/path/to/chrome`). Escribe `bench/RESULTS.md` y los datos en bruto en `bench/results/`. El método, la versión de cada framework y las decisiones tomadas para cada uno se describen en `bench/README.md`.
