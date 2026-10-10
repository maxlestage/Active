---
title: Componentes y vistas
description: Construye vistas con una función por etiqueta HTML, combínalas en componentes que son funciones de Rust y renderízalas en el servidor o en el navegador.
---

En `active`, una vista es un árbol de nodos construido con llamadas a funciones normales: `div()` crea un `<div>`, `.class("card")` añade una clase, `.child(...)` agrega un hijo. Un componente es una función de Rust que devuelve una vista. No hay macros ni lenguaje de plantillas, así que el compilador, rust-analyzer y rustfmt entienden todo lo que escribes.

Todos los ejemplos de esta página empiezan con:

```rust
use active::prelude::*;
```

El prelude pone en alcance las funciones de etiqueta (`div`, `p`, `a`…), `Element`, `Node`, `Seo`, `Document`, `use_state`, `effect`, `island`, `text`, `text_dyn`, `fragment` y `fragment_dyn`.

## Elementos

Cada etiqueta HTML común tiene una función con el mismo nombre en `active::html`: `div()`, `section()`, `h1()`, `a()`, `button()`, `input()`, `table()`, `video()`… Cada una devuelve un `Element` que configuras con métodos encadenados:

```rust
use active::prelude::*;

let link = a().href("/docs/").class("button primary").text("Read the docs");
assert_eq!(
    Node::from(link).render(),
    r#"<a class="button primary" href="/docs/">Read the docs</a>"#
);
```

Algunos nombres difieren de la etiqueta para ser Rust válido o evitar conflictos:

| Función | Etiqueta |
| --- | --- |
| `text_svg()` | `<text>` de SVG |
| `use_svg()` | `<use>` de SVG |
| `linear_gradient()` | `<linearGradient>` de SVG |

Para cualquier otra etiqueta, incluidos los elementos personalizados, usa `el`:

```rust
use active::prelude::*;

let widget = el("my-widget").attr("mode", "compact");
assert_eq!(Node::from(widget).render(), r#"<my-widget mode="compact"></my-widget>"#);
```

No existe una función de etiqueta `html()`, `head()` ni `body()`: [`Document`](../documents/) escribe esas etiquetas alrededor de tu vista.

## Texto

`.text(...)` agrega un hijo de texto, y la función libre `text(...)` crea un nodo de texto. El texto siempre se escapa, así que lo que escriba un usuario nunca puede inyectar marcado:

```rust
use active::prelude::*;

let para = p().text("Tom & Jerry <3");
assert_eq!(Node::from(para).render(), "<p>Tom &amp; Jerry &lt;3</p>");
```

Los argumentos de texto aceptan `&'static str`, `String` y `Cow<'static, str>` (exportado como `active::Str`). Un `&str` prestado que no vive durante todo el programa debe convertirse antes en un `String`; por eso los ejemplos escriben `name.to_string()` o `format!(...)`:

```rust
use active::prelude::*;

fn greeting(name: &str) -> Element {
    p().text(format!("Hello, {name}!"))
}
```

Cuando dos textos son adyacentes, el servidor escribe un comentario vacío `<!---->` entre ellos para que el navegador los mantenga como dos nodos de texto separados. Nunca tienes que ocuparte de ello.

## Atributos

| Método | Resultado |
| --- | --- |
| `.attr(name, value)` | `name="value"` (escapado) |
| `.id(id)` | `id="…"` |
| `.class(classes)` | añade clases; llámalo varias veces para añadir más |
| `.style(css)` | `style="…"` |
| `.href(url)` | `href="…"` |
| `.bool_attr(name, f)` | un atributo booleano como `disabled`, presente mientras `f()` devuelva `true` |

```rust
use active::prelude::*;

let field = input()
    .attr("type", "email")
    .attr("name", "email")
    .attr("placeholder", "you@example.com")
    .bool_attr("required", || true);
assert_eq!(
    Node::from(field).render(),
    r#"<input type="email" name="email" placeholder="you@example.com" required=""/>"#
);
```

Volver a definir un atributo reemplaza su valor, sea cual sea el método que lo definió (`attr`, `attr_dyn`, `bool_attr`, `id`, `href`…), así que un componente puede devolver un elemento y quien lo llama todavía puede sobrescribir un atributo. Las clases son la excepción: se acumulan. Cada `.class(...)` (y `.attr("class", ...)`, que hace lo mismo) se suma a un único atributo `class`, que se escribe primero:

```rust
use active::prelude::*;

fn button_link(href: &str) -> Element {
    a().class("button").href(href.to_string()).attr("role", "button")
}

let link = button_link("/old").href("/new").attr("class", "primary");
assert_eq!(
    Node::from(link).render(),
    r#"<a class="button primary" href="/new" role="button"></a>"#
);
```

Los atributos, las clases e incluso los atributos booleanos pueden seguir el estado de la página: consulta [Estado](../state/#vincular-el-estado-a-la-vista).

## Hijos

`.child(x)` agrega un hijo y `.children(iter)` agrega varios. Cualquier cosa que se convierta en un `Node` puede ser un hijo:

| Valor | Se convierte en |
| --- | --- |
| un `Element` (`p()`, `div()…`) | ese elemento |
| `&'static str`, `String` | texto escapado |
| un `Seo` | etiquetas SEO para el `<head>` (nada en el body) |
| `Option<T>` | el valor, o nada si es `None` |
| `Vec<T>`, `[T; N]` | sus elementos, uno junto a otro |
| un `Node` | el propio nodo |

```rust
use active::prelude::*;

let list = ul()
    .class("menu")
    .children(["Home", "Docs", "Blog"].map(|label| li().text(label)));
assert_eq!(
    Node::from(list).render(),
    r#"<ul class="menu"><li>Home</li><li>Docs</li><li>Blog</li></ul>"#
);
```

## Fragmentos

`fragment(...)` agrupa nodos sin un elemento contenedor, algo útil cuando un componente devuelve varios hermanos:

```rust
use active::prelude::*;

fn title_and_lead() -> Node {
    fragment([h1().text("Pricing"), p().class("lead").text("Simple plans.")])
}

assert_eq!(title_and_lead().render(), r#"<h1>Pricing</h1><p class="lead">Simple plans.</p>"#);
```

## Condiciones

Como `Option<T>` se convierte en un nodo, `bool::then` es la forma más sencilla de mostrar algo según una condición:

```rust
use active::prelude::*;

fn badge(is_new: bool) -> Element {
    span()
        .class("product")
        .text("Oak chair")
        .child(is_new.then(|| em().text(" new")))
}

assert_eq!(Node::from(badge(false)).render(), r#"<span class="product">Oak chair</span>"#);
```

Cuando las ramas producen distintos tipos de nodos, convierte cada rama con `.into()`:

```rust
use active::prelude::*;

fn price(cents: Option<u32>) -> Node {
    match cents {
        Some(c) => strong().text(format!("{}.{:02} €", c / 100, c % 100)).into(),
        None => "Sold out".into(),
    }
}
```

Estas condiciones se deciden una sola vez, al construir la vista. Para una condición que cambia en el navegador, usa [`children_dyn`](../state/#listas-y-condiciones-que-cambian).

## Listas

Transforma un iterador en elementos y pásalo a `.children`:

```rust
use active::prelude::*;

struct Post {
    slug: String,
    title: String,
}

fn post_list(posts: &[Post]) -> Element {
    ul().class("posts").children(posts.iter().map(|post| {
        li().child(a().href(format!("/blog/{}/", post.slug)).text(post.title.clone()))
    }))
}
```

## Componentes

Un componente es una función que devuelve una vista. Sus argumentos son sus props:

```rust
use active::prelude::*;

/// Una tarjeta con un título, un texto y un enlace opcional.
fn card(title: &str, body: &str, link: Option<&str>) -> Element {
    article()
        .class("card")
        .child(h2().text(title.to_string()))
        .child(p().text(body.to_string()))
        .child(link.map(|href| a().href(href.to_string()).text("Read more")))
}

fn features() -> Element {
    section()
        .class("features")
        .child(card("Fast", "Rendered on the server.", None))
        .child(card("Searchable", "Complete SEO tags.", Some("/docs/seo/")))
}
```

### ¿Element o Node?

- Devuelve **`Element`** cuando el componente es un solo elemento. Quien lo llama puede seguir encadenando: `card(...).class("wide").id("first")`.
- Devuelve **`Node`** cuando puede ser varios hermanos (un `fragment`), distintos tipos de nodos, o nada. Las [islas](../islands/) también devuelven `Node`.

Convierte un `Element` en un `Node` con `.into()` o `Node::from(element)`.

### Structs de props

Cuando un componente recibe muchos valores, agrúpalos en un struct:

```rust
use active::prelude::*;

pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub primary: bool,
}

pub fn link_button(props: ButtonProps<'_>) -> Element {
    a().href(props.href.to_string())
        .class("button")
        .class(if props.primary { "primary" } else { "" })
        .text(props.label.to_string())
}
```

Una cadena de clase vacía se ignora, así que `.class(if … { "primary" } else { "" })` es una forma práctica de añadir una clase según una condición.

### Layouts e hijos

Un layout es un componente que recibe otras vistas como argumentos:

```rust
use active::prelude::*;

fn layout(title: &str, content: impl Into<Node>) -> Node {
    fragment([
        Node::from(header().child(a().href("/").text("Acme"))),
        el("main").child(h1().text(title.to_string())).child(content).into(),
        footer().text("© Acme").into(),
    ])
}

let page = layout("About", p().text("We make furniture."));
assert!(page.render().contains("<main><h1>About</h1><p>We make furniture.</p></main>"));
```

## HTML sin escapar

`.html(...)` define el contenido de un elemento como una cadena de HTML que se inserta sin escapar, por ejemplo Markdown renderizado por otro crate:

```rust
use active::prelude::*;

let article = div().class("prose").html("<p>Rendered <em>Markdown</em></p>");
assert_eq!(
    Node::from(article).render(),
    r#"<div class="prose"><p>Rendered <em>Markdown</em></p></div>"#
);
```

`.html(...)` reemplaza los hijos añadidos antes, y un `.child(...)` añadido después reemplaza el HTML. Usa uno u otro en un mismo elemento.

> **Advertencia:** Este HTML se inserta tal cual, sin escapar. Pasa solo HTML que hayas generado o saneado tú mismo, nunca texto escrito por usuarios.

## SVG

Los elementos SVG funcionan como los de HTML, y en el navegador se crean automáticamente en el espacio de nombres SVG:

```rust
use active::prelude::*;

fn check_icon() -> Element {
    svg()
        .attr("viewBox", "0 0 24 24")
        .attr("width", "16")
        .attr("height", "16")
        .attr("aria-hidden", "true")
        .child(path().attr("d", "M5 12l5 5L20 7").attr("fill", "none").attr("stroke", "currentColor"))
}
```

## Renderizado

- `Node::render()` devuelve el HTML de una vista como `String`, y `render_into(&mut out)` lo agrega a un búfer existente.
- `Document::new(view).render()` devuelve una página completa con `<head>` y etiquetas SEO; consulta [Documentos](../documents/).
- En el navegador, las mismas vistas se [hidratan](../islands/) o se [montan](../client-app/).

Los elementos vacíos (`img`, `input`, `br`, `meta`…) se escriben como `<img …/>` e ignoran los hijos.

Una vista se puede renderizar tantas veces como quieras, e imprimir un `Node` con `{:?}` muestra su HTML. En el servidor, los [estados](../state/#en-el-servidor) que usa se liberan automáticamente en cuanto los renderizados posteriores ya no los necesitan.

## Cosas que debes saber

- **`main` en un binario.** En un archivo que define `fn main`, esa función oculta la función de etiqueta `main()` del prelude. Escribe ahí `active::html::main()` o `el("main")`. En un archivo de biblioteca, `main()` funciona como se espera.
- **Nombres de etiqueta cortos.** El prelude incluye etiquetas de una letra (`a`, `b`, `i`, `p`, `q`, `s`, `u`). Una variable local con el mismo nombre oculta la función: `for (i, item) in …` seguido de `i()` no compila. Cambia el nombre de la variable o llama a `active::html::i()`.
- **Los nombres de atributo se escriben tal cual.** Los valores se escapan, los nombres no: nunca construyas un nombre de atributo a partir de lo que escriba un usuario.
