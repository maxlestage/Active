---
title: Islas
description: Añade interactividad a una página renderizada en el servidor con islas WebAssembly: decláralas, pásales props, hidrátalas en el navegador y evita discrepancias.
---

Una isla es una parte interactiva de una página renderizada en el servidor: un contador, un cuadro de búsqueda, un configurador de productos. El servidor la renderiza a HTML como el resto de la página y la marca. En el navegador, el bundle de WebAssembly encuentra las partes marcadas y solo hace interactivas esas. Todo lo que queda fuera de las islas sigue siendo HTML simple y no cuesta nada.

## Declarar una isla

Un componente de isla es una función que recibe sus props como una cadena y devuelve un `Node`:

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

pub fn page() -> Element {
    div()
        .child(h1().text("Hello"))
        .child(island("counter", "5", counter))
}

assert_eq!(
    Node::from(page()).render(),
    r#"<div><h1>Hello</h1><active-island data-island="counter" data-props="5"><button><!--#-->5</button></active-island></div>"#
);
```

`island(name, props, component)` llama al componente con las props, renderiza su vista dentro de un elemento `<active-island>` y registra el nombre y las props en los atributos `data-island` y `data-props`.

El tipo del componente es `active::IslandFn`, es decir, `fn(&str) -> Node`: una función normal (o un closure que no captura nada). Todo lo que la isla necesita debe venir de sus props.

## Registrar las islas en el navegador

El bundle del navegador llama una vez a `active::hydrate` con todas las islas que usan las páginas, bajo los mismos nombres:

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button().on_click(move |_| count.update(|n| *n += 1)).text_dyn(move || count.get().to_string()).into()
}

pub fn search(_props: &str) -> Node {
    input().attr("type", "search").into()
}

/// Todas las islas, por nombre: las usan las páginas del servidor y el bundle del navegador.
pub const ISLANDS: [(&str, active::IslandFn); 2] = [("counter", counter), ("search", search)];

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&ISLANDS);
}
```

`#[wasm_bindgen(start)]` hace que `start` se ejecute en cuanto se carga el módulo. La página [Compilar el cliente](../client-build/) muestra cómo se compila y se carga el bundle; [`Document::client`](../documents/#cargar-el-bundle-del-navegador) escribe el código de carga en la página.

Una isla cuyo nombre no está registrado se queda como HTML estático, sin ningún error. Si una isla no reacciona, revisa cómo están escritos los nombres.

## Props

Las props viajan dentro del HTML como una cadena, escapada en el atributo `data-props`. Usa cualquier formato que puedas volver a analizar:

- un número o una palabra: `island("counter", "5", counter)`, `island("search", "en", search)`;
- JSON para datos estructurados, con [`serde_json`](https://docs.rs/serde_json).

```rust
use active::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct CartProps {
    pub product: String,
    pub price_cents: u32,
}

pub fn add_to_cart(props: &str) -> Node {
    let props: CartProps = serde_json::from_str(props).unwrap_or_default();
    let price = props.price_cents;
    let quantity = use_state(1u32);
    div()
        .class("cart")
        .child(strong().text(props.product))
        .child(
            input()
                .attr("type", "number")
                .attr("min", "1")
                .attr_dyn("value", move || quantity.get().to_string())
                .on_input(move |e| quantity.set(e.value().parse().unwrap_or(1))),
        )
        .child(span().text_dyn(move || {
            let total = quantity.get() * price;
            format!("{}.{:02} €", total / 100, total % 100)
        }))
        .into()
}

fn product_page() -> Element {
    let props = serde_json::to_string(&CartProps { product: "Oak chair".into(), price_cents: 24900 })
        .unwrap();
    section().child(h1().text("Oak chair")).child(island("add-to-cart", props, add_to_cart))
}
```

Mantén las props pequeñas: forman parte del HTML de la página.

## Qué hace la hidratación

Cuando se ejecuta `hydrate`, para cada `<active-island>` de la página que aún no está hidratado:

1. Lee el nombre y busca el componente registrado. Los nombres desconocidos se omiten.
2. Llama al componente con las props, en el navegador. Esto crea los estados de la isla.
3. Recorre el HTML que ya está en la página a la par que la vista, y **reclama** los elementos y nodos de texto existentes en lugar de crear otros nuevos.
4. Conecta los listeners de eventos, vincula los textos, atributos y clases dinámicos a sus estados, y ejecuta los callbacks `on_mount`.
5. Las regiones dinámicas (`children_dyn`, `fragment_dyn`) se reconstruyen a partir del estado del navegador.
6. Marca el elemento con `data-hydrated`, así que volver a llamar a `hydrate` no hace nada.

El visitante ya ve el contenido de la isla antes de que se cargue el bundle; la hidratación solo hace que reaccione. Las islas nunca modifican el `<head>` escrito por el servidor.

### El elemento de isla

`<active-island>` es un elemento personalizado. `Document` añade el estilo `active-island{display:contents}` a cada página, así que el contenedor no afecta a tu diseño: sus hijos se comportan como si estuvieran directamente en el padre. Si renderizas páginas sin `Document`, añade esa regla a tu CSS.

## Discrepancias de hidratación

La hidratación espera que el HTML coincida con la vista que el componente construye en el navegador. Si no coincide (un elemento donde se esperaba texto, una etiqueta distinta), `active` muestra una advertencia en la consola del navegador:

```text
active: re-rendering island (tag mismatch)
```

y reconstruye esa isla desde cero. La página sigue funcionando, pero la isla parpadea y se pierde el trabajo del servidor. Causas habituales:

- **Datos de entrada distintos en cada lado.** La vista depende de algo que el navegador no tiene: la hora actual, un número aleatorio, un valor leído de una base de datos. Pon en sus props todo lo que la isla necesite.
- **Código que cambia según el target.** Un `#[cfg(target_arch = "wasm32")]` que modifica el marcado. Úsalo solo para el comportamiento (manejadores, efectos), nunca para lo que se renderiza.
- **HTML modificado después del renderizado.** Un proxy, una extensión del navegador o un script que reescribe la página.

No se compara el contenido de texto, solo la estructura. La hidratación nunca toca el HTML sin escapar definido con `.html(...)`, algo útil para el contenido que solo el servidor puede producir, como el código con resaltado de sintaxis: renderízalo en el servidor y pasa una cadena vacía en el navegador. Las pestañas de la página de inicio de este sitio funcionan así (consulta `site/src/islands.rs`).

## Islas dentro de islas

Una isla puede contener otras islas. La isla exterior hidrata las interiores junto con ella misma, usando las vistas interiores que construye su propio código, así que se hidratan una sola vez.

## SEO dentro de las islas

Un `Seo` dentro de la vista de una isla se fusiona en el head de la página en el servidor, como cualquier otro (consulta [SEO](../seo/#fusion)). En el navegador, las islas nunca cambian el head.

## Las islas son independientes

Cada isla tiene sus propios estados, creados al hidratarse, y vive tanto como la página. Dos islas no pueden compartir un estado. Si dos partes de la página deben reaccionar la una a la otra (un filtro y una lista), conviértelas en una sola isla. Para datos que deban sobrevivir a una recarga de la página, usa el almacenamiento del navegador desde un [efecto](../state/#efectos).

## ¿Islas o aplicación del lado del cliente?

| | Islas | [Aplicación del lado del cliente](../client-app/) |
| --- | --- | --- |
| HTML desde el servidor | páginas completas | un shell vacío |
| SEO | completo, en cada página | limitado a lo que aportan el shell y el navegador |
| Código en el navegador | solo las partes interactivas | toda la interfaz |
| Navegación | cargas de página normales | en el navegador, sin recargar |
| Ideal para | sitios de contenido, tiendas, documentación | herramientas y paneles detrás de un inicio de sesión |

Ambos usan los mismos componentes, estados y eventos, así que pasar un componente de uno a otro no requiere reescribirlo.
