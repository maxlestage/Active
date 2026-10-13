---
title: Preguntas frecuentes
description: Dudas habituales sobre active: otros frameworks, JavaScript, SEO en apps de una sola página, hidratación, datos asíncronos, rutas, estilos, tamaño y hosting.
---

Respuestas breves a las preguntas más habituales, con enlaces a las páginas que profundizan en cada tema.

## ¿Qué es Active, en una frase?

Una pequeña biblioteca de Rust para construir el front-end de sitios web: los componentes son funciones, el estado vive en handles de `use_state`, cada página recibe etiquetas SEO completas y las partes interactivas se ejecutan como WebAssembly, en páginas renderizadas en el servidor, sitios estáticos o aplicaciones del lado del cliente. Consulta la [Introducción](../introduction/).

## ¿Por qué no Leptos, Dioxus, Yew o Sycamore?

Son proyectos excelentes y más grandes, y en algunos casos son la mejor opción:

- **Leptos** es un framework full-stack: reactividad de grano fino, una macro `view!`, funciones de servidor, un enrutador y un modo de islas.
- **Dioxus** apunta a la web, el escritorio y los móviles desde una sola base de código, con su macro `rsx!`.
- **Yew** y **Sycamore** son frameworks de componentes consolidados para el navegador, con soporte de renderizado en el servidor.

`active` toma decisiones distintas:

- **Sin macros.** Las vistas son llamadas a funciones normales, así que rust-analyzer, rustfmt y los errores del compilador funcionan en todas partes, y no hay sintaxis que aprender.
- **Primero el renderizado en el servidor.** HTML completo en cada página, islas solo donde hacen falta; una página sin islas no envía código.
- **SEO integrado.** Título, descripción, canonical, `hreflang`, Open Graph, Twitter y JSON-LD desde un único builder, con [fusión](../seo/#fusion) automática entre layouts y páginas, en el servidor y en las aplicaciones del lado del cliente.
- **Pequeño.** Una API corta que puedes leer en una tarde ([Referencia de la API](../api/)), sin dependencias en el servidor.

Elige un framework más grande si necesitas funciones de servidor, destinos de escritorio o móviles, o un gran ecosistema de componentes listos para usar. La página [Benchmarks](../benchmarks/) compara tamaños y tiempos de renderizado.

## ¿Tengo que escribir JavaScript?

No. Los componentes, el servidor, la compilación y las pruebas son Rust. El navegador recibe dos piezas de JavaScript que nunca escribes ni editas: el cargador generado por `wasm-bindgen` y una línea `<script type="module">` que escribe [`Document::client`](../documents/#cargar-el-bundle-del-navegador) para arrancarlo. Las API del navegador están disponibles desde Rust a través de [`web-sys`](https://docs.rs/web-sys) (consulta [Eventos](../events/#web-sys)).

## ¿Funciona una página con JavaScript desactivado?

Su contenido, sí: las páginas renderizadas en el servidor y las estáticas son HTML completo, así que los textos, los enlaces y las imágenes funcionan sin ningún script. Las islas se quedan como el HTML que renderizó el servidor, pero no reaccionan, porque arrancar WebAssembly requiere el cargador. Los formularios con un `action` y un `method` reales siguen funcionando como formularios HTML normales (consulta [Eventos](../events/#enviar-un-formulario)). Una aplicación del lado del cliente, en cambio, necesita scripts para mostrar cualquier cosa.

## ¿Es buena una aplicación de una sola página para el SEO?

Menos que las páginas renderizadas en el servidor. Google puede ejecutar scripts e indexar contenido construido en el navegador, pero más tarde y de forma menos fiable que con HTML simple, y la mayoría de los demás rastreadores y bots de vista previa de enlaces (redes sociales, aplicaciones de chat) solo leen el HTML que reciben. Una aplicación del lado del cliente envía un shell, así que solo ven el título y la descripción del shell.

Para las páginas públicas, usa [páginas renderizadas en el servidor con islas](../islands/) o un [sitio estático](../static-site/). Si necesitas una aplicación del lado del cliente y SEO, renderiza en el servidor la primera página de cada URL (consulta [Aplicaciones del lado del cliente](../client-app/#renderizar-la-primera-pagina-en-el-servidor)): así el HTML tiene el título y las etiquetas correctos, y la aplicación toma el control en el navegador.

## ¿Qué es una discrepancia de hidratación?

Cuando el navegador hidrata una isla, espera que el HTML del servidor coincida con la vista que construye él mismo. Si difieren, muestra `active: re-rendering island (… mismatch)` en la consola y reconstruye la isla desde cero. La causa habitual es una vista que depende de algo que solo tiene uno de los dos lados (la hora, un número aleatorio, datos de una base de datos): pasa en sus props todo lo que la isla necesite. Detalles y soluciones: [Islas, Discrepancias de hidratación](../islands/#discrepancias-de-hidratacion).

## ¿Cómo cargo datos de forma asíncrona?

- **En el servidor:** espera primero tus llamadas a la base de datos o a la API, y después construye y renderiza la vista en una sola llamada síncrona, sin ningún `.await` entre medias (el compilador rechaza un `State` conservado a través de un `.await` en un servidor multihilo). Consulta [Vitesse, Datos asíncronos](../vitesse/#datos-asincronos).
- **En el navegador:** inicia un future con `wasm_bindgen_futures::spawn_local` desde un manejador o un efecto, y escribe el resultado en un estado; la vista se actualiza. Consulta [Eventos, Trabajo asíncrono en los manejadores](../events/#trabajo-asincrono-en-los-manejadores).

Los componentes en sí son funciones síncronas.

## ¿Hay un enrutador?

En el servidor, usa el enrutador de tu framework web (rutas de Vitesse en la guía de [Vitesse](../vitesse/)), con una función de página por ruta. En una aplicación del lado del cliente, `active::location()` da la ruta actual como estado y gestiona los enlaces y el botón Atrás, `active::query()` y `active::query_param("q")` dan la cadena de consulta, y `active::navigate()` cambia de página desde el código. Consulta [Aplicaciones del lado del cliente, Enrutamiento](../client-app/#enrutamiento).

## ¿Cómo doy estilo a los componentes?

Con CSS normal. Añade hojas de estilo con `Document::stylesheet("/static/app.css")`, define clases con `.class(...)`, actívalas y desactívalas con `.class_if(...)`, y usa `.style(...)` para estilos en línea puntuales o variables CSS (`.style(format!("--progress:{p}%"))`). Cualquier herramienta de CSS que produzca un archivo `.css` funciona a la par. Los bloques `<style>` en línea pueden ir en `Document::head(...)`.

## ¿Cuánto ocupa el WebAssembly?

Unos 22 kB con gzip (57 kB sin comprimir) para una página con una isla de contador, y 47 kB con gzip para la aplicación de tareas completa de `examples/spa`, compiladas con el perfil optimizado para tamaño y servidas comprimidas. El contenido estático no añade nada, y cada isla adicional solo añade su propio código. Cifras, ajustes y consejos: [Rendimiento](../performance/#tamanos).

## ¿Cuánto cuesta alojarlo en Heroku?

Heroku no tiene plan gratuito. En el momento de escribir esto, la opción más barata, los dynos Eco, cuesta unos 5 USD al mes (se duermen tras 30 minutos sin tráfico), y un dyno Basic, unos 7 USD al mes. Consulta [Desplegar en Heroku, Costos](../deploy-heroku/#costos). Un [sitio estático](../static-site/) se puede alojar gratis en GitHub Pages, Netlify o Cloudflare Pages.

## ¿Cómo se llama el crate?

`active`, en [crates.io](https://crates.io/crates/active), con la documentación en [docs.rs/active](https://docs.rs/active). La página [Publicar el crate](../publishing/) explica cómo se hacen las publicaciones.

## ¿Qué versión de Rust necesito?

Rust 1.85 o posterior: el crate usa la edición 2024, declarada como `rust-version = "1.85"`. Actualiza con `rustup update`.

## ¿Puedo usarlo con Actix Web, Rocket u otro servidor?

Sí. `Document::render()` devuelve un `String`: envíalo con el tipo de contenido `text/html; charset=utf-8` desde cualquier framework. La guía de [Vitesse](../vitesse/) muestra una integración, y `examples/hello` usa solo la biblioteca estándar. Un `Seo` también se puede usar por sí solo en cualquier plantilla.

## ¿Cómo veo los mensajes de pánico del navegador?

Con el perfil `wasm-release`, un pánico detiene el módulo y la consola solo muestra un error genérico. Durante el desarrollo, instala un hook de pánico que imprima el mensaje: añade `console_error_panic_hook = "0.1"` a las dependencias de `wasm32` y llama a `console_error_panic_hook::set_once()` al principio de tu función `#[wasm_bindgen(start)]`. Una compilación de depuración también da mejores mensajes (consulta [Compilar el cliente](../client-build/#compilaciones-mas-rapidas-durante-el-desarrollo)).

## ¿Está listo para producción?

`active` es joven (versión 0.1). La API es pequeña y está probada, y este sitio web funciona con ella, pero puede cambiar antes de la 1.0. Fija la versión en `Cargo.toml` y lee las notas de la publicación cuando actualices. Las issues y las contribuciones son bienvenidas en [GitHub](https://github.com/maxlestage/Active).

## ¿Con qué licencia se distribuye?

`active` es software propietario: © Maxime Nathan Lestage, todos los derechos reservados. Su código fuente se publica en crates.io y GitHub para que puedas leerlo, auditarlo y compilarlo, y puedes usarlo libremente como dependencia de tus propios programas, también comerciales. Modificarlo o redistribuir su código fuente requiere el permiso escrito del autor. Las condiciones completas están en el archivo [LICENSE](https://active-c1883a966650.herokuapp.com/static/LICENSE.txt) del repositorio.
