---
title: Publicar versiones
description: Publica el paquete npm active-react y el crate de Rust active desde un teléfono creando una GitHub Release. Un workflow se encarga del resto.
---

Publicar una nueva versión requiere una sola acción: **publicar una GitHub Release con una etiqueta como `v0.2.0`**. Después, un workflow de GitHub Actions prueba el código y publica el paquete npm y el crate de Rust. Todo ocurre en los sitios web de GitHub, npm y crates.io, así que basta con un teléfono.

Esta página está dirigida a quienes mantienen el repositorio (o tu fork).

## Cómo funciona el workflow de publicación

El workflow está en `.github/workflows/release.yml` y se ejecuta cuando se publica una release:

1. **Lee la versión de la etiqueta.** `v0.2.0` da `0.2.0`. Una etiqueta que no tenga la forma `v1.2.3` detiene el workflow con un error.
2. **Paquete npm** (solo si existe el secreto `NPM_TOKEN`): instala, ejecuta los tests, escribe la versión en `package.json` y después ejecuta `npm publish --access public`.
3. **Crate de Rust** (solo si existe el secreto `CARGO_REGISTRY_TOKEN`): escribe la versión en `crates/active/Cargo.toml`, ejecuta `cargo test` y después `cargo publish`.

La versión viene de la etiqueta, así que **nunca editas los números de versión a mano**. Los archivos del repositorio conservan su número anterior; solo los paquetes publicados llevan el nuevo.

Los jobs de npm y del crate son independientes: si un registro no tiene secreto, su job se omite con una advertencia y el otro publica igualmente. Eso te permite empezar, por ejemplo, solo con npm.

## 1. Crea un token de npm

1. Abre [npmjs.com](https://www.npmjs.com) e inicia sesión (crea una cuenta si hace falta).
2. Toca tu **avatar** (arriba a la derecha) y después **Access Tokens**.
3. Toca **Generate New Token** y después **Granular Access Token**.
4. Dale un nombre, como `github-release`, y una fecha de caducidad.
5. En **Packages and scopes**, establece el permiso en **Read and write**. Elige **All packages** para la primera release, ya que `active-react` todavía no existe en npm. Después de la primera release puedes crear un nuevo token limitado solo a `active-react`.
6. Si tu cuenta exige la autenticación en dos pasos para publicar, activa la opción del token que le permite omitir la 2FA; de lo contrario, el workflow no puede responder a la solicitud de 2FA.
7. Toca **Generate Token** y **cópialo en ese momento**: npm solo lo muestra una vez.

> **Advertencia:** npm limita el tiempo de validez de los tokens de escritura. Anota la fecha de caducidad y reemplaza el secreto antes de que caduque, o la siguiente release fallará en el paso de publicación.

## 2. Crea un token de crates.io

1. Abre [crates.io](https://crates.io) y toca **Log in with GitHub**.
2. Abre el menú de tu avatar y después **Account Settings**. Añade y **verifica tu dirección de correo electrónico**: crates.io se niega a publicar sin ella.
3. Abre **API Tokens** y toca **New Token**.
4. Ponle un nombre, por ejemplo `github-release`, y elige una caducidad.
5. Marca los scopes **publish-new** (para crear el crate la primera vez) y **publish-update** (para las versiones posteriores).
6. Toca **Generate Token** y cópialo.

## 3. Añade los secretos a GitHub

La app móvil de GitHub no muestra la configuración del repositorio, así que usa el sitio web en tu navegador.

1. Abre el repositorio en [github.com](https://github.com) y después **Settings**.
2. En el menú, abre **Secrets and variables** y después **Actions**.
3. Toca **New repository secret**. Name: `NPM_TOKEN`. Secret: pega el token de npm. Toca **Add secret**.
4. Toca de nuevo **New repository secret**. Name: `CARGO_REGISTRY_TOKEN`. Secret: pega el token de crates.io. Toca **Add secret**.

Los nombres de los secretos deben coincidir exactamente. Una vez guardado, nadie puede volver a leer un secreto, ni siquiera tú: para cambiarlo, actualízalo con un nuevo valor.

## 4. Publica una release

1. Abre la página **Releases** del repositorio. En un teléfono, lo más rápido es ir directamente a `https://github.com/maxlestage/Active/releases/new` (cambia el propietario si es un fork).
2. Si estás en la lista de releases, toca **Draft a new release**.
3. Toca **Choose a tag**, escribe `v0.1.0` y selecciona **Create new tag: v0.1.0 on publish**.
4. Mantén `master` como rama de destino.
5. Escribe el título de la release, por ejemplo `v0.1.0`. Toca **Generate release notes** si quieres que GitHub enumere los cambios fusionados.
6. Toca **Publish release**.

## 5. Sigue la ejecución

Abre la pestaña **Actions** del repositorio: en unos segundos aparece una ejecución **Release**. Tócala para ver los tres jobs (`version`, `npm`, `crate`) y sus logs. Una marca verde significa que el job terminó con éxito. Un job al que le falta su secreto también termina en verde, con una advertencia como "NPM_TOKEN secret missing — npm publish skipped" en el resumen de la ejecución.

Después comprueba el resultado:

- [npmjs.com/package/active-react](https://www.npmjs.com/package/active-react) muestra la nueva versión.
- [crates.io/crates/active](https://crates.io/crates/active) muestra la nueva versión, y [docs.rs/active](https://docs.rs/active) genera automáticamente la documentación de la API unos minutos después.

### Si algo falla

- **Fallan los tests:** ese job no publica nada. Corrige el código, fusiona y publica una nueva release con la siguiente versión de parche.
- **npm dice que la versión ya existe:** los registros nunca aceptan dos veces la misma versión, ni siquiera después de despublicarla. Publica la siguiente versión en su lugar.
- **Errores 401 o 403:** el token falta, ha caducado o no tiene permiso de escritura. Crea uno nuevo y actualiza el secreto.
- **Solo un registro recibió la nueva versión:** corrige la causa y publica la siguiente versión de parche, para que ambos registros queden sincronizados.

## Versionado

Sigue el [versionado semántico](https://semver.org): `MAJOR.MINOR.PATCH`.

- **PATCH** (de `0.1.0` a `0.1.1`): solo correcciones de errores.
- **MINOR** (de `0.1.1` a `0.2.0`): nuevas funcionalidades. Mientras la versión empiece por `0.`, usa también un incremento minor para los cambios incompatibles: npm y Cargo consideran incompatibles `0.1.x` y `0.2.x`.
- **MAJOR** (de `1.0.0` a `2.0.0`): cambios incompatibles, una vez que llegues a `1.0.0`.

Usa etiquetas simples `vMAJOR.MINOR.PATCH`. Las etiquetas de prerelease como `v0.2.0-beta.1` pasan la comprobación del workflow, pero el workflow no les da ningún tratamiento especial.

Mantén el paquete npm y el crate con el mismo número de versión: una sola release publica ambos, lo que facilita relacionarlos.

## Después de la primera release

Una vez publicados ambos paquetes, los usuarios los instalan con:

```sh
npm i active-react
```

```sh
cargo add active
```

Recuerda actualizar las instrucciones de [instalación](../installation/) para que dejen de apuntar a GitHub.
