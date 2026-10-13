---
title: Publicar el crate
description: Publica versiones de Active en crates.io cambiando su versión desde el teléfono o con cargo publish desde tu equipo, y descubre cómo la CI revisa cada push.
---

Esta página es para quienes mantienen el repositorio o un fork. Una publicación del crate `active` requiere una sola acción: **cambiar la versión en `crates/active/Cargo.toml` y hacer push a `master`**, por ejemplo de `0.1.0` a `0.2.0`. Cuando la CI pasa, un workflow de GitHub Actions crea la etiqueta de la versión y la GitHub Release, prueba el crate y lo publica en [crates.io](https://crates.io). Todo ocurre en los sitios web de GitHub y crates.io, así que basta con un teléfono.

## Cómo funciona el workflow de publicación

El workflow es `.github/workflows/release.yml`. Se ejecuta cada vez que la CI pasa en un push a `master` y:

1. **Lee la versión** de `crates/active/Cargo.toml`. Si su etiqueta (`v0.2.0` para `0.2.0`) ya existe, la versión ya se publicó y el workflow se detiene ahí.
2. **Crea la etiqueta y la GitHub Release.** Crea la etiqueta `v0.2.0` en el commit que probó la CI, y la GitHub Release con el mismo nombre, con notas generadas por GitHub a partir de los commits desde la publicación anterior.
3. **Comprueba el token.** Sin el secreto `CARGO_REGISTRY_TOKEN`, el job termina con la advertencia "CARGO_REGISTRY_TOKEN secret missing — cargo publish skipped" y no publica nada en crates.io.
4. **Ejecuta las pruebas** con `cargo test -p active`. Una prueba fallida detiene la publicación.
5. **Publica** con `cargo publish -p active --allow-dirty`, salvo si esa versión ya está en crates.io.

Una GitHub Release publicada a mano (paso 4 más abajo) lanza la misma publicación: la versión se lee entonces de la etiqueta de la release, `v0.2.0` da `0.2.0`, y se escribe en `crates/active/Cargo.toml` antes de publicar. Una etiqueta que no tenga la forma `v1.2.3` (opcionalmente con un sufijo de prerrelease como `-beta.1`) detiene el workflow con un error.

## 1. Crear un token de crates.io

1. Abre [crates.io](https://crates.io) y toca **Log in with GitHub**.
2. Abre el menú de tu avatar y luego **Account Settings**. Añade y **verifica tu dirección de correo**: crates.io no permite publicar sin una.
3. Abre **API Tokens** y toca **New Token**.
4. Ponle un nombre, por ejemplo `github-release`, y elige una fecha de caducidad.
5. Selecciona los permisos **publish-new** (necesario para la primera publicación, que crea el crate) y **publish-update** (para las versiones posteriores).
6. Toca **Generate Token** y cópialo enseguida: crates.io solo lo muestra una vez.

## 2. Añadir el secreto en GitHub

La aplicación móvil de GitHub no muestra la configuración del repositorio, así que usa el sitio web en el navegador de tu teléfono.

1. Abre el repositorio en github.com y luego **Settings**.
2. Abre **Secrets and variables** y luego **Actions**.
3. Toca **New repository secret**. Name: `CARGO_REGISTRY_TOKEN`. Secret: pega el token. Toca **Add secret**.

El nombre debe coincidir exactamente. Nadie puede volver a leer un secreto una vez guardado; para reemplazarlo, actualízalo con un valor nuevo.

## 3. Subir la versión

1. En el repositorio, abre `crates/active/Cargo.toml` y toca el icono del lápiz (**Edit**).
2. Cambia la línea `version = "…"` de la sección `[package]`, por ejemplo de `0.1.0` a `0.2.0`.
3. Toca **Commit changes** y haz commit en `master` (o en una rama: el auto-merge la fusiona con `master` cuando la CI está en verde, y lanza la publicación él mismo).

Ese commit es la publicación: cuando la CI pasa, el workflow crea la etiqueta `v0.2.0` y la GitHub Release, y publica el crate. No hay ningún formulario que rellenar.

Sigue el [versionado semántico](https://semver.org): antes de `1.0.0`, sube el número menor (`0.1.x` a `0.2.0`) para cambios incompatibles o funcionalidades nuevas, y el número de parche (`0.2.0` a `0.2.1`) solo para correcciones. Cargo considera incompatibles `0.1.x` y `0.2.x`.

## 4. Crear una publicación a mano (opcional)

No hace falta después del paso 3. Una publicación creada a mano sigue publicando el crate, por ejemplo para una versión cuya etiqueta quieres crear tú:

1. Abre `https://github.com/<owner>/Active/releases/new` (o **Releases** y luego **Draft a new release**).
2. Toca **Choose a tag**, escribe `v0.2.0` (la versión del paso 3 con una `v` delante) y selecciona **Create new tag on publish**.
3. Deja `master` como destino.
4. Escribe el título, por ejemplo `v0.2.0`. **Generate release notes** enumera por ti los cambios fusionados.
5. Toca **Publish release**.

## 5. Comprobar el resultado

Abre la pestaña **Actions**: aparece una ejecución de **Release** en cuanto pasa la ejecución de **CI** del push. Una marca verde significa que el crate está publicado (o que faltaba el token: abre la ejecución para ver la advertencia). Después:

- **Releases** muestra la nueva etiqueta `v0.2.0` y sus notas;
- [crates.io/crates/active](https://crates.io/crates/active) muestra la nueva versión;
- [docs.rs/active](https://docs.rs/active) genera la documentación de la API unos minutos más tarde, con la feature `client` activada.

### Si algo falla

- **La CI falla:** no se crea ninguna etiqueta ni se publica nada. Corrige el código y vuelve a hacer push: la versión se publica en cuanto pasa la CI.
- **Las pruebas fallan en la ejecución de Release:** la etiqueta y la GitHub Release existen, pero no hay nada en crates.io. Corrige el código, sube a la siguiente versión de parche y haz push.
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

Solo se suben los archivos enumerados en el `include` del crate (`src`, `README.md`, `LICENSE`). `cargo package -p active --list` los muestra. Después, haz push del cambio de versión a `master`: el workflow crea la etiqueta y la GitHub Release, y omite crates.io porque esa versión ya está allí.

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

El workflow también tiene un job `merge`: cuando se hace push de una rama y todas las comprobaciones anteriores pasan, fusiona esa rama en `master` y hace push del resultado. Las ramas cuyo nombre empieza por `wip/` y los pull requests solo se comprueban, nunca se fusionan automáticamente. Tras la fusión, [`pages.yml`](../deploy-pages/) publica `master` en GitHub Pages y, con los despliegues automáticos de Heroku activados, el cambio también se pone en línea allí.
