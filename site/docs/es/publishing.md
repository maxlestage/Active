---
title: Publicar el crate
description: Publica versiones de active en crates.io con una GitHub Release desde el teléfono o con cargo publish desde tu equipo, y descubre cómo la CI revisa cada push.
---

Esta página es para quienes mantienen el repositorio o un fork. Una publicación del crate `active` requiere una sola acción: **publicar una GitHub Release cuya etiqueta sea la versión**, como `v0.2.0`. Un workflow de GitHub Actions prueba el crate y lo publica en [crates.io](https://crates.io). Todo ocurre en los sitios web de GitHub y crates.io, así que basta con un teléfono.

## Cómo funciona el workflow de publicación

El workflow es `.github/workflows/release.yml`. Se ejecuta cuando se publica una GitHub Release y:

1. **Lee la versión de la etiqueta.** `v0.2.0` da `0.2.0`. Una etiqueta que no tenga la forma `v1.2.3` (opcionalmente con un sufijo de prerrelease como `-beta.1`) detiene el workflow con un error.
2. **Comprueba el token.** Sin el secreto `CARGO_REGISTRY_TOKEN`, el job termina con la advertencia "CARGO_REGISTRY_TOKEN secret missing — cargo publish skipped" y no publica nada.
3. **Escribe la versión** de la etiqueta en `crates/active/Cargo.toml`.
4. **Ejecuta las pruebas** con `cargo test -p active`. Una prueba fallida detiene la publicación.
5. **Publica** con `cargo publish -p active --allow-dirty`.

Como la versión sale de la etiqueta, el crate publicado siempre coincide con la publicación, aunque olvides cambiar `Cargo.toml`. Subirla también en el repositorio (paso 3 más abajo) hace que el código fuente y crates.io cuenten la misma historia.

## 1. Crear un token de crates.io

1. Abre [crates.io](https://crates.io) y toca **Log in with GitHub**.
2. Abre el menú de tu avatar y luego **Account Settings**. Añade y **verifica tu dirección de correo**: crates.io no permite publicar sin una.
3. Abre **API Tokens** y toca **New Token**.
4. Ponle un nombre, por ejemplo `github-release`, y elige una fecha de caducidad.
5. Selecciona los permisos **publish-new** (necesario para la primera publicación, que crea el crate) y **publish-update** (para las versiones posteriores).
6. Toca **Generate Token** y cópialo enseguida: crates.io solo lo muestra una vez.

## 2. Añadir el secreto en GitHub

La aplicación móvil de GitHub no muestra la configuración del repositorio, así que usa el sitio web en el navegador de tu teléfono.

1. Abre el repositorio en [github.com](https://github.com) y luego **Settings**.
2. Abre **Secrets and variables** y luego **Actions**.
3. Toca **New repository secret**. Name: `CARGO_REGISTRY_TOKEN`. Secret: pega el token. Toca **Add secret**.

El nombre debe coincidir exactamente. Nadie puede volver a leer un secreto una vez guardado; para reemplazarlo, actualízalo con un valor nuevo.

## 3. Subir la versión

1. En el repositorio, abre `crates/active/Cargo.toml` y toca el icono del lápiz (**Edit**).
2. Cambia la línea `version = "…"` de la sección `[package]`, por ejemplo de `0.1.0` a `0.2.0`.
3. Toca **Commit changes** y haz commit en `master`.

Sigue el [versionado semántico](https://semver.org): antes de `1.0.0`, sube el número menor (`0.1.x` a `0.2.0`) para cambios incompatibles o funcionalidades nuevas, y el número de parche (`0.2.0` a `0.2.1`) solo para correcciones. Cargo considera incompatibles `0.1.x` y `0.2.x`.

## 4. Crear una publicación

1. Abre `https://github.com/<owner>/Active/releases/new` (o **Releases** y luego **Draft a new release**).
2. Toca **Choose a tag**, escribe `v0.2.0` (la versión del paso 3 con una `v` delante) y selecciona **Create new tag on publish**.
3. Deja `master` como destino.
4. Escribe el título, por ejemplo `v0.2.0`. **Generate release notes** enumera por ti los cambios fusionados.
5. Toca **Publish release**.

## 5. Comprobar el resultado

Abre la pestaña **Actions**: en unos segundos aparece una ejecución de **Release**. Una marca verde significa que el crate está publicado (o que faltaba el token: abre la ejecución para ver la advertencia). Después:

- [crates.io/crates/active](https://crates.io/crates/active) muestra la nueva versión;
- [docs.rs/active](https://docs.rs/active) genera la documentación de la API unos minutos más tarde, con la feature `client` activada.

### Si algo falla

- **Las pruebas fallan:** no se publica nada. Corrige el código y crea una nueva publicación con la siguiente versión de parche.
- **La versión ya existe:** crates.io nunca acepta dos veces la misma versión, ni siquiera después de un yank. Publica la versión siguiente.
- **401 o 403:** el token falta, está caducado o no tiene el permiso adecuado. Crea uno nuevo y actualiza el secreto.
- **El nombre pertenece a otra persona** (solo en la primera publicación): otra cuenta es dueña de `active` en crates.io. Cambia el nombre del paquete en `crates/active/Cargo.toml` (por ejemplo, `active-web`) y actualiza la documentación.

## Publicar desde tu equipo

La misma publicación sin GitHub Actions:

```sh
cargo login                         # paste the crates.io token once
cargo test -p active
cargo publish -p active --dry-run   # builds the package exactly as it will be uploaded
cargo publish -p active
```

Solo se suben los archivos enumerados en el `include` del crate (`src`, `README.md`, `LICENSE`). `cargo package -p active --list` los muestra. Después, etiqueta el commit (`git tag v0.2.0 && git push --tags`) para que el repositorio registre lo que se publicó.

## Integración continua

`.github/workflows/ci.yml` se ejecuta en cada push y en cada pull request:

| Paso | Comando |
| --- | --- |
| Formato | `cargo fmt --all --check` |
| Lint, servidor | `cargo clippy --workspace --all-targets -- -D warnings` |
| Lint, navegador | `cargo clippy -p active --features client --target wasm32-unknown-unknown -- -D warnings`, y luego lo mismo para el sitio y los ejemplos (`--lib`) |
| Pruebas | `cargo test --workspace` |
| Compilar el sitio | los mismos comandos que el `Dockerfile` que usa [Heroku](../deploy-heroku/) |
| Empaquetado | `cargo package -p active --allow-dirty` |

El último paso detecta los problemas de empaquetado (un archivo que falta, metadatos no válidos) antes de que lo haga una publicación.

El workflow también tiene un job `merge`: cuando se hace push de una rama cuyo nombre empieza por `claude/` y todas las comprobaciones anteriores pasan, fusiona esa rama en `master` y hace push del resultado. Las demás ramas y los pull requests solo se comprueban, nunca se fusionan automáticamente. Con los despliegues automáticos de Heroku activados, una fusión en `master` pone entonces el cambio en línea.
