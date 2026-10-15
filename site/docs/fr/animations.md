---
title: Listes animées et actions
description: Animer les lignes d'une liste à clés quand elles entrent, sortent et se déplacent (FLIP) avec transition_group, et regrouper un comportement DOM avec use_action, les actions de Svelte.
---

[`transition` et `transition_in`](../components/) animent un élément. Les listes demandent plus : les lignes qui **sortent** doivent rester à l'écran pendant leur disparition, et les lignes autour doivent **glisser** vers leur nouvelle place au lieu de sauter. C'est le `<TransitionGroup>` de Vue et l'`animate:flip` de Svelte ; dans `active`, `transition_group`.

```rust
use active::prelude::*;
use active::{transition_group, Dom};
```

## transition_group

Entourez une liste `keyed` (ou `indexed`) :

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
/* Les lignes qui sortent quittent le flux, pour que les autres glissent à leur place. */
.todo-leave-active { position: absolute; }
```

- une ligne **ajoutée** joue `todo-enter-from`, `todo-enter-active`, `todo-enter-to` ;
- une ligne **retirée** reste le temps de jouer `todo-leave-from`, `todo-leave-active`, `todo-leave-to`, puis s'en va ;
- une ligne qui **bouge** (tri, filtre, ligne insérée au-dessus) glisse depuis son ancienne position avec la classe `todo-move`, par la technique FLIP : ses anciennes et nouvelles positions sont mesurées, elle est remise où elle était avec un `transform`, puis le transform est retiré sous la transition.

Rien ne se joue au premier affichage de la liste ni à l'hydratation de la page. Sur le serveur, la liste se rend comme d'habitude. Faites de chaque ligne un seul élément (`li`, `tr`, `div`).

## Actions : use_action

Une action regroupe un comportement qui a besoin de l'élément DOM et doit être défait : une infobulle, un clic à l'extérieur, un `IntersectionObserver`, un graphique tiers, un piège à focus. `use_action` l'appelle une fois l'élément dans la page et appelle ce qu'elle renvoie quand l'élément s'en va (le `use:` de Svelte, les directives personnalisées de Vue) :

```rust
fn click_outside(close: State<bool>) -> impl Fn(&Dom) -> Box<dyn FnOnce()> {
    move |dom: &Dom| {
        // Navigateur : écouter `pointerdown` sur le document, mettre `close` à jour quand la
        // cible est hors de `dom`…
        let _ = dom;
        Box::new(move || { /* retirer l'écouteur */ })
    }
}

let open = use_state(true);
let menu = div().class("menu").use_action(click_outside(open)).text("…");
```

Les effets créés dans une action suivent les états et s'arrêtent avec l'élément : une action peut réagir à un état (`effect(move || chart.update(data.get()))`). Sur le serveur, les actions ne font rien.

Une fonction qui renvoie `impl FnOnce()` depuis `&Dom` a besoin de `+ use<>` pour que le nettoyage n'emprunte pas l'élément : `fn autofocus(dom: &Dom) -> impl FnOnce() + use<>`.
