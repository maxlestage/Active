---
title: Introduction
description: active-react brings SEO tags and Rust/WebAssembly to React 19 in about 1.3 kB gzip, with zero dependencies and support for SSR, SSG and RSC.
---

active-react is a tiny library for React 19 that does two jobs well: it renders the SEO tags every page needs, and it lets your components call Rust compiled to WebAssembly (WASM).

The whole library weighs about **1.3 kB gzip**. It has **zero dependencies** (React is a peer dependency), runs **zero effects**, and works with server rendering (SSR), static site generation (SSG) and React Server Components (RSC).

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="The Acme website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

## What you get

- **`<Seo>` and `createSeo()`**: title, description, canonical URL, Open Graph, Twitter cards, robots, hreflang alternates and JSON-LD structured data, from one component.
- **`wasm()` and `useWasm()`**: declare a Rust module once, load it once, use it from any component. The binary is preloaded during server rendering.
- **`renderWithHead()`**: a server helper for HTML templates (Vite SSR, static generation) that returns the head tags and the body separately.
- **The `active` crate**: the Rust twin of `<Seo>`, producing the same tags from Rust servers, static generators or WebAssembly.

## How it works

### SEO: React does the hoisting

Since React 19, `<title>`, `<meta>` and `<link>` elements rendered anywhere in your component tree are moved ("hoisted") into the document `<head>` by React itself, both on the server and in the browser.

That leaves very little for an SEO library to do. `<Seo>` simply renders the right tags. There is no provider, no context, no store, no effect and no second render pass. When the user navigates, React removes the old page's tags and inserts the new ones, so the `<head>` always matches the page on screen.

### WASM: declare, load once, preload

- `wasm()` **declares** a module. Nothing is downloaded yet.
- `useWasm()` **loads** it the first time a component needs it, with a single shared request, however many components use it. It returns the module's exports, or `undefined` while loading.
- When the binary's URL is known, `useWasm()` asks React to **preload** it. During SSR this puts a `<link rel="preload">` in the `<head>`, so the browser downloads the `.wasm` file in parallel with your JavaScript instead of after it.

## Key numbers

From [the benchmarks](../benchmarks/) (same page for every library, production builds):

| Measure | active-react | For comparison |
| --- | ---: | --- |
| SEO code in the client bundle | 0.74 kB gzip | react-helmet 6.76 kB, react-helmet-async 6.29 kB, @unhead/react 6.07 kB |
| SSR time per page | 42.4 µs | 39.7 µs with hand-written tags, 66.5 µs with react-helmet |
| Head update after a route change | 0.3 ms median | 16.7 ms with react-helmet |
| WASM loader in the client bundle | 0.60 kB gzip | react-wasm 2.07 kB |
| First Rust result on screen, 100 ms RTT | 224 ms with SSR preload | 330 ms without preload |

In short: SEO costs a few percent more than writing the tags by hand, server rendering is 1.5× faster than with react-helmet, and the automatic preload cuts the time to the first Rust result by 32%.

## When to use it

active-react is a good fit when:

- you build with **React 19** and want correct, complete SEO tags with very little code: Vite (SPA, SSR or SSG), React Router v7, TanStack Start, Waku or your own server;
- you want to move heavy work (parsing, maths, image or data processing) to **Rust** without writing loading boilerplate;
- **bundle size** matters to you.

## When not to use it

- **React 18 or older** is not supported. Hoisting tags into `<head>` is a React 19 feature, and the library is built on it.
- **Next.js App Router** has its own Metadata API (`metadata` and `generateMetadata`), integrated with its routing and streaming. You may prefer it for SEO. `wasm()` and `useWasm()` still work in client components.
- **Tiny computations** don't need Rust: every call into WebAssembly has a small fixed cost. See the [performance guide](../performance/).

## Map of the docs

**Getting started**

- [Installation](../installation/): requirements, install commands and entry points.
- [Quick start](../quick-start/): a Vite app with SEO and Rust in five minutes.

**SEO**

- [The `<Seo>` component](../seo-component/): every prop and the HTML it produces.
- [Site defaults](../site-defaults/): `createSeo`, title templates, URLs, robots and hreflang.
- [Structured data](../structured-data/): JSON-LD recipes for rich results.
- [Server rendering](../server-rendering/): SSR, static generation and `renderWithHead`.

**Rust and WebAssembly**

- [Rust setup](../rust-setup/): toolchain, Cargo profile and first build.
- [Loading WebAssembly](../loading-wasm/): `wasm()`, `useWasm()`, preload, errors and Suspense.
- [Raw WebAssembly](../raw-wasm/): tiny modules without wasm-bindgen.
- [Performance guide](../performance/): what the benchmarks teach.
- [The Rust crate](../rust-crate/): the same SEO tags, generated from Rust.

**Shipping**

- [Deploy to Heroku](../deploy-heroku/): from your phone, no computer needed.
- [Publishing](../publishing/): release to npm and crates.io from a GitHub Release.

**Reference**

- [API reference](../api/), [Benchmarks](../benchmarks/) and [FAQ](../faq/).
