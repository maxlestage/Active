---
title: Benchmarks
description: How active-react compares with react-helmet, react-head, @unhead/react and react-wasm on size, SSR speed, browser behaviour and WASM loading.
---

Every design choice in active-react was checked against measurements. This page gives the method, all the results, and how to reproduce them on your machine. For the practical takeaways, read the [performance guide](../performance/).

## Methodology

**Environment:** Intel(R) Xeon(R) Processor @ 2.80GHz (4 cores), Node v22.22.0, Chromium 141.0.7390.37, React 19.3.0.

**The same page for every library**, built in production mode: a `<title>` with a template, a description, a canonical URL, 6 Open Graph tags, 2 Twitter tags, a JSON-LD `Product`, and a body of 30 elements.

**SEO libraries compared:** React 19 with hand-written tags and no library (the baseline), active-react, react-helmet 6.1.0, react-helmet-async 3.0.0, @unhead/react 3.4.2 and react-head 3.4.2.

- **Size:** only the imports each page actually uses, bundled and minified with esbuild, with React and React DOM excluded. Compressed with gzip (level 9) and Brotli (quality 11).
- **Server rendering:** the complete HTML document (head and body). Each library runs in **its own Node process** so JIT state never leaks between them. After a warm-up, renders are timed in batches for about 2.5 seconds; the result is the median of 3 processes.
- **Browser:** headless Chromium, production builds, one fresh page per library. After the measurements, the final `<head>` is audited: no duplicates, correct values.

**WASM:** two Rust crates with the same functions, one with plain `extern "C"` exports (raw) and one with wasm-bindgen, built with each Cargo profile.

- **Loading:** time from navigation start to the first Rust result rendered on screen, on localhost and with **100 ms of round-trip latency emulated through the Chrome DevTools Protocol** (realistic for mobile networks). Median of 15 loads on localhost and 7 with latency.
- **Call cost:** 10 million `add` calls and 300,000 `greet` calls per variant, each variant in its own loop so call sites stay monomorphic; median of 7 runs after warm-up.

## SEO results

### Client bundle size

JavaScript added to the bundle for the imports actually used (minified, React and React DOM excluded).

| Library | min | gzip | brotli |
| --- | ---: | ---: | ---: |
| React 19 native (no library) | 0 | 0 | 0 |
| **active-react** | 1.53 kB | 0.74 kB | 0.62 kB |
| react-helmet-async 3.0.0 | 17.30 kB | 6.29 kB | 5.59 kB |
| @unhead/react 3.4.2 | 15.17 kB | 6.07 kB | 5.55 kB |
| react-head 3.4.2 | 3.20 kB | 1.35 kB | 1.21 kB |
| react-helmet 6.1.0 | 18.46 kB | 6.76 kB | 5.99 kB |

### Server rendering (Node)

Time to produce the complete HTML document (head and body), median of 3 processes of about 2.5 s each.

| Library | µs / page | pages / s | vs React native |
| --- | ---: | ---: | ---: |
| React 19 native (no library) | 39.7 | 25,199 | 0% |
| **active-react** | 42.4 | 23,565 | +7% |
| **active-react** (`renderWithHead`, HTML template) | 44.1 | 22,665 | +11% |
| @unhead/react 3.4.2 | 44.9 | 22,260 | +13% |
| react-head 3.4.2 | 60.7 | 16,474 | +53% |
| react-helmet-async 3.0.0 | 64.9 | 15,405 | +64% |
| react-helmet 6.1.0 | 66.5 | 15,031 | +68% |

### Browser (headless Chromium)

- **Head ready:** from navigation start until title, description and canonical are correct (median of 15 loads).
- **Latency:** between rendering a new page (route change) and an up-to-date `<head>` (300 navigations).
- **Burst:** 1,000 route changes in a row, until the final `<head>` is correct.

| Library | Head ready | Median latency | p95 latency | Burst (nav/s) | Final `<head>` |
| --- | ---: | ---: | ---: | ---: | ---: |
| React 19 native (no library) | 20.9 ms | 0.3 ms | 0.5 ms | 2,267 | Correct |
| **active-react** | 23.8 ms | 0.3 ms | 0.6 ms | 2,144 | Correct |
| react-helmet-async 3.0.0 | 22.7 ms | 0.3 ms | 0.5 ms | 2,402 | Correct |
| @unhead/react 3.4.2 | 24.2 ms | 0.4 ms | 0.7 ms | 4,753 | Correct |
| react-head 3.4.2 | 23.5 ms | 1.4 ms | 5.7 ms | 1,682 | Correct |
| react-helmet 6.1.0 | 31.1 ms | 16.7 ms | 18.6 ms | 3,362 | Correct |

> **Note:** The approaches based on React 19's native hoisting (native, active-react, react-helmet-async 3, react-head) leave the static `<title>` of `index.html` in place. React inserts its own title **before** it, so `document.title` is correct. Remove the static `<title>` from `index.html` if you want a single element.

## WASM results

### Cargo profiles

Same crate, with `lto = true`, `codegen-units = 1`, `panic = "abort"` and `strip = true`. Recursive `fib(30)` measured in Node (V8).

| Profile | Raw wasm | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15.87 kB | 6.76 kB | 3.76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14.46 kB | 6.37 kB | 3.89 ms |
| `opt-level = "s"` | 144 B | 14.71 kB | 6.51 kB | 4.17 ms |
| `opt-level = "z"` | 137 B | 14.72 kB | 6.52 kB | 4.32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13.58 kB | 6.19 kB | 3.88 ms |

### Sizes

| Item | min | gzip |
| --- | ---: | ---: |
| **active-react** `wasm` + `useWasm` | 1.03 kB | 0.60 kB |
| react-wasm 1.0.1 `useWasm` | 5.27 kB | 2.07 kB |
| wasm-bindgen JS glue (3 functions) | 1.89 kB | 0.93 kB |
| Raw `extern "C"` binary (add + fib) | 144 B | 139 B |
| wasm-bindgen binary (add + fib + greet) | 15.87 kB | 6.76 kB |

### Time to the first WASM result

From navigation start until the result of a Rust call is rendered (median). "RTT 100 ms" emulates realistic network latency (mobile, 4G) through the Chrome DevTools Protocol.

| Scenario | localhost | RTT 100 ms |
| --- | ---: | ---: |
| active-react `wasm("/x.wasm")` (raw) | 28.8 ms | 331.5 ms |
| react-wasm 1.0.1 (raw) | 28.5 ms | 327.1 ms |
| Hand-written `useEffect` (raw) | 25.8 ms | 328.9 ms |
| active-react + wasm-bindgen | 23.5 ms | 328.4 ms |
| Hand-written `useEffect` + wasm-bindgen | 24.6 ms | 332.9 ms |
| SSR + hydration, **with** automatic preload (`url`) | 20.2 ms | 224.2 ms |
| SSR + hydration, without preload | 24.5 ms | 330.0 ms |

### Cost of JS to WASM calls (Chromium)

| Measure | Pure JS | Raw wasm | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)` per call | 0.55 ns | 4.05 ns | 4.81 ns |
| Recursive `fib(30)` | 11.4 ms | 2.1 ms | 1.9 ms |
| `greet(str) -> String` per call | 4.7 ns | — | 553.0 ns |

## Reproduce

Prerequisites:

- Node.js 22 or newer;
- Rust with the `wasm32-unknown-unknown` target;
- `wasm-bindgen-cli` 0.2.129: `cargo install wasm-bindgen-cli --version 0.2.129`;
- Chromium for Playwright: set `PLAYWRIGHT_BROWSERS_PATH`, or run `npx playwright install chromium`.

Then, from the repository root:

```sh
cd bench
npm run setup   # builds the library and installs the benchmark dependencies
npm run bench   # everything, about 5 minutes
```

Run one half with `node run.mjs seo` or `node run.mjs wasm`. The script rewrites `bench/RESULTS.md` and keeps the raw data in `bench/results/`. The page written with each library lives in `bench/seo/libs/`, the WASM pages in `bench/wasm/pages/`.

## Caveats

- **SSR numbers vary by about ±5% between runs.** Differences of a few percent (active-react against @unhead/react, for example) are within noise; the large gaps (against react-helmet) are not.
- **`performance.now()` is coarsened to 0.1 ms in Chromium** without cross-origin isolation, so very small durations, like the 0.3 ms head latencies, are approximate.
- **The `fib(30)` columns differ between tables** because they were measured in different engines and setups: Node for the Cargo profiles, Chromium for the call costs.
- **One machine, one page.** Absolute numbers depend on your hardware and your pages; the comparisons between libraries are what matter.
- **The "burst" test** measures raw throughput of 1,000 consecutive route changes, which real users never do. The per-navigation latency is the more meaningful number for real apps.
