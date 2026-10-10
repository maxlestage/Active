# Benchmarks

`active` compared with other Rust web frameworks on the same pages, measured the same way.
Everything here is Rust: the implementations, the runner that drives Chromium and the report
generator. The latest results are in [RESULTS.md](RESULTS.md).

| Framework | Server rendering | Browser |
| --- | --- | --- |
| **active** | `Seo` + `Document` | island + `active::hydrate` |
| [Leptos](https://leptos.dev) 0.8 | `view!` + `to_html`, with and without leptos_meta | `hydrate_body` |
| [Dioxus](https://dioxuslabs.com) 0.7 | `rsx!` + dioxus-ssr | client-side render (see below) |
| [Yew](https://yew.rs) 0.23 | `html!` + `LocalServerRenderer` | `Renderer::hydrate` |
| [Sycamore](https://sycamore.dev) 0.9 | `view!` + `render_to_string` | `sycamore::hydrate` |
| [maud](https://maud.lambda.xyz) 0.27 | `html!` template (baseline) | — |
| Hand-written | `push_str`/`write!` (baseline) | — |

## Running

Prerequisites:

- Rust (tested with 1.97) and the browser target: `rustup target add wasm32-unknown-unknown`.
- The wasm-bindgen CLI, at the exact version of the `wasm-bindgen` crate in `Cargo.lock`:
  `cargo install wasm-bindgen-cli --version 0.2.129` (the runner checks it).
- Chromium or Chrome, for the browser part. chromiumoxide finds it on the `PATH`; otherwise
  set `CHROME=/path/to/chrome` or pass `--chrome /path/to/chrome`.

```sh
cd bench
cargo run --release -p runner              # everything, then RESULTS.md and results/*.json
cargo run --release -p runner -- ssr       # one part: ssr | wasm | browser | report
cargo run --release -p runner -- --quick   # shortened measurements, to check the setup
```

Once everything is compiled, a full run takes about 2½ minutes; the first one also compiles
five frameworks for the server and for the browser, which takes several more minutes. `browser`
needs the output of `wasm`; `report` only rewrites `RESULTS.md` from `results/*.json`. Close
other programs while it runs: every number is relative to the machine and to its load at the
time.

Each piece also runs on its own:

```sh
cargo run --release -p ssr-leptos -- plain        # one SSR implementation: prints one JSON line
BENCH_PRINT=1 cargo run --release -p ssr-yew      # the page it renders, instead of timing it
cargo run --release -p counter-sycamore           # the server HTML of a counter app
```

## Layout

| Path | Contents |
| --- | --- |
| `common/` | What every implementation shares: the page data (`Product`), the HTML checker, the timing loop, the counter page shell and the readiness marks. |
| `ssr/<framework>/` | One binary per framework that renders the SEO page. |
| `counter/<framework>/` | The counter app: the library is the browser bundle, the binary prints the server HTML. |
| `runner/` | Builds everything, runs the measurements, drives Chromium, writes the results. |
| `results/*.json` | Raw results of the last run (every run and every page load). |
| `RESULTS.md` | The report, generated from `results/`. |

It is a separate Cargo workspace (the root one excludes `bench/`), so the frameworks under test
never enter the library's dependency graph. The runner builds one package at a time, so each
gets only its own Cargo features, as in a real project. All packages share the same profiles:
`release` (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`) for the
servers and `wasm-release` (the same with `opt-level = "s"` and `panic = "abort"`) for the
browser bundles.

## What is measured

### 1. Server-side rendering

Every implementation renders the same complete document for product page `i`:

- `<html lang="en">`;
- `<head>`: `<meta charset>`, viewport, the 11 SEO tags (`<title>` built from the template
  `%s · Acme`, description, canonical, `og:title`, `og:description`, `og:type`, `og:url`,
  `og:image`, `og:site_name`, `twitter:card`, `twitter:site`) and a JSON-LD `Product`;
- `<body>`: an `<h1>` and a `<ul>` of 30 `<li>Option {k} — {i·k} €</li>`.

`i` changes on every iteration (1 to 1000), and building the page data is part of the measured
work, the same for everyone (`common/src/page.rs`).

**Correctness.** Before timing, each binary checks its output for several `i` with a small
tokenizer (`common/src/check.rs`): each SEO tag exactly once in the whole document, inside
`<head>`, with the expected value (whatever the attribute order or `<meta>`/`<meta/>`
spelling), the exact JSON-LD, the `<h1>` and the 30 items in order. A wrong page stops the
benchmark. This caught a real Yew pitfall: in hydratable mode Yew wraps raw HTML in
`<!--<#>-->` markers, which inside `<script>` would make the JSON-LD invalid.

**Timing** (`common/src/timing.rs`). Each implementation runs in its own process. It renders for
1 s to warm up, then for 3 s in batches of about 10 ms; a run's result is the median batch, in
µs per page. The runner starts 3 runs per implementation, in turn (all implementations once,
then again), and reports the median of the 3 runs. The table also shows pages per second, the
ratio to the hand-written baseline, the spread of the batches (p10–p90) and the page size.

### 2. WebAssembly size

The same counter in every framework: a button that increments the number shown next to it,
starting at 3 (`counter/`). Each crate is built for `wasm32-unknown-unknown` with the
`wasm-release` profile, then `wasm-bindgen --target web` (no `wasm-opt`). The report gives the
`.wasm` size, its gzip size (level 9) and the gzip size of every JavaScript file wasm-bindgen
generated (the module and, for Dioxus, its snippets: each is a separate download).

### 3. Browser: time to interactive

The runner serves each counter app (its server-rendered HTML and `pkg/`) from a small Rust
HTTP server on `127.0.0.1` and loads it in headless Chromium through chromiumoxide, with the
HTTP cache disabled: one warm-up round, then 21 measured loads per app, on localhost and with
100 ms of latency added to every request (`Network.emulateNetworkConditions`, bandwidth not
limited). Loads are interleaved: one load of each app, then again, so that a slow moment of the
machine does not hit a single framework.

No JavaScript is evaluated. Each app records its own timings from Rust
(`bench_common::mark`): `performance.now()` (milliseconds since navigation start) goes into
`<body data-start>` when its wasm starts and into `<body data-ready>` as soon as the button is
interactive. The runner reads these attributes with `DOM.querySelector` and
`DOM.describeNode`. After the loads, it clicks the button once (`DOM.getContentQuads`, then
`Input.dispatchMouseEvent`) and checks that the number went from 3 to 4.

`data-ready` is recorded right after the hydration call returns for frameworks that hydrate
synchronously (active, Leptos, Sycamore), and in the first effect after the render is applied
for those that render asynchronously (Yew `use_effect_with`, Dioxus `use_effect`).

Every counter page loads its bundle the same way `active`'s `Document::client` does: the
JavaScript module and the `.wasm` are preloaded from `<head>`, then a one-line
`<script type="module">` starts the wasm-bindgen module. This line and wasm-bindgen's generated
glue are the only JavaScript in the pages.

## Framework-specific choices

- **active**: the page component returns its `Seo` next to its content; `Document` merges it
  with the site-wide `Seo` (built once, cloned per page) into `<head>`. The counter is an
  island; the page's `Document` also writes its default tags and
  `<style>active-island{display:contents}</style>`.
- **Leptos**: two SSR rows. *Leptos + leptos_meta* is the idiomatic SEO setup: the page
  declares `<Title formatter>`, `<Meta>`, `<Link>` and `<Script>`, the shell has
  `<MetaTags/>`, and `ServerMetaContextOutput::inject_meta_context` inserts the tags (it
  awaits a tick of Leptos' executor: a current-thread Tokio runtime here, as with
  leptos_axum). *Leptos (plain `<head>`)* writes the same tags as elements in one `view!`.
  `property` is not an HTML attribute, so `og:*` tags in the plain variant use the element
  builder (`meta().attr("property", …)`), as leptos_meta does internally. Each page renders
  in a new `Owner`, like a request.
- **Dioxus**: dioxus-html has no `html` element (dioxus' server wraps the app in an HTML
  template), so `<!DOCTYPE html><html lang="en">` is written around the rendered
  `head`/`body`. One `VirtualDom` per page, one `dioxus_ssr::Renderer` reused (dioxus' server
  pools renderers to keep their template cache). The JSON-LD uses `dangerous_inner_html`.
  **In the browser Dioxus renders on the client** (the default for a dioxus web app): its
  hydration reads `window.initial_dioxus_hydration_data`, which only dioxus' fullstack server
  writes, with an inline script, into the page. Its server HTML is therefore an empty
  `<div id="main">`. Dioxus is built with the `minimal` and `web` features (no logger,
  devtools or asset features).
- **Yew**: `LocalServerRenderer` is the single-thread form of `ServerRenderer` (which only
  moves the same work to a worker thread), driven by a current-thread Tokio runtime. Rendering
  is hydratable (Yew's default). Text children are escaped, so the JSON-LD `<script>` element
  is raw HTML (`Html::from_html_unchecked`), as explained above.
- **Sycamore**: `render_to_string` always adds hydration keys (`data-hk` attributes and
  `<!--/-->` markers), which makes its pages larger. Values are prepared before `view!`, where
  an expression containing a call becomes a reactive closure. `"property"` is quoted: it is
  not an HTML attribute.
- **maud** and **hand-written**: baselines without components. Both write the list numbers
  directly instead of formatting one string per item; the component frameworks call
  `option_label`.
- Every implementation makes the JSON-LD safe for `<script>` the same way (`<` becomes
  `\u003c`): `active` does it itself, the others call `bench_common::script_safe`.

## Caveats

- **One machine.** The results are only comparable with each other, within one run. Virtual
  machines and laptops have noisy neighbors and changing clock speeds; the JSON files keep
  every run and every load so the spread can be checked.
- **SSR is single-thread rendering throughput** of one page type, not a web server benchmark:
  no HTTP, no concurrency, the system allocator for everyone.
- **Sizes are without `wasm-opt`**, which the frameworks' own tools (cargo-leptos, `dx`,
  trunk) usually run and which makes every bundle smaller. gzip comes from flate2
  (zlib-rs backend) at level 9; on these files it is 0.2–1.2 % larger than GNU `gzip -9`.
  Brotli is not measured.
- **Browser timings are cold loads** (cache disabled) of a tiny page, so they are dominated by
  fetching and compiling the bundle; the 100 ms profile adds latency only, so the number of
  sequential requests matters more than bytes (Dioxus' snippets are an extra round trip). On a
  slow link, bundle size (section 2) matters more. On localhost, load-to-load variation (the
  min–max in the table) is larger than the differences between frameworks of similar size, and
  the medians move by several milliseconds from one run to the next on a busy machine: read
  them together with their range. `performance.now()` has a 0.1 ms resolution
  in Chromium for pages that are not cross-origin isolated (isolating them would make Chromium
  start a new renderer process for every navigation).
- **Dioxus is not hydrated** (see above): its first paint has no counter until the wasm runs.
- **Versions** come from `Cargo.lock`; `cargo update` and a new run refresh them. `active` is
  the version in this repository at the time of the run.
