---
title: Layouts, data loading and prefetching
description: Share layouts between pages, nest route tables, load each page's data before showing it, answer the right HTTP status or redirect, and prefetch the next page before the click.
---

[`Routes`](../client-app/) matches paths and keeps a page while only its parameters change. This page covers what full-stack frameworks (Next, Nuxt, SvelteKit, Remix) add on top: **layouts**, **nested tables**, **data loading per route**, **status codes and redirects**, and **prefetching**.

```rust
use active::prelude::*;
use active::{redirect, set_status, Prefetch, Routes};
```

## Layouts

`layout` wraps every page of a table, the page given as the `outlet`:

```rust
let routes = Routes::new()
    .layout(|outlet| {
        div().class("shell")
            .child(header().child(nav().child(a().href("/").text("Home"))))
            .child(main().child(outlet))
            .into()
    })
    .route("/", home)
    .route("/posts/:id", post);
```

In a client-side app (`routes.view(move || active::location().get())`) the layout is built **once**: going from page to page only replaces the outlet, so the layout's states — an open menu, a search field, an audio player — are kept.

## Nested tables

`nest(prefix, routes)` hands every path under `prefix` to another table, with patterns relative to the prefix and its own layout inside the parent's. The prefix's parameters reach the nested pages:

```rust
let user = Routes::new()
    .layout(|outlet| section().child(user_tabs()).child(outlet).into())
    .route("/", user_overview)          // /users/:id
    .route("/posts/:post", user_post);  // /users/:id/posts/:post — `id` is there too

let routes = Routes::new().route("/", home).nest("/users/:id", user);
```

The nested layout stays on screen while the visitor moves between the user's pages. `routes.matches(path)` looks into nested tables, so a path unknown to the nested table is a 404.

## Loading a page's data

`route_with_data(pattern, load, page)` loads the data first and builds the page with it, like SvelteKit's `load` or Remix's loaders. `loading` sets what shows meanwhile:

```rust
async fn load_post(id: u32) -> Option<Post> { /* … */ }

let routes = Routes::new()
    .loading(|| div().class("skeleton").into())
    .route_with_data(
        "/posts/:id",
        |params| load_post(params.parse("id").unwrap_or(0)),
        |post: State<Option<Post>>, params| match untrack(|| post.get()) {
            Some(p) => article().child(h1().text(p.title)).into(),
            None => { set_status(404); p().text("No such post").into() }
        },
    );
```

The page is built once its data is there, with a state that is updated in place when the visitor goes to the same route with other parameters. The data goes through [`ssr::resource`](../streaming/) under `"route:/posts/:id"`, so `ssr::render` and `ssr::stream` send it with the page and the browser does not load it again. `T` implements `Data`.

## Status codes and redirects

On the server, `respond(path)` builds the page and returns a `PageResponse` with the status, the redirect and the body:

```rust
let response = routes.respond(&path);
let html = Document::new(response.body).render();
// status: 200, 404 when no route matches, or what the page asked for
// location: Some("/login") after redirect("/login") (status 302 unless set_status gave a 3xx)
```

Inside a page, `set_status(410)` sets the status and `redirect("/login")` sends the visitor elsewhere. In the browser, `redirect` makes the router go there, replacing the history entry; `set_status` does nothing.

## Prefetching

`Document::prefetch(Prefetch::Hover)` loads the pages the visitor is about to open before the click. The page gets speculation rules (Chrome, Edge) and a small inline script for other browsers, which adds a `<link rel="prefetch">` for each same-site link once:

| `Prefetch` | When |
| --- | --- |
| `Hover` | the pointer rests on the link, or it gets the focus |
| `Press` | the link is pressed (mouse or finger down) |
| `Visible` | the link appears on screen |

`Document::prerender(when)` goes further in Chrome and Edge: the next page is rendered in the background and appears instantly. Keep it for pages without side effects on load. Links with `rel="nofollow"`, `download` or inside `data-no-prefetch` are skipped.

In a client-side app, links do not load documents: `active::on_link_hover(|to| …)` (feature `client`) calls you once per address when the visitor is about to follow a link, to start loading that page's data.
