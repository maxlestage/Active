---
title: Estado y efectos
description: Guarda valores que cambian en handles de use_state, vincúlalos a textos, atributos, clases y listas, ejecuta efectos y descubre cómo se libera el estado.
---

Las partes interactivas de una página necesitan valores que cambian: un contador, el texto de un campo de búsqueda, los elementos de una lista. En `active`, un valor así vive en un **estado**, creado con `use_state`. La vista lo lee a través de pequeños closures y, cuando el estado cambia, solo se actualizan las partes de la página que lo leen.

## use_state

```rust
use active::prelude::*;

let count = use_state(0);
count.set(5);
count.update(|n| *n += 1);
assert_eq!(count.get(), 6);
```

`use_state(initial)` guarda el valor y devuelve un `State<T>`, un handle que apunta a él. Sirve cualquier tipo `'static`: números, `String`, `Vec`, tus propios structs y enums.

| Método | Úsalo para |
| --- | --- |
| `.get()` | obtener una copia del valor (requiere `T: Clone`) |
| `.with(\|v\| …)` | leer a través de una referencia, sin clonar (`items.with(Vec::len)`) |
| `.set(value)` | reemplazar el valor |
| `.update(\|v\| …)` | modificar el valor en el sitio (`list.push(…)`, `*n += 1`) |

`set` y `update` notifican a todo lo que leyó el estado, y así se actualiza la página.

## Por qué los handles son Copy

Un `State<T>` es solo un índice dentro de un almacén, así que implementa `Copy` sea cual sea `T`. Puedes mover el mismo handle a tantos closures como necesites, sin `Rc`, `clone()` ni lifetimes:

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

Cada closure `move` recibe su propia copia del handle; todas las copias apuntan al mismo valor.

## Vincular el estado a la vista

Los métodos que terminan en `_dyn` (y `class_if`, `bool_attr`) reciben un closure en lugar de un valor. En el servidor, el closure se ejecuta una vez para producir el HTML. En el navegador se vuelve a ejecutar cada vez que cambia un estado que leyó, y solo se actualiza ese nodo de texto, ese atributo o esa clase.

| Método | Actualiza |
| --- | --- |
| `.text_dyn(f)` o `text_dyn(f)` | un nodo de texto |
| `.attr_dyn(name, f)` | un atributo (`value` también actualiza el valor actual del campo) |
| `.bool_attr(name, f)` | un atributo booleano como `disabled`, `hidden`, `open` (`checked` también actualiza la casilla) |
| `.class_if(class, f)` | una clase, presente mientras `f()` sea `true` |
| `.html_dyn(f)` | el HTML interior del elemento (solo HTML de confianza) |
| `.children_dyn(f)` o `fragment_dyn(f)` | un grupo de nodos, que se reconstruye (ver más abajo) |

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

El comentario `<!--#-->` marca el texto dinámico para que el navegador pueda encontrarlo cuando [hidrata](../islands/) la página.

> **Consejo:** Para mostrar u ocultar algo, `bool_attr("hidden", …)` o `class_if(…)` es la opción más barata: el elemento se queda en la página y solo cambia un atributo.

## Valores derivados

Un valor calculado a partir de estados es simplemente un closure. Como captura handles `Copy`, el closure también es `Copy` y se puede usar en varios lugares:

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

Todo lo que lee `total()` se actualiza cuando cambia cualquiera de los dos estados.

## Listas y condiciones que cambian

`.children_dyn(f)` agrega una **región dinámica** a un elemento: `f` devuelve los nodos que hay que mostrar y, cada vez que cambia un estado que leyó, la región se reconstruye. Los hijos añadidos antes o después de la región se quedan donde están. `fragment_dyn(f)` crea la misma región como un nodo independiente.

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

En el servidor, la región se escribe entre los marcadores `<!--[-->` y `<!--]-->`. En el navegador:

- solo se reconstruye la región; el resto de la página no se toca;
- se liberan los estados, los efectos y los listeners de eventos creados al construir el contenido anterior;
- los estados creados **fuera** de la región (como `items` arriba) sobreviven: así una lista conserva sus datos mientras se reconstruyen sus elementos.

Una condición que cambia es una región que devuelve algunos nodos o ninguno:

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

### Listas largas: filas por clave

Una región `children_dyn` se reconstruye entera. Eso es rápido para las listas de una página típica (decenas de elementos) y es la opción más simple. Para listas largas o interactivas, `children_keyed` (o la función libre `keyed`) conserva cada fila por clave: cuando cambian los elementos, una fila cuya clave sigue ahí y cuyo elemento es igual conserva su DOM, sus estados, sus listeners y el foco, y se mueve si cambió el orden. Solo se construyen las filas nuevas o modificadas, y las eliminadas se liberan.

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

- La primera closure devuelve los elementos (leer estados suscribe la lista), la segunda una clave que identifica un elemento a lo largo del tiempo (un id, no una posición) y la tercera construye una fila.
- El tipo de los elementos implementa `PartialEq`: una fila cuyo elemento cambió se reconstruye, así que siempre muestra los valores actuales.
- Haz que cada fila sea un solo elemento (`li`, `tr`…).
- Con `children_dyn`, deja los campos de texto **fuera** de la región: se recrean en cada reconstrucción y pierden el foco. Una fila por clave que no cambia conserva su campo.

## Efectos

`effect(f)` ejecuta `f` una vez, y de nuevo cada vez que cambia un estado que leyó. Úsalo para trabajo fuera de la vista: guardar en `localStorage`, programar un temporizador, escribir logs, llamar a una API del navegador.

```rust
use active::prelude::*;

/// Guarda el borrador en el navegador. Los efectos nunca se ejecutan en el servidor,
/// pero el código también debe compilar allí; de ahí la versión vacía.
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

El código solo para el navegador usa [`web-sys`](https://docs.rs/web-sys), declarado con las features que necesita: `web-sys = { version = "0.3", features = ["Storage", "Window"] }` en las dependencias de `wasm32`.

Cómo se comportan los efectos:

- **Solo en el navegador.** En el servidor, `effect` no hace absolutamente nada, así que un efecto puede usar libremente las API del navegador. Nunca calcules en un efecto algo que el HTML necesite; calcúlalo en la vista.
- **Dependencias automáticas.** Un efecto depende de los estados que leyó durante su última ejecución. Si lee `a` solo cuando `flag` es true, deja de reaccionar a `a` en cuanto `flag` pasa a false.
- **Sin bucles infinitos.** Un efecto que escribe un estado que él mismo lee no se vuelve a disparar.
- **Se liberan con su región.** Los efectos creados dentro de una región dinámica se detienen cuando la región se reconstruye.

## Limpiar

A veces un componente pone en marcha algo que vive fuera de la vista: un temporizador, un WebSocket, un bucle de animación, un listener en `window`. `on_cleanup(f)` ejecuta `f` cuando el componente se libera, es decir, cuando la región dinámica que lo construyó se reconstruye o el router muestra otra página:

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
    p().text_dyn(move || format!("{} s en esta página", seconds.get()))
}
```

Las limpiezas se ejecutan antes de liberar los estados del componente, así que todavía pueden leerlos. En el servidor, `f` nunca se ejecuta.

## Leer sin seguir: untrack

Dentro de una región dinámica, cada lectura de un estado suscribe **toda la región**. Un componente construido en la región que lee directamente (fuera de una closure) uno de sus propios estados se reconstruiría, con estados nuevos, cada vez que ese estado cambie. Lee los estados en closures (`text_dyn`, `children_dyn`…) y envuelve en `untrack` una lectura que solo se necesita una vez:

```rust
use active::prelude::*;

let (page, size) = (use_state(1), use_state(20));
let view = ul().children_dyn(move || {
    let size = untrack(|| size.get()); // se lee una vez, no se sigue
    (0..size).map(|i| li().text(format!("Elemento {}", (page.get() - 1) * size + i + 1)).into()).collect()
});
```

Solo lo que lee la closure de la región decide cuándo se reconstruye: construir y montar sus nodos (callbacks `on_mount`, componentes anidados) ya no se sigue.

## Resultados asíncronos: is_alive

Una petición de red puede terminar después de que el usuario haya salido de la página. `State::is_alive()` vale `false` una vez liberado el estado: compruébalo antes de escribir un resultado.

```rust,ignore
fn weather(city: String) -> Element {
    let forecast = use_state(None::<String>);
    wasm_bindgen_futures::spawn_local(async move {
        let text = fetch_forecast(&city).await; // tu propia función asíncrona
        if forecast.is_alive() {
            forecast.set(Some(text));
        }
    });
    p().text_dyn(move || forecast.get().unwrap_or_else(|| "Cargando…".into()))
}
```

## Compartir estado

Para compartir un valor entre componentes, crea el estado en su padre común y pasa el handle hacia abajo como argumento:

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

Las [islas](../islands/) separadas no comparten estados: cada isla es su propia pequeña aplicación. Si dos partes de una página deben comunicarse, ponlas en la misma isla.

## En el servidor

En el servidor, nada cambia una vez renderizada la página, y los efectos y manejadores de eventos ni siquiera se guardan. Aun así, los estados ocupan memoria, así que `active` los libera por ti:

- los estados viven en un almacén que pertenece al hilo actual;
- al final de cada renderizado de nivel superior (`Node::render`, `Node::render_into` o `Document::render`), se liberan los estados que ni este renderizado ni el anterior leyeron o crearon.

Los estados de la página que se está renderizando siguen disponibles: puedes renderizar la misma vista varias veces, leer sus estados después de renderizar o imprimir un `Node` con `{:?}`. Los estados de las peticiones terminadas se liberan tras los siguientes renderizados en el mismo hilo, así que un servidor que renderiza millones de páginas solo mantiene un puñado de estados vivos.

De ahí se deriva una regla: **construye y renderiza la vista sin ningún `.await` entre medias.** En un manejador asíncrono, carga primero tus datos y después construye la vista y renderízala en un solo paso síncrono. Un `State` pertenece al hilo que lo creó, así que no es `Send`: en un servidor multihilo como Axum, conservar uno a través de un `.await` es un error de compilación, no una sorpresa en tiempo de ejecución. Consulta [Axum, Datos asíncronos](../axum/#datos-asincronos).

## Ámbitos

Avanzado: `active::scope(f)` ejecuta `f` y devuelve su resultado junto con un `Scope` que posee todos los estados y efectos creados dentro. `Scope::dispose()` los libera. Las regiones dinámicas lo usan internamente; solo lo necesitas si montas y quitas vistas a mano en el navegador.

```rust
use active::{scope, use_state};

let (count, owner) = scope(|| use_state(1));
assert_eq!(count.get(), 1);
owner.dispose(); // `count` can no longer be used
```

En el servidor, un renderizado que ocurre dentro de un `scope` abierto no libera ningún estado: libera tú mismo el scope.

## Errores comunes

### Leer un estado dentro de su propio update

`count.update(|n| *n = count.get() + 1)` provoca un pánico con "a state was read inside its own `update`; read it before updating". Usa el argumento del closure, o lee primero y escribe después:

```rust
use active::use_state;

let count = use_state(1);
let step = use_state(10);

count.update(|n| *n += step.get()); // the argument, plus another state: fine
let doubled = count.get() * 2;      // read first…
count.set(doubled);                 // …then write
assert_eq!(count.get(), 22);
```

Del mismo modo, llamar a `set` o `update` sobre un estado dentro de su propio `with` provoca un pánico ("a state was updated while being read or updated").

### Usar un estado liberado

Un estado creado dentro de una región dinámica se libera cuando la región se reconstruye; usarlo después provoca un pánico con "this state was disposed". Guarda los datos de larga duración en un estado creado por encima de la región.

### Calcular HTML en un efecto

Los efectos no se ejecutan en el servidor, así que nada de lo que producen aparece en el HTML que ven los buscadores. Calcúlalo en la vista.
