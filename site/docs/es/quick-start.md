---
title: Inicio rápido
description: Crea en unos cinco minutos una app React 19 con etiquetas SEO completas y una función Rust compilada a WebAssembly, con Vite y wasm-pack.
---

En este tutorial vas a crear una app con Vite + React 19, darle etiquetas SEO correctas y después llamar a una función Rust compilada a WebAssembly. Lleva unos cinco minutos si ya tienes Rust instalado.

## 1. Crea la app

```sh
npm create vite@latest my-app -- --template react-ts
cd my-app
npm install
npm i github:maxlestage/Active
```

La plantilla de React de Vite usa React 19, que es lo que active-react requiere. Puedes comprobarlo con `npm ls react`.

## 2. Escribe los valores por defecto de tu sitio

Crea `src/seo.ts`. Estos valores los comparten todas las páginas, así que los escribes una sola vez:

```ts
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "My App",
  titleTemplate: "%s · My App",
  baseUrl: "https://my-app.example", // relative URLs become absolute
  image: "/og.png",
  locale: "en_US",
});
```

`createSeo` devuelve un componente `<Seo>` con estos valores por defecto incorporados. Más información en [valores por defecto del sitio](../site-defaults/).

## 3. Añade SEO a una página

Reemplaza `src/App.tsx`:

```tsx
import { Seo } from "./seo";

export default function App() {
  return (
    <main>
      <Seo title="Home" description="A React 19 app with SEO tags and Rust." canonical="/" />
      <h1>Hello</h1>
    </main>
  );
}
```

Inicia el servidor de desarrollo con `npm run dev` e inspecciona la página con las herramientas de desarrollo de tu navegador. Ahora el `<head>` también contiene:

```html
<title>Home · My App</title>
<meta name="description" content="A React 19 app with SEO tags and Rust."/>
<link rel="canonical" href="https://my-app.example/"/>
<meta property="og:title" content="Home"/>
<meta property="og:description" content="A React 19 app with SEO tags and Rust."/>
<meta property="og:type" content="website"/>
<meta property="og:url" content="https://my-app.example/"/>
<meta property="og:image" content="https://my-app.example/og.png"/>
<meta property="og:site_name" content="My App"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
```

Renderizaste `<Seo>` dentro de `<main>` y, sin embargo, las etiquetas están en el `<head>`: React 19 las trasladó allí.

> **Nota:** `index.html` todavía tiene un `<title>` estático. React inserta su propio título antes que él, así que la pestaña del navegador muestra el correcto. Puedes eliminar el título estático si prefieres un único elemento.

## 4. Crea un crate de Rust

Instala las herramientas una sola vez (consulta [Configuración de Rust](../rust-setup/) para más detalles):

```sh
# Rust itself: https://rustup.rs
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Después crea un crate de biblioteca junto a `src/`:

```sh
cargo new --lib hello --vcs none
```

Reemplaza `hello/Cargo.toml`:

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

Y `hello/src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello {name}, from Rust!")
}

/// Number of primes below `n` (sieve of Eratosthenes).
#[wasm_bindgen]
pub fn count_primes(n: u32) -> u32 {
    let n = n as usize;
    if n < 2 {
        return 0;
    }
    let mut sieve = vec![true; n];
    sieve[0] = false;
    sieve[1] = false;
    let mut i = 2;
    while i * i < n {
        if sieve[i] {
            let mut j = i * i;
            while j < n {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    sieve.iter().filter(|&&p| p).count() as u32
}
```

## 5. Compílalo a WebAssembly

```sh
wasm-pack build hello --target web --out-dir ../src/pkg
```

La ruta de `--out-dir` es relativa al crate, así que el resultado acaba en `src/pkg/`:

- `hello.js`: el código de enlace (glue) JavaScript generado por wasm-bindgen;
- `hello_bg.wasm`: el Rust compilado;
- `hello.d.ts`: los tipos TypeScript de tus funciones.

Guarda el comando como script en `package.json` y ejecútalo antes de cada build:

```json
{
  "scripts": {
    "wasm": "wasm-pack build hello --target web --out-dir ../src/pkg",
    "build": "npm run wasm && tsc -b && vite build"
  }
}
```

## 6. Declara el módulo

Crea `src/wasm.ts`:

```ts
import { wasm } from "active-react";
import url from "./pkg/hello_bg.wasm?url";

export const hello = wasm(() => import("./pkg/hello.js"), { url });
```

Todavía no se descarga nada: `wasm()` solo declara el módulo. El import con `?url` le pide a Vite la URL final del binario, lo que permite a active-react precargarlo durante el renderizado en servidor.

## 7. Llama a Rust desde un componente

Actualiza `src/App.tsx`:

```tsx
import { useWasm } from "active-react";
import { Seo } from "./seo";
import { hello } from "./wasm";

export default function App() {
  const rust = useWasm(hello); // undefined while loading

  return (
    <main>
      <Seo title="Home" description="A React 19 app with SEO tags and Rust." canonical="/" />
      <h1>Hello</h1>
      <p>{rust ? rust.greet("you") : "Loading the Rust module…"}</p>
      <p>{rust && `${rust.count_primes(1_000_000)} primes below one million`}</p>
    </main>
  );
}
```

`rust` está completamente tipado a partir de `hello.d.ts`: prueba a escribir mal `count_primes` y TypeScript lo detectará.

## 8. Ejecútalo

```sh
npm run dev
```

La página muestra primero "Loading the Rust module…" y luego el saludo y 78 498 números primos. Ejecuta `npm run build` y `npm run preview` para probar el build de producción.

## Siguientes pasos

Tu app es una single-page app: las etiquetas SEO las añade JavaScript en el navegador. Google ejecuta JavaScript, pero muchos rastreadores y bots de vista previa de enlaces no lo hacen. Para obtener el mejor SEO, prerrenderiza tus páginas a HTML:

- [Renderizado en servidor](../server-rendering/) explica el SSR, la generación estática y `renderWithHead`.
- El [ejemplo vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg) del repositorio es el mismo tipo de app, más generación estática: cada página se entrega como HTML con su `<head>` completo y el binario `.wasm` precargado, y después se hidrata y llama a Rust.
- [Cargar WebAssembly](../loading-wasm/) trata los errores, Suspense y la carga fuera de React.
