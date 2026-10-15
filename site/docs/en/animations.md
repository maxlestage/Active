---
title: Animated lists and actions
description: Animate the rows of a keyed list as they enter, leave and move (FLIP) with transition_group, and package DOM behaviour with use_action, Svelte's actions.
---

[`transition` and `transition_in`](../components/) animate one element. Lists need more: rows that **leave** must stay on screen while they fade out, and the rows around them should **slide** to their new place instead of jumping. That is Vue's `<TransitionGroup>` and Svelte's `animate:flip`; in `active`, `transition_group`.

```rust
use active::prelude::*;
use active::{transition_group, Dom};
```

## transition_group

Wrap a `keyed` (or `indexed`) list:

```rust
let todos = use_state(vec![/* … */]);
let list = ul().class("todos").child(transition_group(
    "todo",
    keyed(move || todos.get(), |t| t.id, |t| li().text(t.text.clone()).into()),
));
```

```css
.todo-enter-active, .todo-leave-active, .todo-move { transition: all .3s ease; }
.todo-enter-from, .todo-leave-to { opacity: 0; transform: translateX(2rem); }
/* Leaving rows leave the flow, so that the others slide into their place. */
.todo-leave-active { position: absolute; }
```

- an **added** row plays `todo-enter-from`, `todo-enter-active`, `todo-enter-to`;
- a **removed** row stays while it plays `todo-leave-from`, `todo-leave-active`, `todo-leave-to`, then goes;
- a row that **moves** (sorting, filtering, a row inserted above) slides from its old position with the class `todo-move`, using the FLIP technique: its old and new positions are measured, it is put back where it was with a `transform`, then the transform is removed under the transition.

Nothing plays when the list first appears or when the page is hydrated. On the server the list renders as usual. Make each row one element (`li`, `tr`, `div`).

## Actions: use_action

An action packages behaviour that needs the DOM element and must be undone: a tooltip, a click-outside handler, an `IntersectionObserver`, a third-party chart, focus trapping. `use_action` calls it once the element is in the page and calls what it returns when the element goes away (Svelte's `use:`, Vue's custom directives):

```rust
fn click_outside(close: State<bool>) -> impl Fn(&Dom) -> Box<dyn FnOnce()> {
    move |dom: &Dom| {
        // Browser: listen to `pointerdown` on the document, set `close` when the target
        // is outside `dom`…
        let _ = dom;
        Box::new(move || { /* remove the listener */ })
    }
}

let open = use_state(true);
let menu = div().class("menu").use_action(click_outside(open)).text("…");
```

Effects created inside an action follow states and stop with the element, so an action can react to a state (`effect(move || chart.update(data.get()))`). On the server, actions do nothing.

A function returning `impl FnOnce()` from `&Dom` needs `+ use<>` so that the cleanup does not borrow the element: `fn autofocus(dom: &Dom) -> impl FnOnce() + use<>`.
