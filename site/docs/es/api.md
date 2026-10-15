---
title: Referencia de la API
description: Cada elemento público del crate Active con su firma: vistas, elementos, estado, efectos, eventos, Seo, Document, PWA, islas, funciones del navegador y features.
---

Esta página enumera todos los elementos públicos del crate `active`, agrupados por tema, con su firma exacta y una descripción de una línea. Las guías explican cómo usarlos; la documentación generada en [docs.rs/active](https://docs.rs/active) contiene los mismos elementos con ejemplos.

## Features de Cargo

| Feature | Por defecto | Añade |
| --- | --- | --- |
| `client` | desactivada | El runtime del navegador: [`hydrate`](#hydrate), [`mount`](#mount), [`mount_to`](#mount-to), [`location`](#location), [`query`](#query), [`query_param`](#query-param), [`navigate`](#navigate), [`navigate_replace`](#navigate-replace), [`update_head`](#update-head), `Event::raw`, listeners de eventos reales, efectos, callbacks `on_mount`, transiciones, portales, los futures de los recursos y las estrategias de hidratación de las islas. Depende de `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys` y `web-sys`. |

Sin `client`, el crate no tiene dependencias. Los manejadores de eventos, los efectos y los callbacks `on_mount` se aceptan y se ignoran, así que los mismos componentes compilan en el servidor. Activa `client` solo para la compilación `wasm32` (consulta [Instalación](../installation/#un-crate-dos-destinos)).

El crate usa la edición 2024 y necesita Rust 1.85 o posterior. No contiene código `unsafe`.

## Estructura del crate

| Ruta | Contenido |
| --- | --- |
| `active::*` | todos los elementos de abajo |
| `active::html` | una función por etiqueta HTML y SVG |
| `active::forms` | [`Form`](#form), [`Field`](#field), [`Rule`](#rule) y las reglas: formularios validados |
| `active::pwa` | [`Manifest`](#manifest), [`ServiceWorker`](#serviceworker), [`Icon`](#icon) y [`register_script`](#register-script) y [`version`](#version): sitios instalables y sin conexión |
| `active::prelude` | `html::*`, `Document`, `Element`, `Event`, `Node`, `Seo`, `State`, `effect`, `el`, `fragment`, `fragment_dyn`, `indexed`, `island`, `keyed`, `memo`, `on_cleanup`, `switch`, `text`, `text_dyn`, `untrack`, `use_state`, `watch`, `watch_immediate`, `provide`, `use_context`, `expect_context`, `match_path`; con `client`, también `location`, `navigate` y `query_param` |

No están en el prelude: `Str`, `IslandFn`, `Dom`, `Scope`, `scope`, `Watcher`, `Params`, `Routes`, `hydrate`, `mount`, `mount_to`, `query`, `update_head`, ni los elementos añadidos a partir de los puntos fuertes de otros frameworks: `batch`, `resource`, `Resource`, `suspense`, `error_boundary`, `ErrorBoundary`, `report_error`, `portal`, `island_with`, `Hydrate`, `before_navigate`, `Navigation`, `navigate_replace` y el módulo `forms`. Úsalos como `active::hydrate`, etc.

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
| `pub fn style_dyn(self, property: &'static str, f: impl Fn() -> String + 'static) -> Self` | Una propiedad CSS recalculada a partir de estados, añadida al `style` puesto antes; un valor vacío la omite. Un `style` puesto después la reemplaza. |
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
| `pub fn on_submit(self, handler: impl Fn(Event) + 'static) -> Self` | `on("submit", …)` que primero llama a `prevent_default`. |
| `pub fn on_key(self, key: &'static str, handler: impl Fn(Event) + 'static) -> Self` | Listener de `keydown` llamado solo para `key` (`"Enter"`, `"Escape"`…). |
| `pub fn on_mount(self, f: impl Fn(&Dom) + 'static) -> Self` | Ejecuta `f` con el elemento del DOM en cuanto está en la página. |
| `pub fn transition(self, name: &'static str, when: impl Fn() -> bool + 'static) -> Self` | Visible mientras `when` es verdadero, oculto (`hidden`) si no, con las clases `{name}-enter-*` / `{name}-leave-*` en cada cambio; el servidor renderiza `hidden` cuando es falso. Ver [Componentes](../components/#transiciones). |
| `pub fn transition_in(self, name: &'static str) -> Self` | Reproduce `{name}-enter-*` cuando el elemento se añade a la página (no al hidratar). |

**Enlace bidireccional** (el valor se renderiza en el servidor; el listener solo existe con `client`)

| Método | Descripción |
| --- | --- |
| `pub fn bind_value(self, state: State<String>) -> Self` | Valor del campo ⇄ estado: `<input>` y `<textarea>` se escriben en `input`, `<select>` en `change`. Un `<textarea>` renderiza el estado como texto. |
| `pub fn bind_checked(self, state: State<bool>) -> Self` | Casilla ⇄ estado, se escribe en `change`. |
| `pub fn bind_parsed<T>(self, state: State<T>) -> Self` (`T: FromStr + Display + PartialEq`) | Campo ⇄ estado de un tipo convertido (números): se escribe cuando el texto se convierte; si no, se conserva el texto escrito. |
| `pub fn bind_field(self, field: forms::Field) -> Self` | `bind_value` sobre el valor del campo, tocado en `blur`, `aria-invalid="true"` mientras se muestra su error. Ver [Formularios](../forms/). |

### Funciones libres

| Firma | Descripción |
| --- | --- |
| `pub fn el(tag: &'static str) -> Element` | Un elemento con cualquier nombre de etiqueta, p. ej. `el("my-widget")`. |
| `pub fn text(value: impl Into<Str>) -> Node` | Un nodo de texto. |
| `pub fn text_dyn(f: impl Fn() -> String + 'static) -> Node` | Texto recalculado a partir de estados. |
| `pub fn fragment(nodes: impl IntoIterator<Item = impl Into<Node>>) -> Node` | Varios nodos sin elemento contenedor. |
| `pub fn fragment_dyn(f: impl Fn() -> Vec<Node> + 'static) -> Node` | Una región dinámica: en el navegador se reconstruye entera cuando cambian los estados que lee `f`; se liberan los estados, los efectos y los listeners creados para los nodos anteriores, y el foco vuelve al elemento correspondiente de los nodos nuevos. En el servidor se renderiza entre `<!--[-->` y `<!--]-->`. |
| `pub fn keyed<T, K>(items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Node` (`T: PartialEq`, `K: Hash`) | Una lista cuyas filas se conservan por clave: en el navegador, las filas cuya clave sigue ahí y cuyo elemento es igual conservan su DOM, sus estados, sus listeners y el foco (y se mueven si cambió el orden); las filas nuevas o modificadas se construyen y las eliminadas se liberan. En el servidor se renderiza como `fragment_dyn`. Consulta [Estado](../state/#listas-largas-filas-por-clave). |
| `pub fn indexed<T>(items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Node` (`T: Clone + PartialEq`) | Una lista cuyas filas se conservan por posición: la fila `i` se construye una vez con un `State` del elemento `i` y se actualiza en su sitio; las filas se añaden o se quitan al final. Ver [Estado](../state/#filas-por-posicion-indexed). |
| `pub fn switch<V, K>(value: impl Fn() -> V + 'static, kind: impl Fn(&V) -> K + 'static, view: impl Fn(State<V>) -> Node + 'static) -> Node` (`V: Clone + PartialEq`, `K: PartialEq`) | Muestra `view` para `value()`, reconstruida solo cuando cambia `kind`; si no, el `State<V>` de la vista recibe el nuevo valor. Ver [Aplicaciones del lado del cliente](../client-app/#conservar-una-pagina-cuando-cambian-sus-parametros). |
| `pub fn portal(target: &'static str, view: impl Into<Node>) -> Node` | Renderiza `view` al final del elemento que coincide con `target` (solo en el navegador; nada en el servidor), eliminado con el portal. Ver [Componentes](../components/#portales). |

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

Ejecuta `f` ahora, y de nuevo cada vez que cambia un estado que leyó durante su última ejecución, una vez actualizados los memos que lee. Solo en el navegador: sin `client`, no hace nada. Un estado escrito dentro de un efecto avisa a sus lectores cuando el efecto termina; un efecto que escribe un estado que él mismo lee no se vuelve a disparar.

### on_cleanup

```rust
pub fn on_cleanup(f: impl FnOnce() + 'static)
```

Ejecuta `f` cuando se libera el componente en construcción: su región dinámica se reconstruye, el router muestra otra página o su `Scope` se libera. Las limpiezas se ejecutan de la última registrada a la primera, antes de liberar los estados del ámbito. En el servidor, o fuera de cualquier ámbito, `f` nunca se ejecuta.

### memo

```rust
pub fn memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> State<T>
```

Un estado que contiene `f()`, recalculado cuando cambian los estados que lee `f` — antes de que se ejecuten los efectos que lo leen, y solo si una entrada cambió de verdad — y escrito solo si el resultado difiere. En el servidor conserva el valor calculado al crearlo.

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

### watch y watch_immediate

```rust
pub fn watch<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, &T) + 'static) -> Watcher
pub fn watch_immediate<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, Option<&T>) + 'static) -> Watcher
```

Llama a `callback(&nuevo, &anterior)` cada vez que cambia el valor devuelto por `source` (`PartialEq`). `source` se sigue como un efecto, `callback` no. `watch` omite el primer valor; `watch_immediate` llama enseguida a `callback(&primero, None)`. Solo en el navegador, como `effect`; se libera con el componente.

### Watcher

```rust
#[derive(Clone, Copy, Debug)]
pub struct Watcher { /* … */ }
impl Watcher { pub fn stop(self) }
```

Detiene un watcher antes de que se libere su componente. Detenerlo dos veces no hace nada.

### batch

```rust
pub fn batch<R>(f: impl FnOnce() -> R) -> R
```

Ejecuta `f` reteniendo los efectos que disparan sus escrituras hasta que termina: cada uno se ejecuta entonces una vez. Los batch anidados esperan al más externo. Ver [Estado](../state/#agrupar-escrituras-batch).

### provide, use_context y expect_context

```rust
pub fn provide<T: 'static, R>(value: T, f: impl FnOnce() -> R) -> R
pub fn use_context<T: Clone + 'static>() -> Option<T>
pub fn expect_context<T: Clone + 'static>() -> T
```

`provide` da `value` a todo lo que construye `f`, incluidas las regiones dinámicas y las listas por clave (lo conservan al reconstruirse). `use_context` devuelve el valor de tipo `T` proporcionado más cercano, clonado; `expect_context` entra en pánico, nombrando el tipo, si no hay ninguno. Llámalas mientras construyes la vista. Ver [Estado](../state/).

## Datos asíncronos y errores

Ver [Datos asíncronos y errores](../async-data/).

### resource y Resource

```rust
pub fn resource<S, T, F>(source: impl Fn() -> S + 'static, fetch: impl Fn(S) -> F + 'static) -> Resource<T>
where S: Clone + PartialEq + 'static, T: 'static, F: Future<Output = T> + 'static

pub struct Resource<T> { /* … */ } // Copy
```

Carga `fetch(source())` y vuelve a cargar cuando `source` cambia; un resultado más antiguo que la última petición se ignora. En el navegador el future se ejecuta en el bucle de eventos; en el servidor se consulta una vez (un valor listo se renderiza; si no, sigue cargando).

| Método | Descripción |
| --- | --- |
| `pub fn get(self) -> Option<T>` (`T: Clone`) | El último valor, `None` antes del primero. |
| `pub fn with<R>(self, f: impl FnOnce(Option<&T>) -> R) -> R` | Lo lee por referencia. |
| `pub fn loading(self) -> bool` | Si hay una carga en curso. |
| `pub fn refetch(self)` | Vuelve a cargar con la fuente actual. |
| `pub fn set(self, value: T)` | Sustituye el valor de inmediato (actualización optimista). |
| `pub fn state(self) -> State<Option<T>>` | El estado que contiene el valor. |

### suspense

```rust
pub fn suspense(fallback: impl Into<Node>, view: impl FnOnce() -> Node) -> Node
```

Construye `view` una vez y muestra `fallback` en su lugar mientras un recurso creado dentro no haya cargado su primer valor. Las recargas posteriores conservan la vista.

### error_boundary, ErrorBoundary y report_error

```rust
pub fn error_boundary(view: impl Fn() -> Node + 'static, fallback: impl Fn(ErrorBoundary) -> Node + 'static) -> Node
pub fn report_error(error: impl Display) -> bool

#[derive(Clone, Copy, Debug)]
pub struct ErrorBoundary { /* … */ }
impl<T: Into<Node>, E: Display> From<Result<T, E>> for Node
```

Muestra `fallback` en lugar de `view` en cuanto se notifica un error dentro: un `Err` convertido en `Node`, `report_error` durante la construcción (devuelve `false` y lo registra si no hay boundary), o `ErrorBoundary::report` más tarde. `ErrorBoundary` tiene `report(error)`, `errors() -> Vec<String>`, `has_errors() -> bool` y `reset()` (olvida los errores y vuelve a construir la vista); la vista la obtiene con `use_context::<ErrorBoundary>()`.

## Formularios

El módulo `active::forms`. Ver [Formularios y validación](../forms/).

### Form

```rust
#[derive(Clone, Copy, Debug)]
pub struct Form { /* … */ }
```

| Método | Descripción |
| --- | --- |
| `pub fn new() -> Self` | Un formulario sin campos (también `Default`). |
| `pub fn field(self, value: State<String>, rules: impl IntoIterator<Item = Rule>) -> Field` | Añade un campo comprobado por `rules`, en orden. |
| `pub fn is_valid(self) -> bool` | Si todos los campos son válidos. |
| `pub fn submit(self) -> bool` | Muestra a partir de ahora todos los errores; devuelve si el formulario es válido. |
| `pub fn submitted(self) -> bool` | Si se llamó a `submit` desde el último reinicio. |
| `pub fn reset(self)` | Vuelve a ocultar los errores (los valores no cambian). |

### Field

```rust
#[derive(Clone, Copy, Debug)]
pub struct Field { /* … */ }
```

| Método | Descripción |
| --- | --- |
| `pub fn value(self) -> State<String>` | El estado del campo. |
| `pub fn error(self) -> Option<String>` | El error actual. |
| `pub fn is_valid(self) -> bool` | Si no hay error. |
| `pub fn message(self) -> Option<String>` | El error que hay que mostrar: una vez tocado o enviado. |
| `pub fn touched(self) -> bool` / `pub fn touch(self)` | Si el visitante abandonó el campo; marcarlo como abandonado. |

### Rule

```rust
#[derive(Clone)]
pub struct Rule { /* … */ }
impl Rule { pub fn check(&self, value: &str) -> Option<String> }

pub fn rule(valid: impl Fn(&str) -> bool + 'static, message: impl Into<Cow<'static, str>>) -> Rule
pub fn required(message: impl Into<Cow<'static, str>>) -> Rule
pub fn min_length(n: usize, message: impl Into<Cow<'static, str>>) -> Rule
pub fn max_length(n: usize, message: impl Into<Cow<'static, str>>) -> Rule
pub fn email(message: impl Into<Cow<'static, str>>) -> Rule
pub fn range(min: f64, max: f64, message: impl Into<Cow<'static, str>>) -> Rule
```

`check` devuelve el mensaje cuando `value` no es válido. Todas las reglas salvo `required` aceptan un valor vacío.

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
| `pub fn stop_propagation(&self)` | Impide que el evento llegue a los elementos padre. |
| `pub fn checked(&self) -> bool` | Si la casilla o el botón de radio que disparó el evento está marcado; `false` en otro caso. |

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

### version

```rust
pub fn version<I, B>(parts: I) -> String
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
```

Un hash corto de `parts` (10 caracteres hexadecimales, FNV-1a), para `ServiceWorker::new`: calcula el hash de lo que guarda el worker, y cada despliegue que lo cambie instala un worker nuevo.

## Enrutamiento

Disponible en el servidor y en el navegador. Ver [Aplicaciones del lado del cliente](../client-app/).

### match_path

```rust
pub fn match_path(pattern: &str, path: &str) -> Option<Params>
```

Compara `path` (sin su cadena de consulta, su fragmento ni las barras de más) con `pattern`: `posts` coincide con ese segmento, `:id` con un segmento, `:id?` con un segmento opcional, `*rest` (al final) con todo lo demás (`*` solo no captura nada). Los valores se decodifican.

### Params

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Params { /* … */ }
```

| Método | Descripción |
| --- | --- |
| `pub fn get(&self, name: &str) -> Option<&str>` | El valor de un parámetro. |
| `pub fn parse<T: FromStr>(&self, name: &str) -> Option<T>` | El valor convertido; `None` si falta o no es válido. |
| `pub fn iter(&self) -> impl Iterator<Item = (&str, &str)>` | Los parámetros, en el orden del patrón. |
| `pub fn is_empty(&self) -> bool` | Si no hay ninguno. |

### Routes

```rust
#[derive(Clone, Default)]
pub struct Routes { /* … */ }
```

| Método | Descripción |
| --- | --- |
| `pub fn new() -> Self` | Una tabla vacía. |
| `pub fn route(self, pattern: impl Into<String>, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | Añade una ruta; gana el primer patrón que coincide. |
| `pub fn fallback(self, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | La página que se muestra cuando nada coincide; su parámetro `path` contiene toda la ruta. |
| `pub fn resolve(&self, path: &str) -> Option<Params>` | Los parámetros de la ruta que coincide (o del fallback). |
| `pub fn matches(&self, path: &str) -> bool` | Si coincide una ruta distinta del fallback. |
| `pub fn render(&self, path: &str) -> Node` | La página de una ruta. |
| `pub fn view(self, path: impl Fn() -> String + 'static) -> Node` | La página de una ruta que cambia: se conserva mientras coincida la misma ruta (su `State<Params>` se actualiza) y se reconstruye cuando coincide otra. |

### before_navigate y Navigation

```rust
pub fn before_navigate(guard: impl Fn(&str) -> Navigation + 'static)

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation { Continue, Cancel, Redirect(String) }
```

Registra una guarda consultada antes de que el router del navegador vaya a una dirección (enlaces, `navigate`, `navigate_replace`, atrás y adelante). Las guardas se ejecutan en orden de registro; una redirección se vuelve a comprobar (diez seguidas cancelan). Se elimina con el componente que la registró. Nunca se llama en el servidor. Ver [Aplicaciones del lado del cliente](../client-app/#guardas-de-navegacion).

## Islas

### island

```rust
pub fn island(name: &'static str, props: impl Into<Str>, component: IslandFn) -> Node
```

Llama a `component(&props)` y envuelve su vista en `<active-island data-island="name" data-props="props">`, para que la hidrate en el navegador el componente registrado con el nombre `name`.

### island_with y Hydrate

```rust
pub fn island_with(name: &'static str, props: impl Into<Str>, component: IslandFn, when: Hydrate) -> Node

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hydrate { #[default] Load, Idle, Visible, Media(Str), Interaction }
```

Como `island`, hidratada cuando `when` lo indica: de inmediato, cuando el navegador está inactivo, al verse, mientras coincide una media query, o en la primera interacción. Añade `data-hydrate="idle|visible|media:…|interaction"` (nada para `Load`). Ver [Islas](../islands/#cuando-se-hidratan-las-islas).

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

Añade una entrada al historial para `to` (`"/about"`, `"/search?q=rust"`, `"/docs#install"`), actualiza `location()` y `query()` (solo los que cambiaron, así que solo se reconstruyen las vistas que dependen de ellos) y después se desplaza al principio, o al elemento cuyo id es el `#fragment`. Primero se consultan las guardas registradas con [`before_navigate`](#before-navigate-y-navigation); la posición de desplazamiento de la página abandonada se guarda para el botón atrás.

### navigate_replace

```rust
pub fn navigate_replace(to: &str)
```

Como `navigate`, sustituyendo la entrada actual del historial en lugar de añadir una.

### update_head

```rust
pub fn update_head(seo: &Seo)
```

Escribe `seo` en el `<head>` de la página: define `document.title` (con un título vacío cuando `seo` no tiene ni título ni nombre de sitio) y reemplaza las etiquetas de descripción, robots, canonical, alternativas `hreflang`, Open Graph, Twitter y JSON-LD, así como cualquier metaetiqueta con el mismo nombre o la misma propiedad que una de las metaetiquetas adicionales. Los demás elementos del head no se tocan.

## Streaming, enrutamiento, i18n y herramientas

Ninguno de estos elementos está en el preludio: úsalos como `active::ssr::stream`, `active::persisted`, `active::i18n::I18n`, etc. Las guías: [Streaming y datos del servidor](../streaming/), [Layouts, carga de datos y precarga](../routing/), [Listas animadas y acciones](../animations/), [Formularios y validación](../forms/), [Estado persistente y depuración](../stores/), [Traducciones y formato](../i18n/).

```rust
// active::ssr — see streaming
pub fn render(build: impl FnOnce() -> Document) -> String
pub fn stream(build: impl FnOnce() -> Document) -> Stream
pub fn resource<S, T: Data, F>(key: impl Into<String>, source: impl Fn() -> S + 'static, fetch: impl Fn(S) -> F + 'static) -> Resource<T>
impl Stream {
    pub fn poll_chunk(&mut self, cx: &mut Context<'_>) -> Poll<Option<String>>
    pub async fn next_chunk(&mut self) -> Option<String>
    pub fn now_or_never_all(&mut self) -> String
}
pub trait Data: Sized {                 // active::Data
    fn write_data(&self, out: &mut String);
    fn read_data(json: &str) -> Option<Self>;
    fn to_data(&self) -> String;
}

// Routes — see routing
impl Routes {
    pub fn layout(self, layout: impl Fn(Node) -> Node + 'static) -> Self
    pub fn nest(self, prefix: &str, routes: Routes) -> Self
    pub fn loading(self, view: impl Fn() -> Node + 'static) -> Self
    pub fn route_with_data<T: Data + Clone + PartialEq, F>(self, pattern: impl Into<String>,
        load: impl Fn(Params) -> F + 'static, page: impl Fn(State<T>, State<Params>) -> Node + 'static) -> Self
    pub fn respond(&self, path: &str) -> PageResponse   // status, location, body
}
pub fn set_status(status: u16)
pub fn redirect(to: impl Into<String>)
impl Document {
    pub fn prefetch(self, when: Prefetch) -> Self       // Prefetch::{Hover, Press, Visible}
    pub fn prerender(self, when: Prefetch) -> Self
}
pub fn on_link_hover(f: impl Fn(&str) + 'static)       // feature `client`

// Lists and elements — see animations
pub fn transition_group(name: &'static str, list: Node) -> Node
impl Element {
    pub fn use_action<C: FnOnce() + 'static>(self, action: impl Fn(&Dom) -> C + 'static) -> Self
    pub fn bind_checked_field(self, field: forms::Field<bool>) -> Self
    pub fn bind_parsed_field<T: FromStr + Display + PartialEq>(self, field: forms::Field<T>) -> Self
}

// Forms — see forms
impl Form { pub fn field_with<T>(self, value: State<T>, check: impl Fn(&T) -> Option<String> + 'static) -> Field<T>; pub fn is_pending(self) -> bool }
impl<T> Field<T> { pub fn validate_async<F>(self, check: impl Fn(T) -> F + 'static) -> Self; pub fn is_pending(self) -> bool }

// Stores and debugging — see stores
pub fn persisted<T: Data>(key: impl Into<String>, initial: T) -> State<T>
pub fn persisted_in<T: Data>(storage: Storage, key: impl Into<String>, initial: T) -> State<T>  // Storage::{Local, Session}
// active::debug
pub fn stats() -> Stats                  // states, effects, effect_runs
pub fn effect(name: &'static str, f: impl Fn() + 'static)
pub fn runs(name: &str) -> u64
pub fn log(on: bool)
pub fn log_stats(label: &str)

// active::i18n — see i18n
pub struct I18n;  // Copy: new, messages, locale, locale_state, set_locale, t, t_with, number, currency, percent, date, plural
pub fn format_message(locale: &str, message: &str, args: &[(&str, Arg)]) -> String
pub fn plural(locale: &str, n: f64) -> Plural
pub fn format_number(locale: &str, n: f64) -> String
pub fn format_fixed(locale: &str, n: f64, decimals: usize) -> String
pub fn format_currency(locale: &str, amount: f64, currency: &str) -> String
pub fn format_percent(locale: &str, ratio: f64, decimals: usize) -> String
pub fn format_date(locale: &str, date: Date, style: DateStyle) -> String
```

Una página enviada en streaming añade dos marcadores: `<active-suspense data-suspense="n">` alrededor del fallback de una frontera enviada después, y `<script type="application/json" data-active-data="clave">` con el valor de un `ssr::resource`.

## Marcadores HTML

La salida del servidor contiene algunos marcadores de los que depende la hidratación. Forman parte del formato: no elimines los comentarios HTML de las páginas con islas.

| Marcador | Significado |
| --- | --- |
| `<!--#-->` | inicio de un texto dinámico |
| `<!---->` | separador entre dos textos adyacentes |
| `<!--[-->` … `<!--]-->` | una región dinámica |
| `<active-island data-island data-props>` | una isla; recibe `data-hydrated` una vez hidratada |
| `data-hydrate="…"` en una isla | su estrategia de hidratación (`island_with`) |

## Pánicos

- Usar un `State` después de liberarlo provoca un pánico con "this state was disposed". En el navegador ocurre después de que se reconstruya su región dinámica o se libere su scope; en el servidor, después de dos renderizados de nivel superior que no lo usaron.
- Leer un estado dentro de su propio closure de `update` provoca un pánico con "a state was read inside its own `update`; read it before updating". Llamar a `set` o `update` sobre un estado dentro de su propio `with` provoca un pánico con "a state was updated while being read or updated".
