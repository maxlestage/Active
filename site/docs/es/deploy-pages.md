---
title: Desplegar en GitHub Pages
description: Publica gratis el sitio de active en GitHub Pages desde un teléfono: activa Pages una vez y cada fusión en master estará en línea unos minutos después.
---

GitHub Pages aloja archivos estáticos gratis, en `https://<propietario>.github.io/<repositorio>/`. Este repositorio publica allí su sitio automáticamente: el workflow `pages.yml` convierte las páginas del servidor Rust en archivos con `active-site export` y luego las despliega. Todo lo que sigue se puede hacer desde un teléfono.

A diferencia de [Heroku](../deploy-heroku/), Pages no ejecuta ningún servidor: sin costo y sin máquina que mantener, pero también sin código en el lado del servidor. La exportación sustituye las pocas cosas que el servidor calculaba en cada petición (consulta [Qué cambia en un alojamiento estático](#que-cambia-en-un-alojamiento-estatico)).

## Activar Pages (una sola vez)

1. Abre el repositorio en GitHub (la app de GitHub o el sitio web en un navegador móvil).
2. Ve a **Settings** y luego a **Pages**.
3. En **Build and deployment**, cambia **Source** a **GitHub Actions**.

Eso es todo: no hay que elegir rama ni carpeta, el workflow sube el sitio por sí mismo. Para un fork, haz lo mismo en tu fork; su dirección pasa a ser `https://<tú>.github.io/Active/`.

> **Consejo:** para publicar enseguida, abre la pestaña **Actions**, elige **Pages** y luego **Run workflow**. Si no, la próxima fusión en `master` publica el sitio.

## Cómo funciona la publicación

1. Haces push de una rama, o lo hace Claude.
2. El workflow de CI la comprueba (formato, lints, tests, compilaciones, exportación) y, cuando todo está en verde, la fusiona en `master`. Las ramas cuyo nombre empieza por `wip/` se comprueban pero nunca se fusionan.
3. Cuando la CI termina bien, arranca el workflow **Pages**: compila el bundle WebAssembly, exporta el sitio y lo despliega. Unos minutos después, la nueva versión está en línea.

Una CI en rojo no publica nada: el sitio en línea siempre viene de una versión que pasó todas las comprobaciones. La pestaña Actions muestra cada ejecución y, para Pages, la dirección del sitio publicado.

> **Nota:** Pages espera a la CI en lugar de escuchar los push a `master`, porque un push hecho por un workflow (la fusión automática) no arranca otros workflows.

Mientras Pages no esté activado, el workflow Pages se detiene en su primer job con una advertencia en lugar de fallar.

## El comando de exportación

El workflow ejecuta el mismo comando que puedes ejecutar en tu equipo:

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://maxlestage.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` contiene entonces todas las páginas en los tres idiomas (`en/docs/seo/index.html`…), los archivos estáticos, el bundle del navegador, `404.html`, `sitemap.xml`, `robots.txt` y los índices de búsqueda. Cada archivo sale de los handlers del propio servidor: el sitio estático y el servidor envían el mismo HTML.

| Variable | Función | En GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Prefijo de cada enlace, porque un sitio de proyecto vive bajo `/<repositorio>/` | `/Active` (lo da `actions/configure-pages`) |
| `SITE_URL` | Dirección pública, para las URL canónicas, Open Graph, hreflang y el sitemap | `https://<propietario>.github.io/Active` |

Deja `BASE_PATH` vacío para un sitio servido en la raíz de su dominio (un dominio propio u otro alojamiento estático).

Para comprobar una exportación en local, sirve la carpeta que contiene `dist/` bajo la misma ruta, por ejemplo con [`miniserve`](https://crates.io/crates/miniserve):

```sh
mkdir -p preview && cp -r dist preview/Active
cargo install miniserve
miniserve preview --index index.html
```

y abre `http://localhost:8080/Active/`.

## Qué cambia en un alojamiento estático

| En el servidor | En la exportación estática |
| --- | --- |
| `/` redirige al idioma del visitante (`Accept-Language`) | `/` lleva a la página de inicio en inglés y enlaza los tres idiomas |
| La cortina de apertura se reproduce una vez por visita (una cookie) | Sin cortina |
| Las páginas desconocidas responden 404 en el idioma del visitante | GitHub sirve `404.html`, en inglés |
| Compresión Brotli y gzip, cabeceras de caché | Compresión y caché propias de GitHub |

La búsqueda en la documentación funciona igual en ambos casos: el navegador descarga una vez el índice de su idioma (`/search/es.txt`, unos pocos kilobytes) y busca localmente con el mismo código Rust que el servidor.

## Un dominio propio

En **Settings**, **Pages**, escribe tu dominio en **Custom domain** y sigue las instrucciones de DNS de GitHub. El sitio se sirve entonces en la raíz del dominio: `actions/configure-pages` da una ruta base vacía y tu dominio como dirección, sin cambiar nada en el workflow.

## Tu propio sitio

Para un proyecto `active` propio, genera las páginas con un pequeño `main` (consulta [Sitios estáticos](../static-site/)) y reutiliza su workflow, o copia `pages.yml` y sustituye los pasos de compilación y exportación por tus propios comandos.
