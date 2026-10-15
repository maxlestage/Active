---
title: Async data and errors
description: Load data with resource, show a fallback with suspense while it loads, keep the old data during reloads, and replace a failed part of the page with error_boundary.
---

Pages wait for things: a user profile from an API, search results, a file read from IndexedDB. `active` gives that waiting a shape: a **resource** holds data loaded asynchronously, **suspense** shows a fallback until the data of a view is there, and an **error boundary** shows something else when a part of the page fails. The ideas come from Solid (`createResource`, `<Suspense>`, `<ErrorBoundary>`) and React (Suspense, error boundaries, transitions).

All examples on this page start with:

```rust
use active::prelude::*;
use active::{error_boundary, resource, suspense, ErrorBoundary, Resource};
```

## resource

`resource(source, fetch)` calls `fetch` with the value of `source` and stores what the returned future gives:

```rust
#[derive(Clone, PartialEq)]
struct User {
    name: String,
}

async fn load_user(id: u32) -> Result<User, String> {
    // In the browser: gloo-net, reqwest or web-sys's fetch.
    Ok(User { name: format!("User {id}") })
}

fn profile(id: State<u32>) -> Element {
    let user = resource(move || id.get(), load_user);
    div().children_dyn(move || match user.get() {
        None => vec![p().text("Loading…").into()],
        Some(Ok(user)) => vec![h2().text(user.name).into()],
        Some(Err(error)) => vec![p().class("error").text(error).into()],
    })
}
```

- `source` is followed like an effect: when `id` changes, `fetch` is called again with the new id. If an earlier request ends after a newer one, its result is ignored, so a slow answer never overwrites a recent one.
- The value is `None` until the first result arrives, then the latest result. Make `T` a `Result` to keep errors, as above.
- A `Resource` is `Copy`, like a state: move it into as many closures as needed.

| Method | What it does |
| --- | --- |
| `get()` / `with(f)` | The latest value (`None` before the first one); reading it subscribes the view. |
| `loading()` | Whether a load is in progress, the first one or a reload. |
| `refetch()` | Loads again with the current source: a "Refresh" or "Retry" button. |
| `set(value)` | Replaces the value at once, for an optimistic update. |
| `state()` | The `State<Option<T>>` holding the value, for a `memo` or a `watch`. |

### In the browser and on the server

In the browser (feature `client`), the future runs on the browser's event loop through `wasm-bindgen-futures`. On the server there is no event loop: the future is polled **once**. A value that is already there (a cache, `async { value }`, data the page was rendered with) is rendered at once; a real request stays loading, and the server renders the loading state or the suspense fallback. The browser then loads the data after hydration.

## Keeping the old data during a reload

While a reload runs, the previous value stays readable: the page keeps showing it instead of flashing back to "Loading…". Show a discreet indicator with `loading()`, as React's `useTransition` does:

```rust
fn results(query: State<String>) -> Element {
    let found = resource(move || query.get(), |q| async move { vec![format!("Result for {q}")] });
    div()
        .class_if("dimmed", move || found.loading())
        .children_dyn(move || {
            found
                .get()
                .unwrap_or_default()
                .into_iter()
                .map(|r| p().text(r).into())
                .collect()
        })
}
```

## suspense

`suspense(fallback, view)` shows `fallback` until every resource created by `view`, however deep, has its first value, then shows the view:

```rust
fn dashboard() -> Node {
    suspense(p().class("skeleton").text("Loading the dashboard…"), || {
        let stats = resource(|| (), |()| async { 42 });
        let news = resource(|| (), |()| async { String::from("Release 1.0") });
        div()
            .child(p().text_dyn(move || format!("{} visits", stats.get().unwrap_or(0))))
            .child(p().text_dyn(move || news.get().unwrap_or_default()))
            .into()
    })
}
```

- `view` is built once, right away, so its resources load in parallel.
- Only first loads count: a later reload keeps the view on screen with its previous data (use `loading()` there).
- Suspense boundaries nest: a resource counts in the closest one.
- On the server, a resource that is not ready at once leaves the fallback in the HTML.

## error_boundary

`error_boundary(view, fallback)` builds `view`, and replaces it with `fallback` as soon as an error is reported inside:

```rust
fn price(raw: &str) -> Result<Node, std::num::ParseFloatError> {
    let value: f64 = raw.parse()?;
    Ok(span().text(format!("{value:.2} €")).into())
}

fn product() -> Node {
    error_boundary(
        || div().child(price("nine")).into(),
        |boundary| {
            div()
                .class("error")
                .child(p().text(format!("Something went wrong: {}", boundary.errors().join(", "))))
                .child(button().text("Try again").on_click(move |_| boundary.reset()))
                .into()
        },
    )
}
```

Errors reach the closest boundary in three ways:

- **A `Result` in the view.** `Result<T, E>` converts into a `Node` when `T` does and `E: Display`: an `Ok` shows its node, an `Err` is reported and shows nothing. A component can simply return `Result<Node, MyError>` and use `?`.
- **`report_error(error)`** while building the view.
- **`ErrorBoundary::report(error)`** later, from an event handler or the end of a request: get the boundary with `use_context::<ErrorBoundary>()` while building, then move it into the closure.

```rust
fn save_button() -> Element {
    let boundary = use_context::<ErrorBoundary>();
    button().text("Save").on_click(move |_| {
        if let Some(boundary) = boundary {
            boundary.report("The server did not answer");
        }
    })
}
```

The fallback receives the `ErrorBoundary`: `errors()` lists the messages, `has_errors()` says whether there are any, and `reset()` forgets them and builds the view again. Everything the failed view created (states, effects, listeners) is released when the fallback replaces it.

Without a boundary, `report_error` writes the error to the browser console (or to the standard error on the server) and the `Err` shows nothing.

> Rust components do not throw exceptions. A **panic** is not caught: in the browser it stops the WebAssembly module. Return `Result`s for errors that can happen.

## Grouping writes: batch

When a result sets several states, wrap the writes in `active::batch` so that the views reading them update once (see [State and effects](../state/)):

```rust
fn load_more(items: State<Vec<String>>, page: State<u32>, more: Vec<String>) {
    active::batch(|| {
        items.update(|list| list.extend(more));
        page.update(|p| *p += 1);
    });
}
```
