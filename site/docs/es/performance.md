---
title: Rendimiento
description: Por qué active renderiza rápido en el servidor y pesa poco en el navegador: los ajustes de compilación que importan, tamaños medidos y cómo reducir los tuyos.
---

`active` está diseñado para que el camino rápido sea el camino por defecto: en el servidor, las páginas se escriben directamente en una cadena, el navegador recibe primero el HTML completo y solo las partes interactivas cargan código. Esta página explica qué lo hace rápido, muestra tamaños medidos y enumera lo que puedes ajustar. Para comparaciones con otros frameworks de Rust, consulta [Benchmarks](../benchmarks/).

## En el servidor

- **Escritura directa en una cadena.** El renderizado recorre la vista una sola vez y agrega el HTML a un único `String`. No hay DOM virtual, ni árbol intermedio que comparar, ni plantilla que analizar en tiempo de ejecución. `Document::render` escribe primero el body, recogiendo sus valores `Seo` por el camino, y luego la página a su alrededor, así que cada región dinámica se evalúa una sola vez; `Seo` reserva el espacio que necesitan sus etiquetas antes de escribirlas.
- **Escapado barato.** Los textos y los valores de atributos se recorren byte a byte y se copian por tramos; solo se reemplazan los cinco caracteres especiales (`&`, `<`, `>`, `"`, `'`).
- **No se ejecuta nada que no produzca HTML.** Los manejadores de eventos y los efectos ni siquiera se guardan en el servidor.
- **Memoria acotada.** Los estados viven en un almacén por hilo, y los que ya no usa ningún renderizado se liberan al final de los renderizados siguientes, así que un servidor que renderiza millones de páginas solo mantiene un puñado de estados vivos (consulta [Estado, En el servidor](../state/#en-el-servidor)).
- **Sin dependencias.** Sin la feature `client`, el crate no tiene dependencias, lo que mantiene reducidos los tiempos de compilación y el tamaño de los binarios.

## En el navegador

- **Primero el HTML.** Los visitantes ven el contenido en cuanto llega el HTML; el bundle se precarga en paralelo (`Document::client` escribe enlaces `modulepreload` y `preload`) en lugar de descubrirse después de que se ejecute el cargador.
- **Solo islas.** Las partes estáticas de una página no envían código y nunca se vuelven a renderizar. Una página sin islas no carga nada de WebAssembly.
- **La hidratación reutiliza el HTML.** Las islas reclaman los elementos que renderizó el servidor en lugar de crear otros nuevos.
- **Actualizaciones de grano fino.** Cada `text_dyn`, `attr_dyn`, `class_if` o `bool_attr` está vinculado a un nodo del DOM. Cambiar un estado solo vuelve a ejecutar los vínculos que lo leen: ningún componente se vuelve a renderizar y no hay diffing.
- **Handles Copy.** Un `State` son dos enteros. Moverlo dentro de closures no cuesta nada y no necesita conteo de referencias.
- **Pagas solo por lo que usas.** El código que mantiene actualizado el `<head>` en las aplicaciones del lado del cliente solo se incluye cuando el bundle llama a `mount` o `mount_to`; un bundle que solo hidrata islas no lo contiene.

Las regiones dinámicas (`children_dyn`, `fragment_dyn`) son la excepción: reconstruyen todo su contenido cuando cambia un estado que leen. Mantenlas pequeñas o divídelas (consulta [Mantén las regiones pequeñas](../state/#manten-las-regiones-pequenas)).

## Ajustes de compilación

Estos perfiles, del `Cargo.toml` raíz del repositorio, dan un servidor rápido y un bundle pequeño:

```toml
[profile.release]
opt-level = 3       # fastest code, for the server
lto = true          # optimize across crates, drop unused code
codegen-units = 1   # one unit: better optimization, slower build
strip = true        # no symbols in the binaries

[profile.wasm-release]
inherits = "release"
opt-level = "s"     # optimize for size: smaller downloads
panic = "abort"     # no unwinding machinery
```

| Ajuste | Por qué |
| --- | --- |
| `opt-level = "s"` | Los navegadores descargan el bundle: el tamaño importa más que el último porcentaje de velocidad. `"z"` está aún más orientado al tamaño; mide ambos. |
| `lto = true` | La optimización en tiempo de enlazado elimina las partes de `std`, `web-sys` y tus dependencias que no usas. |
| `codegen-units = 1` | Permite al compilador ver todo el crate a la vez. |
| `strip = true` | Elimina los nombres de los símbolos. Desactívalo temporalmente para analizar el bundle (ver más abajo). |
| `panic = "abort"` | Un pánico detiene el módulo en lugar de desenrollar la pila. Ya es el valor por defecto para `wasm32-unknown-unknown`; el perfil lo indica explícitamente. |

La sección `[profile.dev.package."*"]` de este repositorio también optimiza las dependencias en las compilaciones de depuración (`opt-level = 2`), así que el servidor de desarrollo arranca rápido y tu propio código sigue compilando rápido.

## Tamaños

Medido en este repositorio con Rust 1.97, `wasm-bindgen` 0.2.129 y el perfil `wasm-release`, sin `wasm-opt`. Gzip con nivel 9 y Brotli con calidad 11 (los niveles que usaría un hosting estático o un paso de compilación).

| Bundle | `_bg.wasm` sin comprimir | gzip | Brotli |
| --- | ---: | ---: | ---: |
| `examples/hello`: una isla de contador | 55 408 B | 21 926 B | 18 644 B |
| `examples/spa`: aplicación de tareas con enrutamiento, formularios y almacenamiento | 105 895 B | 42 271 B | 35 381 B |
| Este sitio web: cuatro islas, animaciones, búsqueda | 177 913 B | 64 747 B | 54 520 B |

Los cargadores generados por `wasm-bindgen` añaden 4.2 kB (hello), 5.2 kB (spa) y 7.6 kB (sitio web) con gzip.

El mismo `examples/hello` compilado con el perfil `release` normal (`opt-level = 3`) da 61 100 B sin comprimir y 23 051 B con gzip: el perfil de tamaño ahorra alrededor de un 9 % sin comprimir y un 5 % comprimido.

La mayor parte de un bundle pequeño es maquinaria compartida (asignador de memoria, formateo de cadenas, el runtime de hidratación y reactividad, los bindings de `web-sys`), que se paga una vez por página. Cada isla adicional solo añade su propio código: este sitio web, con cuatro islas, sus animaciones y un cliente de búsqueda, ocupa unas tres veces lo que ocupa el contador solo.

## Mide el tuyo

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
ls -l pkg/my_site_bg.wasm                 # raw
gzip -9 -c pkg/my_site_bg.wasm | wc -c    # gzip
brotli -c -q 11 pkg/my_site_bg.wasm | wc -c   # Brotli, if the brotli tool is installed
```

Mide siempre el archivo `_bg.wasm` que produce `wasm-bindgen`: el archivo de `target/` todavía contiene metadatos que `wasm-bindgen` elimina y es varias veces más grande.

Para ver qué ocupa espacio, compila una vez sin eliminar los símbolos e inspecciona el resultado con [twiggy](https://github.com/rustwasm/twiggy):

```sh
cargo install twiggy
CARGO_PROFILE_WASM_RELEASE_STRIP=false cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg-debug target/wasm32-unknown-unknown/wasm-release/my_site.wasm
twiggy top -n 20 pkg-debug/my_site_bg.wasm
```

## Hazlo más pequeño

- **Sirve archivos comprimidos.** Brotli ahorra alrededor de un 15 % respecto a gzip en estos bundles. Con Axum, `tower_http::compression::CompressionLayer` comprime las respuestas al vuelo; los hostings estáticos y las CDN suelen hacerlo por ti.
- **Caché para siempre.** Pon un hash del contenido en la URL del bundle y sírvelo como `immutable` (consulta [Documentos, Caché a largo plazo](../documents/#cache-a-largo-plazo)). Así, los visitantes que vuelven no descargan nada.
- **Deja el trabajo pesado en el servidor.** Analizar Markdown, resaltar código o formatear fechas puede hacerse en el servidor y llegar a la isla como HTML o como props. Este sitio resalta sus ejemplos de código en el servidor y pasa el resultado como HTML sin escapar, así que el bundle no contiene ningún resaltador.
- **Vigila las dependencias.** Cada crate que usa el código del navegador acaba en el bundle. Comprueba el tamaño después de añadir uno, y activa solo las features de `web-sys` que realmente uses.
- **Prueba `opt-level = "z"`** y quédate con él si resulta más pequeño para tu código.
- **Opcionalmente, ejecuta `wasm-opt`** de [Binaryen](https://github.com/WebAssembly/binaryen) después de `wasm-bindgen`: `wasm-opt -Oz -o out.wasm pkg/my_site_bg.wasm`. Suele recortar unos cuantos puntos porcentuales más. Si rechaza el módulo por una característica de WebAssembly, activa esa característica con la opción `--enable-…` correspondiente. Este repositorio no lo usa, así que las cifras de arriba son sin él.

## Renderizado en el servidor

Los tiempos de renderizado en el servidor dependen de tus páginas y de tu máquina. La página [Benchmarks](../benchmarks/) compara una página completa renderizada por `active` con código `format!` escrito a mano y con otros frameworks de Rust, con el método y las cifras.
