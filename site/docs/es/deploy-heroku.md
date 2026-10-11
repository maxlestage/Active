---
title: Desplegar en Heroku
description: Pon en línea un sitio de active con Heroku usando solo un teléfono: haz un fork en GitHub, despliega con un botón y redespliega en cada push a master.
---

Esta guía pone en línea con [Heroku](https://www.heroku.com) un sitio hecho con `active`, usando solo un teléfono y un navegador web. Toma como ejemplo este repositorio (el sitio web de `active`); las últimas secciones muestran cómo desplegar tu propia aplicación de la misma manera.

Heroku construye el `Dockerfile` del repositorio (servidor Rust y bundle de WebAssembly), ejecuta el servidor y te da una dirección que termina en `herokuapp.com`. Una vez conectado a GitHub, cada push a `master` se publica por sí solo.

## Antes de empezar

Necesitas dos cuentas:

- **GitHub.** Un fork del repositorio te permite cambiar el sitio y desplegar tus cambios. Sirven tanto la aplicación de GitHub como github.com en un navegador móvil.
- **Heroku**, en [signup.heroku.com](https://signup.heroku.com), con un **método de pago** añadido en Account settings, Billing. Heroku no tiene plan gratuito: consulta [Costos](#costos) más abajo.

> **Consejo:** El panel de Heroku funciona en un navegador móvil. Si una página se ve apretada, cambia el navegador al modo "Sitio de escritorio".

## 1. Hacer un fork del repositorio

1. Abre [maxlestage.github.io/Active](https://maxlestage.github.io/Active/).
2. Toca **Fork**, conserva el nombre por defecto `Active` y confirma con **Create fork**.

Ahora tienes `https://github.com/<you>/Active`, que puedes editar desde la aplicación o el sitio web de GitHub. Para desplegar el repositorio original sin cambios, puedes saltarte este paso.

## 2. Desplegar con el botón

El botón lee `app.json` y crea una aplicación de Heroku que construye el repositorio con Docker.

1. Abre el enlace de despliegue. Para el repositorio original es heroku.com/deploy?template=https://maxlestage.github.io/Active/. Para tu fork, usa `https://heroku.com/deploy?template=https://github.com/<you>/Active`.
2. Inicia sesión en Heroku si te lo pide.
3. **App name:** elige un nombre en minúsculas como `my-active-site`. Tu dirección será `my-active-site-<random>.herokuapp.com` o similar, y se muestra al final.
4. **Region:** United States o Europe, la que esté más cerca de tus visitantes.
5. **SITE_URL:** déjalo vacío por ahora (consulta [Configuración](#configuracion)).
6. Toca **Deploy app**.

Heroku construye ahora la imagen Docker: instala el target de WebAssembly y `wasm-bindgen-cli`, y compila el bundle del navegador y el servidor en modo release. La primera compilación tarda varios minutos porque todo se compila desde cero. Cuando aparezca "Your app was successfully deployed", toca **View**.

## 3. Volver a desplegar automáticamente desde GitHub

El botón despliega una sola vez. Para publicar cada cambio que subas:

1. Abre [dashboard.heroku.com](https://dashboard.heroku.com) y toca la aplicación que acabas de crear.
2. Abre la pestaña **Deploy**.
3. En **Deployment method**, toca **GitHub**, luego **Connect to GitHub**, y autoriza a Heroku.
4. Busca `Active` (tu fork) y toca **Connect**.
5. En **Automatic deploys**, elige la rama **`master`** y toca **Enable Automatic Deploys**.

A partir de ahora, cada commit que llegue a `master` (un archivo editado en GitHub, un pull request fusionado) inicia una nueva compilación. Sigue las compilaciones en la pestaña **Activity**. La sección **Manual deploy** despliega una rama cuando lo necesites.

> **Advertencia:** Crea la aplicación con el botón y después conéctala. Una aplicación creada con **New → Create new app** en el panel usa la pila de buildpacks por defecto de Heroku, que ignora `heroku.yml` y no puede construir este repositorio. Pasar una aplicación existente a la pila de contenedores requiere la CLI de Heroku (`heroku stack:set container --app <name>`), que no puedes usar desde un teléfono.

### Editar el sitio desde un teléfono

Con los despliegues automáticos activados, todo el ciclo funciona desde la aplicación o el sitio web de GitHub: abre un archivo de tu fork (por ejemplo, una página de `site/docs/en/`), toca el icono del lápiz, edita y haz commit en `master`. Heroku vuelve a compilar y el cambio está en línea unos minutos después.

## Cómo se despliega el repositorio

Cuatro archivos le indican a Heroku qué hacer. No necesitas cambiarlos para desplegar este sitio.

| Archivo | Función |
| --- | --- |
| `app.json` | Lo lee el botón Deploy: descripción de la aplicación, `"stack": "container"`, la variable opcional `SITE_URL`, un dyno `web` |
| `heroku.yml` | Indica a la pila de contenedores que construya el proceso `web` a partir del `Dockerfile` |
| `Dockerfile` | Dos etapas: una imagen de Rust compila el bundle y el servidor, y una pequeña imagen de Debian los ejecuta |
| `.dockerignore` | Deja `target/`, `pkg/`, `bench/` y los datos de Git fuera de la compilación |

`heroku.yml` tiene tres líneas:

```yaml
build:
  docker:
    web: Dockerfile
```

El `Dockerfile` ejecuta los mismos comandos que la CI y que [Compilar el cliente](../client-build/):

```dockerfile
FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY examples examples
COPY site site
RUN cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm \
 && cargo build -p active-site --bin active-site --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/active-site /app/active-site
COPY --from=build /src/site/static /app/site/static
COPY --from=build /src/site/docs /app/site/docs
COPY --from=build /src/site/pkg /app/site/pkg
ENV SITE_DIR=/app/site
CMD ["/app/active-site"]
```

La imagen final solo contiene el binario del servidor y los archivos que sirve, así que arranca en una fracción de segundo.

## Configuración

Heroku pasa los ajustes al servidor como variables de entorno, llamadas **config vars**. Defínelas en la pestaña **Settings** de la aplicación, en **Config Vars**, **Reveal Config Vars**. Cambiar una reinicia la aplicación.

| Variable | La define | Significado |
| --- | --- | --- |
| `PORT` | Heroku | El puerto en el que debe escuchar el servidor. El servidor la lee y, en local, usa `3000` si no existe. Nunca la definas tú. |
| `SITE_URL` | tú, opcional | Dirección pública usada en las URL canónicas, las URL de Open Graph, `sitemap.xml` y `robots.txt`, por ejemplo `https://www.example.com`. Si está vacía, el servidor usa la dirección de cada petición. |

Define `SITE_URL` en cuanto uses un dominio propio, para que los buscadores vean siempre la misma dirección.

## Logs

- **Logs del servidor:** en la aplicación, toca **More** (arriba a la derecha) y luego **View logs**. Verás cada petición y todo lo que imprima el servidor.
- **Logs de compilación:** en la pestaña **Activity**, toca **View build log** junto a una compilación.

Con la CLI de Heroku instalada en tu equipo, `heroku logs --tail --app <name>` muestra esos mismos logs en tiempo real.

## Dominio propio

1. En **Settings**, en **Domains**, toca **Add domain** y escribe tu dominio, por ejemplo `www.example.com`. Heroku muestra un **DNS target**.
2. En tu registrador de dominios, añade un registro **CNAME**: nombre `www`, valor el DNS target. Para un dominio raíz (`example.com`), usa un registro ALIAS, ANAME o CNAME aplanado si tu proveedor lo ofrece.
3. Define la config var `SITE_URL` como `https://www.example.com`.

Los cambios de DNS tardan desde unos minutos hasta unas horas. Para usar HTTPS en tu dominio, recurre a las opciones de SSL de la misma sección **Domains**; los certificados automáticos dependen del tipo de dyno, así que comprueba qué incluye el tuyo.

## Costos

Heroku no tiene plan gratuito. En el momento de escribir esto:

- Los dynos **Eco** cuestan unos 5 USD al mes por un fondo de 1000 horas de dyno compartido por todas tus aplicaciones Eco. Una aplicación Eco se duerme tras 30 minutos sin tráfico, y la siguiente visita espera unos segundos mientras despierta.
- Los dynos **Basic** cuestan unos 7 USD al mes por dyno y nunca se duermen.

Cambia el tipo de dyno en la pestaña **Resources**. Consulta [heroku.com/pricing](https://www.heroku.com/pricing) para ver los precios actuales. Un servidor hecho con `active` necesita muy poca memoria, así que el dyno más pequeño basta para un sitio de documentación o un blog.

> **Consejo:** Si tu sitio no tiene lógica del lado del servidor, puedes alojar en su lugar un [sitio estático](../static-site/) gratis en GitHub Pages o en servicios similares.

## Desplegar tu propia aplicación

Para una aplicación organizada como en el [Inicio rápido](../quick-start/) (paquete `my-site`, `pkg/` servido por el servidor), copia `heroku.yml` y `app.json` en tu repositorio, adapta el nombre y la descripción de `app.json` y usa este `Dockerfile`:

```dockerfile
FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY . .
RUN cargo build --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm \
 && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/my-site /app/my-site
COPY --from=build /src/pkg /app/pkg
CMD ["/app/my-site"]
```

Añade un `.dockerignore` que contenga `target` y `pkg`, y haz que el servidor escuche en `PORT` en todas las interfaces:

```rust
#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    let app = axum::Router::new(); // your routes
    axum::serve(listener, app).await.unwrap();
}
```

Mantén la versión de `wasm-bindgen-cli` del `Dockerfile` igual a la versión de `wasm-bindgen` de `Cargo.lock`. Después despliega con `https://heroku.com/deploy?template=https://github.com/<you>/<repo>` y conecta GitHub como antes.

## Solución de problemas

### La compilación falla

Abre **Activity**, luego el log de la compilación fallida, y lee las últimas líneas.

- **"No default language could be detected"** o un error de buildpack: la aplicación no está en la pila de contenedores. Crea una aplicación nueva con el botón Deploy.
- **Un error de esquema o de versión de `wasm-bindgen`:** la versión de la CLI del `Dockerfile` difiere de la del crate `wasm-bindgen` de `Cargo.lock`. Iguálalas.
- **Un error de compilación de Rust:** el mismo error aparece en local con los comandos de [Compilar el cliente](../client-build/). Corrígelo, haz commit y la siguiente compilación lo tendrá en cuenta.

### Application error

La página muestra "Application error". Abre los logs:

- `code=H10` (la aplicación falló): el servidor se detuvo al arrancar. Las líneas justo encima explican por qué.
- `R10` (tiempo de arranque agotado): el servidor no escuchó en `PORT` en 60 segundos. Escucha en `0.0.0.0` y en el puerto de la variable `PORT`.
- `code=H14` (no hay procesos web en ejecución): abre **Resources** y activa el dyno `web`.
