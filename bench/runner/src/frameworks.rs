//! Every implementation under test: one entry per SSR variant and per counter app.

/// An SSR implementation: a binary of package `package`, run with `args`.
pub struct SsrBench {
    pub id: &'static str,
    pub name: &'static str,
    pub package: &'static str,
    pub args: &'static [&'static str],
    /// Crates whose versions identify the implementation (looked up in `Cargo.lock`).
    pub crates: &'static [&'static str],
    /// How the page is produced, in a few words.
    pub how: &'static str,
}

/// The hand-written baseline the others are compared to.
pub const BASELINE: &str = "handwritten";

pub const SSR: &[SsrBench] = &[
    SsrBench {
        id: "handwritten",
        name: "Hand-written",
        package: "ssr-handwritten",
        args: &[],
        crates: &[],
        how: "`push_str`/`write!` into one `String`, values escaped",
    },
    SsrBench {
        id: "maud",
        name: "maud",
        package: "ssr-maud",
        args: &[],
        crates: &["maud"],
        how: "`html!` compile-time template",
    },
    SsrBench {
        id: "active",
        name: "active",
        package: "ssr-active",
        args: &[],
        crates: &["active"],
        how: "components + `Seo` merged into `<head>` by `Document`",
    },
    SsrBench {
        id: "leptos",
        name: "Leptos + leptos_meta",
        package: "ssr-leptos",
        args: &["meta"],
        crates: &["leptos", "leptos_meta"],
        how: "`<Title>`/`<Meta>`/`<Link>`/`<Script>` injected at `<MetaTags/>` (as leptos_axum does)",
    },
    SsrBench {
        id: "leptos-plain",
        name: "Leptos (plain `<head>`)",
        package: "ssr-leptos",
        args: &["plain"],
        crates: &["leptos"],
        how: "whole document in one `view!`, `to_html()`",
    },
    SsrBench {
        id: "dioxus",
        name: "Dioxus",
        package: "ssr-dioxus",
        args: &[],
        crates: &["dioxus", "dioxus-ssr"],
        how: "`rsx!` `head`+`body`, `VirtualDom` + reused `dioxus_ssr::Renderer`",
    },
    SsrBench {
        id: "yew",
        name: "Yew",
        package: "ssr-yew",
        args: &[],
        crates: &["yew"],
        how: "`html!` document, `LocalServerRenderer` on a current-thread Tokio runtime",
    },
    SsrBench {
        id: "sycamore",
        name: "Sycamore",
        package: "ssr-sycamore",
        args: &[],
        crates: &["sycamore"],
        how: "`view!` document, `render_to_string`",
    },
];

/// A counter app: package `package`, whose library `lib` is the browser bundle.
pub struct CounterApp {
    pub id: &'static str,
    pub name: &'static str,
    pub package: &'static str,
    pub lib: &'static str,
    pub crates: &'static [&'static str],
    /// How the browser takes over the server HTML.
    pub mode: &'static str,
    /// When `data-ready` is recorded.
    pub ready: &'static str,
}

pub const COUNTERS: &[CounterApp] = &[
    CounterApp {
        id: "active",
        name: "active",
        package: "counter-active",
        lib: "counter_active",
        crates: &["active"],
        mode: "island hydration (`active::hydrate`)",
        ready: "after `active::hydrate` returns (synchronous)",
    },
    CounterApp {
        id: "leptos",
        name: "Leptos",
        package: "counter-leptos",
        lib: "counter_leptos",
        crates: &["leptos"],
        mode: "hydration (`hydrate_body`)",
        ready: "after `leptos::mount::hydrate_body` returns (synchronous)",
    },
    CounterApp {
        id: "dioxus",
        name: "Dioxus",
        package: "counter-dioxus",
        lib: "counter_dioxus",
        crates: &["dioxus"],
        mode: "client-side render (`dioxus::launch`)",
        ready: "first `use_effect` (after the first render is applied to the DOM)",
    },
    CounterApp {
        id: "yew",
        name: "Yew",
        package: "counter-yew",
        lib: "counter_yew",
        crates: &["yew"],
        mode: "hydration (`Renderer::hydrate`)",
        ready: "first `use_effect_with` (after the hydrated render is committed)",
    },
    CounterApp {
        id: "sycamore",
        name: "Sycamore",
        package: "counter-sycamore",
        lib: "counter_sycamore",
        crates: &["sycamore"],
        mode: "hydration (`sycamore::hydrate`)",
        ready: "after `sycamore::hydrate` returns (synchronous)",
    },
];
