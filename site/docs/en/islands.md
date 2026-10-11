---
title: Islands
description: Make parts of a server-rendered page interactive with WebAssembly islands: declare them, pass props, hydrate them in the browser and avoid mismatches.
---

An island is an interactive part of a server-rendered page: a counter, a search box, a product configurator. The server renders it to HTML like the rest of the page and marks it. In the browser, the WebAssembly bundle finds the marked parts and makes only those interactive. Everything outside the islands stays plain HTML and costs nothing.

## Declaring an island

An island component is a function that takes its props as a string and returns a `Node`:

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

pub fn page() -> Element {
    div()
        .child(h1().text("Hello"))
        .child(island("counter", "5", counter))
}

assert_eq!(
    Node::from(page()).render(),
    r#"<div><h1>Hello</h1><active-island data-island="counter" data-props="5"><button><!--#-->5</button></active-island></div>"#
);
```

`island(name, props, component)` calls the component with the props, renders its view inside an `<active-island>` element, and records the name and props in `data-island` and `data-props` attributes.

The component's type is `active::IslandFn`, which is `fn(&str) -> Node`: a plain function (or a closure that captures nothing). Everything the island needs must come from its props.

## Registering islands in the browser

The browser bundle calls `active::hydrate` once, with every island the pages use, under the same names:

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button().on_click(move |_| count.update(|n| *n += 1)).text_dyn(move || count.get().to_string()).into()
}

pub fn search(_props: &str) -> Node {
    input().attr("type", "search").into()
}

/// Every island, by name: used by the server pages and by the browser bundle.
pub const ISLANDS: [(&str, active::IslandFn); 2] = [("counter", counter), ("search", search)];

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&ISLANDS);
}
```

`#[wasm_bindgen(start)]` makes `start` run as soon as the module is loaded. The [client build](../client-build/) page shows how the bundle is compiled and loaded; [`Document::client`](../documents/#loading-the-browser-bundle) writes the loading code into the page.

An island whose name is not registered is left as static HTML, without any error. Check the spelling of names if an island does not react.

## Props

Props travel inside the HTML as a string, escaped in the `data-props` attribute. Use whatever format you can parse back:

- a number or a word: `island("counter", "5", counter)`, `island("search", "en", search)`;
- JSON for structured data, with [`serde_json`](https://docs.rs/serde_json).

```rust
use active::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct CartProps {
    pub product: String,
    pub price_cents: u32,
}

pub fn add_to_cart(props: &str) -> Node {
    let props: CartProps = serde_json::from_str(props).unwrap_or_default();
    let price = props.price_cents;
    let quantity = use_state(1u32);
    div()
        .class("cart")
        .child(strong().text(props.product))
        .child(
            input()
                .attr("type", "number")
                .attr("min", "1")
                .attr_dyn("value", move || quantity.get().to_string())
                .on_input(move |e| quantity.set(e.value().parse().unwrap_or(1))),
        )
        .child(span().text_dyn(move || {
            let total = quantity.get() * price;
            format!("{}.{:02} €", total / 100, total % 100)
        }))
        .into()
}

fn product_page() -> Element {
    let props = serde_json::to_string(&CartProps { product: "Oak chair".into(), price_cents: 24900 })
        .unwrap();
    section().child(h1().text("Oak chair")).child(island("add-to-cart", props, add_to_cart))
}
```

Keep props small: they are part of the page's HTML.

## What hydration does

When `hydrate` runs, for each `<active-island>` of the page that is not hydrated yet:

1. It reads the name and finds the registered component. Unknown names are skipped.
2. It calls the component with the props, in the browser. This creates the island's states.
3. It walks the HTML that is already in the page alongside the view, and **claims** the existing elements and text nodes instead of creating new ones.
4. It attaches event listeners, binds dynamic text, attributes and classes to their states, and runs `on_mount` callbacks.
5. Dynamic regions (`children_dyn`, `fragment_dyn`) are rebuilt from the browser's state.
6. It marks the element with `data-hydrated`, so calling `hydrate` again does nothing.

The visitor already sees the island's content before the bundle loads; hydration only makes it react. The `<head>` written by the server is never changed by islands.

### The island element

`<active-island>` is a custom element. `Document` adds the style `active-island{display:contents}` to every page, so the wrapper does not affect your layout: its children behave as if they were directly in the parent. If you render pages without `Document`, add that rule to your CSS.

## Hydration mismatches

Hydration expects the HTML to match the view the component builds in the browser. If it does not (an element where text was expected, a different tag), `active` prints a warning in the browser console:

```text
active: re-rendering island (tag mismatch)
```

and rebuilds that island from scratch. The page still works, but the island flickers and the work of the server is lost. Common causes:

- **Different input on each side.** The view depends on something the browser does not have: the current time, a random number, a value read from a database. Put every input the island needs into its props.
- **Code that differs by target.** A `#[cfg(target_arch = "wasm32")]` that changes the markup. Use it only for behaviour (handlers, effects), never for what is rendered.
- **HTML changed after rendering.** A proxy, a browser extension or a script that rewrites the page.

Text content is not compared, only the structure. Raw HTML set with `.html(...)` is never touched by hydration, which is useful for content only the server can produce, such as syntax-highlighted code: render it on the server, pass an empty string in the browser. This website's home page tabs work this way (see `site/src/islands.rs`).

## Islands inside islands

An island can contain other islands. The outer island hydrates the inner ones together with itself, using the inner views built by its own code, so they are hydrated once.

## SEO inside islands

A `Seo` inside an island's view is merged into the page's head on the server like any other (see [SEO](../seo/#merging)). In the browser, islands never change the head.

## Islands are independent

Each island has its own states, created when it hydrates, and lives as long as the page. Two islands cannot share a state. If two parts of the page must react to each other (a filter and a list), make them one island. For data that should survive a page load, use the browser storage from an [effect](../state/#effects).

## Islands or a client-side app?

| | Islands | [Client-side app](../client-app/) |
| --- | --- | --- |
| HTML from the server | complete pages | an empty shell |
| SEO | full, on every page | limited to what the shell and the browser provide |
| Code in the browser | only the interactive parts | the whole interface |
| Navigation | regular page loads | in the browser, without reloading |
| Good for | content sites, shops, docs | tools and dashboards behind a login |

Both use the same components, states and events, so moving a component from one to the other needs no rewrite.
