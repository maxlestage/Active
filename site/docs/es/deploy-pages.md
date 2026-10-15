---
title: Desplegar en GitHub Pages
description: Publica gratis en GitHub Pages un sitio hecho con active desde un teléfono: expórtalo a archivos estáticos, activa Pages una vez y cada push a master estará en línea unos minutos después.
---

GitHub Pages aloja archivos estáticos gratis, en `https://<propietario>.github.io/<repositorio>/`. Un sitio hecho con `active` se puede publicar allí: sus páginas se escriben en archivos (consulta [Sitios estáticos](../static-site/)) y luego un workflow de GitHub Actions las despliega. Todo lo que sigue se puede hacer desde un teléfono. Los ejemplos usan el sitio de este repositorio, que tiene un comando `export`; el sitio en sí lo sirve Heroku, en <https://active-c1883a966650.herokuapp.com/>.

A diferencia de [Heroku](../deploy-heroku/), Pages no ejecuta ningún servidor: sin costo y sin máquina que mantener, pero también sin código en el lado del servidor. La exportación sustituye las pocas cosas que el servidor calculaba en cada petición (consulta [Qué cambia en un alojamiento estático](#que-cambia-en-un-alojamiento-estatico)).

## Activar Pages (una sola vez)

1. Abre el repositorio en GitHub (la app de GitHub o el sitio web en un navegador móvil).
2. Ve a **Settings** y luego a **Pages**.
3. En **Build and deployment**, cambia **Source** a **GitHub Actions**.

Eso es todo: no hay que elegir rama ni carpeta, el workflow sube el sitio por sí mismo.

> **Consejo:** añade `workflow_dispatch:` al `on:` del workflow para publicar enseguida desde la pestaña **Actions** (**Pages** y luego **Run workflow**). Si no, el próximo push a `master` publica el sitio.

## Cómo funciona la publicación

1. Añade un workflow que compile el sitio, lo exporte y lo despliegue, como el de [Sitios estáticos](../static-site/#github-pages), con el nombre `.github/workflows/pages.yml`.
2. Haz push a `master` (también desde la app de GitHub): el workflow compila el bundle WebAssembly, exporta el sitio y lo despliega. Unos minutos después, la nueva versión está en línea.

Una compilación que falla no publica nada: el sitio en línea siempre viene de una versión que compiló. La pestaña Actions muestra cada ejecución y, para Pages, la dirección del sitio publicado.

> **Nota:** si tu CI fusiona ella misma las ramas en `master`, arranca el workflow Pages cuando la CI termine bien (`workflow_run`) en lugar de escuchar los push a `master`, porque un push hecho por un workflow no arranca otros workflows.

## El comando de exportación

La compilación y la exportación son comandos que también puedes ejecutar en tu equipo. Para el sitio de este repositorio:

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://<propietario>.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` contiene entonces todas las páginas en los tres idiomas (`en/docs/seo/index.html`…), los archivos estáticos, el bundle del navegador, `404.html`, `sitemap.xml`, `robots.txt` y los índices de búsqueda. Cada archivo sale de los handlers del propio servidor: el sitio estático y el servidor envían el mismo HTML.

| Variable | Función | En GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Prefijo de cada enlace, porque un sitio de proyecto vive bajo `/<repositorio>/` | `/<repositorio>`, aquí `/Active` (lo da `actions/configure-pages`) |
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

En **Settings**, **Pages**, escribe tu dominio en **Custom domain** y sigue las instrucciones de DNS de GitHub. El sitio se sirve entonces en la raíz del dominio: expórtalo con una ruta base vacía y tu dominio como dirección (un workflow que los lee de `actions/configure-pages` no necesita cambios).

## Tu propio sitio

Para un proyecto `active` propio, genera las páginas con un pequeño `main` (consulta [Sitios estáticos](../static-site/)) y reutiliza su workflow, sustituyendo los pasos de compilación y exportación por tus propios comandos si tu sitio tiene un servidor, como el sitio de este repositorio.
