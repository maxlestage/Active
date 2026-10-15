---
title: Listas animadas y acciones
description: Animar las filas de una lista con claves cuando entran, salen y se mueven (FLIP) con transition_group, y agrupar un comportamiento del DOM con use_action, las acciones de Svelte.
---

[`transition` y `transition_in`](../components/) animan un elemento. Las listas piden más: las filas que **salen** deben quedarse en pantalla mientras desaparecen, y las filas de alrededor deben **deslizarse** a su nuevo sitio en lugar de saltar. Es el `<TransitionGroup>` de Vue y el `animate:flip` de Svelte; en `active`, `transition_group`.

```rust
use active::prelude::*;
use active::{transition_group, Dom};
```

## transition_group

Envuelve una lista `keyed` (o `indexed`):

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
/* Las filas que salen dejan el flujo, para que las demás se deslicen a su sitio. */
.todo-leave-active { position: absolute; }
```

- una fila **añadida** reproduce `todo-enter-from`, `todo-enter-active`, `todo-enter-to`;
- una fila **quitada** se queda mientras reproduce `todo-leave-from`, `todo-leave-active`, `todo-leave-to`, y luego se va;
- una fila que **se mueve** (ordenar, filtrar, una fila insertada encima) se desliza desde su posición anterior con la clase `todo-move`, con la técnica FLIP: se miden sus posiciones antigua y nueva, se devuelve a donde estaba con un `transform` y luego se quita el transform bajo la transición.

No se reproduce nada cuando la lista aparece por primera vez ni al hidratar la página. En el servidor la lista se renderiza como siempre. Haz que cada fila sea un solo elemento (`li`, `tr`, `div`).

## Acciones: use_action

Una acción agrupa un comportamiento que necesita el elemento del DOM y hay que deshacer: un tooltip, un clic fuera, un `IntersectionObserver`, un gráfico de terceros, una trampa de foco. `use_action` la llama cuando el elemento está en la página y llama a lo que devuelve cuando el elemento se va (el `use:` de Svelte, las directivas personalizadas de Vue):

```rust
fn click_outside(close: State<bool>) -> impl Fn(&Dom) -> Box<dyn FnOnce()> {
    move |dom: &Dom| {
        // Navegador: escuchar `pointerdown` en el documento, actualizar `close` cuando el
        // objetivo está fuera de `dom`…
        let _ = dom;
        Box::new(move || { /* quitar el listener */ })
    }
}

let open = use_state(true);
let menu = div().class("menu").use_action(click_outside(open)).text("…");
```

Los efectos creados dentro de una acción siguen a los estados y se detienen con el elemento: una acción puede reaccionar a un estado (`effect(move || chart.update(data.get()))`). En el servidor, las acciones no hacen nada.

Una función que devuelve `impl FnOnce()` desde `&Dom` necesita `+ use<>` para que la limpieza no tome prestado el elemento: `fn autofocus(dom: &Dom) -> impl FnOnce() + use<>`.
