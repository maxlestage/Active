---
title: Persisted state and debugging
description: Keep a state in localStorage or sessionStorage, in sync between tabs, with persisted, and watch the reactive runtime with active::debug (live states and effects, named effects that count and log their runs).
---

```rust
use active::prelude::*;
use active::{debug, persisted, persisted_in, Storage};
```

## persisted

`persisted(key, initial)` is a state kept in `localStorage`: a theme, a cart, the last search, a draft. It starts with the stored value (or `initial`), every change is written back, and a change made in another tab of the site updates it here too (Svelte's persisted stores, VueUse's `useLocalStorage`, Zustand's `persist`):

```rust
let dark = persisted("theme-dark", false);
let toggle = button()
    .on_click(move |_| dark.update(|d| *d = !*d))
    .text_dyn(move || if dark.get() { "Light".into() } else { "Dark".into() });
```

`persisted_in(Storage::Session, key, initial)` uses `sessionStorage` instead: kept across reloads, one per tab, not shared.

Values are written as JSON with the [`Data` trait](../streaming/) (strings, numbers, `bool`, `Option`, `Vec`, maps, tuples, and your own types). A stored value that cannot be read back (an older format) is replaced by `initial`.

On the server, and where storage is blocked, it is a plain state holding `initial`: the server renders `initial`, and the browser shows the stored value once it hydrates. For values that change the layout a lot, render them inside a `transition` or keep the server and the browser in agreement with a cookie.

Derived values need nothing new: a [`memo`](../state/) of a persisted state is a derived store.

## Debugging: active::debug

`debug::stats()` counts what the reactive runtime of this thread holds:

```rust
let stats = debug::stats();
println!("{stats}"); // "1520 states, 310 effects, 98211 effect runs"
debug::log_stats("after closing the dialog"); // to the console (the terminal natively)
```

Open and close a dialog, change pages a few times: numbers that keep growing mean something is never released — usually a state created outside any component inside a region that is rebuilt often.

`debug::effect(name, f)` is an `effect` that counts its runs under `name`; `debug::runs(name)` reads the count, and after `debug::log(true)` every run is written to the console (`active: effect "cart total" ran (3)`). It shows what re-runs when a state changes, and how often:

```rust
debug::log(true);
debug::effect("cart total", move || total.set(cart.with(|c| c.iter().map(|i| i.price).sum())));
```
