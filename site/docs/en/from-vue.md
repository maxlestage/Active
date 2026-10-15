---
title: Coming from Vue
description: What each Vue feature is called in active, from ref and computed to v-model, provide/inject, watch and Vue Router, with a complete component written both ways.
---

If you know Vue 3 and its Composition API, you already know how `active` works: a component sets up its reactive state once, and the view follows it with fine-grained updates. There is no virtual DOM and no re-render: like Vue's compiled templates (and Vapor mode), only the text, attribute or class that depends on a state is updated. The difference is that everything is plain Rust: a component is a function, the template is a chain of method calls, and the compiler checks it all.

## Vue to active

| Vue | active |
| --- | --- |
| `ref(0)`, `reactive({…})` | `use_state(0)`, `use_state(MyStruct {…})` |
| `computed(() => …)` | `memo(move \|\| …)`, or a plain `move \|\|` closure |
| `watch(source, (new, old) => …)` | `watch(move \|\| …, move \|new, old\| …)` |
| `watch(…, { immediate: true })` | `watch_immediate(…)` |
| `watchEffect(() => …)` | `effect(move \|\| …)` |
| `onMounted` | `.on_mount(\|el\| …)` on an element |
| `onUnmounted` | `on_cleanup(move \|\| …)` |
| `provide` / `inject` | `provide(value, \|\| view)` / `use_context::<T>()` |
| props | function arguments, or a props struct |
| `emit('save')` | a callback argument: `on_save: impl Fn() + 'static` |
| slots, named slots | `Node` / `Element` arguments (`children: Vec<Node>`) |
| scoped slots | a closure argument: `row: impl Fn(&Item) -> Node` |
| `{{ text }}` | `.text_dyn(move \|\| …)` |
| `v-if` / `v-else` | `.children_dyn(move \|\| if … { … } else { … })` |
| `v-show` | `.bool_attr("hidden", move \|\| …)` |
| `v-for` with `:key` | `.children_keyed(items, key, row)` |
| `v-model` | `.bind_value(state)`, `.bind_checked(state)` |
| `v-model.number` | `.bind_parsed(state)` |
| `<Transition>` with `v-show` | `.transition("fade", move \|\| shown.get())` ([Components](../components/#transitions)) |
| `<Teleport to="body">` | `portal("body", view)` |
| `<Suspense>` and async setup | `suspense(fallback, \|\| view)` and `resource(source, fetch)` ([Async data](../async-data/)) |
| `router.replace`, `beforeEach` | `navigate_replace(to)`, `before_navigate(guard)` |
| VeeValidate, `useField` | `active::forms` ([Forms](../forms/)) |
| `:class="{ active: on }"` | `.class_if("active", move \|\| …)` |
| `:style="{ width: w }"` | `.style_dyn("width", move \|\| …)` |
| `:attr="value"` | `.attr_dyn("attr", move \|\| …)` |
| `@click` | `.on_click(move \|e\| …)` |
| `@submit.prevent` | `.on_submit(move \|e\| …)` |
| `@keydown.enter` | `.on_key("Enter", move \|e\| …)` |
| `.stop` | `e.stop_propagation()` |
| `v-html` | `.html(…)`, `.html_dyn(…)` |
| Vue Router routes, `:id`, `*` | `Routes::new().route("/posts/:id", page)`, `match_path` |
| `<router-view>` | `routes.view(move \|\| active::location().get())` |
| `router.push('/x')` | `active::navigate("/x")` |
| `route.params.id` | `params.with(\|p\| p.get("id") …)` |
| `route.query.q` | `active::query_param("q")` |
| Pinia store | a `Copy` struct of `State`s given with `provide` |
| `useHead` (Unhead) | a `Seo` node anywhere in the view |
| Nuxt SSR + hydration | `Document::new(view).render()` + `island` / `hydrate` |

## A component, both ways

The Vue component below keeps a list of to-dos, a draft bound to a field, a computed count and a watcher:

```html
<script setup>
const todos = ref([{ id: 1, text: 'Write docs', done: false }])
const draft = ref('')
const left = computed(() => todos.value.filter(t => !t.done).length)
const finished = ref(false)
watch(left, (now, before) => { finished.value = now === 0 && before > 0 })
function add() { todos.value.push({ id: todos.value.length + 1, text: draft.value, done: false }); draft.value = '' }
</script>

<template>
  <div>
    <form @submit.prevent="add"><input v-model="draft"></form>
    <ul><li v-for="t in todos" :key="t.id" :class="{ done: t.done }">{{ t.text }}</li></ul>
    <p v-show="left > 0">{{ left }} left</p>
  </div>
</template>
```

The same component in `active`:

```rust
use active::prelude::*;

/// <script setup> + <template>, in one function.
fn todo_list() -> Element {
    let todos = use_state(vec![(1u32, "Write docs".to_string(), false)]);
    let draft = use_state(String::new());                                  // ref('')
    let left = memo(move || todos.with(|t| t.iter().filter(|t| !t.2).count())); // computed
    let finished = use_state(false);
    watch(move || left.get(), move |now, before| {                        // watch
        finished.set(*now == 0 && *before > 0);
    });
    div()
        .child(
            form()
                .on_submit(move |_| {                                      // @submit.prevent
                    let text = draft.get();
                    todos.update(|t| t.push((t.len() as u32 + 1, text, false)));
                    draft.set(String::new());
                })
                .child(input().bind_value(draft)),                         // v-model
        )
        .child(ul().children_keyed(                                         // v-for :key
            move || todos.get(),
            |t| t.0,
            |t| li().class(if t.2 { "done" } else { "" }).text(t.1.clone()).into(),
        ))
        .child(p().bool_attr("hidden", move || left.get() == 0)            // v-show
            .text_dyn(move || format!("{} left", left.get())))             // {{ }}
}
```

## What is different

- **Setup runs once, the view is code.** The body of the function is Vue's `setup`: it runs once per component. What changes later lives in closures (`text_dyn`, `class_if`, `children_dyn`…), which the browser re-runs when a state they read changes.
- **States are `Copy` handles.** `use_state` returns a small `State<T>` you move into as many closures as you like; there is no `.value` and no proxy: read with `.get()` or `.with(|v| …)`, write with `.set(…)` or `.update(|v| …)`.
- **Server first.** Pages are rendered to HTML on the server with every SEO tag, and only islands are hydrated, like Nuxt with islands. Effects and watchers never run on the server.
- **Types instead of runtime checks.** Props are function arguments and structs, emits are callbacks, slots are `Node` arguments: a missing prop or a misspelled event is a compile error.

## Not built in yet

- **`<TransitionGroup>`, and leave transitions of elements removed by a dynamic region**: `.transition` animates an element that stays in the page (shown and hidden); keep a leaving element with `.transition(name, …)` and remove it afterwards.
- **`<KeepAlive>`**: `switch` and `Routes::view` keep a page while only its parameters change; keep data that must survive a page change in a state created above the router.

For React, Svelte, Solid, Angular, Astro and the others, see [Coming from other frameworks](../from-others/).
