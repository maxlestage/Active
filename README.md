# active-react

**English** · [Français](README.fr.md) · [Español](README.es.md)

**SEO + Rust/WebAssembly for React 19, in ~1.3 kB gzip.** Zero dependencies, zero effects. Works with SSR, static generation and Server Components. A Rust twin, the [`active`](crates/active) crate, generates the same tags from Rust.

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="Acme's website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

📚 **Full documentation (English, French, Spanish):** the presentation site in [`site/`](site). Deploy your own copy with the [Deploy to Heroku](#deploy-the-site-on-heroku-from-a-phone) button, or read the pages directly in [`site/docs/`](site/docs).

## Why it is fast

The design comes from the benchmarks. Details and method are in [`bench/RESULTS.md`](bench/RESULTS.md).

- **SEO without a `<head>` manager.** Since React 19, `<title>`, `<meta>` and `<link>` rendered anywhere are hoisted into `<head>` by React itself, on the server and in the browser. `<Seo>` only renders the right tags: no context, no store, no effect, no second pass. The cost stays within a few percent of hand-written tags, and SSR is 1.5× faster than react-helmet.
- **WASM: one shared, preloaded fetch.** `wasm()` declares a module without downloading anything. `useWasm()` loads it once (`instantiateStreaming`), however many components use it. When the `.wasm` URL is known, server rendering emits `<link rel="preload">`, so the browser downloads the binary **in parallel** with the JavaScript. With 100 ms of network latency, the first Rust result arrives **32% sooner**.

| SEO (same page: 11 tags + JSON-LD) | Client JS (gzip) | SSR (µs/page) | Navigation latency (median) |
| --- | ---: | ---: | ---: |
| React 19, no library (baseline) | 0 | 39.7 | 0.3 ms |
| **active-react** | **0.74 kB** | **42.4** | **0.3 ms** |
| @unhead/react 3.4.2 | 6.07 kB | 44.9 | 0.4 ms |
| react-head 3.4.2 | 1.35 kB | 60.7 | 1.4 ms |
| react-helmet-async 3.0.0 | 6.29 kB | 64.9 | 0.3 ms |
| react-helmet 6.1.0 | 6.76 kB | 66.5 | 16.7 ms |

| WASM (wasm-bindgen crate) | JS (gzip) | Rust result on screen, 100 ms RTT |
| --- | ---: | ---: |
| **active-react**, SSR + automatic preload | **0.60 kB** | **224 ms** |
| active-react, SSR without preload | 0.60 kB | 330 ms |
| active-react, client rendering | 0.60 kB | 328 ms |
| react-wasm 1.0.1, client rendering (raw `.wasm`) | 2.07 kB | 327 ms |
| Hand-written `useEffect`, client rendering | 0 | 333 ms |

Intel Xeon 2.8 GHz, Node 22, Chromium 141, React 19.3. SSR numbers vary by about ±5% between runs.

## Install

```sh
npm i github:maxlestage/Active   # until the first npm release, then: npm i active-react
cargo add active                 # optional: the Rust crate, once published
```

Peer dependencies: `react >= 19`, `react-dom >= 19`.

## SEO

```tsx
// seo.ts — your site's defaults, written once
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com", // relative URLs (canonical, image, alternates) are resolved against it
  image: "/og.png",
  locale: "en_US",
  twitter: "@acme",
});
```

```tsx
// any page
<Seo
  title="Oak chair"
  description="Solid oak, delivered in 48 hours."
  canonical="/products/oak-chair"
  type="product"
  alternates={{ en: "/products/oak-chair", fr: "/fr/produits/chaise-chene" }}
  jsonLd={{ "@context": "https://schema.org", "@type": "Product", name: "Oak chair" }}
/>
```

This renders `<title>`, the description, canonical and hreflang links, robots, Open Graph and Twitter tags. JSON-LD is escaped against `</script>` injection. `<Seo>` uses no hook, so it also works in Server Components. Render one `<Seo>` per page.

**Server rendering.** If React renders `<html>` (React Router, TanStack Start, Waku, a custom server), there is nothing to do. For HTML templates (Vite SSR, static generation), use `active-react/server`:

```tsx
import { renderWithHead } from "active-react/server";

const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

## WebAssembly (Rust)

```sh
wasm-pack build --target web --out-dir ../src/pkg
```

```tsx
import { wasm, useWasm } from "active-react";
import url from "./pkg/my_crate_bg.wasm?url"; // Vite; optional, enables the SSR preload

export const myCrate = wasm(() => import("./pkg/my_crate.js"), { url });

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(myCrate);
  if (!rust) return <Spinner />;
  return <p>Mean: {rust.mean(data)}</p>; // typed from the generated .d.ts
}
```

- `useWasm(mod)` returns the exports, or `undefined` while loading. It always returns `undefined` during SSR and hydration, so the server and client HTML match.
- A loading error is thrown to the nearest error boundary, and `mod.load()` retries.
- For Suspense, call `use(mod.load())` in a client component. Outside React, `await mod.load()`.
- A raw `extern "C"` module works without any glue: `wasm<{ add(a: number, b: number): number }>("/math.wasm")`.

Recommended Cargo profile (validated by the benchmarks): `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.

## The Rust crate `active`

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

It produces the same tags as `<Seo>`, for Axum, Actix, static generators or WebAssembly. It has zero dependencies. See [`crates/active`](crates/active).

## Repository

| Folder | Contents |
| --- | --- |
| `src/`, `test/` | The `active-react` library and its tests (vitest) |
| `crates/active/` | The Rust crate `active` (crates.io) |
| `site/` | Presentation site + documentation (en/fr/es): Vite, React 19, active-react, Rust/WASM hero and playground |
| `bench/` | Benchmarks against react-helmet, react-helmet-async, @unhead/react, react-head and react-wasm |
| `examples/vite-ssg/` | Minimal Vite + Rust + static generation example |

## Deploy the site on Heroku from a phone

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/maxlestage/Active)

1. Open this README on your phone and tap **Deploy to Heroku**. Sign in, choose an app name, then tap **Deploy app**.
2. To redeploy on every change: in the Heroku dashboard, open **Deploy → GitHub**, connect the repository, then **Enable Automatic Deploys** from `master`.

Heroku has no free plan. An Eco dyno costs about $5 a month. The build uses only Node: the WASM module of the site is committed. Step-by-step guide: `site/docs/en/deploy-heroku.md`.

## Release to npm and crates.io from a phone

1. Add the repository secrets `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Open **Releases → Draft a new release**, create a tag such as `v0.1.0`, then tap **Publish release**.

The `release.yml` workflow takes the version from the tag, runs the tests and publishes the `active-react` package and the `active` crate. Guide: `site/docs/en/publishing.md`.

## Development

```sh
npm install && npm test && npm run build      # library
cd crates/active && cargo test                # Rust crate
cd site && npm install && npm run dev         # site (npm run build && npm start for production)
cd bench && npm run setup && npm run bench    # benchmarks (needs Rust, wasm-bindgen-cli, Chromium)
```

Every branch `claude/*` that passes CI is merged into `master` automatically (`.github/workflows/ci.yml`).

## License

[MIT](LICENSE)
