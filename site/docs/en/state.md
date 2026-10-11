---
title: State and effects
description: Keep changing values in use_state handles, bind them to text, attributes, classes and lists, run effects in the browser, and learn how state is freed.
---

Interactive parts of a page need values that change: a counter, the text of a search field, the items of a list. In `active` such a value lives in a **state**, created with `use_state`. The view reads it through small closures, and when the state changes, only the parts of the page that read it are updated.

## use_state

```rust
use active::prelude::*;

let count = use_state(0);
count.set(5);
count.update(|n| *n += 1);
assert_eq!(count.get(), 6);
```

`use_state(initial)` stores the value and returns a `State<T>`, a handle to it. Any `'static` type works: numbers, `String`, `Vec`, your own structs and enums.

| Method | Use it to |
| --- | --- |
| `.get()` | get a copy of the value (requires `T: Clone`) |
| `.with(\|v\| …)` | read through a reference, without cloning (`items.with(Vec::len)`) |
| `.set(value)` | replace the value |
| `.update(\|v\| …)` | change the value in place (`list.push(…)`, `*n += 1`) |

`set` and `update` notify everything that read the state, which updates the page.

## Why handles are Copy

A `State<T>` is only an index into a store, so it implements `Copy` whatever `T` is. You can move the same handle into as many closures as you need, without `Rc`, `clone()` or lifetimes:

```rust
use active::prelude::*;

fn counter() -> Element {
    let count = use_state(0);
    div()
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
}
```

Each `move` closure gets its own copy of the handle; all copies point to the same value.

## Binding state to the view

Methods ending in `_dyn` (and `class_if`, `bool_attr`) take a closure instead of a value. On the server the closure runs once to produce the HTML. In the browser it runs again every time a state it read changes, and only that text node, attribute or class is updated.

| Method | Updates |
| --- | --- |
| `.text_dyn(f)` or `text_dyn(f)` | a text node |
| `.attr_dyn(name, f)` | an attribute (`value` also updates the field's current value) |
| `.bool_attr(name, f)` | a boolean attribute such as `disabled`, `hidden`, `open` (`checked` also updates the checkbox) |
| `.class_if(class, f)` | one class, added while `f()` is `true` |
| `.html_dyn(f)` | the element's inner HTML (trusted HTML only) |
| `.children_dyn(f)` or `fragment_dyn(f)` | a group of nodes, rebuilt (see below) |

```rust
use active::prelude::*;

let stock = use_state(3);
let view = div()
    .class("product")
    .class_if("sold-out", move || stock.get() == 0)
    .attr_dyn("data-stock", move || stock.get().to_string())
    .child(button().bool_attr("disabled", move || stock.get() == 0).text("Buy"))
    .child(span().text_dyn(move || format!("{} left", stock.get())));

assert_eq!(
    Node::from(view).render(),
    r#"<div class="product" data-stock="3"><button>Buy</button><span><!--#-->3 left</span></div>"#
);
```

The `<!--#-->` comment marks dynamic text so the browser can find it when it [hydrates](../islands/) the page.

> **Tip:** To show or hide something, `bool_attr("hidden", …)` or `class_if(…)` is the cheapest option: the element stays in the page and only an attribute changes.

## Derived values

A value computed from states is just a closure. Since it captures `Copy` handles, the closure is `Copy` too and can be used in several places:

```rust
use active::prelude::*;

let price = use_state(120u32);
let quantity = use_state(2u32);
let total = move || price.get() * quantity.get();

let view = p()
    .text_dyn(move || format!("Total: {} €", total()))
    .class_if("expensive", move || total() > 200);
assert!(Node::from(view).render().contains("Total: 240 €"));
```

Anything that reads `total()` is updated when either state changes.

When the inputs change often but the result rarely does, store the result in a **memo**: `memo(f)` returns a `State` that is written only when `f`'s new result differs from the previous one, so what reads it re-runs only then.

```rust
use active::prelude::*;

let positions = use_state(vec![("VER", 1), ("NOR", 2)]);
// Updated every second while the names (and so this menu) rarely change.
let names = memo(move || positions.with(|p| p.iter().map(|(n, _)| *n).collect::<Vec<_>>()));
let menu = select().children_dyn(move || names.get().into_iter().map(|n| option().text(n).into()).collect());
```

## Lists and conditions that change

`.children_dyn(f)` appends a **dynamic region** to an element: `f` returns the nodes to show, and whenever a state it read changes, the region is rebuilt. Children added before or after the region stay where they are. `fragment_dyn(f)` creates the same region as a standalone node.

```rust
use active::prelude::*;

let items = use_state(vec!["Milk".to_string(), "Bread".to_string()]);
let view = div()
    .child(ul().child(li().text("Always first")).children_dyn(move || {
        items.with(|list| list.iter().map(|item| li().text(item.clone()).into()).collect())
    }))
    .child(button().on_click(move |_| items.update(|list| list.push("Eggs".into()))).text("Add eggs"));

assert_eq!(
    Node::from(view).render(),
    "<div><ul><li>Always first</li><!--[--><li>Milk</li><li>Bread</li><!--]--></ul><button>Add eggs</button></div>"
);
```

On the server the region is written between `<!--[-->` and `<!--]-->` markers. In the browser:

- only the region is rebuilt, the rest of the page is untouched;
- the states, effects and event listeners created while building the previous content are released; listeners are removed from their elements first, so an event still on its way (the click that rebuilt the region, bubbling up through the old elements) never reaches them;
- states created **outside** the region (like `items` above) survive, which is how a list keeps its data while its items are rebuilt;
- the focus stays put: if it was inside the region, the element that takes the focused one's place gets it back (the one with the same `id`, or else the one at the same position with the same tags and classes; a text field also keeps its selection when its text is unchanged). A menu rebuilt in another language keeps the keyboard where it was; another page, built from other elements, takes nothing.

A condition that changes is a region that returns either some nodes or none:

```rust
use active::prelude::*;

fn details() -> Element {
    let open = use_state(false);
    div()
        .child(button().on_click(move |_| open.update(|o| *o = !*o)).text("Details"))
        .children_dyn(move || {
            if open.get() {
                vec![p().text("Delivered in 48 hours.").into()]
            } else {
                Vec::new()
            }
        })
}
```

### Long lists: keyed rows

A `children_dyn` region is rebuilt as a whole. That is fast for the lists of a typical page (tens of items) and the simplest choice. For long or interactive lists, `children_keyed` (or the free function `keyed`) keeps each row by key: when the items change, a row whose key is still there and whose item is equal keeps its DOM, its states, its listeners and the focus, and moves if the order changed. Only new or changed rows are built, and removed ones are released.

```rust
use active::prelude::*;

#[derive(Clone, PartialEq)]
struct Todo {
    id: u32,
    text: String,
    done: bool,
}

fn list(todos: State<Vec<Todo>>) -> Element {
    ul().children_keyed(
        move || todos.get(),
        |todo| todo.id,
        |todo| {
            li().class(if todo.done { "done" } else { "" })
                .text(todo.text.clone())
                .into()
        },
    )
}
```

- The first closure returns the items (reading states subscribes the list), the second a key that identifies an item over time (an id, not a position), the third builds a row.
- The item type implements `PartialEq`: a row whose item changed is rebuilt, so it always shows the current values.
- Make each row one element (`li`, `tr`…).
- With `children_dyn`, keep text fields **outside** the region: they are recreated on each rebuild and lose focus. A keyed row that does not change keeps its field.

### Rows by position: indexed

`children_keyed` follows each item: when an item changes, its row is rebuilt. For lists whose items change **in place** — a ranking refreshed every second, the results of another season shown in the same table — `children_indexed` (or the free function `indexed`) keeps each row by **position** and never rebuilds it: row `i` is built once with a `State` holding the `i`-th item, and only the texts, attributes and classes bound to that state are updated. Rows are added or removed at the end when the list grows or shrinks.

```rust
use active::prelude::*;

#[derive(Clone, PartialEq)]
struct Line {
    driver: String,
    gap: String,
}

fn timing(lines: State<Vec<Line>>) -> Element {
    ol().children_indexed(
        move || lines.get(),
        |line| {
            li().child(span().text_dyn(move || line.with(|l| l.driver.clone())))
                .child(span().class("gap").text_dyn(move || line.with(|l| l.gap.clone())))
                .into()
        },
    )
}
```

Rows keep their DOM, so CSS entrance animations do not replay and a field keeps its focus. Pick `indexed` when positions matter more than identities, `keyed` when rows must follow their item (a to-do list being reordered).

## Effects

`effect(f)` runs `f` once, then again whenever a state it read changes. Use it for work outside the view: saving to `localStorage`, setting a timer, logging, calling a browser API.

```rust
use active::prelude::*;

/// Saves the draft in the browser. Effects never run on the server, but the code must
/// compile there too, hence the empty version.
#[cfg(target_arch = "wasm32")]
fn save(text: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("draft", text);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn save(_text: &str) {}

fn editor() -> Element {
    let draft = use_state(String::new());
    effect(move || draft.with(|text| save(text)));
    textarea().on_input(move |e| draft.set(e.value()))
}
```

The browser-only code uses [`web-sys`](https://docs.rs/web-sys), declared with the features it needs: `web-sys = { version = "0.3", features = ["Storage", "Window"] }` under the `wasm32` dependencies.

How effects behave:

- **Browser only.** On the server `effect` does nothing at all, so an effect can freely use browser APIs. Never compute something the HTML needs in an effect; compute it in the view.
- **Automatic dependencies.** An effect depends on the states it read during its last run. If it reads `a` only when `flag` is true, it stops reacting to `a` once `flag` is false.
- **No glitches.** When a state changes, the memos computed from it are brought up to date before the effects that read them run, so an effect never sees a value and a memo derived from it out of step. Each effect runs once per change, and a memo is recomputed only when one of its inputs really changed.
- **One at a time.** A state written inside an effect notifies its readers once that effect has finished (and before the `set` that started it all returns).
- **No infinite loops.** An effect that writes a state it reads does not trigger itself again; two effects that each write what the other reads are stopped after a thousand runs.
- **Released with their region.** Effects created inside a dynamic region stop when the region is rebuilt.

## Cleaning up

A component sometimes starts something that lives outside the view: a timer, a WebSocket, an animation loop, a listener on `window`. `on_cleanup(f)` runs `f` when the component is released, that is when the dynamic region that built it re-renders or the router shows another page:

```rust
use active::prelude::*;

fn clock() -> Element {
    let seconds = use_state(0u32);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, closure::Closure};
        let tick = Closure::<dyn Fn()>::new(move || seconds.update(|s| *s += 1));
        let window = web_sys::window().expect("window");
        let id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 1000)
            .unwrap_or(0);
        on_cleanup(move || {
            window.clear_interval_with_handle(id);
            drop(tick);
        });
    }
    p().text_dyn(move || format!("{} s on this page", seconds.get()))
}
```

Cleanups run before the component's states are released, so they can still read them. On the server, `f` never runs.

## Reading without following: untrack

Inside a dynamic region, every state read subscribes the **whole region**. A component built inside the region that reads one of its own states directly (not in a closure) would be rebuilt, with fresh states, each time that state changes. Read states in closures (`text_dyn`, `children_dyn`…), and wrap a read that is only needed once in `untrack`:

```rust
use active::prelude::*;

let (page, size) = (use_state(1), use_state(20));
let view = ul().children_dyn(move || {
    let size = untrack(|| size.get()); // read once, not followed
    (0..size).map(|i| li().text(format!("Item {}", (page.get() - 1) * size + i + 1)).into()).collect()
});
```

Only what the region's closure reads decides when it re-renders: building and mounting its nodes (`on_mount` callbacks, nested components) is already untracked.

## Async results: is_alive

A network request may finish after the user left the page. `State::is_alive()` is `false` once the state was released: check it before writing a result.

```rust,ignore
fn weather(city: String) -> Element {
    let forecast = use_state(None::<String>);
    wasm_bindgen_futures::spawn_local(async move {
        let text = fetch_forecast(&city).await; // your own async function
        if forecast.is_alive() {
            forecast.set(Some(text));
        }
    });
    p().text_dyn(move || forecast.get().unwrap_or_else(|| "Loading…".into()))
}
```

## Sharing state

To share a value between components, create the state in their common parent and pass the handle down as an argument:

```rust
use active::prelude::*;

fn search_box(query: State<String>) -> Element {
    input().attr("type", "search").on_input(move |e| query.set(e.value()))
}

fn result_count(query: State<String>) -> Element {
    p().text_dyn(move || format!("Searching for “{}”", query.get()))
}

fn search() -> Element {
    let query = use_state(String::new());
    div().child(search_box(query)).child(result_count(query))
}
```

Separate [islands](../islands/) do not share states: each island is its own small app. If two parts of a page must talk to each other, put them in the same island.

## On the server

On the server, nothing changes once a page is rendered, and effects and event handlers are not even stored. States still take memory, so `active` releases them for you:

- states live in a store that belongs to the current thread;
- at the end of each top-level render (`Node::render`, `Node::render_into` or `Document::render`), the states that neither this render nor the previous one read or created are released.

The states of the page being rendered stay available: you can render the same view several times, read its states after rendering, or print a `Node` with `{:?}`. The states of finished requests are released after the next renders on the same thread, so a server that renders millions of pages keeps only a handful of live states.

One rule follows: **build and render the view without an `.await` in between.** In an async handler, load your data first, then build the view and render it in one synchronous step. A `State` belongs to the thread that created it, so it is not `Send`: in a multi-threaded server such as Vitesse, keeping one across an `.await` is a compile error, not a runtime surprise. See [Vitesse, Async data](../vitesse/#async-data).

## Scopes

Advanced: `active::scope(f)` runs `f` and returns its result together with a `Scope` that owns every state and effect created inside. `Scope::dispose()` releases them. Dynamic regions use this internally; you need it only if you mount and remove views by hand in the browser.

```rust
use active::{scope, use_state};

let (count, owner) = scope(|| use_state(1));
assert_eq!(count.get(), 1);
owner.dispose(); // `count` can no longer be used
```

On the server, a render that happens inside an open `scope` releases no state: dispose the scope yourself.

## Common mistakes

### Reading a state inside its own update

`count.update(|n| *n = count.get() + 1)` panics with "a state was read inside its own `update`; read it before updating". Use the closure's argument, or read first and then write:

```rust
use active::use_state;

let count = use_state(1);
let step = use_state(10);

count.update(|n| *n += step.get()); // the argument, plus another state: fine
let doubled = count.get() * 2;      // read first…
count.set(doubled);                 // …then write
assert_eq!(count.get(), 22);
```

Likewise, calling `set` or `update` on a state inside its own `with` panics ("a state was updated while being read or updated").

### Using a released state

A state created inside a dynamic region is released when the region is rebuilt; using it afterwards panics with "this state was disposed". Keep long-lived data in a state created above the region.

### Computing HTML in an effect

Effects do not run on the server, so anything they produce is missing from the HTML that search engines see. Compute it in the view instead.
