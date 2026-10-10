---
title: Aplicaciones del lado del cliente
description: Crea una aplicación de una sola página que corre en el navegador: móntala con mount_to, enruta con location y navigate, y actualiza títulos y metaetiquetas.
---

Una aplicación del lado del cliente (a menudo llamada aplicación de una sola página) se renderiza por completo en el navegador. El servidor, o cualquier hosting estático, envía una pequeña página HTML; después, el bundle de WebAssembly construye toda la interfaz, cambia de página sin recargar y actualiza el título y las metaetiquetas a medida que el visitante navega.

Es la opción adecuada para herramientas, paneles y todo lo que está detrás de un inicio de sesión, donde los buscadores no importan y la interfaz es interactiva en todas partes. Para contenido público, mejor usa [páginas renderizadas en el servidor con islas](../islands/) o un [sitio estático](../static-site/): envían HTML completo que cualquier rastreador puede leer. Los tres modos usan los mismos componentes, así que puedes combinarlos.

## Qué vas a construir

El repositorio contiene un ejemplo completo, [`examples/spa`](https://github.com/maxlestage/Active/tree/master/examples/spa): una lista de tareas con dos páginas, un formulario, filtros guardados en la URL (`/?show=done`), una lista guardada en `localStorage` y un título de página por ruta. Ejecútalo desde la raíz del repositorio:

```sh
cargo build -p spa --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/spa/pkg target/wasm32-unknown-unknown/wasm-release/spa.wasm
cargo run -p spa
```

Las secciones siguientes explican cada parte. La estructura del crate es la misma que en [Instalación](../installation/#un-crate-dos-destinos).

## El shell

La página que envía el servidor es un [`Document`](../documents/) normal cuyo body es un contenedor vacío, más el bundle:

```rust
use active::prelude::*;

/// SEO compartido por todas las páginas.
pub fn site_seo() -> Seo {
    Seo::new()
        .site_name("Todos")
        .title_template("%s · Todos")
        .description("A to-do list written in Rust, running as WebAssembly.")
}

/// La misma página para todas las URL: la aplicación rellena #app en el navegador.
pub fn shell() -> String {
    Document::new(div().id("app").child(p().text("Loading…")))
        .lang("en")
        .seo(site_seo())
        .stylesheet("/app.css")
        .client("/pkg/spa.js", "/pkg/spa_bg.wasm")
        .render()
}
```

El texto "Loading…" se muestra hasta que arranca el bundle. Como el shell ya tiene el título y la descripción del sitio, las vistas previas de enlaces y los rastreadores que no ejecutan WebAssembly siguen viendo algo con sentido.

## Montar la aplicación

El punto de entrada del navegador monta la vista raíz en el contenedor:

```rust
use active::prelude::*;

fn app() -> Node {
    h1().text("Hello from the browser").into()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app());
}
```

`mount_to(selector, view)` busca el primer elemento que coincide con el selector CSS, elimina su contenido y construye la vista dentro. Si ningún elemento coincide, muestra una advertencia en la consola.

Para añadir una vista a una página sin reemplazar nada (un widget dentro de una página existente), usa `active::mount(&node, &element)`, que agrega la vista después de los hijos actuales del elemento.

## Enrutamiento

`active::location()` devuelve un `State<String>` con la ruta actual (`"/"`, `"/about"`). Léelo en una [región dinámica](../state/#listas-y-condiciones-que-cambian) para mostrar la página correspondiente:

```rust
use active::prelude::*;

fn home() -> Node {
    section().child(Seo::new().title("Home")).child(h1().text("Home")).into()
}

fn about() -> Node {
    section()
        .child(Seo::new().title("About").description("Who we are."))
        .child(h1().text("About"))
        .into()
}

fn not_found() -> Node {
    section().child(Seo::new().title("Not found").noindex(true)).child(h1().text("Not found")).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let link = move |href: &'static str, label: &'static str| {
        a().href(href).class_if("current", move || path.get() == href).text(label)
    };
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .child(nav().child(link("/", "Home")).child(link("/about", "About")))
        .child(el("main").children_dyn(move || match path.get().as_str() {
            "/" => vec![home()],
            "/about" => vec![about()],
            _ => vec![not_found()],
        }))
        .into()
}
```

La primera llamada a `location()`, `query()` o `query_param()` inicia el enrutador:

- **Los enlaces funcionan sin más.** Los clics en elementos `<a href>` que apuntan al mismo sitio cambian la ruta sin recargar la página. Los enlaces con un `target` (distinto de `_self`), con un atributo `download` o con `rel="external"` (una página que renderiza el propio servidor, un archivo), los clics con una tecla modificadora (Ctrl, ⌘, Shift, Alt) o con el botón central, los enlaces a otros sitios y los enlaces a un ancla de la página actual (`#section`) se dejan en manos del navegador.
- **Atrás y Adelante funcionan.** Los botones del historial del navegador actualizan la ruta y la cadena de consulta.
- **Solo se reconstruye la región.** Cuando cambia la ruta, se reconstruye la región que la lee; la barra de navegación y todo lo demás se quedan en su sitio.

`location()`, `query()`, `query_param()` y `navigate()` solo existen con la feature `client`, así que el código que los llama se compila solo para el navegador (aquí, detrás de `#[cfg(target_arch = "wasm32")]`). Los componentes de las páginas compilan en todas partes.

### Cambiar de página desde el código

`active::navigate(to)` va a otra página desde un manejador, por ejemplo después de guardar un formulario:

```rust
use active::prelude::*;

fn save_button() -> Element {
    button().text("Save").on_click(|_| {
        // … guardar y luego:
        #[cfg(target_arch = "wasm32")]
        active::navigate("/saved");
    })
}
```

Añade una entrada al historial del navegador, actualiza `location()` y `query()`, y después se desplaza al principio de la página, o al elemento cuyo `id` coincide con un `#fragment` (`navigate("/docs#install")`).

### Rutas con parámetros

Haz `match` sobre los segmentos de la ruta para extraer parámetros:

```rust
use active::prelude::*;

fn post(slug: &str) -> Node {
    article().child(Seo::new().title(slug.to_string())).child(h1().text(slug.to_string())).into()
}

fn route(path: &str) -> Vec<Node> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => vec![h1().text("Blog").into()],
        ["posts", slug] => vec![post(slug)],
        _ => vec![h1().text("Not found").into()],
    }
}

assert!(Node::from(div().children(route("/posts/hello-rust"))).render().contains("<h1>hello-rust</h1>"));
```

En la aplicación: `.children_dyn(move || route(&path.get()))`.

### Cadenas de consulta

`active::query()` devuelve la cadena de consulta sin el `?` (`"q=rust&page=2"`) como un estado, y `active::query_param(name)` devuelve un parámetro, decodificado, o `None`. Ambos siguen al enrutador: los enlaces, `navigate` y los botones del historial los actualizan. Pasar de `/search?q=a` a `/search?q=b` reconstruye las regiones que leen la consulta, mientras que las que solo leen la ruta se quedan como están.

Guarda en la URL lo que deba sobrevivir a una recarga o poder compartirse en un enlace: un filtro, una búsqueda, un número de página. `examples/spa` guarda ahí su filtro:

```rust
use active::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

/// El filtro forma parte de la URL: `/`, `/?show=active`, `/?show=done`.
#[cfg(target_arch = "wasm32")]
fn filter() -> Filter {
    match active::query_param("show").as_deref() {
        Some("active") => Filter::Active,
        Some("done") => Filter::Done,
        _ => Filter::All,
    }
}

#[cfg(target_arch = "wasm32")]
fn filter_links() -> Element {
    let link = |value: Filter, href: &'static str, label: &'static str| {
        a().href(href).class_if("on", move || filter() == value).text(label)
    };
    nav()
        .child(link(Filter::All, "/", "All"))
        .child(link(Filter::Active, "/?show=active", "Active"))
        .child(link(Filter::Done, "/?show=done", "Done"))
}
```

La lista lee `filter()` dentro de su `children_dyn`, así que hacer clic en un enlace solo reconstruye la lista. El fragmento de la URL (`#…`) no forma parte de ninguno de los dos estados: el navegador se desplaza a las anclas por sí mismo.

## Títulos y metaetiquetas

Cada página pone un `Seo` en su vista, exactamente igual que en el servidor. En una aplicación montada, `active` mantiene sincronizado el `<head>`:

- cuando se monta la aplicación, los valores `Seo` de la vista se fusionan y se escriben en el head;
- cada vez que se reconstruye una región dinámica (un cambio de página), sus valores `Seo` se fusionan **sobre los de sus ancestros** y el head se actualiza: el título, la descripción, robots, canonical, las alternativas, Open Graph, Twitter, JSON-LD y las metaetiquetas adicionales;
- cuando el nuevo contenido no tiene `Seo`, el head vuelve a los valores de los ancestros;
- cuando los valores fusionados no tienen ni título ni nombre de sitio, el título de la página se vacía en lugar de conservar el de la página anterior.

Pon los valores por defecto del sitio (nombre del sitio, plantilla de título, descripción por defecto) en un `Seo` de la **vista raíz**, como hace el ejemplo del enrutador de arriba, para que el título de cada página reciba la plantilla. El `Seo` pasado a `Document::seo` en el shell solo lo usa el servidor.

Los demás elementos del head (hojas de estilo, el charset, tus propias etiquetas `<meta>` que `Seo` no gestiona) no se tocan.

Para cambiar el head desde un manejador, llama a `active::update_head(&seo)` con un `Seo` completo; reemplaza las etiquetas de los tipos enumerados arriba.

## Conservar datos entre páginas

Los estados creados dentro de una página se liberan cuando el enrutador sale de ella: una página que se vuelve a mostrar empieza de cero. Los datos que deben sobrevivir a los cambios de página van en un estado creado **por encima** del enrutador, que se pasa a las páginas:

```rust
use active::prelude::*;

#[derive(Clone)]
struct Todo {
    text: String,
    done: bool,
}

fn todo_page(todos: State<Vec<Todo>>) -> Node {
    let left = move || todos.with(|list| list.iter().filter(|t| !t.done).count());
    section().child(p().text_dyn(move || format!("{} left", left()))).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let todos = use_state(Vec::<Todo>::new()); // survives page changes
    div()
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![todo_page(todos)],
            _ => vec![h1().text("Not found").into()],
        })
        .into()
}
```

Para conservar datos entre visitas, guárdalos en `localStorage` desde un [efecto](../state/#efectos), como hace `examples/spa`.

## Servir la aplicación

Cada URL de la aplicación debe devolver el shell, para que funcione recargar `/about` o abrir un enlace compartido. Las peticiones del bundle (`/pkg/…`) devuelven los archivos.

Con Axum, sirve `/pkg` como archivos y responde con el shell a todo lo demás:

```rust
use axum::{Router, response::Html, routing::get};
use tower_http::services::ServeDir;

fn shell() -> String {
    String::from("<!DOCTYPE html>…") // the shell() function above
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/pkg", ServeDir::new("pkg"))
        .fallback(get(|| async { Html(shell()) }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`examples/spa` hace lo mismo con un servidor escrito solo con la biblioteca estándar. En un hosting estático, escribe el shell en `index.html` (consulta [Sitios estáticos](../static-site/)) y configura una ruta de respaldo:

| Hosting | Respaldo a `index.html` |
| --- | --- |
| Netlify | un archivo `_redirects` que contenga `/* /index.html 200` |
| Cloudflare Pages | automático cuando el sitio no tiene `404.html` |
| GitHub Pages | sin reescrituras: copia `index.html` a `404.html` (las páginas se sirven entonces con un estado 404) |

## Renderizar la primera página en el servidor

Las páginas de un enrutador también pueden renderizarse en el servidor: la aplicación recibe la ruta como un `State`, creado a partir de la petición en el servidor y a partir de `location()` en el navegador.

```rust
use active::prelude::*;

pub fn app(path: State<String>) -> Node {
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![section().child(Seo::new().title("Home")).text("Welcome").into()],
            _ => vec![section().child(Seo::new().title("Not found").noindex(true)).into()],
        })
        .into()
}

/// Servidor: el HTML completo de `path`, con su título y sus metaetiquetas.
pub fn page(path: &str) -> String {
    Document::new(div().id("app").child(app(use_state(path.to_string()))))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app(active::location()));
}

assert!(page("/").contains("<title>Home · Acme</title>"));
```

En el servidor, `Document` encuentra el `Seo` de la página que muestra la región dinámica, así que cada URL obtiene su propio título y su propia descripción. En el navegador, `mount_to` reemplaza el HTML del servidor por la aplicación en vivo. A diferencia de las [islas](../islands/), el contenido se reconstruye en lugar de hidratarse, lo cual es sencillo y suficiente para la mayoría de las aplicaciones.

## Tamaño

La aplicación `examples/spa` completa (enrutamiento, formularios, filtros, almacenamiento) se compila en unos 120 kB de WebAssembly, 47 kB con gzip, con el perfil `wasm-release`. Consulta [Rendimiento](../performance/) para saber cómo medirlo y reducirlo.
