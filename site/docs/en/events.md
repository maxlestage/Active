---
title: Events and forms
description: Handle clicks, typing, keys and form submissions with on, on_click and on_input, build controlled forms, and reach the DOM with on_mount and web-sys.
---

Event handlers make a view interactive: a click updates a state, typing filters a list, submitting a form sends data. Handlers are Rust closures attached with `.on(...)`. They run in the browser once the view is [hydrated](../islands/) or [mounted](../client-app/); on the server they are accepted and ignored, so the same component compiles for both.

## Listening to events

```rust
use active::prelude::*;

fn like_button() -> Element {
    let likes = use_state(0);
    button()
        .attr("type", "button")
        .on_click(move |_| likes.update(|n| *n += 1))
        .text_dyn(move || format!("♥ {}", likes.get()))
}
```

| Method | Listens to |
| --- | --- |
| `.on(name, handler)` | any DOM event: `"submit"`, `"change"`, `"keydown"`, `"focus"`, `"mouseenter"`… |
| `.on_click(handler)` | `"click"` |
| `.on_input(handler)` | `"input"`, fired on every keystroke in a field |

A handler is any `Fn(Event)` closure. Because it is `Fn` (it may run many times), it changes the page through [states](../state/), whose `Copy` handles can be moved in freely.

## The Event value

| Method | Returns |
| --- | --- |
| `e.value()` | the current value of the `<input>`, `<textarea>` or `<select>` that fired the event (`""` otherwise) |
| `e.key()` | the key of a keyboard event: `"Enter"`, `"Escape"`, `"ArrowDown"`, `"k"`… (`""` otherwise) |
| `e.prevent_default()` | cancels the browser's default action: following a link, submitting a form… |
| `e.stop_propagation()` | stops the event from reaching the listeners of the parent elements |
| `e.checked()` | whether the checkbox or radio button that fired the event is checked (`false` otherwise) |
| `e.current_target()` | the element the listener is attached to, as an `Option<Dom>` |
| `e.raw()` | the underlying `web_sys::Event` (feature `client` only) |

`Dom` is `web_sys::Element` in the browser and an empty placeholder on the server, where `current_target()` returns `None`.

## Text fields

Read what the user types with `on_input` and `e.value()`. To also control the field from your code (clear it, fill it), bind its value with `attr_dyn("value", …)`: in the browser this sets the field's current value, not only the attribute.

```rust
use active::prelude::*;

fn name_field() -> Element {
    let name = use_state(String::new());
    div()
        .child(
            input()
                .attr("type", "text")
                .attr("placeholder", "Your name")
                .attr_dyn("value", move || name.get())
                .on_input(move |e| name.set(e.value())),
        )
        .child(button().attr("type", "button").on_click(move |_| name.set(String::new())).text("Clear"))
        .child(p().text_dyn(move || format!("Hello, {}!", name.get())))
}
```

`<textarea>` works the same way.

## Checkboxes and selects

Bind a checkbox with `bool_attr("checked", …)`, which also sets its checked state in the browser, and a `<select>` with `attr_dyn("value", …)`:

```rust
use active::prelude::*;

fn preferences() -> Element {
    let newsletter = use_state(true);
    let size = use_state("m".to_string());
    form()
        .child(
            label()
                .child(
                    input()
                        .attr("type", "checkbox")
                        .bool_attr("checked", move || newsletter.get())
                        .on("change", move |_| newsletter.update(|on| *on = !*on)),
                )
                .text(" Send me the newsletter"),
        )
        .child(
            select()
                .attr_dyn("value", move || size.get())
                .on("change", move |e| size.set(e.value()))
                .children([("s", "Small"), ("m", "Medium"), ("l", "Large")].map(|(value, label)| {
                    option()
                        .attr("value", value)
                        .bool_attr("selected", move || size.get() == value)
                        .text(label)
                })),
        )
}
```

HTML has no `value` attribute on `<select>`, so the server marks the chosen `<option>` with `selected`; in the browser, the `value` binding keeps the menu in sync afterwards.

## Two-way binding

Writing `attr_dyn("value", …)` and `on_input` for every field gets repetitive. `bind_value(state)` does both, like Vue's `v-model`: the field shows the state, and what the user types goes back into it.

```rust
use active::prelude::*;

fn profile() -> Element {
    let name = use_state(String::from("Ada"));
    let bio = use_state(String::new());
    let newsletter = use_state(true);
    let age = use_state(36u32);
    form()
        .child(input().attr("type", "text").bind_value(name))
        .child(textarea().bind_value(bio))
        .child(input().attr("type", "checkbox").bind_checked(newsletter))
        .child(input().attr("type", "number").bind_parsed(age))
        .child(p().text_dyn(move || format!("{}, {}", name.get(), age.get())))
}

assert!(Node::from(profile()).render().contains(r#"<input type="number" value="36"/>"#));
```

| Method | Binds | Vue |
| --- | --- | --- |
| `.bind_value(State<String>)` | the text of an `<input>` or a `<textarea>`, the value of a `<select>` (written on `input`, on `change` for a `<select>`) | `v-model` |
| `.bind_checked(State<bool>)` | a checkbox (written on `change`) | `v-model` on a checkbox |
| `.bind_parsed(State<T>)` | a number, or any `T: FromStr + Display + PartialEq`: written when the text parses; otherwise the state keeps its last value and the text typed is left alone | `v-model.number` |

The server renders the current value (`value="…"`, `checked`, the text of a `<textarea>`), so the form is filled before the WebAssembly loads. For a `<select>`, also mark the chosen `<option>` with `bool_attr("selected", …)`, as above.

## Submitting a form

Handle `"submit"` on the `<form>` and call `prevent_default()` so the browser does not load a new page:

```rust
use active::prelude::*;

fn newsletter_form() -> Element {
    let email = use_state(String::new());
    let sent = use_state(false);
    form()
        .attr("action", "/subscribe")
        .attr("method", "post")
        .on("submit", move |e| {
            e.prevent_default();
            if email.with(|v| v.contains('@')) {
                sent.set(true);
                email.set(String::new());
            }
        })
        .child(
            input()
                .attr("type", "email")
                .attr("name", "email")
                .attr_dyn("value", move || email.get())
                .on_input(move |e| email.set(e.value())),
        )
        .child(button().text("Subscribe"))
        .child(p().bool_attr("hidden", move || !sent.get()).text("Thanks, see you soon!"))
}
```

> **Tip:** Keep a real `action` and `method` on forms rendered by the server. Before the WebAssembly bundle has loaded (or if it fails), the form still works as a plain HTML form; once it is hydrated, your handler takes over.

## Keyboard

```rust
use active::prelude::*;

fn search_field(open: State<bool>) -> Element {
    input().attr("type", "search").on("keydown", move |e| {
        if e.key() == "Escape" {
            e.prevent_default();
            open.set(false);
        }
    })
}
```

## Modifiers

Vue's event modifiers have short equivalents:

| Vue | active |
| --- | --- |
| `@submit.prevent="save"` | `.on_submit(save)`: cancels the page load, then calls `save` |
| `@keydown.enter="send"` | `.on_key("Enter", send)`, with any key name: `"Escape"`, `"ArrowDown"`, `"k"`… |
| `@click.stop="…"` | `.on_click(move \|e\| { e.stop_propagation(); … })` |
| `@click.prevent="…"` | `.on_click(move \|e\| { e.prevent_default(); … })` |

```rust
use active::prelude::*;

fn chat() -> Element {
    let message = use_state(String::new());
    let sent = use_state(Vec::<String>::new());
    let send = move || {
        let text = message.get();
        if !text.trim().is_empty() {
            sent.update(|list| list.push(text));
            message.set(String::new());
        }
    };
    form()
        .on_submit(move |_| send())
        .child(
            textarea()
                .bind_value(message)
                .on_key("Escape", move |_| message.set(String::new())),
        )
        .child(button().text("Send"))
}
```

## Reaching the DOM

### on_mount

`.on_mount(f)` runs `f` with the element once it is in the page, in the browser only: when an island is hydrated, or when a view is mounted. Use it to focus a field, draw on a canvas or start an animation.

```rust
use active::prelude::*;

fn focused_input() -> Element {
    input().attr("type", "text").on_mount(|_element| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(input) = _element.dyn_ref::<web_sys::HtmlElement>() {
                let _ = input.focus();
            }
        }
    })
}
```

### web-sys

`active` covers what components usually need. For the rest of the browser APIs, add [`web-sys`](https://docs.rs/web-sys) to the `wasm32` dependencies with the features you use, and keep that code behind `#[cfg(target_arch = "wasm32")]` so the server build ignores it:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = ["HtmlElement", "Storage", "Window"] }
```

`e.raw()` gives the full `web_sys::Event`, for example to stop propagation:

```rust
use active::prelude::*;

fn inner_button() -> Element {
    button().on_click(|_e| {
        #[cfg(target_arch = "wasm32")]
        _e.raw().stop_propagation();
    })
}
```

The underscore in `_e` avoids an "unused variable" warning in the server build.

## Async work in handlers

A handler cannot be `async`, but it can start a future with [`wasm-bindgen-futures`](https://docs.rs/wasm-bindgen-futures) and write the result into a state when it completes:

```rust
use active::prelude::*;

#[cfg(target_arch = "wasm32")]
fn load_text(url: &'static str, target: State<String>) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::{JsFuture, spawn_local};

    spawn_local(async move {
        let Some(window) = web_sys::window() else { return };
        let Ok(response) = JsFuture::from(window.fetch_with_str(url)).await else { return };
        let Ok(response) = response.dyn_into::<web_sys::Response>() else { return };
        let Ok(body) = response.text() else { return };
        if let Ok(text) = JsFuture::from(body).await {
            target.set(text.as_string().unwrap_or_default());
        }
    });
}

fn quote() -> Element {
    let text = use_state(String::from("Click to load a quote."));
    div()
        .child(p().text_dyn(move || text.get()))
        .child(button().on_click(move |_| {
            #[cfg(target_arch = "wasm32")]
            load_text("/api/quote", text);
        }).text("Load"))
}
```

This needs `wasm-bindgen-futures = "0.4"` and the web-sys features `Response` and `Window`. The docs search of this website works this way: see `site/src/client/search.rs`.

> **Warning:** When the future completes, the state must still exist. If the component may be removed meanwhile (it sits in a dynamic region that can be rebuilt), create the state above that region.

## Links

Links are plain `a().href(...)` elements. In a [client-side app](../client-app/), the router started by `active::location()` follows same-site links without reloading the page, and `active::navigate("/path")` changes page from a handler.
