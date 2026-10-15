---
title: Venir de Vue
description: Le nom de chaque fonctionnalité de Vue dans active, de ref et computed à v-model, provide/inject, watch et Vue Router, avec un composant complet écrit des deux façons.
---

Si vous connaissez Vue 3 et sa Composition API, vous savez déjà comment fonctionne `active` : un composant met en place son état réactif une fois, et la vue le suit avec des mises à jour à granularité fine. Pas de DOM virtuel ni de re-rendu : comme les templates compilés de Vue (et le mode Vapor), seul le texte, l'attribut ou la classe qui dépend d'un état est mis à jour. La différence : tout est du Rust ordinaire. Un composant est une fonction, le template est une chaîne d'appels de méthodes, et le compilateur vérifie tout.

## De Vue à active

| Vue | active |
| --- | --- |
| `ref(0)`, `reactive({…})` | `use_state(0)`, `use_state(MyStruct {…})` |
| `computed(() => …)` | `memo(move \|\| …)`, ou une simple closure `move \|\|` |
| `watch(source, (new, old) => …)` | `watch(move \|\| …, move \|new, old\| …)` |
| `watch(…, { immediate: true })` | `watch_immediate(…)` |
| `watchEffect(() => …)` | `effect(move \|\| …)` |
| `onMounted` | `.on_mount(\|el\| …)` sur un élément |
| `onUnmounted` | `on_cleanup(move \|\| …)` |
| `provide` / `inject` | `provide(value, \|\| view)` / `use_context::<T>()` |
| props | des arguments de fonction, ou une structure de props |
| `emit('save')` | un argument callback : `on_save: impl Fn() + 'static` |
| slots, slots nommés | des arguments `Node` / `Element` (`children: Vec<Node>`) |
| slots à portée (scoped slots) | un argument closure : `row: impl Fn(&Item) -> Node` |
| `{{ text }}` | `.text_dyn(move \|\| …)` |
| `v-if` / `v-else` | `.children_dyn(move \|\| if … { … } else { … })` |
| `v-show` | `.bool_attr("hidden", move \|\| …)` |
| `v-for` avec `:key` | `.children_keyed(items, key, row)` |
| `v-model` | `.bind_value(state)`, `.bind_checked(state)` |
| `v-model.number` | `.bind_parsed(state)` |
| `:class="{ active: on }"` | `.class_if("active", move \|\| …)` |
| `:style="{ width: w }"` | `.style_dyn("width", move \|\| …)` |
| `:attr="value"` | `.attr_dyn("attr", move \|\| …)` |
| `@click` | `.on_click(move \|e\| …)` |
| `@submit.prevent` | `.on_submit(move \|e\| …)` |
| `@keydown.enter` | `.on_key("Enter", move \|e\| …)` |
| `.stop` | `e.stop_propagation()` |
| `v-html` | `.html(…)`, `.html_dyn(…)` |
| routes de Vue Router, `:id`, `*` | `Routes::new().route("/posts/:id", page)`, `match_path` |
| `<router-view>` | `routes.view(move \|\| active::location().get())` |
| `router.push('/x')` | `active::navigate("/x")` |
| `route.params.id` | `params.with(\|p\| p.get("id") …)` |
| `route.query.q` | `active::query_param("q")` |
| Pinia store | une structure `Copy` de `State` donnée avec `provide` |
| `useHead` (Unhead) | un nœud `Seo` n'importe où dans la vue |
| SSR Nuxt + hydratation | `Document::new(view).render()` + `island` / `hydrate` |

## Un composant, des deux façons

Le composant Vue ci-dessous garde une liste de tâches, un brouillon lié à un champ, un compte calculé et un watcher :

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

Le même composant avec `active` :

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

## Ce qui change

- **Le setup s'exécute une fois, la vue est du code.** Le corps de la fonction est le `setup` de Vue : il s'exécute une fois par composant. Ce qui change ensuite vit dans des closures (`text_dyn`, `class_if`, `children_dyn`…), que le navigateur relance quand un état qu'elles lisent change.
- **Les états sont des handles `Copy`.** `use_state` renvoie un petit `State<T>` que vous déplacez dans autant de closures que vous voulez ; pas de `.value` ni de proxy : lisez avec `.get()` ou `.with(|v| …)`, écrivez avec `.set(…)` ou `.update(|v| …)`.
- **Le serveur d'abord.** Les pages sont rendues en HTML côté serveur avec toutes les balises SEO, et seuls les îlots sont hydratés, comme Nuxt avec ses îlots. Les effets et les watchers ne s'exécutent jamais sur le serveur.
- **Des types plutôt que des vérifications à l'exécution.** Les props sont des arguments de fonction et des structures, les emits des callbacks, les slots des arguments `Node` : une prop manquante ou un événement mal orthographié est une erreur de compilation.

## Pas encore intégré

- **`<Transition>`** : basculez des classes avec `class_if` et utilisez les transitions CSS ; pour un élément qui disparaît, retardez le changement d'état avec un minuteur.
- **`<Teleport>`** : affichez les dialogues avec l'élément natif `<dialog>` (sa couche supérieure échappe à `overflow` et `z-index`), ou atteignez un autre nœud dans `on_mount`.
- **`<KeepAlive>`** : `switch` et `Routes::view` gardent une page tant que seuls ses paramètres changent ; gardez les données qui doivent survivre à un changement de page dans un état créé au-dessus du routeur.
- **`<Suspense>` et composants asynchrones** : lancez la requête dans le composant, affichez un contenu provisoire à partir d'un `State<Option<T>>`, et vérifiez `is_alive()` avant d'écrire le résultat (voir État).
