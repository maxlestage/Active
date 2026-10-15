---
title: Streaming and server data
description: Stream a page with ssr::stream so the visitor reads it while the slow parts load, and send resource values along with ssr::resource so the browser hydrates without loading them again.
---

A page often waits for one slow request (recommendations, comments, a price from another service) while everything else is ready. `active::ssr` removes that wait twice over:

- **Streaming** (`ssr::stream`): the server sends the whole page at once with the fallback of every `suspense` that is still loading, then, as each request finishes, the content of that boundary and a few bytes of JavaScript that put it in place. The visitor reads, scrolls and clicks links during the wait. Boundaries arrive in the order their data does (out of order), as in React 18, Solid and Marko.
- **Server data** (`ssr::resource`): the values loaded on the server travel with the HTML as JSON; when the browser hydrates the page, the resource starts with that value instead of loading it a second time (Next's and Nuxt's payloads).

```rust
use active::prelude::*;
use active::{ssr, suspense};
```

## Server data: ssr::resource

`ssr::resource(key, source, fetch)` is a [resource](../async-data/) with a name. Build the page inside `ssr::render`, which renders the `Document` and writes the value of every `ssr::resource` that is ready at the end of the body:

```rust
async fn load_product(id: u32) -> Result<Product, String> { /* database, API… */ }

fn product_page(id: u32) -> Node {
    suspense(p().text("Loading…"), move || {
        let product = ssr::resource(format!("product:{id}"), move || id, load_product);
        div().children_dyn(move || match product.get() {
            Some(Ok(p)) => vec![h1().text(p.name).into()],
            Some(Err(e)) => vec![p().class("error").text(e).into()],
            None => vec![],
        }).into()
    })
}

let html = ssr::render(|| Document::new(product_page(7)).client("/pkg/app.js", "/pkg/app_bg.wasm"));
// …<script type="application/json" data-active-data="product:7">{"Ok":{…}}</script></body></html>
```

In the browser, the same component runs while hydrating: the first load of `product:7` reads the JSON from the page (and removes it), so `suspense` shows the content straight away and no request leaves. A changed source or `refetch` loads normally.

Give each resource of a page its own key. A resource whose value is not there when the page is rendered is simply not sent: the browser loads it.

## The Data trait

Values are written as JSON with the `active::Data` trait, implemented for strings, numbers, `bool`, `char`, `Option`, `Vec`, `Result` (`{"Ok": …}`), tuples and maps with string keys, in the same JSON as `serde_json`. For your own types, write the two methods, by hand or with `serde_json`:

```rust
impl active::Data for Product {
    fn write_data(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).unwrap_or_default());
    }
    fn read_data(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}
```

## Streaming: ssr::stream

`ssr::stream(build)` builds and renders the page at once and returns a `Stream` of HTML chunks:

1. the page, with the fallback of each `suspense` still loading wrapped in `<active-suspense>`;
2. for each boundary whose data arrived, a `<template>` with its content and `<script>__activeSwap(n)</script>`, which swaps it in;
3. the end of the page.

The futures of the resources (plain `resource` and `ssr::resource`) run while the stream is polled, and a boundary is sent as soon as everything it waits for is there. Boundaries inside a streamed boundary are streamed in turn. Values of `ssr::resource` go out just before the content that uses them.

```rust
let mut stream = ssr::stream(|| Document::new(page()).client("/pkg/app.js", "/pkg/app_bg.wasm"));
while let Some(chunk) = stream.next_chunk().await {
    body.send(chunk).await?; // text/html; charset=utf-8
}
```

A stream belongs to the thread that created it (its states live in that thread's arena): drive it on that thread (a `tokio::task::LocalSet`, a single-threaded runtime) and forward the chunks to the response body through a channel. `poll_chunk` is there for adapters to any `Stream` trait.

To bound the wait, call `stream.now_or_never_all()` when your time limit is reached: it sends what is ready and closes the page; the boundaries still loading keep their fallback and the browser loads their data when it hydrates.

## Good to know

- The `<head>` is sent with the first chunk: put the page's `Seo` outside the streamed boundaries.
- The browser bundle starts once the whole document has arrived (module scripts wait for it), so hydration always sees the final HTML.
- Without streaming, `Document::render` and `ssr::render` render the fallback of what is not ready, as before.
