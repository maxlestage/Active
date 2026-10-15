---
title: Si vienes de Vue
description: Cómo se llama cada funcionalidad de Vue en active, de ref y computed a v-model, provide/inject, watch y Vue Router, con un componente completo escrito de las dos formas.
---

Si conoces Vue 3 y su Composition API, ya sabes cómo funciona `active`: un componente prepara su estado reactivo una vez, y la vista lo sigue con actualizaciones de grano fino. No hay DOM virtual ni re-renderizado: como las plantillas compiladas de Vue (y el modo Vapor), solo se actualiza el texto, el atributo o la clase que depende de un estado. La diferencia es que todo es Rust normal: un componente es una función, la plantilla es una cadena de llamadas a métodos, y el compilador lo comprueba todo.

## De Vue a active

| Vue | active |
| --- | --- |
| `ref(0)`, `reactive({…})` | `use_state(0)`, `use_state(MyStruct {…})` |
| `computed(() => …)` | `memo(move \|\| …)`, o un simple closure `move \|\|` |
| `watch(source, (new, old) => …)` | `watch(move \|\| …, move \|new, old\| …)` |
| `watch(…, { immediate: true })` | `watch_immediate(…)` |
| `watchEffect(() => …)` | `effect(move \|\| …)` |
| `onMounted` | `.on_mount(\|el\| …)` en un elemento |
| `onUnmounted` | `on_cleanup(move \|\| …)` |
| `provide` / `inject` | `provide(value, \|\| view)` / `use_context::<T>()` |
| props | argumentos de función, o un struct de props |
| `emit('save')` | un argumento callback: `on_save: impl Fn() + 'static` |
| slots, slots con nombre | argumentos `Node` / `Element` (`children: Vec<Node>`) |
| scoped slots | un argumento closure: `row: impl Fn(&Item) -> Node` |
| `{{ text }}` | `.text_dyn(move \|\| …)` |
| `v-if` / `v-else` | `.children_dyn(move \|\| if … { … } else { … })` |
| `v-show` | `.bool_attr("hidden", move \|\| …)` |
| `v-for` con `:key` | `.children_keyed(items, key, row)` |
| `v-model` | `.bind_value(state)`, `.bind_checked(state)` |
| `v-model.number` | `.bind_parsed(state)` |
| `<Transition>` con `v-show` | `.transition("fade", move \|\| shown.get())` ([Componentes](../components/#transiciones)) |
| `<Teleport to="body">` | `portal("body", view)` |
| `<Suspense>` y setup asíncrono | `suspense(fallback, \|\| view)` y `resource(source, fetch)` ([Datos asíncronos](../async-data/)) |
| `router.replace`, `beforeEach` | `navigate_replace(to)`, `before_navigate(guard)` |
| VeeValidate, `useField` | `active::forms` ([Formularios](../forms/)) |
| `:class="{ active: on }"` | `.class_if("active", move \|\| …)` |
| `:style="{ width: w }"` | `.style_dyn("width", move \|\| …)` |
| `:attr="value"` | `.attr_dyn("attr", move \|\| …)` |
| `@click` | `.on_click(move \|e\| …)` |
| `@submit.prevent` | `.on_submit(move \|e\| …)` |
| `@keydown.enter` | `.on_key("Enter", move \|e\| …)` |
| `.stop` | `e.stop_propagation()` |
| `v-html` | `.html(…)`, `.html_dyn(…)` |
| rutas de Vue Router, `:id`, `*` | `Routes::new().route("/posts/:id", page)`, `match_path` |
| `<router-view>` | `routes.view(move \|\| active::location().get())` |
| `router.push('/x')` | `active::navigate("/x")` |
| `route.params.id` | `params.with(\|p\| p.get("id") …)` |
| `route.query.q` | `active::query_param("q")` |
| Pinia store | un struct `Copy` de `State` dado con `provide` |
| `useHead` (Unhead) | un nodo `Seo` en cualquier lugar de la vista |
| SSR de Nuxt + hidratación | `Document::new(view).render()` + `island` / `hydrate` |

## Un componente, de las dos formas

El componente Vue de abajo mantiene una lista de tareas, un borrador enlazado a un campo, un recuento calculado y un watcher:

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

El mismo componente con `active`:

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

## Qué cambia

- **El setup se ejecuta una vez, la vista es código.** El cuerpo de la función es el `setup` de Vue: se ejecuta una vez por componente. Lo que cambia después vive en closures (`text_dyn`, `class_if`, `children_dyn`…), que el navegador vuelve a ejecutar cuando cambia un estado que leen.
- **Los estados son handles `Copy`.** `use_state` devuelve un pequeño `State<T>` que mueves a tantos closures como quieras; no hay `.value` ni proxy: lee con `.get()` o `.with(|v| …)`, escribe con `.set(…)` o `.update(|v| …)`.
- **Primero el servidor.** Las páginas se renderizan a HTML en el servidor con todas las etiquetas SEO, y solo se hidratan las islas, como Nuxt con islas. Los efectos y los watchers nunca se ejecutan en el servidor.
- **Tipos en lugar de comprobaciones en tiempo de ejecución.** Las props son argumentos de función y structs, los emits son callbacks, los slots son argumentos `Node`: una prop que falta o un evento mal escrito es un error de compilación.

## Aún no integrado

- **`<TransitionGroup>`, y transiciones de salida de elementos que retira una región dinámica**: `.transition` anima un elemento que permanece en la página (mostrado y oculto); conserva un elemento que sale con `.transition(name, …)` y elimínalo después.
- **`<KeepAlive>`**: `switch` y `Routes::view` conservan una página mientras solo cambian sus parámetros; guarda los datos que deben sobrevivir a un cambio de página en un estado creado por encima del router.

Para React, Svelte, Solid, Angular, Astro y los demás, ver [Si vienes de otros frameworks](../from-others/).
