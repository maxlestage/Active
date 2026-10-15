---
title: Datos asíncronos y errores
description: Cargar datos con resource, mostrar un respaldo con suspense mientras cargan, conservar los datos anteriores durante una recarga y sustituir una parte de la página que falla con error_boundary.
---

Las páginas esperan cosas: un perfil desde una API, resultados de búsqueda, un archivo leído de IndexedDB. `active` da forma a esa espera: un **recurso** contiene datos cargados de forma asíncrona, **suspense** muestra un respaldo hasta que los datos de una vista están listos, y una **error boundary** muestra otra cosa cuando una parte de la página falla. Las ideas vienen de Solid (`createResource`, `<Suspense>`, `<ErrorBoundary>`) y de React (Suspense, error boundaries, transiciones).

Todos los ejemplos de esta página empiezan con:

```rust
use active::prelude::*;
use active::{error_boundary, resource, suspense, ErrorBoundary, Resource};
```

## resource

`resource(source, fetch)` llama a `fetch` con el valor de `source` y guarda lo que da el future devuelto:

```rust
#[derive(Clone, PartialEq)]
struct User {
    name: String,
}

async fn load_user(id: u32) -> Result<User, String> {
    // En el navegador: gloo-net, reqwest o el fetch de web-sys.
    Ok(User { name: format!("Usuario {id}") })
}

fn profile(id: State<u32>) -> Element {
    let user = resource(move || id.get(), load_user);
    div().children_dyn(move || match user.get() {
        None => vec![p().text("Cargando…").into()],
        Some(Ok(user)) => vec![h2().text(user.name).into()],
        Some(Err(error)) => vec![p().class("error").text(error).into()],
    })
}
```

- `source` se sigue como un efecto: cuando `id` cambia, se vuelve a llamar a `fetch` con el nuevo id. Si una petición anterior termina después de una más reciente, su resultado se ignora: una respuesta lenta nunca sobrescribe una reciente.
- El valor es `None` hasta que llega el primer resultado, y luego el último resultado. Haz de `T` un `Result` para conservar los errores, como arriba.
- Un `Resource` es `Copy`, como un estado: muévelo a tantos closures como haga falta.

| Método | Qué hace |
| --- | --- |
| `get()` / `with(f)` | El último valor (`None` antes del primero); leerlo suscribe la vista. |
| `loading()` | Si hay una carga en curso, la primera o una recarga. |
| `refetch()` | Vuelve a cargar con la fuente actual: un botón «Actualizar» o «Reintentar». |
| `set(value)` | Sustituye el valor de inmediato, para una actualización optimista. |
| `state()` | El `State<Option<T>>` que contiene el valor, para un `memo` o un `watch`. |

### En el navegador y en el servidor

En el navegador (feature `client`), el future se ejecuta en el bucle de eventos del navegador mediante `wasm-bindgen-futures`. El servidor no tiene bucle de eventos: allí el future se consulta **una vez**. Un valor que ya está disponible (una caché, `async { value }`, datos con los que se renderiza la página) se renderiza de inmediato; una petición real sigue cargando, y el servidor renderiza el estado de carga o el respaldo de suspense. Después el navegador carga los datos tras la hidratación.

## Conservar los datos anteriores durante una recarga

Durante una recarga, el valor anterior sigue siendo legible: la página lo sigue mostrando en lugar de volver a «Cargando…». Muestra un indicador discreto con `loading()`, como hace `useTransition` de React:

```rust
fn results(query: State<String>) -> Element {
    let found = resource(move || query.get(), |q| async move { vec![format!("Resultado para {q}")] });
    div()
        .class_if("dimmed", move || found.loading())
        .children_dyn(move || {
            found
                .get()
                .unwrap_or_default()
                .into_iter()
                .map(|r| p().text(r).into())
                .collect()
        })
}
```

## suspense

`suspense(fallback, view)` muestra `fallback` hasta que cada recurso creado por `view`, a cualquier profundidad, tiene su primer valor, y luego muestra la vista:

```rust
fn dashboard() -> Node {
    suspense(p().class("skeleton").text("Cargando el panel…"), || {
        let stats = resource(|| (), |()| async { 42 });
        let news = resource(|| (), |()| async { String::from("Versión 1.0") });
        div()
            .child(p().text_dyn(move || format!("{} visitas", stats.get().unwrap_or(0))))
            .child(p().text_dyn(move || news.get().unwrap_or_default()))
            .into()
    })
}
```

- `view` se construye una vez, de inmediato, para que sus recursos carguen en paralelo.
- Solo cuentan las primeras cargas: una recarga posterior mantiene la vista en pantalla con sus datos anteriores (usa `loading()` ahí).
- Los suspense se anidan: un recurso cuenta en el más cercano.
- En el servidor, un recurso que no está listo de inmediato deja el respaldo en el HTML.

## error_boundary

`error_boundary(view, fallback)` construye `view`, y la sustituye por `fallback` en cuanto se notifica un error dentro:

```rust
fn price(raw: &str) -> Result<Node, std::num::ParseFloatError> {
    let value: f64 = raw.parse()?;
    Ok(span().text(format!("{value:.2} €")).into())
}

fn product() -> Node {
    error_boundary(
        || div().child(price("nueve")).into(),
        |boundary| {
            div()
                .class("error")
                .child(p().text(format!("Algo salió mal: {}", boundary.errors().join(", "))))
                .child(button().text("Reintentar").on_click(move |_| boundary.reset()))
                .into()
        },
    )
}
```

Los errores llegan a la boundary más cercana de tres maneras:

- **Un `Result` en la vista.** `Result<T, E>` se convierte en `Node` cuando `T` lo hace y `E: Display`: un `Ok` muestra su nodo, un `Err` se notifica y no muestra nada. Un componente puede devolver simplemente `Result<Node, MiError>` y usar `?`.
- **`report_error(error)`** mientras se construye la vista.
- **`ErrorBoundary::report(error)`** más tarde, desde un manejador de eventos o al final de una petición: obtén la boundary con `use_context::<ErrorBoundary>()` durante la construcción y muévela al closure.

```rust
fn save_button() -> Element {
    let boundary = use_context::<ErrorBoundary>();
    button().text("Guardar").on_click(move |_| {
        if let Some(boundary) = boundary {
            boundary.report("El servidor no respondió");
        }
    })
}
```

El respaldo recibe la `ErrorBoundary`: `errors()` lista los mensajes, `has_errors()` dice si hay alguno, y `reset()` los olvida y vuelve a construir la vista. Todo lo que la vista fallida creó (estados, efectos, listeners) se libera cuando el respaldo la sustituye.

Sin boundary, `report_error` escribe el error en la consola del navegador (o en la salida de error del servidor) y el `Err` no muestra nada.

> Los componentes de Rust no lanzan excepciones. Un **panic** no se captura: en el navegador detiene el módulo WebAssembly. Devuelve `Result` para los errores que pueden ocurrir.

## Agrupar escrituras: batch

Cuando un resultado modifica varios estados, envuelve las escrituras en `active::batch` para que las vistas que los leen se actualicen una sola vez (ver [Estado y efectos](../state/)):

```rust
fn load_more(items: State<Vec<String>>, page: State<u32>, more: Vec<String>) {
    active::batch(|| {
        items.update(|list| list.extend(more));
        page.update(|p| *p += 1);
    });
}
```
