---
title: Streaming y datos del servidor
description: Enviar una página en streaming con ssr::stream para que el visitante la lea mientras cargan las partes lentas, y mandar los valores de los recursos con ssr::resource para que el navegador hidrate sin volver a cargarlos.
---

Una página suele esperar una sola petición lenta (recomendaciones, comentarios, un precio de otro servicio) mientras todo lo demás está listo. `active::ssr` elimina esa espera dos veces:

- **Streaming** (`ssr::stream`): el servidor envía enseguida la página entera con el fallback de cada `suspense` que sigue cargando y luego, a medida que termina cada petición, el contenido de esa frontera y unos bytes de JavaScript que lo colocan. El visitante lee, desplaza y pulsa enlaces durante la espera. Las fronteras llegan en el orden en que llegan sus datos (desordenadas), como en React 18, Solid y Marko.
- **Datos del servidor** (`ssr::resource`): los valores cargados en el servidor viajan con el HTML como JSON; cuando el navegador hidrata la página, el recurso empieza con ese valor en lugar de cargarlo otra vez (los payloads de Next y Nuxt).

```rust
use active::prelude::*;
use active::{ssr, suspense};
```

## Datos del servidor: ssr::resource

`ssr::resource(clave, source, fetch)` es un [recurso](../async-data/) con nombre. Construye la página dentro de `ssr::render`, que renderiza el `Document` y escribe al final del body el valor de cada `ssr::resource` listo:

```rust
async fn load_product(id: u32) -> Result<Product, String> { /* base de datos, API… */ }

fn product_page(id: u32) -> Node {
    suspense(p().text("Cargando…"), move || {
        let product = ssr::resource(format!("product:{id}"), move || id, load_product);
        div().children_dyn(move || match product.get() {
            Some(Ok(p)) => vec![h1().text(p.name).into()],
            Some(Err(e)) => vec![p().class("error").text(e).into()],
            None => vec![],
        }).into()
    })
}

let html = ssr::render(|| Document::new(product_page(7)).client("/pkg/app.js", "/pkg/app_bg.wasm"));
// …<script type="application/json" data-active-data="product:7">{"Ok":{…}}</script></body></html>
```

En el navegador, el mismo componente se ejecuta al hidratar: la primera carga de `product:7` lee el JSON de la página (y lo quita), así que `suspense` muestra el contenido al momento y no sale ninguna petición. Una fuente que cambia o `refetch` cargan con normalidad.

Da a cada recurso de una página su propia clave. Un recurso cuyo valor no está cuando se renderiza la página simplemente no se envía: el navegador lo carga.

## El trait Data

Los valores se escriben en JSON con el trait `active::Data`, implementado para cadenas, números, `bool`, `char`, `Option`, `Vec`, `Result` (`{"Ok": …}`), tuplas y mapas con claves de texto, con el mismo JSON que `serde_json`. Para tus propios tipos, escribe los dos métodos, a mano o con `serde_json`:

```rust
impl active::Data for Product {
    fn write_data(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).unwrap_or_default());
    }
    fn read_data(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}
```

## Streaming: ssr::stream

`ssr::stream(build)` construye y renderiza la página enseguida y devuelve un `Stream` de trozos de HTML:

1. la página, con el fallback de cada `suspense` que sigue cargando envuelto en `<active-suspense>`;
2. para cada frontera cuyos datos han llegado, un `<template>` con su contenido y `<script>__activeSwap(n)</script>`, que lo coloca;
3. el final de la página.

Los futures de los recursos (`resource` simple y `ssr::resource`) avanzan mientras se consulta el stream, y una frontera sale en cuanto todo lo que espera está ahí. Las fronteras dentro de una frontera enviada se envían a su vez. Los valores de `ssr::resource` salen justo antes del contenido que los usa.

```rust
let mut stream = ssr::stream(|| Document::new(page()).client("/pkg/app.js", "/pkg/app_bg.wasm"));
while let Some(chunk) = stream.next_chunk().await {
    body.send(chunk).await?; // text/html; charset=utf-8
}
```

Un stream pertenece al hilo que lo creó (sus estados viven en la arena de ese hilo): hazlo avanzar en ese hilo (un `tokio::task::LocalSet`, un runtime de un solo hilo) y pasa los trozos al body de la respuesta por un canal. `poll_chunk` sirve para adaptarlo a cualquier trait `Stream`.

Para limitar la espera, llama a `stream.now_or_never_all()` cuando se alcance tu plazo: envía lo que está listo y cierra la página; las fronteras que siguen cargando conservan su fallback y el navegador carga sus datos al hidratar.

## Conviene saber

- El `<head>` sale con el primer trozo: pon el `Seo` de la página fuera de las fronteras en streaming.
- El bundle del navegador arranca cuando ha llegado todo el documento (los scripts module lo esperan): la hidratación siempre ve el HTML final.
- Sin streaming, `Document::render` y `ssr::render` renderizan el fallback de lo que no está listo, como antes.
