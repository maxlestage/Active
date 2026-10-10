---
title: Installation
description: Install active-react from GitHub today or from npm once published. TypeScript types are included and SEO-only apps never ship the WASM code.
---

active-react is a single package with two entry points. Installing it takes one command; the Rust toolchain is only needed if you use the WebAssembly part.

## Requirements

| Tool | Version | Needed for |
| --- | --- | --- |
| `react` and `react-dom` | 19 or newer | Everything (peer dependencies) |
| Node.js | 20 or newer | Server rendering and static generation |
| Rust | Stable, with the `wasm32-unknown-unknown` target | Only the WebAssembly part |
| TypeScript | Optional | Types are bundled |

Why React 19? Since version 19, React moves `<title>`, `<meta>` and `<link>` into the `<head>` by itself. active-react is built on that feature, so React 18 is not supported.

Check your versions:

```sh
npm ls react react-dom
```

If you are still on React 18, upgrade first:

```sh
npm i react@latest react-dom@latest
npm i -D @types/react@latest @types/react-dom@latest
```

## Install

### From GitHub (today)

The package is not on npm yet. Install it straight from the repository:

```sh
npm i github:maxlestage/Active
```

npm clones the repository and runs its `prepare` script, which compiles the library to `dist/`. The package is still called `active-react`, so your imports are the same as with the future npm release. Your `package.json` gets this line:

```json
{
  "dependencies": {
    "active-react": "github:maxlestage/Active"
  }
}
```

> **Tip:** Add `#` and a commit hash (or a tag) to pin an exact version: `npm i github:maxlestage/Active#<commit-hash>`.

### From npm (once published)

```sh
npm i active-react
```

The same with other package managers:

```sh
pnpm add active-react
yarn add active-react
bun add active-react
```

> **Note:** pnpm, Yarn and Bun also accept `github:maxlestage/Active`. Some of them block the install scripts of dependencies by default; if `node_modules/active-react/dist` is missing after the install, allow the package's build script or use npm.

## Entry points

| Import from | Exports | Use it in |
| --- | --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` | Your components, rendered on the server or in the browser |
| `active-react/server` | `renderWithHead` | Server code only |

```tsx
import { Seo, createSeo, wasm, useWasm, type SeoProps, type Wasm } from "active-react";
import { renderWithHead } from "active-react/server";
```

> **Warning:** Never import `active-react/server` from browser code. It pulls in `react-dom/server`, which you don't want in your client bundle.

## TypeScript

Types ship with the package: there is no `@types/active-react` to install. Hover any prop in your editor to read its documentation.

The package uses the `exports` field of `package.json`, so TypeScript needs a modern module resolution to find `active-react/server`:

```json
{
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
```

`"bundler"` is the default in Vite templates. `"node16"` and `"nodenext"` work too. The legacy `"node"` setting cannot resolve the `/server` entry point.

The package is ESM only (`"type": "module"`): use `import`, not `require`.

> **Note:** Importing a `.wasm` file with Vite's `?url` suffix needs Vite's client types. Vite templates already include them (`vite/client`).

## Tree-shaking

The package declares `"sideEffects": false`, so your bundler keeps only what you import:

| You import | Added to your client bundle |
| --- | ---: |
| `Seo` and/or `createSeo` | 0.74 kB gzip |
| `wasm` and `useWasm` | 0.60 kB gzip |
| Everything | about 1.3 kB gzip |

An SEO-only app never ships the WebAssembly loader, and an app that only uses Rust never ships the SEO code. These sizes come from the [benchmarks](../benchmarks/): minified with esbuild, with React and React DOM excluded since your app already has them.

## Next steps

- Follow the [quick start](../quick-start/) to build a small app with SEO and Rust.
- Read about [the `<Seo>` component](../seo-component/).
- Set up the [Rust toolchain](../rust-setup/) if you plan to use WebAssembly.
