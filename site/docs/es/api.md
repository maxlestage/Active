---
title: Referencia de la API
description: Cada elemento público del crate active con su firma: vistas, elementos, estado, efectos, eventos, Seo, Document, PWA, islas, funciones del navegador y features.
---

Esta página enumera todos los elementos públicos del crate `active`, agrupados por tema, con su firma exacta y una descripción de una línea. Las guías explican cómo usarlos; la documentación generada en [docs.rs/active](https://docs.rs/active) contiene los mismos elementos con ejemplos.

## Features de Cargo

| Feature | Por defecto | Añade |
| --- | --- | --- |
| `client` | desactivada | El runtime del navegador: [`hydrate`](#hydrate), [`mount`](#mount), [`mount_to`](#mount-to), [`location`](#location), [`query`](#query), [`query_param`](#query-param), [`navigate`](#navigate), [`update_head`](#update-head), `Event::raw`, listeners de eventos reales, efectos y callbacks `on_mount`. Depende de `wasm-bindgen` y `web-sys`. |

Sin `client`, el crate no tiene dependencias. Los manejadores de eventos, los efectos y los callbacks `on_mount` se aceptan y se ignoran, así que los mismos componentes compilan en el servidor. Activa `client` solo para la compilación `wasm32` (consulta [Instalación](../installation/#un-crate-dos-destinos)).

El crate usa la edición 2024 y necesita Rust 1.85 o posterior. No contiene código `unsafe`.

## Estructura del crate

| Ruta | Contenido |
| --- | --- |
| `active::*` | todos los elementos de abajo |
| `active::html` | una función por etiqueta HTML y SVG |
| `active::pwa` | [`Manifest`](#manifest), [`ServiceWorker`](#serviceworker), [`Icon`](#icon) y [`register_script`](#register-script): sitios instalables y sin conexión |
| `active::prelude` | `html::*`, `Document`, `Element`, `Event`, `Node`, `Seo`, `State`, `effect`, `el`, `fragment`, `fragment_dyn`, `indexed`, `island`, `keyed`, `memo`, `on_cleanup`, `switch`, `text`, `text_dyn`, `untrack`, `use_state`; con `client`, también `location`, `navigate` y `query_param` |

No están en el prelude: `Str`, `IslandFn`, `Dom`, `Scope`, `scope`, `hydrate`, `mount`, `mount_to`, `query`, `update_head`. Úsalos como `active::hydrate`, etc.

## Vistas

### Node

```rust
pub enum Node {
    Empty,
    Element(Box<Element>),
    Text(Str),
    DynText(/* closure */),
    Fragment(Vec<Node>),
    Seo(Box<Seo>),
    Island(Box</* island data */>),
    Dynamic(/* closure */),
}
```

Una pieza de una vista. Rara vez construirás las variantes a mano: lo hacen las funciones de etiqueta, `text`, `fragment`, `island` y las conversiones `From`.

| Variante | Significado |
| --- | --- |
| `Empty` | nada (el valor `Default`) |
| `Element` | un elemento HTML |
| `Text` | texto escapado |
| `DynText` | texto calculado a partir de estados ([`text_dyn`](#funciones-libres)) |
| `Fragment` | varios nodos, uno junto a otro |
| `Seo` | etiquetas SEO, que `Document` escribe en el `<head>` |
| `Island` | una isla ([`island`](#island)) |
| `Dynamic` | una región que se reconstruye cuando cambian sus estados ([`fragment_dyn`](#funciones-libres)) |
| `Keyed` | una lista cuyas filas se conservan por clave ([`keyed`](#funciones-libres)) |

| Método | Descripción |
| --- | --- |
| `pub fn render(&self) -> String` | Renderiza la vista a HTML. Se puede llamar cualquier número de veces. |
| `pub fn render_into(&self, out: &mut String)` | Igual, pero agrega el resultado a `out`. |

Implementa `Clone`, `Default` y `Debug` (que imprime el HTML renderizado).

Conversiones a `Node` (`From`): `Element`, `&'static str`, `String`, `Str`, `Seo`, `Vec<T>`, `Option<T>` y `[T; N]` para cualquier `T: Into<Node>`.

### Element

Un elemento HTML en construcción, creado por una [función de etiqueta](#funciones-de-etiqueta) o por [`el`](#funciones-libres). Cada método recibe `self` y devuelve el `Element` actualizado (`#[must_use]`), así que las llamadas se encadenan. Implementa `Clone` e `Into<Node>`.

**Atributos y clases**

| Método | Descripción |
| --- | --- |
| `pub fn attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Define un atributo; volver a definir el mismo nombre reemplaza el valor. `attr("class", …)` añade clases, igual que `class`. |
| `pub fn attr_dyn(self, name: impl Into<Str>, f: impl Fn() -> String + 'static) -> Self` | Atributo recalculado a partir de estados (reemplaza uno anterior con el mismo nombre). Para `value`, también define el valor actual del campo en el navegador. |
| `pub fn bool_attr(self, name: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Atributo booleano, presente mientras `f()` sea true (reemplaza uno anterior con el mismo nombre). Para `checked`, también define el estado de la casilla en el navegador. |
| `pub fn id(self, id: impl Into<Str>) -> Self` | `id="…"` (reemplaza un id anterior). |
| `pub fn class(self, class: impl Into<Str>) -> Self` | Añade clases (separadas por espacios); las cadenas vacías se ignoran. |
| `pub fn class_if(self, class: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Añade `class` mientras `f()` sea true. |
| `pub fn style(self, style: impl Into<Str>) -> Self` | `style="…"` (reemplaza un estilo anterior). |
| `pub fn href(self, href: impl Into<Str>) -> Self` | `href="…"` (reemplaza un href anterior). |

**Contenido**

| Método | Descripción |
| --- | --- |
| `pub fn child(self, child: impl Into<Node>) -> Self` | Agrega un hijo. Después de `html`/`html_dyn`, empieza una nueva lista de hijos. |
| `pub fn children(self, children: impl IntoIterator<Item = impl Into<Node>>) -> Self` | Agrega varios hijos. |
| `pub fn text(self, value: impl Into<Str>) -> Self` | Agrega texto escapado. |
| `pub fn text_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Agrega texto recalculado a partir de estados. |
| `pub fn children_dyn(self, f: impl Fn() -> Vec<Node> + 'static) -> Self` | Agrega una región dinámica, que se reconstruye cuando cambian los estados que lee `f`. Equivale a `.child(fragment_dyn(f))`. |
| `pub fn children_indexed<T>(self, items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Self` (`T: Clone + PartialEq`) | Agrega una lista conservada por posición. Equivale a `.child(indexed(items, row))`. |
| `pub fn children_keyed<T, K>(self, items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Self` (`T: PartialEq`, `K: Hash`) | Agrega una lista conservada por clave. Equivale a `.child(keyed(items, key, row))`. |
| `pub fn html(self, html: impl Into<Str>) -> Self` | Reemplaza el contenido por HTML de confianza sin escapar. |
| `pub fn html_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Igual, pero recalculado a partir de estados. |

**Eventos y acceso al DOM** (solo en el navegador; se ignoran sin `client`)

| Método | Descripción |
| --- | --- |
| `pub fn on(self, event: &'static str, handler: impl Fn(Event) + 'static) -> Self` | Escucha un evento del DOM. |
| `pub fn on_click(self, handler: impl Fn(Event) + 'static) -> Self` | `on("click", …)`. |
| `pub fn on_input(self, handler: impl Fn(Event) + 'static) -> Self` | `on("input", …)`. |
| `pub fn on_mount(self, f: impl Fn(&Dom) + 'static) -> Self` | Ejecuta `f` con el elemento del DOM en cuanto está en la página. |

### Funciones libres

| Firma | Descripción |
| --- | --- |
| `pub fn el(tag: &'static str) -> Element` | Un elemento con cualquier nombre de etiqueta, p. ej. `el("my-widget")`. |
| `pub fn text(value: impl Into<Str>) -> Node` | Un nodo de texto. |
| `pub fn text_dyn(f: impl Fn() -> String + 'static) -> Node` | Texto recalculado a partir de estados. |
| `pub fn fragment(nodes: impl IntoIterator<Item = impl Into<Node>>) -> Node` | Varios nodos sin elemento contenedor. |
| `pub fn fragment_dyn(f: impl Fn() -> Vec<Node> + 'static) -> Node` | Una región dinámica: en el navegador se reconstruye entera cuando cambian los estados que lee `f`; se liberan los estados, los efectos y los listeners creados para los nodos anteriores. En el servidor se renderiza entre `<!--[-->` y `<!--]-->`. |
| `pub fn keyed<T, K>(items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Node` (`T: PartialEq`, `K: Hash`) | Una lista cuyas filas se conservan por clave: en el navegador, las filas cuya clave sigue ahí y cuyo elemento es igual conservan su DOM, sus estados, sus listeners y el foco (y se mueven si cambió el orden); las filas nuevas o modificadas se construyen y las eliminadas se liberan. En el servidor se renderiza como `fragment_dyn`. Consulta [Estado](../state/#listas-largas-filas-por-clave). |
| `pub fn indexed<T>(items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Node` (`T: Clone + PartialEq`) | Una lista cuyas filas se conservan por posición: la fila `i` se construye una vez con un `State` del elemento `i` y se actualiza en su sitio; las filas se añaden o se quitan al final. Ver [Estado](../state/#filas-por-posición-indexed). |
| `pub fn switch<V, K>(value: impl Fn() -> V + 'static, kind: impl Fn(&V) -> K + 'static, view: impl Fn(State<V>) -> Node + 'static) -> Node` (`V: Clone + PartialEq`, `K: PartialEq`) | Muestra `view` para `value()`, reconstruida solo cuando cambia `kind`; si no, el `State<V>` de la vista recibe el nuevo valor. Ver [Aplicaciones del lado del cliente](../client-app/#conservar-una-página-cuando-cambian-sus-parámetros). |

### Str

```rust
pub type Str = std::borrow::Cow<'static, str>;
```

El tipo de cadena de los nombres y valores de atributos, los textos y las URL: un `&'static str` o un `String`.

### Funciones de etiqueta

`active::html` tiene una función por etiqueta, cada una `pub fn name() -> Element`:

`a` `abbr` `address` `area` `article` `aside` `audio` `b` `bdi` `blockquote` `br` `button` `canvas` `caption` `cite` `code` `col` `colgroup` `data` `datalist` `dd` `del` `details` `dfn` `dialog` `div` `dl` `dt` `em` `embed` `fieldset` `figcaption` `figure` `footer` `form` `h1` `h2` `h3` `h4` `h5` `h6` `header` `hgroup` `hr` `i` `iframe` `img` `input` `ins` `kbd` `label` `legend` `li` `main` `map` `mark` `menu` `meter` `nav` `noscript` `object` `ol` `optgroup` `option` `output` `p` `picture` `pre` `progress` `q` `s` `samp` `search` `section` `select` `small` `source` `span` `strong` `sub` `summary` `sup` `table` `tbody` `td` `template` `textarea` `tfoot` `th` `thead` `time` `tr` `track` `u` `ul` `var` `video` `wbr`

SVG: `svg` `circle` `ellipse` `g` `line` `path` `polygon` `polyline` `rect` `tspan` `defs` `stop`, más `text_svg` (`<text>`), `use_svg` (`<use>`) y `linear_gradient` (`<linearGradient>`).

Los elementos vacíos (`area`, `br`, `col`, `embed`, `hr`, `img`, `input`, `source`, `track`, `wbr`, y `base`, `link`, `meta` mediante `el`) se escriben como `<tag …/>` e ignoran los hijos. No hay función `html`, `head` ni `body`: [`Document`](#document) escribe esas etiquetas.

## Estado

### use_state

```rust
pub fn use_state<T: 'static>(initial: T) -> State<T>
```

Crea un valor reactivo que pertenece al componente que se está construyendo. En el navegador se libera junto con su región dinámica; en el servidor, al final del segundo renderizado de nivel superior que ni lo lee ni lo crea.

### State

```rust
pub struct State<T> { /* index into the state store */ }
```

Un handle a un valor reactivo. Implementa `Copy`, `Clone` y `Debug` para cualquier `T`. No es ni `Send` ni `Sync`: el valor vive en el almacén del hilo que lo creó, así que un handle no puede moverse a otro hilo ni conservarse a través de un `.await` en un future multihilo (`Send`).

| Método | Descripción |
| --- | --- |
| `pub fn get(self) -> T where T: Clone` | Un clon del valor. Dentro de un efecto o de un vínculo, crea una suscripción. |
| `pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R` | Lee el valor por referencia. Crea una suscripción igual que `get`. |
| `pub fn set(self, value: T)` | Reemplaza el valor y vuelve a ejecutar lo que lo leyó. |
| `pub fn update(self, f: impl FnOnce(&mut T))` | Modifica el valor en el sitio y vuelve a ejecutar lo que lo leyó. |
| `pub fn is_alive(self) -> bool` | `false` una vez liberado el estado (su región se reconstruyó). Compruébalo antes de escribir desde código asíncrono. |

### effect

```rust
pub fn effect(f: impl Fn() + 'static)
```

Ejecuta `f` ahora, y de nuevo cada vez que cambia un estado que leyó durante su última ejecución. Solo en el navegador: sin `client`, no hace nada. Un efecto que escribe un estado que él mismo lee no se vuelve a disparar.

### on_cleanup

```rust
pub fn on_cleanup(f: impl FnOnce() + 'static)
```

Ejecuta `f` cuando se libera el componente en construcción: su región dinámica se reconstruye, el router muestra otra página o su `Scope` se libera. Las limpiezas se ejecutan de la última registrada a la primera, antes de liberar los estados del ámbito. En el servidor, o fuera de cualquier ámbito, `f` nunca se ejecuta.

### memo

```rust
pub fn memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> State<T>
```

Un estado que contiene `f()`, recalculado cuando cambian los estados que lee `f` y escrito solo si el resultado difiere. En el servidor conserva el valor calculado al crearlo.

### untrack

```rust
pub fn untrack<R>(f: impl FnOnce() -> R) -> R
```

Ejecuta `f` sin suscribir el efecto o la región dinámica en curso a los estados que `f` lee.

### scope y Scope

```rust
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Scope)
pub struct Scope(/* … */);
impl Scope { pub fn dispose(self) }
```

`scope` ejecuta `f` y devuelve su resultado con un `Scope` que posee los estados, los efectos y los listeners de eventos creados dentro. `dispose` los libera. `Scope` es `#[must_use]`. En el servidor, un renderizado dentro de un scope abierto no vacía el almacén de estados.

## Eventos

### Event

Un evento del DOM que se pasa a los manejadores. Implementa `Debug` y `Clone`.

| Método | Descripción |
| --- | --- |
| `pub fn value(&self) -> String` | Valor actual del `<input>`, `<textarea>` o `<select>` que disparó el evento; vacío en otro caso. |
| `pub fn key(&self) -> String` | La tecla de un evento de teclado (`"Enter"`, `"k"`…); vacía en otro caso. |
| `pub fn prevent_default(&self)` | Cancela la acción por defecto del navegador. |
| `pub fn current_target(&self) -> Option<Dom>` | El elemento al que está conectado el listener. `None` en el servidor. |
| `pub fn raw(&self) -> &web_sys::Event` | El evento subyacente. Solo con `client`. |

### Dom

```rust
pub type Dom = web_sys::Element; // with `client`
pub struct Dom;                  // without: an empty placeholder (Debug, Clone, Copy, Default)
```

El tipo de elemento que recibe `on_mount` y que devuelve `Event::current_target`.

## SEO

### Seo

```rust
pub struct Seo { /* … */ }
```

Las etiquetas del `<head>` de una página. Implementa `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`, `Display` (misma salida que `to_html`) e `Into<Node>`. `#[must_use]`: los setters devuelven el valor actualizado. Reglas y ejemplos: [SEO](../seo/).

| Método | Descripción |
| --- | --- |
| `pub fn new() -> Self` | Vacío; solo renderiza `og:type` (`website`) y `twitter:card` (`summary`). |
| `pub fn title(self, title: impl Into<String>) -> Self` | `<title>` y `og:title`. |
| `pub fn title_template(self, template: impl Into<String>) -> Self` | Plantilla para `<title>`; el primer `%s` se reemplaza por el título. |
| `pub fn description(self, description: impl Into<String>) -> Self` | Meta description y `og:description`. |
| `pub fn canonical(self, url: impl Into<String>) -> Self` | `<link rel="canonical">` y `og:url`. |
| `pub fn image(self, url: impl Into<String>) -> Self` | `og:image`; cambia `twitter:card` a `summary_large_image`. |
| `pub fn og_type(self, og_type: impl Into<String>) -> Self` | `og:type`. |
| `pub fn site_name(self, site_name: impl Into<String>) -> Self` | `og:site_name`, y el título cuando no se define ninguno. |
| `pub fn locale(self, locale: impl Into<String>) -> Self` | `og:locale`. |
| `pub fn twitter(self, handle: impl Into<String>) -> Self` | `twitter:site`. |
| `pub fn base_url(self, url: impl Into<String>) -> Self` | Origen para las URL relativas de canonical, image y alternate. |
| `pub fn noindex(self, noindex: bool) -> Self` | `noindex` en `<meta name="robots">`. |
| `pub fn nofollow(self, nofollow: bool) -> Self` | `nofollow` en `<meta name="robots">`. |
| `pub fn alternate(self, lang: impl Into<String>, url: impl Into<String>) -> Self` | `<link rel="alternate" hreflang>`; volver a usar el mismo idioma reemplaza la URL. |
| `pub fn json_ld(self, json: impl Into<String>) -> Self` | Un `<script>` JSON-LD, uno por llamada; `<` se escapa como `\u003c`. |
| `pub fn meta_name(self, name: impl Into<String>, content: impl Into<String>) -> Self` | `<meta name content>` adicional. |
| `pub fn meta_property(self, property: impl Into<String>, content: impl Into<String>) -> Self` | `<meta property content>` adicional. |
| `pub fn merge(&mut self, other: &Seo)` | Aplica `other` encima: sus valores definidos ganan, las alternativas y las meta adicionales se reemplazan por clave, y el JSON-LD se añade al final. |
| `pub fn merged(&self, other: &Seo) -> Seo` | `merge` en un valor nuevo. |
| `pub fn to_html(&self) -> String` | Renderiza las etiquetas. |
| `pub fn write_html(&self, out: &mut String)` | Agrega las etiquetas a `out`. |

## Documentos

### Document

```rust
pub struct Document { /* … */ }
```

Una página HTML completa. Implementa `Clone` y `Default`. Los métodos del builder reciben `self` y son `#[must_use]`. Guía: [Documentos](../documents/).

| Método | Descripción |
| --- | --- |
| `pub fn new(body: impl Into<Node>) -> Self` | Una página cuyo `<body>` es `body`. |
| `pub fn lang(self, lang: impl Into<Str>) -> Self` | `<html lang>`. |
| `pub fn seo(self, seo: Seo) -> Self` | SEO base, fusionado con cada `Seo` del body; reemplaza una base anterior. |
| `pub fn stylesheet(self, href: impl Into<Str>) -> Self` | `<link rel="stylesheet">`, en el orden de las llamadas. |
| `pub fn head(self, html: impl Into<Str>) -> Self` | HTML de confianza añadido al `<head>`. |
| `pub fn client(self, js: impl Into<Str>, wasm: impl Into<Str>) -> Self` | Precarga y arranca el bundle de `wasm-bindgen --target web --no-typescript`. |
| `pub fn manifest(self, href: &str, manifest: &pwa::Manifest) -> Self` | `<link rel="manifest">` hacia `href`, `theme-color`, el título de la pantalla de inicio y el icono de Apple (el icono PNG más grande de tipo `any`). |
| `pub fn service_worker(self, url: &str) -> Self` | Un script en línea que registra el service worker servido en `url` cuando la página ha cargado. |
| `pub fn viewport(self, content: impl Into<Str>) -> Self` | `content` de `<meta name="viewport">` (por defecto `width=device-width, initial-scale=1`). |
| `pub fn html_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Atributo en `<html>`. |
| `pub fn body_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Atributo en `<body>`. |
| `pub fn final_seo(&self) -> Seo` | El SEO fusionado de la página, incluidas las islas y el contenido actual de las regiones dinámicas (evaluadas por separado de `render`). |
| `pub fn render(&self) -> String` | La página completa. Renderiza primero el body y recoge sus valores `Seo` por el camino, así que cada región dinámica se evalúa una sola vez. Se puede llamar cualquier número de veces. |

## PWA

El módulo `active::pwa`. Guía: [Apps instalables (PWA)](../pwa/).

### Manifest

```rust
pub struct Manifest { /* … */ }
```

El manifiesto de la app web. Implementa `Clone`, `Debug`, `PartialEq` y `Eq`. Los métodos del builder toman `self` y son `#[must_use]`.

| Método | Descripción |
| --- | --- |
| `pub fn new(name: impl Into<String>) -> Self` | Un manifiesto llamado `name`, `start_url` `./`, `display` `standalone`, tema `#000000`, fondo `#ffffff`. |
| `pub fn short_name(self, name: impl Into<String>) -> Self` | El nombre bajo el icono. |
| `pub fn description(self, description: impl Into<String>) -> Self` | Lo que hace la app. |
| `pub fn id(self, id: impl Into<String>) -> Self` | La identidad de la app (por defecto `start_url`). |
| `pub fn start_url(self, url: impl Into<String>) -> Self` | La página que abre el icono. |
| `pub fn scope(self, scope: impl Into<String>) -> Self` | Las URL que forman parte de la app (por defecto la carpeta del manifiesto). |
| `pub fn display(self, display: impl Into<String>) -> Self` | `standalone`, `fullscreen`, `minimal-ui` o `browser`. |
| `pub fn orientation(self, orientation: impl Into<String>) -> Self` | `portrait`, `landscape`, `any`… |
| `pub fn theme_color(self, color: impl Into<String>) -> Self` | Color de la barra de estado y del título de la ventana. |
| `pub fn background_color(self, color: impl Into<String>) -> Self` | Color de la pantalla de inicio al arrancar. |
| `pub fn lang(self, lang: impl Into<String>) -> Self` | Idioma de los textos. |
| `pub fn categories<I, S>(self, categories: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | Categorías de las tiendas; reemplaza las anteriores. |
| `pub fn icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | Un icono de tipo `any`. |
| `pub fn maskable_icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | Un icono de tipo `maskable` (contenido dentro del 80 % central). |
| `pub fn name(&self) -> &str` | El nombre. |
| `pub fn theme(&self) -> &str` | El color del tema. |
| `pub fn icons(&self) -> &[Icon]` | Los iconos. |
| `pub fn to_json(&self) -> String` | El manifiesto en JSON (cadenas escapadas, `<` como `\u003c`). |

### Icon

```rust
pub struct Icon { pub src: String, pub sizes: String, pub mime: String, pub purpose: String }
```

Un icono del manifiesto: URL, `"192x192"` o `"any"`, tipo MIME, y `"any"`, `"maskable"` o `"monochrome"`.

### ServiceWorker

```rust
pub struct ServiceWorker { /* … */ }
```

El service worker sin conexión. Implementa `Clone`, `Debug`, `PartialEq` y `Eq`. Los métodos del builder toman `self` y son `#[must_use]`.

| Método | Descripción |
| --- | --- |
| `pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self` | Un worker cuya caché es `{name}-{version}`; una versión nueva reemplaza las cachés de las anteriores. |
| `pub fn precache<I, S>(self, urls: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | Las URL que se guardan al instalar; se suman a las anteriores. |
| `pub fn offline_page(self, url: impl Into<String>) -> Self` | La página que se muestra sin conexión para una página nunca guardada; también se guarda de antemano. |
| `pub fn network_only(self, prefix: impl Into<String>) -> Self` | Las rutas que empiezan por `prefix` siempre van a la red. |
| `pub fn cache_name(&self) -> String` | `{name}-{version}`. |
| `pub fn to_js(&self) -> String` | El script: las páginas primero de la red (sin conexión, la copia guardada y luego la página sin conexión), los demás archivos `GET` del sitio desde la caché, actualizados en segundo plano. |

### register_script

```rust
pub fn register_script(url: &str) -> String
```

El script en línea que escribe `Document::service_worker`: registra el worker de `url` en el evento `load`. Calcula su hash para autorizarlo en una Content-Security-Policy.

## Islas

### island

```rust
pub fn island(name: &'static str, props: impl Into<Str>, component: IslandFn) -> Node
```

Llama a `component(&props)` y envuelve su vista en `<active-island data-island="name" data-props="props">`, para que la hidrate en el navegador el componente registrado con el nombre `name`.

### IslandFn

```rust
pub type IslandFn = fn(&str) -> Node;
```

Un componente de isla: construye su vista a partir de una cadena de props, de forma idéntica en el servidor y en el navegador.

## Funciones del navegador

Disponibles con la feature `client`, en la raíz del crate.

### hydrate

```rust
pub fn hydrate(islands: &[(&str, IslandFn)])
```

Hidrata cada `<active-island>` de la página cuyo nombre esté en `islands`: ejecuta el componente con sus props y vincula los listeners, los textos, los atributos y las clases dinámicos al HTML existente. Las islas ya hidratadas y los nombres desconocidos se omiten. Si la estructura no coincide, muestra una advertencia en la consola y renderiza la isla desde cero. Nunca cambia el `<head>`.

### mount

```rust
pub fn mount(view: &Node, parent: &web_sys::Element)
```

Crea el DOM de `view` después de los hijos existentes de `parent`. Fusiona los valores `Seo` de la vista y los escribe en el head con `update_head`; las regiones dinámicas de la vista actualizan el head cuando se reconstruyen.

### mount_to

```rust
pub fn mount_to(selector: &str, view: impl Into<Node>)
```

Reemplaza el contenido del primer elemento que coincide con el `selector` CSS por `view` (usando `mount`). Muestra una advertencia en la consola si nada coincide. Es el punto de entrada habitual de una [aplicación del lado del cliente](../client-app/).

### location

```rust
pub fn location() -> State<String>
```

La ruta actual (`window.location.pathname`, p. ej. `"/search"`), como estado. La primera llamada a `location`, `query` o `query_param` inicia el enrutador: Atrás y Adelante lo actualizan, y los clics en enlaces `<a href>` del mismo origen llaman a `navigate` en lugar de cargar una página (salvo los enlaces con un `target` distinto de `_self` o con un atributo `download`, los clics con una tecla modificadora o con un botón que no sea el principal, y las anclas de la página actual).

### query

```rust
pub fn query() -> State<String>
```

La cadena de consulta actual sin el `?` (`"q=rust&page=2"`, vacía si no hay ninguna), como un estado que el enrutador actualiza igual que `location`.

### query_param

```rust
pub fn query_param(name: &str) -> Option<String>
```

Un parámetro de la cadena de consulta, decodificado (`query_param("q")` en `/search?q=caf%C3%A9` da `Some("café")`), o `None`. Leerlo en una vista, una región dinámica o un efecto crea una suscripción a `query`.

### navigate

```rust
pub fn navigate(to: &str)
```

Añade una entrada al historial para `to` (`"/about"`, `"/search?q=rust"`, `"/docs#install"`), actualiza `location()` y `query()` (solo los que cambiaron, así que solo se reconstruyen las vistas que dependen de ellos) y después se desplaza al principio, o al elemento cuyo id es el `#fragment`.

### update_head

```rust
pub fn update_head(seo: &Seo)
```

Escribe `seo` en el `<head>` de la página: define `document.title` (con un título vacío cuando `seo` no tiene ni título ni nombre de sitio) y reemplaza las etiquetas de descripción, robots, canonical, alternativas `hreflang`, Open Graph, Twitter y JSON-LD, así como cualquier metaetiqueta con el mismo nombre o la misma propiedad que una de las metaetiquetas adicionales. Los demás elementos del head no se tocan.

## Marcadores HTML

La salida del servidor contiene algunos marcadores de los que depende la hidratación. Forman parte del formato: no elimines los comentarios HTML de las páginas con islas.

| Marcador | Significado |
| --- | --- |
| `<!--#-->` | inicio de un texto dinámico |
| `<!---->` | separador entre dos textos adyacentes |
| `<!--[-->` … `<!--]-->` | una región dinámica |
| `<active-island data-island data-props>` | una isla; recibe `data-hydrated` una vez hidratada |

## Pánicos

- Usar un `State` después de liberarlo provoca un pánico con "this state was disposed". En el navegador ocurre después de que se reconstruya su región dinámica o se libere su scope; en el servidor, después de dos renderizados de nivel superior que no lo usaron.
- Leer un estado dentro de su propio closure de `update` provoca un pánico con "a state was read inside its own `update`; read it before updating". Llamar a `set` o `update` sobre un estado dentro de su propio `with` provoca un pánico con "a state was updated while being read or updated".
