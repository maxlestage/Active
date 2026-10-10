---
title: Preguntas frecuentes
description: Respuestas a las preguntas habituales sobre active-react: React 18 y Next.js, errores de wasm-bindgen, tipos MIME, navegadores compatibles y licencia.
---

## ¿Funciona con React 18?

No. active-react se basa en una funcionalidad de React 19: los elementos `<title>`, `<meta>` y `<link>` renderizados en cualquier parte del árbol son trasladados al `<head>` por el propio React. Eso es lo que permite que `<Seo>` sea un componente simple, sin provider ni efecto. Con React 18, las etiquetas se quedarían en el body. Actualiza con `npm i react@latest react-dom@latest`.

## ¿Puedo usarlo con Next.js?

Con el **App Router**, Next.js tiene su propia Metadata API (`export const metadata` y `generateMetadata`), integrada con su enrutamiento, su streaming y sus convenciones de archivos. Dale preferencia para el SEO, para que los dos sistemas no produzcan etiquetas duplicadas.

La parte de WebAssembly sí se puede usar en Next.js: declara el módulo con `wasm()` y llama a `useWasm()` en componentes cliente (archivos que empiezan por `"use client"`). Obtén la URL del binario con `new URL("./pkg/my_crate_bg.wasm", import.meta.url).href`, ya que el sufijo `?url` es específico de Vite.

## ¿Funciona con otros frameworks?

Sí. React Router v7 (modo framework), TanStack Start y Waku no necesitan ninguna configuración. Estos frameworks renderizan el documento completo con React 19, así que las etiquetas que renderiza `<Seo>` acaban automáticamente en el `<head>` del HTML del servidor. Consulta [renderizado en servidor](../server-rendering/).

## ¿Puede una página tener dos `<Seo>`?

Obtienes dos conjuntos de etiquetas: dos títulos, dos descripciones, dos enlaces canónicos. React traslada todo lo que recibe y no fusiona las etiquetas. Renderiza un único `<Seo>` por página, en el componente de la página, y coloca los valores que comparten todas las páginas en [`createSeo`](../site-defaults/) en lugar de en un segundo `<Seo>` dentro de un layout.

## ¿Por qué hay dos etiquetas `<title>`?

Es el `<title>` estático de tu `index.html`. React inserta el título de la página **antes** que él, así que `document.title`, la pestaña del navegador y los buscadores usan el correcto. Puedes eliminar el título estático de `index.html` para tener un único elemento.

Eso sí, elimina de `index.html` las etiquetas estáticas `<meta name="description">` y `og:*`: esas sí quedarían duplicadas.

## ¿Google lee el JSON-LD en el body?

Sí. Google lee los scripts JSON-LD en cualquier parte del documento, `<head>` o `<body>`. React 19 no traslada los scripts normales, así que el `<script type="application/ld+json">` se queda donde se renderiza `<Seo>`. Comprueba tus páginas con la [prueba de resultados enriquecidos](https://search.google.com/test/rich-results). Más información en [datos estructurados](../structured-data/).

## ¿Puedo usar active-react sin Rust?

Por supuesto. La parte SEO no necesita Rust en absoluto y, gracias al tree-shaking, una app que solo usa SEO envía únicamente 0,74 kB gzip: el cargador de WebAssembly nunca se incluye si no importas `wasm` ni `useWasm`.

## Versiones de wasm-bindgen que no coinciden

El error completo empieza así:

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary
```

La herramienta de línea de comandos `wasm-bindgen` debe tener exactamente la misma versión que el crate `wasm-bindgen` de tu `Cargo.lock`. Instala el CLI correspondiente:

```sh
cargo install -f wasm-bindgen-cli --version 0.2.129
```

o fija el crate a la versión de tu CLI en `Cargo.toml` con `wasm-bindgen = "=0.2.129"`. wasm-pack evita el problema por completo, ya que descarga por ti el CLI adecuado. Consulta [Configuración de Rust](../rust-setup/).

## Mi servidor no envía application/wasm

Tu módulo se carga igualmente: cuando el `Content-Type` no es `application/wasm`, active-react descarga el archivo completo y lo compila después, en lugar de compilarlo mientras se descarga. Solo es un poco más lento. Para corregir la cabecera:

```text
# nginx (inside http, server or location)
types { application/wasm wasm; }

# Apache (.htaccess or server config)
AddType application/wasm .wasm
```

Los servidores de desarrollo y de previsualización de Vite y `express.static` de Express ya envían el tipo correcto. Con un servidor Node escrito a mano, define `content-type: application/wasm` para los archivos `.wasm`.

## ¿Puedo ejecutar Rust en el servidor durante el SSR?

`useWasm()` devuelve deliberadamente `undefined` en el servidor y durante la hidratación, para que el HTML del servidor y el primer renderizado en el cliente coincidan siempre. El resultado de Rust aparece justo después de la hidratación.

Si necesitas un resultado de Rust dentro del HTML del servidor, calcúlalo en el servidor antes de renderizar y pásalo como prop. Con la salida `--target web` de wasm-bindgen, Node puede cargar el binario de forma síncrona:

```ts
// server only
import { readFileSync } from "node:fs";
import { count_primes, initSync } from "./pkg/hello.js";

initSync({ module: readFileSync(new URL("./pkg/hello_bg.wasm", import.meta.url)) });
const primes = count_primes(1_000_000); // pass it to your page as a prop
```

En un servidor Rust, llama directamente a tu código Rust y usa el [crate `active`](../rust-crate/) para las etiquetas SEO.

## ¿Qué navegadores son compatibles?

Todos los navegadores actuales: Chrome, Edge, Firefox y Safari, en escritorio y en móvil. WebAssembly en sí es compatible en todas partes desde 2017. La compilación en streaming (`WebAssembly.instantiateStreaming`), que se usa para los módulos `.wasm` puros servidos como `application/wasm`, requiere Safari 15 o posterior. Los navegadores que no admiten los enlaces `preload` simplemente los ignoran y cargan el módulo con normalidad.

## ¿Cuánto pesa realmente?

Unos 1,3 kB gzip si lo usas todo: 0,74 kB para la parte SEO y 0,60 kB para el cargador de WebAssembly, medido con esbuild y sin contar React. Con el tree-shaking, solo pagas por lo que importas. Los [benchmarks](../benchmarks/) lo comparan con las alternativas.

## ¿Cuál es la licencia?

MIT, tanto para el paquete npm como para el crate de Rust. Puedes usarlo en proyectos comerciales y de código cerrado; conserva el aviso de copyright.

## ¿Cómo puedo contribuir?

Las issues y las pull requests son bienvenidas en [GitHub](https://github.com/maxlestage/Active). Para trabajar en el código:

```sh
git clone https://github.com/maxlestage/Active.git
cd Active
npm install
npm test            # vitest: SSR, client (jsdom) and wasm tests
npm run typecheck
npm run build       # dist/ (ESM + .d.ts)
```

El crate de Rust está en `crates/active` (`cargo test`), este sitio web en `site/` y los benchmarks en `bench/` (`npm run setup && npm run bench`). Ejecuta los tests antes de abrir una pull request e incluye una ejecución de los benchmarks si tu cambio puede afectar al rendimiento.
