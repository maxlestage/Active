---
title: FAQ
description: Answers to common questions about active-react, from React 18 and Next.js support to wasm-bindgen errors, MIME types, browser support and licensing.
---

## Does it work with React 18?

No. active-react relies on a React 19 feature: `<title>`, `<meta>` and `<link>` rendered anywhere in the tree are hoisted into the `<head>` by React itself. That is what lets `<Seo>` be a plain component with no provider and no effect. On React 18 the tags would stay in the body. Upgrade with `npm i react@latest react-dom@latest`.

## Can I use it with Next.js?

With the **App Router**, Next.js has its own Metadata API (`export const metadata` and `generateMetadata`), integrated with its routing, streaming and file conventions. Prefer it for SEO, so the two systems don't produce duplicate tags.

The WebAssembly part can still be used in Next.js: declare the module with `wasm()` and call `useWasm()` in client components (files starting with `"use client"`). Get the binary's URL with `new URL("./pkg/my_crate_bg.wasm", import.meta.url).href`, since the `?url` suffix is specific to Vite.

## Does it work with other frameworks?

Yes. React Router v7 (framework mode), TanStack Start and Waku need nothing to configure. These frameworks render the whole document with React 19, so the tags `<Seo>` renders land in the `<head>` of the server HTML automatically. See [server rendering](../server-rendering/).

## Can a page have two `<Seo>`?

You get two sets of tags: two titles, two descriptions, two canonical links. React hoists everything it is given and doesn't merge tags. Render a single `<Seo>` per page, in the page component, and put the values shared by all pages in [`createSeo`](../site-defaults/) rather than in a second `<Seo>` in a layout.

## Why are there two `<title>` tags?

That is the static `<title>` from your `index.html`. React inserts the page's title **before** it, so `document.title`, the browser tab and search engines use the right one. You can delete the static title from `index.html` to keep a single element.

Do remove static `<meta name="description">` and `og:*` tags from `index.html`, though: those would be duplicated.

## Does Google read JSON-LD in the body?

Yes. Google reads JSON-LD scripts anywhere in the document, `<head>` or `<body>`. React 19 doesn't hoist regular scripts, so the `<script type="application/ld+json">` stays where `<Seo>` is rendered. Check your pages with the [Rich Results Test](https://search.google.com/test/rich-results). More in [structured data](../structured-data/).

## Can I use active-react without Rust?

Absolutely. The SEO part needs no Rust at all, and thanks to tree-shaking an SEO-only app only ships 0.74 kB gzip: the WebAssembly loader is never included if you don't import `wasm` or `useWasm`.

## wasm-bindgen version mismatch

The full error starts with:

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary
```

The `wasm-bindgen` command-line tool must be exactly the same version as the `wasm-bindgen` crate in your `Cargo.lock`. Either install the matching CLI:

```sh
cargo install -f wasm-bindgen-cli --version 0.2.129
```

or pin the crate to your CLI version in `Cargo.toml` with `wasm-bindgen = "=0.2.129"`. wasm-pack avoids the problem entirely by downloading the right CLI for you. See [Rust setup](../rust-setup/).

## My server doesn't send application/wasm

Your module still loads: when the `Content-Type` isn't `application/wasm`, active-react downloads the whole file and compiles it afterwards instead of compiling while downloading. It is only a bit slower. To fix the header:

```text
# nginx (inside http, server or location)
types { application/wasm wasm; }

# Apache (.htaccess or server config)
AddType application/wasm .wasm
```

Vite's dev and preview servers and Express's `express.static` already send the right type. With a hand-written Node server, set `content-type: application/wasm` for `.wasm` files.

## Can I run Rust on the server during SSR?

`useWasm()` deliberately returns `undefined` on the server and during hydration, so the server HTML and the first client render always match. The Rust result appears right after hydration.

If you need a Rust result inside the server HTML, compute it on the server before rendering and pass it as a prop. With wasm-bindgen's `--target web` output, Node can load the binary synchronously:

```ts
// server only
import { readFileSync } from "node:fs";
import { count_primes, initSync } from "./pkg/hello.js";

initSync({ module: readFileSync(new URL("./pkg/hello_bg.wasm", import.meta.url)) });
const primes = count_primes(1_000_000); // pass it to your page as a prop
```

On a Rust server, call your Rust code directly, and use the [`active` crate](../rust-crate/) for the SEO tags.

## Which browsers are supported?

All current browsers: Chrome, Edge, Firefox and Safari, on desktop and mobile. WebAssembly itself has been supported everywhere since 2017. Streaming compilation (`WebAssembly.instantiateStreaming`), used for raw `.wasm` modules served as `application/wasm`, needs Safari 15 or newer. Browsers that don't support `preload` links simply ignore them and load the module normally.

## How big is it, really?

About 1.3 kB gzip if you use everything: 0.74 kB for the SEO part and 0.60 kB for the WebAssembly loader, measured with esbuild, React excluded. With tree-shaking you only pay for what you import. The [benchmarks](../benchmarks/) compare it with the alternatives.

## What is the license?

MIT, for both the npm package and the Rust crate. You can use it in commercial and closed-source projects; keep the copyright notice.

## How can I contribute?

Issues and pull requests are welcome on [GitHub](https://github.com/maxlestage/Active). To work on the code:

```sh
git clone https://github.com/maxlestage/Active.git
cd Active
npm install
npm test            # vitest: SSR, client (jsdom) and wasm tests
npm run typecheck
npm run build       # dist/ (ESM + .d.ts)
```

The Rust crate lives in `crates/active` (`cargo test`), this website in `site/`, and the benchmarks in `bench/` (`npm run setup && npm run bench`). Please run the tests before opening a pull request, and include a benchmark run if your change could affect performance.
