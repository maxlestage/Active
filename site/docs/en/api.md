---
title: API reference
description: Every export of active-react with its TypeScript signature, props, defaults and behaviour, plus a summary of the active Rust crate.
---

active-react has two entry points:

| Entry point | Exports |
| --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` |
| `active-react/server` | `renderWithHead` |

## Seo

```ts
function Seo(props: SeoProps): JSX.Element;
```

Renders the SEO tags of a page: `<title>`, meta description, robots, canonical, hreflang alternates, Open Graph, Twitter and JSON-LD. React 19 hoists the `<title>`, `<meta>` and `<link>` tags into the `<head>`, on the server and in the browser. The JSON-LD `<script>` stays where the component is rendered.

Uses no hook, no context and no effect, so it can render in Server Components. Render one `<Seo>` per page. Guide: [the `<Seo>` component](../seo-component/).

## SeoProps

```ts
interface SeoProps {
  title?: string;
  titleTemplate?: string;
  description?: string;
  canonical?: string;
  image?: string;
  type?: string;
  siteName?: string;
  locale?: string;
  twitter?: string;
  baseUrl?: string;
  noindex?: boolean;
  nofollow?: boolean;
  alternates?: Record<string, string>;
  jsonLd?: object | object[];
  meta?: Array<{ name?: string; property?: string; content: string }>;
}
```

| Prop | Default | Description |
| --- | --- | --- |
| `title` | none | Page title. `titleTemplate` is applied to it for `<title>`; used as is for `og:title`. Falls back to `siteName` |
| `titleTemplate` | none | Template where the first `%s` is replaced by `title`, for example `"%s · Acme"`. Unused without `title` |
| `description` | none | `<meta name="description">` and `og:description` |
| `canonical` | none | `<link rel="canonical">` and `og:url`. Relative paths are resolved with `baseUrl` |
| `image` | none | `og:image`. Relative paths are resolved with `baseUrl`. Switches `twitter:card` to `summary_large_image` |
| `type` | `"website"` | `og:type` |
| `siteName` | none | `og:site_name`, and the title when `title` is missing |
| `locale` | none | `og:locale`, for example `"en_US"` |
| `twitter` | none | `twitter:site`, for example `"@acme"` |
| `baseUrl` | none | Origin (or prefix) for relative `canonical`, `image` and `alternates` URLs, for example `"https://acme.com"` |
| `noindex` | `false` | Adds `noindex` to `<meta name="robots">` |
| `nofollow` | `false` | Adds `nofollow` to `<meta name="robots">` |
| `alternates` | none | hreflang alternates: `{ en: "/en", fr: "/fr", "x-default": "/en" }` |
| `jsonLd` | none | JSON-LD structured data, one object or an array, serialized with `<` escaped |
| `meta` | none | Extra meta tags: `[{ name: "theme-color", content: "#000" }]` |

Always rendered: `og:type` and `twitter:card` (`summary`, or `summary_large_image` with an image). `<meta name="robots">` only appears when `noindex` or `nofollow` is true.

URL resolution: URLs with a scheme (`https:`, `mailto:`…) or starting with `//` are kept; otherwise `baseUrl`, minus one trailing slash, is prepended, with a `/` in between when the path doesn't start with one.

## createSeo

```ts
function createSeo(defaults: SeoProps): (props: SeoProps) => ReactNode;
```

Returns a `<Seo>` component with site-wide defaults. Page props are merged over the defaults with a shallow spread (`{ ...defaults, ...props }`): object and array props replace the default, and an explicit `undefined` removes it.

```ts
export const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
```

Guide: [site defaults](../site-defaults/).

## wasm

```ts
function wasm<T>(source: () => Promise<T>, options?: { url?: string }): Wasm<T>;

function wasm<T = WebAssembly.Exports>(
  source: string | URL,
  options?: { imports?: WebAssembly.Imports },
): Wasm<T>;
```

Declares a WebAssembly module. **Nothing is fetched** until `load()` or `useWasm()`.

- **Loader overload**, for wasm-bindgen or wasm-pack output: `wasm(() => import("./pkg/my_crate.js"), { url })`. If the module has a default export function (`--target web`), it is called once with `{ module_or_path: url }`, or with no argument when `url` is missing. Otherwise (`--target bundler`, or a plain object) the module is used as is. `url` is the public URL of the `.wasm` binary and enables preloading.
- **URL overload**, for a raw module without glue: `wasm<Exports>("/my_crate.wasm", { imports })`. The file is fetched and compiled with `WebAssembly.instantiateStreaming` when the response has the `application/wasm` content type, or with `WebAssembly.instantiate` on its bytes otherwise. A non-2xx response rejects with `wasm: <url> responded <status>`. Resolves with the instance's exports.

Guides: [loading WebAssembly](../loading-wasm/), [raw WebAssembly](../raw-wasm/).

## Wasm

```ts
interface Wasm<T> {
  /** URL of the `.wasm` binary when known; preloaded from `<head>` by `useWasm`. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, `undefined` before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies `listener` when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}
```

| Member | Behaviour |
| --- | --- |
| `url` | The loader overload's `url` option, or the URL passed to the URL overload |
| `load()` | Starts loading on the first call. Later calls return the same promise. After a failure, the next call starts a new attempt |
| `get()` | Synchronous read: the exports, `undefined` while not loaded, or throws the loading error |
| `subscribe(listener)` | Calls `listener` when loading succeeds or fails, and starts loading if needed. Returns an unsubscribe function |

## useWasm

```ts
function useWasm<T>(mod: Wasm<T>): T | undefined;
```

Returns the module's exports, or `undefined` while loading. Built on `useSyncExternalStore`:

- the first component to use a module starts loading it; all components share the same load;
- it always returns `undefined` during server rendering and hydration, so server and client markup match;
- when loading fails, the error is thrown during render, to the nearest error boundary; call `mod.load()` to retry;
- while the module is not loaded and `mod.url` is known, it calls React DOM's `preload(url, { as: "fetch", crossOrigin: "anonymous", type: "application/wasm" })`. During server rendering this emits `<link rel="preload" as="fetch" crossorigin type="application/wasm">` in the `<head>`.

For Suspense, use React's `use(mod.load())` in a component that only renders in the browser.

## renderWithHead

```ts
import { renderWithHead } from "active-react/server";

function renderWithHead(node: ReactNode): { head: string; html: string };
```

For HTML templates where React doesn't render `<html>` itself, such as Vite SSR with `<!--app-head-->` and `<!--app-html-->` placeholders, or static generation. Renders `node` with `renderToString` and splits the result:

- `head`: the tags React hoisted (title, meta, link, preloads), to insert in the template's `<head>`;
- `html`: the rest of the markup, to insert in the app container.

```tsx
const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

Synchronous: Suspense boundaries that are still pending render their fallback. If your tree renders `<html>`, use `renderToString` or `renderToPipeableStream` from `react-dom/server` directly. Guide: [server rendering](../server-rendering/).

## Rust crate

The `active` crate ([crates.io](https://crates.io/crates/active)) generates the same tags from Rust:

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head: String = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

| Item | Description |
| --- | --- |
| `Seo::new()` | Empty builder; `Seo` also implements `Default`, `Clone`, `Debug`, `PartialEq`, `Eq` and `Hash` |
| `title`, `title_template`, `description`, `canonical`, `image`, `og_type`, `site_name`, `locale`, `twitter`, `base_url` | Setters taking `impl Into<String>` |
| `noindex(bool)`, `nofollow(bool)` | Robots flags |
| `alternate(lang, url)` | Adds or replaces an hreflang alternate |
| `json_ld(json)` | Adds a JSON-LD script from a serialized JSON string |
| `meta_name(name, content)`, `meta_property(property, content)` | Extra meta tags |
| `to_html(&self) -> String` | Renders the tags |
| `write_html(&self, out: &mut String)` | Appends the tags to `out` |
| `impl Display` | `format!("{seo}")` renders the tags |

Guide: [the Rust crate](../rust-crate/). Full documentation: [docs.rs/active](https://docs.rs/active).
