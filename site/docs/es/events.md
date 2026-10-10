---
title: Eventos y formularios
description: Gestiona clics, escritura, teclas y envíos de formularios con on, on_click y on_input, crea formularios controlados y accede al DOM con on_mount y web-sys.
---

Los manejadores de eventos hacen interactiva una vista: un clic actualiza un estado, escribir filtra una lista, enviar un formulario manda datos. Los manejadores son closures de Rust que se conectan con `.on(...)`. Se ejecutan en el navegador una vez que la vista está [hidratada](../islands/) o [montada](../client-app/); en el servidor se aceptan y se ignoran, así que el mismo componente compila para ambos lados.

## Escuchar eventos

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

| Método | Escucha |
| --- | --- |
| `.on(name, handler)` | cualquier evento del DOM: `"submit"`, `"change"`, `"keydown"`, `"focus"`, `"mouseenter"`… |
| `.on_click(handler)` | `"click"` |
| `.on_input(handler)` | `"input"`, que se dispara con cada pulsación de tecla en un campo |

Un manejador es cualquier closure `Fn(Event)`. Como es `Fn` (puede ejecutarse muchas veces), cambia la página a través de [estados](../state/), cuyos handles `Copy` se pueden mover dentro libremente.

## El valor Event

| Método | Devuelve |
| --- | --- |
| `e.value()` | el valor actual del `<input>`, `<textarea>` o `<select>` que disparó el evento (`""` en otro caso) |
| `e.key()` | la tecla de un evento de teclado: `"Enter"`, `"Escape"`, `"ArrowDown"`, `"k"`… (`""` en otro caso) |
| `e.prevent_default()` | cancela la acción por defecto del navegador: seguir un enlace, enviar un formulario… |
| `e.current_target()` | el elemento al que está conectado el listener, como `Option<Dom>` |
| `e.raw()` | el `web_sys::Event` subyacente (solo con la feature `client`) |

`Dom` es `web_sys::Element` en el navegador y un marcador de posición vacío en el servidor, donde `current_target()` devuelve `None`.

## Campos de texto

Lee lo que escribe el usuario con `on_input` y `e.value()`. Para controlar además el campo desde tu código (vaciarlo, rellenarlo), vincula su valor con `attr_dyn("value", …)`: en el navegador, esto define el valor actual del campo, no solo el atributo.

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

`<textarea>` funciona igual.

## Casillas y selects

Vincula una casilla de verificación con `bool_attr("checked", …)`, que también define su estado marcado en el navegador, y un `<select>` con `attr_dyn("value", …)`:

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

En HTML, `<select>` no tiene atributo `value`, así que el servidor marca la `<option>` elegida con `selected`; después, en el navegador, el vínculo de `value` mantiene sincronizado el menú.

## Enviar un formulario

Gestiona `"submit"` en el `<form>` y llama a `prevent_default()` para que el navegador no cargue una página nueva:

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

> **Consejo:** Mantén un `action` y un `method` reales en los formularios renderizados por el servidor. Antes de que se cargue el bundle de WebAssembly (o si falla), el formulario sigue funcionando como un formulario HTML normal; una vez hidratado, tu manejador toma el control.

## Teclado

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

## Acceder al DOM

### on_mount

`.on_mount(f)` ejecuta `f` con el elemento en cuanto está en la página, solo en el navegador: cuando se hidrata una isla o cuando se monta una vista. Úsalo para dar el foco a un campo, dibujar en un canvas o iniciar una animación.

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

`active` cubre lo que los componentes suelen necesitar. Para el resto de las API del navegador, añade [`web-sys`](https://docs.rs/web-sys) a las dependencias de `wasm32` con las features que uses, y mantén ese código detrás de `#[cfg(target_arch = "wasm32")]` para que la compilación del servidor lo ignore:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = ["HtmlElement", "Storage", "Window"] }
```

`e.raw()` da acceso al `web_sys::Event` completo, por ejemplo para detener la propagación:

```rust
use active::prelude::*;

fn inner_button() -> Element {
    button().on_click(|_e| {
        #[cfg(target_arch = "wasm32")]
        _e.raw().stop_propagation();
    })
}
```

El guion bajo de `_e` evita la advertencia "unused variable" en la compilación del servidor.

## Trabajo asíncrono en los manejadores

Un manejador no puede ser `async`, pero puede iniciar un future con [`wasm-bindgen-futures`](https://docs.rs/wasm-bindgen-futures) y escribir el resultado en un estado cuando termine:

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

Esto requiere `wasm-bindgen-futures = "0.4"` y las features `Response` y `Window` de web-sys. La búsqueda de la documentación de este sitio funciona así: consulta [`site/src/client/search.rs`](https://github.com/maxlestage/Active/blob/master/site/src/client/search.rs).

> **Advertencia:** Cuando el future termina, el estado todavía debe existir. Si el componente puede eliminarse mientras tanto (porque está en una región dinámica que puede reconstruirse), crea el estado por encima de esa región.

## Enlaces

Los enlaces son elementos `a().href(...)` normales. En una [aplicación del lado del cliente](../client-app/), el enrutador que inicia `active::location()` sigue los enlaces del mismo sitio sin recargar la página, y `active::navigate("/path")` cambia de página desde un manejador.
