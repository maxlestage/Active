---
title: Layouts, carga de datos y precarga
description: Compartir layouts entre páginas, anidar tablas de rutas, cargar los datos de cada página antes de mostrarla, responder el código HTTP correcto o redirigir, y precargar la página siguiente antes del clic.
---

[`Routes`](../client-app/) asocia rutas con páginas y conserva una página mientras solo cambian sus parámetros. Esta página cubre lo que los frameworks full-stack (Next, Nuxt, SvelteKit, Remix) añaden encima: **layouts**, **tablas anidadas**, **carga de datos por ruta**, **códigos HTTP y redirecciones**, y **precarga**.

```rust
use active::prelude::*;
use active::{redirect, set_status, Prefetch, Routes};
```

## Layouts

`layout` envuelve cada página de una tabla, que recibe la página como `outlet`:

```rust
let routes = Routes::new()
    .layout(|outlet| {
        div().class("shell")
            .child(header().child(nav().child(a().href("/").text("Inicio"))))
            .child(main().child(outlet))
            .into()
    })
    .route("/", home)
    .route("/posts/:id", post);
```

En una aplicación del lado del cliente (`routes.view(move || active::location().get())`) el layout se construye **una sola vez**: pasar de una página a otra solo sustituye el outlet, así que los estados del layout — un menú abierto, un campo de búsqueda, un reproductor — se conservan.

## Tablas anidadas

`nest(prefijo, routes)` entrega cada ruta bajo `prefijo` a otra tabla, con patrones relativos al prefijo y su propio layout dentro del del padre. Los parámetros del prefijo llegan a las páginas anidadas:

```rust
let user = Routes::new()
    .layout(|outlet| section().child(user_tabs()).child(outlet).into())
    .route("/", user_overview)          // /users/:id
    .route("/posts/:post", user_post);  // /users/:id/posts/:post — `id` también está

let routes = Routes::new().route("/", home).nest("/users/:id", user);
```

El layout anidado se queda en pantalla mientras el visitante pasa de una página del usuario a otra. `routes.matches(ruta)` mira dentro de las tablas anidadas: una ruta desconocida para la tabla anidada es un 404.

## Cargar los datos de una página

`route_with_data(patrón, load, page)` carga primero los datos y construye la página con ellos, como el `load` de SvelteKit o los loaders de Remix. `loading` define lo que se muestra mientras tanto:

```rust
async fn load_post(id: u32) -> Option<Post> { /* … */ }

let routes = Routes::new()
    .loading(|| div().class("skeleton").into())
    .route_with_data(
        "/posts/:id",
        |params| load_post(params.parse("id").unwrap_or(0)),
        |post: State<Option<Post>>, params| match untrack(|| post.get()) {
            Some(p) => article().child(h1().text(p.title)).into(),
            None => { set_status(404); p().text("No existe esa entrada").into() }
        },
    );
```

La página se construye cuando sus datos están, con un estado que se actualiza en su sitio cuando el visitante va a la misma ruta con otros parámetros. Los datos pasan por [`ssr::resource`](../streaming/) con la clave `"route:/posts/:id"`: `ssr::render` y `ssr::stream` los envían con la página y el navegador no los vuelve a cargar. `T` implementa `Data`.

## Códigos HTTP y redirecciones

En el servidor, `respond(ruta)` construye la página y devuelve una `PageResponse` con el código, la redirección y el contenido:

```rust
let response = routes.respond(&path);
let html = Document::new(response.body).render();
// status: 200, 404 si ninguna ruta coincide, o lo que pidió la página
// location: Some("/login") tras redirect("/login") (código 302 salvo que set_status diera un 3xx)
```

Dentro de una página, `set_status(410)` fija el código y `redirect("/login")` manda al visitante a otro sitio. En el navegador, `redirect` hace que el router vaya allí sustituyendo la entrada del historial; `set_status` no hace nada.

## Precarga

`Document::prefetch(Prefetch::Hover)` carga las páginas que el visitante va a abrir antes del clic. La página recibe speculation rules (Chrome, Edge) y un pequeño script inline para los demás navegadores, que añade una vez un `<link rel="prefetch">` por cada enlace del mismo sitio:

| `Prefetch` | Cuándo |
| --- | --- |
| `Hover` | el puntero se detiene en el enlace, o este recibe el foco |
| `Press` | se pulsa el enlace (ratón o dedo abajo) |
| `Visible` | el enlace aparece en pantalla |

`Document::prerender(cuándo)` va más allá en Chrome y Edge: la página siguiente se renderiza en segundo plano y aparece al instante. Resérvalo para páginas sin efectos secundarios al cargar. Los enlaces con `rel="nofollow"`, `download` o dentro de `data-no-prefetch` se omiten.

En una aplicación del lado del cliente los enlaces no cargan documentos: `active::on_link_hover(|to| …)` (feature `client`) te llama una vez por dirección cuando el visitante va a seguir un enlace, para empezar a cargar los datos de esa página.
