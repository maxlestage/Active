---
title: Performance guide
description: What the benchmarks teach about loading, calling and sizing Rust/WebAssembly in React, and why SEO with active-react costs almost nothing.
---

Every design choice in active-react comes from measurements. This page turns the [benchmarks](../benchmarks/) into practical advice: what makes a WebAssembly module fast to load, fast to call and small, and what the SEO part costs.

## Loading: the network decides

The benchmarks measured the time from navigation to the first Rust result on screen, with every way of loading a module in a client-rendered page:

| Scenario (client rendering) | localhost | 100 ms RTT |
| --- | ---: | ---: |
| active-react, raw `.wasm` | 28.8 ms | 331.5 ms |
| react-wasm 1.0.1, raw `.wasm` | 28.5 ms | 327.1 ms |
| Hand-written `useEffect`, raw `.wasm` | 25.8 ms | 328.9 ms |
| active-react + wasm-bindgen | 23.5 ms | 328.4 ms |
| Hand-written `useEffect` + wasm-bindgen | 24.6 ms | 332.9 ms |

With realistic latency, they all land around **330 ms**. The loading code doesn't matter: the time goes into network round trips (HTML, then JavaScript, then the `.wasm` file) and compilation.

So two things make a real difference:

1. **Start the download earlier.** With server rendering, active-react emits a preload for the binary, which then downloads in parallel with the JavaScript. Same page, same module: **224.2 ms with preload against 330.0 ms without, 32% faster**.
2. **Ship a smaller binary.** Fewer bytes to download and compile. See [binary size](#binary-size) below.

> **Tip:** Give `wasm()` the `url` option and render on the server or at build time. You get the preload for free. See [loading WebAssembly](../loading-wasm/#preloading).

## Calls: batch your work

Calling into WebAssembly is fast, but not free. Measured in Chromium:

| Measure | Pure JS | Raw wasm | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)`, per call | 0.55 ns | 4.05 ns | 4.81 ns |
| `fib(30)`, recursive | 11.4 ms | 2.1 ms | 1.9 ms |
| `greet(str) -> String`, per call | 4.7 ns | — | 553.0 ns |

Three lessons:

- **Rust is about 5× faster on real work.** `fib(30)` takes 2.1 ms in WebAssembly against 11.4 ms in JavaScript.
- **Each call costs about 4 ns**, against 0.6 ns for a JavaScript function. Ten million tiny calls waste time crossing the boundary.
- **Strings are expensive: about 550 ns per call.** wasm-bindgen copies and re-encodes every string in both directions.

The rule: **send big chunks of work, not many small calls.** Pass a whole typed array once instead of calling Rust for each item:

```rust
use wasm_bindgen::prelude::*;

// One call processes the whole dataset.
#[wasm_bindgen]
pub fn smooth(values: &[f64], window: usize) -> Vec<f64> {
    let window = window.max(1);
    values
        .windows(window)
        .map(|w| w.iter().sum::<f64>() / window as f64)
        .collect()
}
```

```ts
// Good: one call, one copy in, one copy out.
const smoothed = rust.smooth(new Float64Array(samples), 5);

// Slow: one boundary crossing per item (scale being a tiny Rust function).
const scaled = samples.map((x) => rust.scale(x, 2));
```

Keep strings for the edges (a title, a result to display), not for inner loops.

## When Rust is worth it

Good candidates: heavy loops over numbers, parsing, compression, image and audio processing, simulations, cryptography, search over large in-memory data. A task that takes milliseconds in JavaScript and runs in one call is the sweet spot.

Poor candidates: small functions called very often from JavaScript, string formatting, DOM work (WebAssembly can't touch the DOM without going through JavaScript).

## Binary size

| Item | Minified | Gzip |
| --- | ---: | ---: |
| active-react `wasm` + `useWasm` | 1.03 kB | 0.60 kB |
| react-wasm 1.0.1 `useWasm` | 5.27 kB | 2.07 kB |
| wasm-bindgen JS glue (3 functions) | 1.89 kB | 0.93 kB |
| Raw `extern "C"` binary (add + fib) | 144 B | 139 B |
| wasm-bindgen binary (add + fib + greet) | 15.87 kB | 6.76 kB |

The wasm-bindgen binary is about 100 times larger than the raw one, and almost all of that comes from the `greet` function: `format!` and the `String` machinery (allocator, UTF-8 handling, formatting) weigh about 15 kB. Pure numeric functions compile to a few dozen bytes.

To keep binaries small:

- Avoid `format!`, `println!` and `to_string()` in code that ships to the browser when you can return numbers instead.
- Watch heavy dependencies: `serde_json`, `regex` or `chrono` each add a lot.
- For numbers-only modules, skip wasm-bindgen entirely: see [raw WebAssembly](../raw-wasm/).

## Cargo profiles

Same crate, measured for size and speed (`lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` in every row; `fib(30)` measured in Node):

| Profile | Raw wasm | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15.87 kB | 6.76 kB | 3.76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14.46 kB | 6.37 kB | 3.89 ms |
| `opt-level = "s"` | 144 B | 14.71 kB | 6.51 kB | 4.17 ms |
| `opt-level = "z"` | 137 B | 14.72 kB | 6.52 kB | 4.32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13.58 kB | 6.19 kB | 3.88 ms |

## Recommendations

- **Default to `opt-level = 3`** with `lto`, `codegen-units = 1`, `panic = "abort"` and `strip`. It is the fastest profile, and the size difference with the others is small.
- **If every kilobyte counts**, use `opt-level = "z"` plus `wasm-opt -Oz` (from binaryen). It is about 8% smaller gzipped (6.19 kB against 6.76 kB) with similar speed in these measurements.
- `opt-level = "s"` or `"z"` alone are slower and barely smaller: not worth it on their own.
- **Preload** the binary with server rendering and the `url` option.
- **Batch** calls and avoid strings in hot paths.

## SEO costs almost nothing

The SEO part was measured against hand-written tags (React 19 with no library) and the popular alternatives, on the same page:

- **Server rendering:** 42.4 µs per page, against 39.7 µs for hand-written tags, so +7%. That is **1.5× faster than react-helmet** (66.5 µs). With `renderWithHead` and an HTML template: 44.1 µs.
- **Head updates on navigation:** a median of **0.3 ms** between rendering a new page and an up-to-date `<head>`, the same as hand-written tags.
- **Bundle size:** 0.74 kB gzip, against 6.07 to 6.76 kB for @unhead/react, react-helmet-async and react-helmet.

The reason is simple: React 19 does the work. `<Seo>` renders tags and nothing else: no context, no effect and no second pass over the tree.
