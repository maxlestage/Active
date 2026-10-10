# Benchmarks

Compares active-react with the alternatives on the same page, then rewrites [`RESULTS.md`](RESULTS.md).

```sh
npm run setup   # builds the library and installs the benchmark dependencies
npm run bench   # everything (≈ 5 min); `node run.mjs seo` or `node run.mjs wasm` for one part
```

Requirements: Node ≥ 22, Rust with the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` 0.2.129 (`cargo install wasm-bindgen-cli --version 0.2.129`) and Chromium for Playwright (`PLAYWRIGHT_BROWSERS_PATH` or `npx playwright install chromium`).

## What is measured

**SEO**: React 19 without a library (baseline), active-react, react-helmet, react-helmet-async, @unhead/react and react-head.

- `seo/shared.tsx` is the common page (11 tags + JSON-LD + 30 elements). `seo/libs/*` has the same page written with each library.
- **Size**: only the imports actually used, minified by esbuild, React excluded.
- **SSR** (`seo/ssr/*`, `seo/measure.ts`): a complete HTML document, one Node process per library, median of 3 runs.
- **Browser** (`seo/client.tsx`): headless Chromium, production builds. It measures page load, the latency of a route change and a burst of 1,000 navigations, then checks that the final `<head>` is correct, without duplicates.

**WASM**: the crates `wasm/rust/raw` (`extern "C"`) and `wasm/rust/bindgen` (wasm-bindgen).

- **Cargo profiles** (`opt-level` 3/s/z, with or without `wasm-opt`): size and speed.
- **Loading** (`wasm/pages/*`): active-react, react-wasm and a hand-written `useEffect`, on localhost and with 100 ms of emulated round-trip time. SSR is also compared with and without preload.
- **Call cost**: JS, raw wasm and wasm-bindgen (`add`, `fib(30)`, strings).

The results are explained in English, French and Spanish in the documentation: `site/docs/<lang>/benchmarks.md`.
