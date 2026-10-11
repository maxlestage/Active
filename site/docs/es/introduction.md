---
title: Introducción
description: active es una pequeña biblioteca front-end para Rust: componentes como funciones, HTML con etiquetas SEO completas y WebAssembly solo donde haga falta.
---

`active` es una pequeña biblioteca de Rust para construir el front-end de un sitio web. Escribes los componentes como funciones de Rust normales, guardas el estado en handles de `use_state`, describes cada página para los buscadores con `Seo` y compilas las partes interactivas a WebAssembly. El mismo código se ejecuta en el servidor y en el navegador, y nunca escribes JavaScript.

## Un primer vistazo

Un componente es una función que devuelve una vista. Las vistas se construyen con una función por etiqueta HTML y métodos encadenados:

```rust
use active::prelude::*;

/// Una tarjeta de producto: HTML simple, renderizado en el servidor.
fn card(name: &str, price: u32) -> Element {
    article()
        .class("card")
        .child(h2().text(name.to_string()))
        .child(p().class("price").text(format!("{price} €")))
}

/// Un contador: renderizado en el servidor, interactivo en el navegador.
fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

fn page() -> String {
    let body = div()
        .child(Seo::new().title("Oak chair").description("Solid oak, built to last."))
        .child(card("Oak chair", 249))
        .child(island("counter", "0", counter));
    Document::new(body).lang("en").render()
}
```

`page()` devuelve un documento HTML completo: el doctype, el `<head>` con el título, la descripción y las etiquetas Open Graph, y después el body. La tarjeta es HTML estático. El contador es una *isla*: su HTML también se renderiza en el servidor y, en cuanto se carga el bundle de WebAssembly, la misma función `counter` toma el control del botón que ya existe.

## Tres formas de entregar tu sitio

Los mismos componentes funcionan en tres tipos de proyecto. Elige el que mejor encaje, o combínalos.

| Modo | Cómo funciona | Ideal para |
| --- | --- | --- |
| Páginas renderizadas en el servidor con islas | Un servidor Rust (Vitesse o cualquier otro) renderiza cada petición con `Document::render()`. El navegador solo hidrata las islas. | Sitios con datos dinámicos, inicio de sesión, búsqueda, comercio electrónico |
| Sitio estático | Un pequeño programa renderiza cada página como archivo HTML en tiempo de compilación. Las islas se siguen hidratando. | Documentación, blogs, landing pages, portafolios |
| Aplicación del lado del cliente | Toda la interfaz se monta en el navegador desde WebAssembly. | Paneles y herramientas detrás de un inicio de sesión, donde el SEO no importa |

Consulta [Vitesse](../vitesse/), [Sitios estáticos](../static-site/) y [Aplicaciones del lado del cliente](../client-app/) para cada uno.

## Por qué el renderizado en el servidor va primero

La mayoría de los sitios web son sobre todo texto e imágenes, con unas pocas partes interactivas. `active` está construido en torno a ese hecho:

- **Los buscadores y las vistas previas de enlaces leen HTML.** Cada página llega completa desde el servidor, con su título, descripción, URL canónica, enlaces `hreflang`, etiquetas Open Graph y JSON-LD. Nada depende de que un rastreador ejecute tu código.
- **El contenido aparece antes de que se ejecute ningún código.** Los visitantes ven la página en cuanto llega el HTML. El bundle de WebAssembly se descarga en paralelo y solo despierta a las islas.
- **Las partes estáticas no cuestan nada en el navegador.** Un párrafo renderizado en el servidor nunca se vuelve a renderizar, y no se envía código para él.

Cuando una página no necesita ninguna interactividad, no envía nada de WebAssembly.

## Decisiones de diseño

- **Los componentes son funciones.** Sin macros ni sintaxis especial: rust-analyzer, rustfmt y los mensajes de error del compilador funcionan como siempre.
- **Estado de grano fino.** `use_state` devuelve un pequeño handle `Copy` que mueves dentro de los closures. Cuando un estado cambia, solo se actualiza el texto, el atributo o la clase que lo lee. No hay DOM virtual ni se vuelve a renderizar el componente.
- **Un solo lenguaje.** Los componentes, el servidor, la compilación y las pruebas son Rust. El único JavaScript que recibe el navegador es el cargador generado por `wasm-bindgen` y una línea `<script type="module">` escrita por `Document::client`.
- **Pequeño.** La compilación para el servidor no tiene dependencias. La feature `client` añade `wasm-bindgen` y `web-sys`. Una página con una isla de contador carga unos 22 kB de WebAssembly comprimido con gzip (consulta [Rendimiento](../performance/)).

La API toma prestadas ideas conocidas (componentes como funciones, `use_state`, efectos), pero `active` es una biblioteca de Rust de principio a fin.

## Cómo encajan las piezas

```text
                 tu crate: src/lib.rs (componentes e islas)
                  /                                       \
     compilación nativa (servidor o generador)   compilación wasm32 (feature "client")
     Document::new(view).render() -> HTML        active::hydrate(&[("counter", counter)])
                  \                                       /
                   navegador: primero llega el HTML, luego despiertan las islas
```

Un solo crate contiene tus componentes. Se compila dos veces: una para el servidor (o el generador del sitio estático) y otra a WebAssembly para el navegador. [Instalación](../installation/) muestra el `Cargo.toml` que lo hace posible.

## Lo que `active` no hace

`active` es pequeño a propósito. No incluye un servidor web, una capa de base de datos, un bundler ni una solución de CSS. Usa el servidor Rust que prefieras (esta documentación muestra [Vitesse](../vitesse/)), archivos CSS normales, y `cargo` junto con `wasm-bindgen` para compilar.

## Próximos pasos

1. [Instala](../installation/) las herramientas y el crate.
2. Sigue el [Inicio rápido](../quick-start/) para tener una página con una isla funcionando en cinco minutos.
3. Aprende sobre [componentes](../components/), [estado](../state/) y [eventos](../events/).
4. Haz que tus páginas sean fáciles de encontrar con [SEO](../seo/).
