---
title: Raw WebAssembly
description: Ship tiny numeric Rust functions as a plain .wasm file with extern C exports, no JavaScript glue, and load them with wasm(url).
---

wasm-bindgen is convenient, but it adds JavaScript glue and memory helpers to every module. For pure number crunching you can skip it: export plain functions from Rust and load the `.wasm` file directly. In the benchmarks, a module with two functions weighed **144 bytes**, against 15.87 kB for the wasm-bindgen version (which also handled a string).

## When to use it

Raw modules are a great fit when every function takes and returns numbers only: maths, physics, checksums, interpolation, colour conversion, scoring.

As soon as you need strings, arrays or objects, use [wasm-bindgen](../rust-setup/) instead (see [limitations](#limitations)).

## Write the Rust

Create a library crate with no dependencies:

```toml
[package]
name = "math"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Export functions with `#[unsafe(no_mangle)]` and `extern "C"`:

```rust
// src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn fib(n: u32) -> u32 {
    if n < 2 { n } else { fib(n - 1).wrapping_add(fib(n - 2)) }
}
```

- `#[unsafe(no_mangle)]` keeps the function name as is, so JavaScript can find it. This is the Rust 2024 edition syntax; older editions write `#[no_mangle]`.
- `extern "C"` gives the function a stable calling convention that WebAssembly understands.

## Build

```sh
cargo build --release --target wasm32-unknown-unknown
```

The binary is at `target/wasm32-unknown-unknown/release/math.wasm`. No other tool is needed. Copy it where your app serves static files, for example Vite's `public/` folder:

```sh
cp target/wasm32-unknown-unknown/release/math.wasm ../public/math.wasm
```

## Load it

Pass the URL of the file to `wasm()`, with the type of its exports:

```ts
// src/wasm.ts
import { wasm } from "active-react";

export const math = wasm<{ add(a: number, b: number): number; fib(n: number): number }>("/math.wasm");
```

Then use it like any other module:

```tsx
import { useWasm } from "active-react";
import { math } from "./wasm";

export function Fibonacci({ n }: { n: number }) {
  const m = useWasm(math);
  return <p>fib({n}) = {m ? m.fib(n) : "…"}</p>;
}
```

Since the URL is the source itself, raw modules are always preloaded during server rendering. You can also let Vite fingerprint the file instead of using `public/`:

```ts
import mathUrl from "./math.wasm?url";

export const math = wasm<{ add(a: number, b: number): number }>(mathUrl);
```

> **Note:** Without a type argument, the exports are typed as `WebAssembly.Exports`. Write the interface by hand: it is your contract with the Rust code.

## The MIME type

active-react fetches the file and compiles it **while it downloads** with `WebAssembly.instantiateStreaming`, which is the fastest path. Browsers only allow it when the server sends the `Content-Type: application/wasm` header.

If the header is missing or different, active-react falls back to downloading the whole file first (`arrayBuffer()`), then compiling it. Your module still works, just a little later. Vite's dev and preview servers and most static hosts send the right header; check yours in the browser's network tab, and see the [FAQ](../faq/) to fix it.

A non-2xx response is an error, for example `wasm: /math.wasm responded 404`, thrown to your error boundary like any [loading error](../loading-wasm/#handling-errors).

## Passing imports

A raw module can also call JavaScript. Declare the functions in Rust, inside an import module named `env`:

```rust
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn now() -> f64;
    fn report(ms: f64);
}

#[unsafe(no_mangle)]
pub extern "C" fn timed_fib(n: u32) -> u32 {
    let start = unsafe { now() };
    let result = fib(n);
    unsafe { report(now() - start) };
    result
}
```

Then provide them with the `imports` option:

```ts
export const math = wasm<{ timed_fib(n: number): number }>("/math.wasm", {
  imports: {
    env: {
      now: () => performance.now(),
      report: (ms: number) => console.log(`fib took ${ms} ms`),
    },
  },
});
```

> **Tip:** Recent Rust versions refuse to link functions that are declared but not defined. The `#[link(wasm_import_module = "env")]` attribute marks them as imports explicitly.

## Numbers at the boundary

WebAssembly functions only exchange numbers, and a few conversions are worth knowing:

| Rust | JavaScript |
| --- | --- |
| `i32`, `f32`, `f64` | `number` |
| `u32` | `number`, but read as **signed**: values above 2,147,483,647 come back negative. Use `result >>> 0` |
| `i64`, `u64` | `bigint` |
| `bool` | `number`: `1` for true, `0` for false |
| `usize`, pointers | `number` (32-bit on `wasm32`) |

## Limitations

- **Numbers only.** Strings, arrays and structs live in the module's linear memory. To pass them, you must export an allocator, copy bytes into `memory.buffer` from JavaScript, pass a pointer and a length, then free the memory. wasm-bindgen generates exactly this code for you, correctly.
- **Types are written by hand.** Nothing checks that your TypeScript interface matches the Rust signatures.
- **No panic messages.** A panic traps and throws a generic `RuntimeError` in JavaScript, without the Rust message.

If any of these gets in your way, switch to [wasm-bindgen](../rust-setup/). The JavaScript glue only adds 0.93 kB gzip for three functions.
