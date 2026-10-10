---
title: Cargar WebAssembly
description: Declara un módulo Rust con wasm(), úsalo con useWasm(), precárgalo durante el SSR y gestiona los errores, Suspense y la carga fuera de React.
---

Dos funciones cubren WebAssembly en active-react: `wasm()` declara un módulo y `useWasm()` lo usa en un componente. Esta página trata ambas, además del preload, los errores y las formas menos habituales de cargar un módulo.

## Declarar un módulo

Declara cada módulo una sola vez, en su propio archivo, e impórtalo donde lo necesites:

```ts
// src/wasm.ts
import { wasm } from "active-react";
import url from "./pkg/hello_bg.wasm?url";

export const hello = wasm(() => import("./pkg/hello.js"), { url });
```

- El primer argumento es una función que importa el glue generado por wasm-bindgen o wasm-pack. Como es un `import()` dinámico, tu bundler coloca el glue en un chunk aparte.
- `url` es la URL pública del binario `.wasm`. Es opcional, pero habilita el preload que se describe más abajo.

**En este punto no se descarga nada.** Declarar módulos no cuesta nada, así que puedes declarar tantos como quieras.

## Usarlo en un componente

```tsx
import { useWasm } from "active-react";
import { hello } from "./wasm";

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(hello);
  if (!rust) return <p>Loading…</p>;
  return <p>Mean: {rust.mean(data)}</p>;
}
```

`useWasm(mod)` devuelve las exportaciones del módulo, o `undefined` mientras se carga:

- El primer componente que lo llama inicia la descarga. Todos los demás componentes comparten la misma petición: el módulo se descarga, se compila y se inicializa **una sola vez** para toda la app.
- Cuando termina la carga, todos los componentes que lo usan se vuelven a renderizar con las exportaciones.
- Una vez cargado, los componentes posteriores reciben las exportaciones en su primer renderizado, sin estado de carga.
- Siempre devuelve `undefined` en el servidor y durante la hidratación, para que el marcado del servidor y el del cliente coincidan. Consulta [renderizado en servidor](../server-rendering/#hidratacion-y-webassembly).

## Exportaciones tipadas

Las exportaciones se tipan a partir del archivo `.d.ts` que wasm-bindgen genera junto al glue. Si tu función Rust es `pub fn mean(values: &[f64]) -> f64`, entonces `rust.mean` es `(values: Float64Array) => number`, y TypeScript detecta las erratas y los argumentos incorrectos. No hay nada que configurar: el tipo se propaga desde el `import()`.

## Preload

Cuando se conoce `url`, `useWasm()` le pide a React que precargue el binario. Durante el renderizado en servidor, esto añade una etiqueta al `<head>`:

```html
<link rel="preload" href="/assets/hello_bg-CSLcOxPe.wasm" as="fetch" crossorigin="" type="application/wasm"/>
```

El navegador empieza a descargar el archivo `.wasm` en cuanto lee el `<head>`, en paralelo con tu JavaScript, en lugar de esperar a que React se ejecute. En los benchmarks, con 100 ms de latencia de red, el primer resultado de Rust apareció a los **224 ms en lugar de a los 330 ms (−32 %)**.

`as="fetch"` y `crossorigin` corresponden a la forma en que el binario se solicita realmente después (con `fetch()`), así que el navegador reutiliza la respuesta precargada en lugar de descargar el archivo dos veces.

En una app solo de cliente, el preload se añade durante el primer renderizado: una pequeña ventaja, ya que no hay HTML de servidor donde colocarlo.

### Obtener la URL

Con Vite, importa el binario con el sufijo `?url`. Vite copia el archivo a `dist/assets`, añade un hash de contenido a su nombre y te da la URL final, la misma en el build del cliente y en el del servidor:

```ts
import url from "./pkg/hello_bg.wasm?url";
```

Con webpack 5 o Rspack, el patrón `new URL()` emite el archivo y te da su URL (comprueba que el build de tu servidor produce la misma URL pública):

```ts
const url = new URL("./pkg/hello_bg.wasm", import.meta.url).href;
```

Sin `url`, todo sigue funcionando; el glue de wasm-bindgen encuentra el binario junto a él, pero no hay preload.

## Manejo de errores

Si el módulo no se carga (error de red, 404, binario no válido), `useWasm()` lanza el error durante el renderizado, así que llega al **error boundary** más cercano. Volver a llamar a `mod.load()` reintenta la carga. Hazlo cuando el boundary se reinicie:

```tsx
import { ErrorBoundary } from "react-error-boundary";
import { hello } from "./wasm";

<ErrorBoundary
  onReset={() => void hello.load().catch(() => {})}
  fallbackRender={({ resetErrorBoundary }) => (
    <p>
      The Rust module could not load. <button onClick={resetErrorBoundary}>Try again</button>
    </p>
  )}
>
  <Stats data={data} />
</ErrorBoundary>;
```

`react-error-boundary` es opcional; cualquier error boundary sirve. Lo importante es llamar a `load()` antes de que los hijos se vuelvan a renderizar.

## Suspense como alternativa

¿Prefieres Suspense a comprobar `undefined`? `use()` de React funciona con `mod.load()`, que siempre devuelve la misma promesa:

```tsx
"use client";
import { Suspense, use } from "react";
import { hello } from "./wasm";

function Stats({ data }: { data: Float64Array }) {
  const rust = use(hello.load()); // suspends until loaded
  return <p>Mean: {rust.mean(data)}</p>;
}

export function StatsPanel({ data }: { data: Float64Array }) {
  return (
    <Suspense fallback={<p>Loading…</p>}>
      <Stats data={data} />
    </Suspense>
  );
}
```

> **Advertencia:** Usa este patrón solo en componentes que se renderizan en el navegador. Durante el renderizado en servidor, `load()` intentaría descargar el binario en el servidor. `useWasm()` es la opción adecuada para los componentes renderizados en el servidor.

## Fuera de React

Un módulo declarado es un objeto simple, utilizable en cualquier parte:

```ts
import { hello } from "./wasm";

// Load on demand, for example in an event handler.
button.addEventListener("click", async () => {
  const rust = await hello.load();
  output.textContent = rust.greet("you");
});

// Read synchronously: the exports, or undefined if not loaded yet.
const maybeRust = hello.get();
```

`load()` y `useWasm()` comparten el mismo estado: cargar en un sitio hace que el módulo esté disponible en todas partes.

## Cargar pronto

Por defecto, la carga empieza con el primer componente que necesita el módulo. Para iniciarla en cuanto se ejecute el JavaScript de tu app, llama a `load()` a nivel de módulo, solo en el navegador:

```ts
export const hello = wasm(() => import("./pkg/hello.js"), { url });

if (typeof window !== "undefined") {
  hello.load().catch(() => {}); // a failure still reaches useWasm() and your error boundary
}
```

## Targets de wasm-bindgen

- **`--target web`** (recomendado): el glue exporta una función asíncrona `init` como exportación por defecto. active-react la llama por ti, con `{ module_or_path: url }` cuando proporcionaste una `url`.
- **`--target bundler`**: el bundler instancia el binario por sí mismo (Vite necesita un plugin para ello). El módulo se usa tal cual, sin `init`, y no hay preload.

Para un archivo `.wasm` sin ningún glue, pasa su URL en lugar de una función: consulta [WebAssembly puro](../raw-wasm/).

## Tests

En los tests unitarios, declara un módulo falso con funciones simples. Un loader que devuelve un objeto sin exportación por defecto `init` se usa tal cual:

```ts
import { wasm } from "active-react";

export const fakeHello = wasm(async () => ({
  greet: (name: string) => `Hello ${name}`,
  mean: (values: Float64Array) => values.reduce((a, b) => a + b, 0) / values.length,
}));
```

## El objeto Wasm

`wasm()` devuelve un `Wasm<T>`, donde `T` es el tipo de las exportaciones:

```ts
interface Wasm<T> {
  /** URL of the .wasm binary when known; preloaded by useWasm. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, undefined before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies listener when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}
```

`subscribe` y `get` son lo que `useWasm()` pasa a `useSyncExternalStore` de React. Puedes usarlos para conectar un módulo a otra biblioteca de estado.
