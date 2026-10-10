---
title: Renderizado en servidor y generación estática
description: Incluye tus etiquetas SEO y el preload del WASM en el propio HTML con el SSR de React 19, una plantilla HTML con renderWithHead o la generación estática.
---

`<Seo>` funciona solo en el navegador, pero el SEO da lo mejor de sí cuando las etiquetas ya están en el HTML que envía el servidor. Esta página muestra las dos formas de conseguirlo y cómo encaja la generación estática.

## Por qué importa

- **Los rastreadores ven las etiquetas de inmediato.** Google puede ejecutar JavaScript, pero renderiza las páginas en una segunda pasada que puede llegar más tarde. Otros buscadores ejecutan menos JavaScript, o ninguno.
- **Las vistas previas de enlaces necesitan HTML.** Los bots que generan las vistas previas para Facebook, LinkedIn, X, Slack, Discord o las aplicaciones de mensajería no ejecutan JavaScript en absoluto. Sin renderizado en servidor, solo ven tu `index.html`.
- **Rust arranca antes.** Cuando el servidor renderiza un componente que llama a `useWasm()`, también emite un preload para el binario `.wasm`. En los benchmarks, el primer resultado de Rust apareció a los 224 ms en lugar de a los 330 ms (100 ms de latencia de red).

active-react admite las dos configuraciones: React renderiza el documento completo, o React renderiza dentro de una plantilla HTML.

## Modo 1: React renderiza el documento completo

Si tu árbol de React renderiza `<html>`, no hay nada que hacer: el renderizador de servidor de React coloca por ti las etiquetas trasladadas en el `<head>`. Es el caso de React Router v7 (modo framework), TanStack Start, Waku y la mayoría de los servidores personalizados.

```tsx
import { renderToPipeableStream } from "react-dom/server";
import { App } from "./App";

function Document({ url }: { url: string }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
      </head>
      <body>
        <App url={url} />
      </body>
    </html>
  );
}

// In your request handler (Express, Fastify, node:http…)
const { pipe } = renderToPipeableStream(<Document url={req.url} />, {
  bootstrapModules: ["/assets/client.js"],
  onShellReady() {
    res.setHeader("content-type", "text/html; charset=utf-8");
    pipe(res);
  },
});
```

`renderToString(<Document url={url} />)` funciona de la misma manera si no necesitas streaming.

> **Consejo:** Renderiza `<Seo>` en el shell, fuera de los límites de Suspense que esperan datos, y carga los datos de la página antes de renderizar (los loaders de rutas hacen esto). Así, las etiquetas llegan en los primeros bytes del `<head>`.

## Modo 2: plantilla HTML

La configuración SSR de Vite y muchos generadores estáticos mantienen una plantilla `index.html` y dejan que React renderice solo la app dentro de ella. La plantilla tiene dos marcadores de posición:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <!--app-head-->
  </head>
  <body>
    <div id="root"><!--app-html--></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Aquí React nunca ve el `<head>`, así que no puede colocar las etiquetas en él. `renderWithHead` de `active-react/server` resuelve esto: renderiza tu app y devuelve por separado las etiquetas trasladadas y el marcado del body.

```tsx
// src/entry-server.tsx
import { renderWithHead } from "active-react/server";
import { App } from "./App";

export function render(url: string) {
  return renderWithHead(<App url={url} />); // { head, html }
}
```

`head` contiene las etiquetas `<title>`, `<meta>` y `<link>`, incluido el preload del `.wasm`. `html` es el marcado de la app, incluido el script JSON-LD.

### Un servidor SSR con Vite

Un servidor Express mínimo, adaptado de la plantilla SSR de Vite:

```js
// server.js
import fs from "node:fs/promises";
import express from "express";

const prod = process.env.NODE_ENV === "production";
const app = express();
let vite;

if (prod) {
  app.use(express.static("dist/client", { index: false }));
} else {
  const { createServer } = await import("vite");
  vite = await createServer({ server: { middlewareMode: true }, appType: "custom" });
  app.use(vite.middlewares);
}

app.use(async (req, res, next) => {
  try {
    const url = req.originalUrl;
    let template, render;
    if (prod) {
      template = await fs.readFile("dist/client/index.html", "utf8");
      ({ render } = await import("./dist/server/entry-server.js"));
    } else {
      template = await vite.transformIndexHtml(url, await fs.readFile("index.html", "utf8"));
      ({ render } = await vite.ssrLoadModule("/src/entry-server.tsx"));
    }
    const { head, html } = render(url);
    const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
    res.status(200).set({ "Content-Type": "text/html" }).send(page);
  } catch (error) {
    vite?.ssrFixStacktrace(error);
    next(error);
  }
});

app.listen(process.env.PORT ?? 5173);
```

Compila ambos lados con `vite build --outDir dist/client` y `vite build --ssr src/entry-server.tsx --outDir dist/server`.

En el cliente, hidrata el marcado del servidor:

```tsx
// src/main.tsx
import { hydrateRoot } from "react-dom/client";
import { App } from "./App";

hydrateRoot(document.getElementById("root")!, <App url={location.pathname} />);
```

> **Nota:** `renderWithHead` usa `renderToString`, que es síncrono: los límites de Suspense que todavía están esperando renderizan su fallback. Carga los datos antes de llamarlo, o usa el modo 1 con streaming.

## Generación estática

La generación estática de sitios (SSG) ejecuta la misma función `render` en tiempo de build, una vez por ruta, y escribe un archivo HTML por página. No se necesita ningún servidor en tiempo de ejecución: cualquier hosting estático sirve.

Este es el script `prerender.js` del [ejemplo vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg), donde `entry-server.tsx` también exporta la lista de rutas:

```js
// prerender.js — run after the client and SSR builds
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { render, urls } from "./dist-server/entry-server.js";

const template = readFileSync("dist/index.html", "utf8");
for (const url of urls) {
  const { head, html } = render(url);
  const file = `dist${url}index.html`;
  mkdirSync(file.slice(0, file.lastIndexOf("/")), { recursive: true });
  writeFileSync(file, template.replace("<!--app-head-->", head).replace("<!--app-html-->", html));
  console.log("prerendered", file);
}
rmSync("dist-server", { recursive: true });
```

```json
{
  "scripts": {
    "build": "vite build && vite build --ssr src/entry-server.tsx --outDir dist-server && node prerender.js"
  }
}
```

Cada página generada contiene sus propias etiquetas SEO y el preload del `.wasm`, y después se hidrata y llama a Rust.

### Añadir un sitemap

Ya tienes la lista de rutas, así que genera `sitemap.xml` y `robots.txt` en el mismo script:

```js
const origin = "https://acme.com";
const entries = urls.map((url) => `  <url><loc>${origin}${url}</loc></url>`).join("\n");
writeFileSync(
  "dist/sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${entries}\n</urlset>\n`,
);
writeFileSync("dist/robots.txt", `User-agent: *\nAllow: /\nSitemap: ${origin}/sitemap.xml\n`);
```

Después envía la URL del sitemap en Google Search Console.

## Server Components

`<Seo>` no usa ningún hook, así que se puede renderizar directamente en un React Server Component, por ejemplo con datos obtenidos en el servidor:

```tsx
// A Server Component
import { Seo } from "../seo";
import { db } from "../db";

export default async function ProductPage({ slug }: { slug: string }) {
  const product = await db.products.find(slug);
  return (
    <main>
      <Seo title={product.name} description={product.summary} canonical={`/products/${slug}`} />
      <h1>{product.name}</h1>
    </main>
  );
}
```

`useWasm()` es un hook que se ejecuta en el navegador, así que los componentes que llaman a Rust deben ser componentes cliente (`"use client"`). La declaración del módulo con `wasm()` puede estar en cualquier archivo compartido.

## Single-page apps

Sin servidor, las etiquetas las añade JavaScript después de que se cargue la página. En la mayoría de los casos eso le basta a Google, y el `<head>` se mantiene sincronizado en cada navegación. Pero las vistas previas de enlaces y los rastreadores que no ejecutan JavaScript solo verán `index.html`.

Si no puedes ejecutar un servidor, la generación estática suele ser la mejora más sencilla: prerrenderiza al menos las páginas que más se comparten.

## Hidratación y WebAssembly

`useWasm()` siempre devuelve `undefined` en el servidor **y durante la hidratación**, aunque el módulo ya esté cargado en el navegador. Por eso, el primer renderizado en el cliente produce exactamente el marcado del servidor y React nunca informa de una discrepancia de hidratación. Justo después de la hidratación, el componente se vuelve a renderizar con las exportaciones.

Escribe tus componentes como si Rust todavía pudiera no estar disponible:

```tsx
function Primes() {
  const rust = useWasm(hello);
  return <p>{rust ? `${rust.count_primes(1_000_000)} primes` : "Counting…"}</p>;
}
```

> **Advertencia:** No uses una condición sobre `typeof window` para llamar a Rust durante el renderizado. El servidor y el primer renderizado en el cliente no coincidirían, y React tendría que descartar el marcado del servidor.
