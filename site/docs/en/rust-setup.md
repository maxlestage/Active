---
title: Rust setup
description: Install Rust, the wasm32 target and wasm-pack or wasm-bindgen-cli, configure Cargo for small and fast binaries, and build your first module.
---

This page sets up everything you need to compile Rust to WebAssembly for active-react. You only need it for the WASM part: the SEO features work without Rust.

## Install Rust

Rust is installed with **rustup**, the official toolchain manager. On macOS and Linux:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs). Then open a new terminal and check:

```sh
rustc --version
cargo --version
```

Keep Rust up to date with `rustup update`.

## Add the WebAssembly target

Rust compiles for your machine by default. Add the browser target once:

```sh
rustup target add wasm32-unknown-unknown
```

## Pick a build tool

Rust produces a `.wasm` file; **wasm-bindgen** then generates the JavaScript glue and TypeScript types that let you pass strings, arrays and more. You can drive it in two ways.

### wasm-pack (easiest)

```sh
cargo install wasm-pack
```

wasm-pack runs Cargo, downloads the right version of wasm-bindgen for your crate automatically, and optimizes the result. One command does everything.

### wasm-bindgen-cli (more control)

```sh
cargo install wasm-bindgen-cli --version 0.2.129
```

The CLI version **must exactly match** the version of the `wasm-bindgen` crate in your `Cargo.lock`. Pin the crate with `=` so the two can never drift apart:

```toml
[dependencies]
wasm-bindgen = "=0.2.129"
```

If they differ, the build stops with an error like this one:

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary:

  rust Wasm file schema version: 0.2.129
     this binary schema version: 0.2.100
```

Fix it by installing the matching CLI (`cargo install -f wasm-bindgen-cli --version 0.2.129`) or by changing the crate version. This is the setup used by the repository's example and benchmarks.

## Cargo.toml

A WebAssembly library needs `crate-type = ["cdylib"]`. Here is the full file, with the release profile validated by the [benchmarks](../benchmarks/):

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

What each profile setting does:

| Setting | Effect |
| --- | --- |
| `opt-level = 3` | Optimize for speed. `"s"` and `"z"` optimize for size; the benchmarks found them only slightly smaller and a bit slower |
| `lto = true` | Link-time optimization across all crates: removes unused code from dependencies, smaller and faster output |
| `codegen-units = 1` | Compile the crate as one unit so the optimizer sees everything. Slower builds, better code |
| `panic = "abort"` | No stack unwinding machinery in the binary. A panic traps and throws a `RuntimeError` in JavaScript |
| `strip = true` | Remove symbol names and debug information from the binary |

These settings only affect `--release` builds, so debug builds stay fast to compile.

## Write Rust functions

Mark each function you want to call from JavaScript with `#[wasm_bindgen]`:

```rust
use wasm_bindgen::prelude::*;

/// Numbers: i32, u32, f64… become JavaScript numbers.
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Strings: &str in, String out.
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello {name}, from Rust!")
}

/// Typed arrays: &[f64] receives a Float64Array.
#[wasm_bindgen]
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Vec<f64> comes back as a new Float64Array.
#[wasm_bindgen]
pub fn normalize(values: &[f64]) -> Vec<f64> {
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    if max <= 0.0 {
        return values.to_vec();
    }
    values.iter().map(|v| v / max).collect()
}
```

How common types cross the boundary:

| Rust | TypeScript |
| --- | --- |
| `i32`, `u32`, `f32`, `f64` | `number` |
| `i64`, `u64` | `bigint` |
| `bool` | `boolean` |
| `&str`, `String` | `string` |
| `&[f64]`, `Vec<f64>` | `Float64Array` |
| `&[u8]`, `Vec<u8>` | `Uint8Array` |
| `Option<T>` | `T \| undefined` (arguments also accept `null`) |

Strings and arrays are copied between JavaScript and WebAssembly memory, which costs time: see the [performance guide](../performance/).

## Build

With wasm-pack, from your project root:

```sh
wasm-pack build hello --target web --out-dir ../src/pkg
```

`--out-dir` is relative to the crate folder. With wasm-bindgen-cli, it takes two commands:

```sh
cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm
```

Use `--target web` (recommended): active-react initializes it for you and can preload the binary. `--target bundler` also works if your bundler supports WebAssembly ESM integration.

## Generated files

| File | Contents |
| --- | --- |
| `hello.js` | JavaScript glue: your functions, plus a default `init` export |
| `hello_bg.wasm` | The compiled Rust |
| `hello.d.ts` | TypeScript types of your functions |
| `hello_bg.wasm.d.ts` | Types of the low-level exports (you rarely need them) |
| `package.json`, `.gitignore` | Only with wasm-pack |

The generated types are precise. For the functions above:

```ts
export function add(a: number, b: number): number;
export function greet(name: string): string;
export function mean(values: Float64Array): number;
export function normalize(values: Float64Array): Float64Array;
```

> **Note:** File names use the crate name with dashes turned into underscores: a crate named `my-crate` produces `my_crate.js` and `my_crate_bg.wasm`.

## Project layout

A layout that works well with Vite:

```text
my-app/
├── hello/                 Rust crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── src/
│   ├── pkg/               generated, not committed
│   ├── App.tsx
│   ├── seo.ts             createSeo defaults
│   └── wasm.ts            wasm() declarations
├── index.html
└── package.json
```

Add `hello/target` and `src/pkg` to `.gitignore` (wasm-pack already writes a `.gitignore` inside `src/pkg`).

## npm scripts

Wrap the build in a script so nobody has to remember the flags:

```json
{
  "scripts": {
    "wasm": "wasm-pack build hello --target web --out-dir ../src/pkg",
    "dev": "npm run wasm && vite",
    "build": "npm run wasm && tsc -b && vite build"
  }
}
```

The same script with wasm-bindgen-cli, as in the [vite-ssg example](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg):

```json
{
  "scripts": {
    "wasm": "cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm"
  }
}
```

After changing Rust code, run `npm run wasm` again: Vite picks up the new files and reloads the page.

Next: [load the module in React](../loading-wasm/).
