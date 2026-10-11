# active

**English** · [Français](README.fr.md) · [Español](README.es.md)

**A simple, fast front-end library for Rust: components, state, routing and complete SEO tags, rendered on the server and run in the browser as WebAssembly.**

```rust
use active::prelude::*;

fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

fn page() -> String {
    let body = div()
        .child(Seo::new().title("Home").description("My site, in Rust."))
        .child(h1().text("Hello"))
        .child(island("counter", "0", counter)); // the only part sent to the browser
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}
```

The server sends finished HTML with every SEO tag. In the browser, the same `counter` function, compiled to WebAssembly, takes over the button that is already on screen. No JavaScript to write, no virtual DOM, no second render of the static content.

The same components also make **client-side apps**: `active::mount_to("#app", app())` renders the whole app in the browser, `active::location()` and `active::navigate()` route without reloading, and each page's `Seo` updates the title and meta tags ([`examples/spa`](examples/spa)). Or write them to files for a **static site**.

📚 **Documentation in English, French and Spanish**: the website in [`site/`](site), itself built with `active`. Run it with `cargo run -p active-site` or deploy your own copy on Heroku from a phone (below). The pages are also readable on GitHub in [`site/docs/`](site/docs).

## Why

- **Simple.** Components are Rust functions returning a view, the ideas of React hooks without the JavaScript. State is `use_state`, a `Copy` handle you move into closures. There are no macros to learn and no build plugin.
- **SEO first.** The server always renders the full page. `Seo` covers title, description, robots, canonical, hreflang, Open Graph, Twitter and JSON-LD; put one in a layout, a page and a component, they are merged with no duplicate tag.
- **Fast.** Server rendering writes straight into a `String`. In the browser, only islands are hydrated, with fine-grained bindings: a state change updates the exact text, attribute or class that depends on it. See the [benchmarks](#benchmarks).
- **Light.** The server build has no dependency. The browser bundle uses `wasm-bindgen` and `web-sys`, nothing else.
- **Installable.** `active::pwa` writes the web app manifest and an offline service worker, and `Document::manifest` / `Document::service_worker` add their tags: your site goes on the home screen and keeps working without a network, with no JavaScript to write. The project's website is a PWA ([guide](site/docs/en/pwa.md)).

## Benchmarks

Measured against the main Rust frameworks on the same pages, all in Rust ([method and raw data](bench/RESULTS.md)):

| | Server render of a product page | WebAssembly + glue for a counter (gzip) | Interactive at 100 ms latency |
| --- | ---: | ---: | ---: |
| **active** | **5.6 µs** | **36.8 kB** | **224 ms** |
| Leptos 0.8 + leptos_meta | 13.4 µs | 60.9 kB | 227 ms |
| Leptos 0.8, `<head>` by hand | 5.2 µs | — | — |
| Sycamore 0.9 | 33.0 µs | 45.8 kB | 227 ms |
| Yew 0.23 | 17.2 µs | 96.3 kB | 228 ms |
| Dioxus 0.7 | 24.2 µs | 170.4 kB | 339 ms |
| maud 0.27 (template only) | 1.4 µs | — | — |

The smallest bundle of all, and server rendering 2.4 times faster than the other framework that merges SEO tags from components (Leptos with leptos_meta). Intel Xeon 2.8 GHz, Rust 1.97, Chromium 141; run it yourself with `cd bench && cargo run --release -p runner`.

## Install

```sh
cargo add active                      # server
cargo add active --features client    # browser bundle, for wasm32-unknown-unknown
```

Two complete apps: [`examples/hello`](examples/hello), a server page with an island (one crate for both sides), and [`examples/spa`](examples/spa), a client-side to-do app with routing. Run the first:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello   # http://localhost:3000
```

## Repository

| Folder | Contents |
| --- | --- |
| [`crates/active`](crates/active) | The `active` crate: views, state, SEO, documents, islands, hydration |
| [`examples/hello`](examples/hello) | The smallest app: a page and a counter island, std-only server |
| [`examples/spa`](examples/spa) | A client-side app: to-do list, routing, forms, `localStorage` |
| [`site`](site) | Presentation website and documentation (en/fr/es): Vitesse server, WebAssembly animations, search and playground, installable and offline (PWA) |
| [`bench`](bench) | Benchmarks against Leptos, Dioxus, Yew, Sycamore, maud and hand-written HTML |

## Publish the site on GitHub Pages (free)

1. On GitHub, open **Settings → Pages** and set **Source** to **GitHub Actions** (once; the GitHub app or the mobile website works).
2. That's all. After every green CI run, which merges the branch into `master`, the [`pages.yml`](.github/workflows/pages.yml) workflow exports the site to static files with `active-site export` and publishes it at `https://maxlestage.github.io/Active/`. To publish right away: **Actions → Pages → Run workflow**.

Guide: [`site/docs/en/deploy-pages.md`](site/docs/en/deploy-pages.md).

## Deploy the site on Heroku from a phone

1. Open `https://heroku.com/deploy?template=<the address of your repository>` in your phone's browser, choose a name and tap **Deploy app**. Heroku reads [`app.json`](app.json) and builds the [`Dockerfile`](Dockerfile) (see [`heroku.yml`](heroku.yml)): the WebAssembly bundle, then the server, all from the Rust sources.
2. To redeploy on every change: in the app's **Deploy** tab, connect the repository and **Enable Automatic Deploys** from `master`.

Heroku has no free plan: an Eco dyno costs about $5 a month. Step-by-step guide: [`site/docs/en/deploy-heroku.md`](site/docs/en/deploy-heroku.md).

## Publish to crates.io from a phone

1. Create a token on [crates.io/settings/tokens](https://crates.io/settings/tokens) and add it as the repository secret `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Open **Releases → Draft a new release**, create a tag such as `v0.1.0`, then tap **Publish release**.

The [`release.yml`](.github/workflows/release.yml) workflow sets the crate version from the tag, runs the tests and publishes `active`. Guide: [`site/docs/en/publishing.md`](site/docs/en/publishing.md).

## Development

```sh
cargo test --workspace                     # library, site and examples
cargo run -p active --example page         # print a complete page
cargo run -p active-site                   # the website on http://localhost:3000
```

The website's browser bundle is built like the example above (`-p active-site`, output in `site/pkg`); see the [`Dockerfile`](Dockerfile). Every branch that passes CI is merged into `master` automatically, except branches named `wip/…` ([`ci.yml`](.github/workflows/ci.yml)); `master` is then published on GitHub Pages ([`pages.yml`](.github/workflows/pages.yml)).

## License

Active is proprietary software: © 2026 Maxime Nathan Lestage, all rights reserved. It is not open source. The source code is published on crates.io and GitHub so that it can be read, audited and built, and you may use it freely as a dependency of your own programs, commercial ones included. Modifying it or redistributing its source needs the author's written permission. Full terms: [LICENSE](LICENSE).

## Credits

Designed and developed by Maxime Nathan Lestage, as part of one family of Rust projects: [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/) and [FerroSwift](https://maxlestage.github.io/FerroSwift/).
