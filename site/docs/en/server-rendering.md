---
title: Server rendering and static generation
description: Put your SEO tags and the WASM preload in the HTML itself with React 19 SSR, an HTML template with renderWithHead, or static generation.
---

`<Seo>` works in the browser alone, but SEO is at its best when the tags are already in the HTML the server sends. This page shows the two ways to do that and how static generation fits in.

## Why it matters

- **Crawlers see the tags immediately.** Google can run JavaScript, but it renders pages in a second pass that may come later. Other search engines run less JavaScript, or none.
- **Link previews need HTML.** The bots that build previews for Facebook, LinkedIn, X, Slack, Discord or messaging apps don't run JavaScript at all. Without server rendering, they only see your `index.html`.
- **Rust starts sooner.** When the server renders a component that calls `useWasm()`, it also emits a preload for the `.wasm` binary. In the benchmarks, the first Rust result appeared after 224 ms instead of 330 ms (100 ms network latency).

active-react supports both setups: React renders the whole document, or React renders inside an HTML template.

## Mode 1: React renders the whole document

If your React tree renders `<html>`, there is nothing to do: React's server renderer puts the hoisted tags in the `<head>` for you. That is the case for React Router v7 (framework mode), TanStack Start, Waku and most custom servers.

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

`renderToString(<Document url={url} />)` works the same way if you don't need streaming.

> **Tip:** Render `<Seo>` in the shell, outside Suspense boundaries that wait for data, and load the page data before rendering (route loaders do this). The tags then land in the very first bytes of the `<head>`.

## Mode 2: HTML template

Vite's SSR setup and many static generators keep an `index.html` template and let React render only the app inside it. The template has two placeholders:

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

Here React never sees the `<head>`, so it can't put the tags there. `renderWithHead` from `active-react/server` solves this: it renders your app and returns the hoisted tags and the body markup separately.

```tsx
// src/entry-server.tsx
import { renderWithHead } from "active-react/server";
import { App } from "./App";

export function render(url: string) {
  return renderWithHead(<App url={url} />); // { head, html }
}
```

`head` contains the `<title>`, `<meta>` and `<link>` tags, including the `.wasm` preload. `html` is the app markup, including the JSON-LD script.

### A Vite SSR server

A minimal Express server, adapted from Vite's SSR template:

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

Build both sides with `vite build --outDir dist/client` and `vite build --ssr src/entry-server.tsx --outDir dist/server`.

On the client, hydrate the server markup:

```tsx
// src/main.tsx
import { hydrateRoot } from "react-dom/client";
import { App } from "./App";

hydrateRoot(document.getElementById("root")!, <App url={location.pathname} />);
```

> **Note:** `renderWithHead` uses `renderToString`, which is synchronous: Suspense boundaries that are still waiting render their fallback. Load data before calling it, or use mode 1 with streaming.

## Static generation

Static site generation (SSG) runs the same `render` function at build time, once per route, and writes one HTML file per page. No server is needed at runtime: any static host works.

This is the `prerender.js` script of the [vite-ssg example](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg), where `entry-server.tsx` also exports the list of routes:

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

Each generated page contains its own SEO tags and the `.wasm` preload, then hydrates and calls Rust.

### Add a sitemap

You already have the list of routes, so generate `sitemap.xml` and `robots.txt` in the same script:

```js
const origin = "https://acme.com";
const entries = urls.map((url) => `  <url><loc>${origin}${url}</loc></url>`).join("\n");
writeFileSync(
  "dist/sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${entries}\n</urlset>\n`,
);
writeFileSync("dist/robots.txt", `User-agent: *\nAllow: /\nSitemap: ${origin}/sitemap.xml\n`);
```

Then submit the sitemap URL in Google Search Console.

## Server Components

`<Seo>` uses no hook, so it can be rendered directly in a React Server Component, for example with data fetched on the server:

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

`useWasm()` is a hook that runs in the browser, so components that call Rust must be client components (`"use client"`). Declaring the module with `wasm()` can live in any shared file.

## Single-page apps

Without a server, the tags are added by JavaScript after the page loads. That is fine for Google in most cases, and the `<head>` stays in sync on every navigation. But link previews and crawlers that don't run JavaScript will only see `index.html`.

If you can't run a server, static generation is usually the easiest upgrade: prerender at least the pages people share most.

## Hydration and WebAssembly

`useWasm()` always returns `undefined` on the server **and during hydration**, even if the module is already loaded in the browser. The first client render therefore produces exactly the server's markup, and React never reports a hydration mismatch. Right after hydration, the component re-renders with the exports.

Write components as if Rust might not be there yet:

```tsx
function Primes() {
  const rust = useWasm(hello);
  return <p>{rust ? `${rust.count_primes(1_000_000)} primes` : "Counting…"}</p>;
}
```

> **Warning:** Don't branch on `typeof window` to call Rust during render. The server and the first client render would disagree, and React would have to throw away the server markup.
