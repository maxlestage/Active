---
title: API reference
description: Every public item of the active crate with its signature: views, elements, state, effects, events, Seo, Document, PWA, islands, browser functions and feature flags.
---

This page lists every public item of the `active` crate, grouped by topic, with its exact signature and a one-line description. The guides explain how to use them; the generated documentation on [docs.rs/active](https://docs.rs/active) has the same items with examples.

## Feature flags

| Feature | Default | Adds |
| --- | --- | --- |
| `client` | off | The browser runtime: [`hydrate`](#hydrate), [`mount`](#mount), [`mount_to`](#mount-to), [`location`](#location), [`query`](#query), [`query_param`](#query-param), [`navigate`](#navigate), [`navigate_replace`](#navigate-replace), [`update_head`](#update-head), `Event::raw`, real event listeners, effects, `on_mount` callbacks, transitions, portals, futures run by resources and the island hydration strategies. Depends on `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys` and `web-sys`. |

Without `client`, the crate has no dependencies. Event handlers, effects and `on_mount` callbacks are accepted and ignored, so the same components compile on the server. Enable `client` only for the `wasm32` build (see [Installation](../installation/#one-crate-two-targets)).

The crate uses the 2024 edition and needs Rust 1.85 or newer. It contains no `unsafe` code.

## Crate layout

| Path | Contents |
| --- | --- |
| `active::*` | every item below |
| `active::html` | one function per HTML and SVG tag |
| `active::forms` | [`Form`](#form), [`Field`](#field), [`Rule`](#rule) and the rules: validated forms |
| `active::pwa` | [`Manifest`](#manifest), [`ServiceWorker`](#serviceworker), [`Icon`](#icon) and [`register_script`](#register-script) and [`version`](#version): installable, offline sites |
| `active::prelude` | `html::*`, `Document`, `Element`, `Event`, `Node`, `Seo`, `State`, `effect`, `el`, `fragment`, `fragment_dyn`, `indexed`, `island`, `keyed`, `memo`, `on_cleanup`, `switch`, `text`, `text_dyn`, `untrack`, `use_state`, `watch`, `watch_immediate`, `provide`, `use_context`, `expect_context`, `match_path`; with `client`, also `location`, `navigate` and `query_param` |

Not in the prelude: `Str`, `IslandFn`, `Dom`, `Scope`, `scope`, `Watcher`, `Params`, `Routes`, `hydrate`, `mount`, `mount_to`, `query`, `update_head`, and the items added for other frameworks' strengths: `batch`, `resource`, `Resource`, `suspense`, `error_boundary`, `ErrorBoundary`, `report_error`, `portal`, `island_with`, `Hydrate`, `before_navigate`, `Navigation`, `navigate_replace` and the `forms` module. Use them as `active::hydrate`, etc.

## Views

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

One piece of a view. You rarely build variants by hand: tag functions, `text`, `fragment`, `island` and the `From` conversions do it.

| Variant | Meaning |
| --- | --- |
| `Empty` | nothing (the `Default`) |
| `Element` | an HTML element |
| `Text` | escaped text |
| `DynText` | text computed from states ([`text_dyn`](#free-functions)) |
| `Fragment` | several nodes side by side |
| `Seo` | SEO tags, written in the `<head>` by `Document` |
| `Island` | an island ([`island`](#island)) |
| `Dynamic` | a region rebuilt when its states change ([`fragment_dyn`](#free-functions)) |
| `Keyed` | a list whose rows are kept by key ([`keyed`](#free-functions)) |

| Method | Description |
| --- | --- |
| `pub fn render(&self) -> String` | Renders the view to HTML. Can be called any number of times. |
| `pub fn render_into(&self, out: &mut String)` | Same, appending to `out`. |

Implements `Clone`, `Default` and `Debug` (which prints the rendered HTML).

Conversions into `Node` (`From`): `Element`, `&'static str`, `String`, `Str`, `Seo`, `Vec<T>`, `Option<T>` and `[T; N]` for any `T: Into<Node>`.

### Element

An HTML element under construction, created by a [tag function](#tag-functions) or [`el`](#free-functions). Every method takes `self` and returns the updated `Element` (`#[must_use]`), so calls chain. Implements `Clone` and `Into<Node>`.

**Attributes and classes**

| Method | Description |
| --- | --- |
| `pub fn attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Sets an attribute; setting the same name again replaces the value. `attr("class", …)` adds classes like `class`. |
| `pub fn attr_dyn(self, name: impl Into<Str>, f: impl Fn() -> String + 'static) -> Self` | Attribute recomputed from states (replaces a previous one with the same name). For `value`, also sets the field's current value in the browser. |
| `pub fn bool_attr(self, name: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Boolean attribute, present while `f()` is true (replaces a previous one with the same name). For `checked`, also sets the checkbox state in the browser. |
| `pub fn id(self, id: impl Into<Str>) -> Self` | `id="…"` (replaces a previous id). |
| `pub fn class(self, class: impl Into<Str>) -> Self` | Adds classes (space separated); empty strings are ignored. |
| `pub fn class_if(self, class: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Adds `class` while `f()` is true. |
| `pub fn style(self, style: impl Into<Str>) -> Self` | `style="…"` (replaces a previous style). |
| `pub fn style_dyn(self, property: &'static str, f: impl Fn() -> String + 'static) -> Self` | One CSS property recomputed from states, added to the `style` set before it; an empty value leaves it out. A `style` set after replaces it. |
| `pub fn href(self, href: impl Into<Str>) -> Self` | `href="…"` (replaces a previous href). |

**Content**

| Method | Description |
| --- | --- |
| `pub fn child(self, child: impl Into<Node>) -> Self` | Appends a child. After `html`/`html_dyn`, starts a new list of children. |
| `pub fn children(self, children: impl IntoIterator<Item = impl Into<Node>>) -> Self` | Appends several children. |
| `pub fn text(self, value: impl Into<Str>) -> Self` | Appends escaped text. |
| `pub fn text_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Appends text recomputed from states. |
| `pub fn children_dyn(self, f: impl Fn() -> Vec<Node> + 'static) -> Self` | Appends a dynamic region, rebuilt when the states `f` read change. Same as `.child(fragment_dyn(f))`. |
| `pub fn children_indexed<T>(self, items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Self` (`T: Clone + PartialEq`) | Appends a list kept by position. Same as `.child(indexed(items, row))`. |
| `pub fn children_keyed<T, K>(self, items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Self` (`T: PartialEq`, `K: Hash`) | Appends a list kept by key. Same as `.child(keyed(items, key, row))`. |
| `pub fn html(self, html: impl Into<Str>) -> Self` | Replaces the content with trusted, unescaped HTML. |
| `pub fn html_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Same, recomputed from states. |

**Events and DOM access** (browser only; ignored without `client`)

| Method | Description |
| --- | --- |
| `pub fn on(self, event: &'static str, handler: impl Fn(Event) + 'static) -> Self` | Listens to a DOM event. |
| `pub fn on_click(self, handler: impl Fn(Event) + 'static) -> Self` | `on("click", …)`. |
| `pub fn on_input(self, handler: impl Fn(Event) + 'static) -> Self` | `on("input", …)`. |
| `pub fn on_submit(self, handler: impl Fn(Event) + 'static) -> Self` | `on("submit", …)` that first calls `prevent_default`. |
| `pub fn on_key(self, key: &'static str, handler: impl Fn(Event) + 'static) -> Self` | `keydown` listener called only for `key` (`"Enter"`, `"Escape"`…). |
| `pub fn on_mount(self, f: impl Fn(&Dom) + 'static) -> Self` | Runs `f` with the DOM element once it is in the page. |
| `pub fn transition(self, name: &'static str, when: impl Fn() -> bool + 'static) -> Self` | Shown while `when` is true, hidden (`hidden`) otherwise, with `{name}-enter-*` / `{name}-leave-*` classes during each change; the server renders `hidden` when false. See [Components](../components/#transitions). |
| `pub fn transition_in(self, name: &'static str) -> Self` | Plays `{name}-enter-*` when the element is added to the page (not when hydrated). |

**Two-way binding** (the value is rendered on the server; the listener only exists with `client`)

| Method | Description |
| --- | --- |
| `pub fn bind_value(self, state: State<String>) -> Self` | Field value ⇄ state: `<input>` and `<textarea>` written on `input`, `<select>` on `change`. A `<textarea>` renders the state as its text. |
| `pub fn bind_checked(self, state: State<bool>) -> Self` | Checkbox ⇄ state, written on `change`. |
| `pub fn bind_parsed<T>(self, state: State<T>) -> Self` (`T: FromStr + Display + PartialEq`) | Field ⇄ state of a parsed type (numbers): written when the text parses, the text typed is kept otherwise. |
| `pub fn bind_field(self, field: forms::Field) -> Self` | `bind_value` on the field's value, touched on `blur`, `aria-invalid="true"` while its error is shown. See [Forms](../forms/). |

### Free functions

| Signature | Description |
| --- | --- |
| `pub fn el(tag: &'static str) -> Element` | An element with any tag name, e.g. `el("my-widget")`. |
| `pub fn text(value: impl Into<Str>) -> Node` | A text node. |
| `pub fn text_dyn(f: impl Fn() -> String + 'static) -> Node` | Text recomputed from states. |
| `pub fn fragment(nodes: impl IntoIterator<Item = impl Into<Node>>) -> Node` | Several nodes without a wrapper element. |
| `pub fn fragment_dyn(f: impl Fn() -> Vec<Node> + 'static) -> Node` | A dynamic region: rebuilt as a whole in the browser when the states `f` read change; the states, effects and listeners created for the previous nodes are released, and the focus goes back to the matching element of the new nodes. Rendered between `<!--[-->` and `<!--]-->` on the server. |
| `pub fn keyed<T, K>(items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Node` (`T: PartialEq`, `K: Hash`) | A list whose rows are kept by key: in the browser, rows whose key is still there and whose item is equal keep their DOM, states, listeners and focus (and move if the order changed); new or changed rows are built, removed ones released. Rendered like `fragment_dyn` on the server. See [State](../state/#long-lists-keyed-rows). |
| `pub fn indexed<T>(items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Node` (`T: Clone + PartialEq`) | A list whose rows are kept by position: row `i` is built once with a `State` of the `i`-th item and updated in place; rows are added or removed at the end. See [State](../state/#rows-by-position-indexed). |
| `pub fn switch<V, K>(value: impl Fn() -> V + 'static, kind: impl Fn(&V) -> K + 'static, view: impl Fn(State<V>) -> Node + 'static) -> Node` (`V: Clone + PartialEq`, `K: PartialEq`) | Shows `view` for `value()`, rebuilt only when `kind` changes; otherwise the view's `State<V>` receives the new value. See [Client-side apps](../client-app/#keeping-a-page-when-its-parameters-change). |
| `pub fn portal(target: &'static str, view: impl Into<Node>) -> Node` | Renders `view` at the end of the element matching `target` (browser only; nothing on the server), removed with the portal. See [Components](../components/#portals). |

### Str

```rust
pub type Str = std::borrow::Cow<'static, str>;
```

The string type of attribute names and values, text and URLs: a `&'static str` or a `String`.

### Tag functions

`active::html` has one function per tag, each `pub fn name() -> Element`:

`a` `abbr` `address` `area` `article` `aside` `audio` `b` `bdi` `blockquote` `br` `button` `canvas` `caption` `cite` `code` `col` `colgroup` `data` `datalist` `dd` `del` `details` `dfn` `dialog` `div` `dl` `dt` `em` `embed` `fieldset` `figcaption` `figure` `footer` `form` `h1` `h2` `h3` `h4` `h5` `h6` `header` `hgroup` `hr` `i` `iframe` `img` `input` `ins` `kbd` `label` `legend` `li` `main` `map` `mark` `menu` `meter` `nav` `noscript` `object` `ol` `optgroup` `option` `output` `p` `picture` `pre` `progress` `q` `s` `samp` `search` `section` `select` `small` `source` `span` `strong` `sub` `summary` `sup` `table` `tbody` `td` `template` `textarea` `tfoot` `th` `thead` `time` `tr` `track` `u` `ul` `var` `video` `wbr`

SVG: `svg` `circle` `ellipse` `g` `line` `path` `polygon` `polyline` `rect` `tspan` `defs` `stop`, plus `text_svg` (`<text>`), `use_svg` (`<use>`) and `linear_gradient` (`<linearGradient>`).

Void elements (`area`, `br`, `col`, `embed`, `hr`, `img`, `input`, `source`, `track`, `wbr`, and `base`, `link`, `meta` through `el`) are written as `<tag …/>` and ignore children. There is no `html`, `head` or `body` function: [`Document`](#document) writes them.

## State

### use_state

```rust
pub fn use_state<T: 'static>(initial: T) -> State<T>
```

Creates a reactive value owned by the component being built. In the browser it is released with its dynamic region; on the server, at the end of the second top-level render that neither reads nor creates it.

### State

```rust
pub struct State<T> { /* index into the state store */ }
```

A handle to a reactive value. Implements `Copy`, `Clone` and `Debug` for every `T`. It is neither `Send` nor `Sync`: the value lives in the store of the thread that created it, so a handle cannot move to another thread or be kept across an `.await` in a multi-threaded (`Send`) future.

| Method | Description |
| --- | --- |
| `pub fn get(self) -> T where T: Clone` | A clone of the value. Inside an effect or binding, subscribes it. |
| `pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R` | Reads the value by reference. Subscribes like `get`. |
| `pub fn set(self, value: T)` | Replaces the value and re-runs what read it. |
| `pub fn update(self, f: impl FnOnce(&mut T))` | Changes the value in place and re-runs what read it. |
| `pub fn is_alive(self) -> bool` | `false` once the state was released (its region re-rendered). Check it before writing from async code. |

### effect

```rust
pub fn effect(f: impl Fn() + 'static)
```

Runs `f` now, then again whenever a state it read during its last run changes, once the memos it reads are up to date. Browser only: without `client`, does nothing. A state written inside an effect notifies its readers once the effect has finished; an effect that writes a state it reads does not re-trigger itself.

### on_cleanup

```rust
pub fn on_cleanup(f: impl FnOnce() + 'static)
```

Runs `f` when the component being built is released: its dynamic region re-renders, the router shows another page, or its `Scope` is disposed. Cleanups run last-registered first, before the scope's states are released. On the server, or outside any scope, `f` never runs.

### memo

```rust
pub fn memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> State<T>
```

A state holding `f()`, recomputed when the states `f` reads change — before the effects that read it run, and only if an input really changed — and written only when the result differs. On the server it keeps the value computed at creation.

### untrack

```rust
pub fn untrack<R>(f: impl FnOnce() -> R) -> R
```

Runs `f` without subscribing the running effect or dynamic region to the states `f` reads.

### scope and Scope

```rust
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Scope)
pub struct Scope(/* … */);
impl Scope { pub fn dispose(self) }
```

`scope` runs `f` and returns its result with a `Scope` owning the states, effects and event listeners created inside. `dispose` releases them. `Scope` is `#[must_use]`. On the server, a render inside an open scope does not empty the state store.

### watch and watch_immediate

```rust
pub fn watch<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, &T) + 'static) -> Watcher
pub fn watch_immediate<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, Option<&T>) + 'static) -> Watcher
```

Calls `callback(&new, &old)` each time the value returned by `source` changes (`PartialEq`). `source` is followed like an effect, `callback` is not. `watch` skips the first value; `watch_immediate` calls `callback(&first, None)` right away. Browser only, like `effect`; released with the component.

### Watcher

```rust
#[derive(Clone, Copy, Debug)]
pub struct Watcher { /* … */ }
impl Watcher { pub fn stop(self) }
```

Stops a watcher before its component is released. Stopping twice does nothing.

### batch

```rust
pub fn batch<R>(f: impl FnOnce() -> R) -> R
```

Runs `f`, holding back the effects its writes trigger until it returns: each then runs once. Nested batches wait for the outermost one. See [State](../state/#grouping-writes-batch).

### provide, use_context and expect_context

```rust
pub fn provide<T: 'static, R>(value: T, f: impl FnOnce() -> R) -> R
pub fn use_context<T: Clone + 'static>() -> Option<T>
pub fn expect_context<T: Clone + 'static>() -> T
```

`provide` gives `value` to everything built by `f`, dynamic regions and keyed lists included (they keep it when they rebuild). `use_context` returns the closest provided value of type `T`, cloned; `expect_context` panics, naming the type, when there is none. Call them while building the view. See [State](../state/).

## Async data and errors

See [Async data and errors](../async-data/).

### resource and Resource

```rust
pub fn resource<S, T, F>(source: impl Fn() -> S + 'static, fetch: impl Fn(S) -> F + 'static) -> Resource<T>
where S: Clone + PartialEq + 'static, T: 'static, F: Future<Output = T> + 'static

pub struct Resource<T> { /* … */ } // Copy
```

Loads `fetch(source())` and loads again when `source` changes; a result older than the latest request is ignored. In the browser the future runs on the event loop; on the server it is polled once (a ready value is rendered, otherwise it stays loading).

| Method | Description |
| --- | --- |
| `pub fn get(self) -> Option<T>` (`T: Clone`) | The latest value, `None` before the first one. |
| `pub fn with<R>(self, f: impl FnOnce(Option<&T>) -> R) -> R` | Reads it through a reference. |
| `pub fn loading(self) -> bool` | Whether a load is in progress. |
| `pub fn refetch(self)` | Loads again with the current source. |
| `pub fn set(self, value: T)` | Replaces the value now (optimistic update). |
| `pub fn state(self) -> State<Option<T>>` | The state holding the value. |

### suspense

```rust
pub fn suspense(fallback: impl Into<Node>, view: impl FnOnce() -> Node) -> Node
```

Builds `view` once and shows `fallback` in its place while a resource created inside has not loaded its first value. Later reloads keep the view.

### error_boundary, ErrorBoundary and report_error

```rust
pub fn error_boundary(view: impl Fn() -> Node + 'static, fallback: impl Fn(ErrorBoundary) -> Node + 'static) -> Node
pub fn report_error(error: impl Display) -> bool

#[derive(Clone, Copy, Debug)]
pub struct ErrorBoundary { /* … */ }
impl<T: Into<Node>, E: Display> From<Result<T, E>> for Node
```

Shows `fallback` instead of `view` once an error is reported inside: an `Err` converted into a `Node`, `report_error` while building (returns `false` and logs when there is no boundary), or `ErrorBoundary::report` later. `ErrorBoundary` has `report(error)`, `errors() -> Vec<String>`, `has_errors() -> bool` and `reset()` (forgets the errors and builds the view again); the view gets it with `use_context::<ErrorBoundary>()`.

## Forms

The `active::forms` module. See [Forms and validation](../forms/).

### Form

```rust
#[derive(Clone, Copy, Debug)]
pub struct Form { /* … */ }
```

| Method | Description |
| --- | --- |
| `pub fn new() -> Self` | A form without fields (also `Default`). |
| `pub fn field(self, value: State<String>, rules: impl IntoIterator<Item = Rule>) -> Field` | Adds a field checked by `rules`, in order. |
| `pub fn is_valid(self) -> bool` | Whether every field is valid. |
| `pub fn submit(self) -> bool` | Shows every error from now on; returns whether the form is valid. |
| `pub fn submitted(self) -> bool` | Whether `submit` was called since the last reset. |
| `pub fn reset(self)` | Hides the errors again (values are left alone). |

### Field

```rust
#[derive(Clone, Copy, Debug)]
pub struct Field { /* … */ }
```

| Method | Description |
| --- | --- |
| `pub fn value(self) -> State<String>` | The field's state. |
| `pub fn error(self) -> Option<String>` | The current error. |
| `pub fn is_valid(self) -> bool` | Whether there is no error. |
| `pub fn message(self) -> Option<String>` | The error to show: once touched or submitted. |
| `pub fn touched(self) -> bool` / `pub fn touch(self)` | Whether the visitor has left the field; mark it as left. |

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

`check` returns the message when `value` is not valid. All rules but `required` accept an empty value.

## Events

### Event

A DOM event passed to handlers. Implements `Debug` and `Clone`.

| Method | Description |
| --- | --- |
| `pub fn value(&self) -> String` | Current value of the `<input>`, `<textarea>` or `<select>` that fired the event; empty otherwise. |
| `pub fn key(&self) -> String` | The key of a keyboard event (`"Enter"`, `"k"`…); empty otherwise. |
| `pub fn prevent_default(&self)` | Cancels the browser's default action. |
| `pub fn current_target(&self) -> Option<Dom>` | The element the listener is attached to. `None` on the server. |
| `pub fn raw(&self) -> &web_sys::Event` | The underlying event. Only with `client`. |
| `pub fn stop_propagation(&self)` | Stops the event from reaching the parent elements. |
| `pub fn checked(&self) -> bool` | Whether the checkbox or radio button that fired the event is checked; `false` otherwise. |

### Dom

```rust
pub type Dom = web_sys::Element; // with `client`
pub struct Dom;                  // without: an empty placeholder (Debug, Clone, Copy, Default)
```

The element type given to `on_mount` and returned by `Event::current_target`.

## SEO

### Seo

```rust
pub struct Seo { /* … */ }
```

The `<head>` tags of a page. Implements `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`, `Display` (same output as `to_html`) and `Into<Node>`. `#[must_use]`: setters return the updated value. Rules and examples: [SEO](../seo/).

| Method | Description |
| --- | --- |
| `pub fn new() -> Self` | Empty; renders only `og:type` (`website`) and `twitter:card` (`summary`). |
| `pub fn title(self, title: impl Into<String>) -> Self` | `<title>` and `og:title`. |
| `pub fn title_template(self, template: impl Into<String>) -> Self` | Template for `<title>`; the first `%s` is replaced by the title. |
| `pub fn description(self, description: impl Into<String>) -> Self` | Meta description and `og:description`. |
| `pub fn canonical(self, url: impl Into<String>) -> Self` | `<link rel="canonical">` and `og:url`. |
| `pub fn image(self, url: impl Into<String>) -> Self` | `og:image`; switches `twitter:card` to `summary_large_image`. |
| `pub fn og_type(self, og_type: impl Into<String>) -> Self` | `og:type`. |
| `pub fn site_name(self, site_name: impl Into<String>) -> Self` | `og:site_name`, and the title when none is set. |
| `pub fn locale(self, locale: impl Into<String>) -> Self` | `og:locale`. |
| `pub fn twitter(self, handle: impl Into<String>) -> Self` | `twitter:site`. |
| `pub fn base_url(self, url: impl Into<String>) -> Self` | Origin for relative canonical, image and alternate URLs. |
| `pub fn noindex(self, noindex: bool) -> Self` | `noindex` in `<meta name="robots">`. |
| `pub fn nofollow(self, nofollow: bool) -> Self` | `nofollow` in `<meta name="robots">`. |
| `pub fn alternate(self, lang: impl Into<String>, url: impl Into<String>) -> Self` | `<link rel="alternate" hreflang>`; same language again replaces the URL. |
| `pub fn json_ld(self, json: impl Into<String>) -> Self` | A JSON-LD `<script>`, one per call; `<` is escaped as `\u003c`. |
| `pub fn meta_name(self, name: impl Into<String>, content: impl Into<String>) -> Self` | Extra `<meta name content>`. |
| `pub fn meta_property(self, property: impl Into<String>, content: impl Into<String>) -> Self` | Extra `<meta property content>`. |
| `pub fn merge(&mut self, other: &Seo)` | Applies `other` on top: its set values win, alternates and extra meta are replaced by key, JSON-LD is appended. |
| `pub fn merged(&self, other: &Seo) -> Seo` | `merge` into a new value. |
| `pub fn to_html(&self) -> String` | Renders the tags. |
| `pub fn write_html(&self, out: &mut String)` | Appends the tags to `out`. |

## Documents

### Document

```rust
pub struct Document { /* … */ }
```

A complete HTML page. Implements `Clone` and `Default`. Builder methods take `self` and are `#[must_use]`. Guide: [Documents](../documents/).

| Method | Description |
| --- | --- |
| `pub fn new(body: impl Into<Node>) -> Self` | A page whose `<body>` is `body`. |
| `pub fn lang(self, lang: impl Into<Str>) -> Self` | `<html lang>`. |
| `pub fn seo(self, seo: Seo) -> Self` | Base SEO, merged with every `Seo` in the body; replaces a previous base. |
| `pub fn stylesheet(self, href: impl Into<Str>) -> Self` | `<link rel="stylesheet">`, in call order. |
| `pub fn head(self, html: impl Into<Str>) -> Self` | Trusted HTML appended to `<head>`. |
| `pub fn client(self, js: impl Into<Str>, wasm: impl Into<Str>) -> Self` | Preloads and starts the `wasm-bindgen --target web --no-typescript` bundle. |
| `pub fn manifest(self, href: &str, manifest: &pwa::Manifest) -> Self` | `<link rel="manifest">` to `href`, `theme-color`, the home screen title and the Apple touch icon (the largest PNG icon of purpose `any`). |
| `pub fn service_worker(self, url: &str) -> Self` | An inline script that registers the service worker served at `url` once the page has loaded. |
| `pub fn viewport(self, content: impl Into<Str>) -> Self` | `content` of `<meta name="viewport">` (default `width=device-width, initial-scale=1`). |
| `pub fn html_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Attribute on `<html>`. |
| `pub fn body_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Attribute on `<body>`. |
| `pub fn final_seo(&self) -> Seo` | The merged SEO of the page, including islands and the current content of dynamic regions (evaluated separately from `render`). |
| `pub fn render(&self) -> String` | The whole page. Renders the body first and collects its `Seo` values on the way, so each dynamic region is evaluated once. Can be called any number of times. |

## PWA

The `active::pwa` module. Guide: [Installable apps (PWA)](../pwa/).

### Manifest

```rust
pub struct Manifest { /* … */ }
```

The web app manifest. Implements `Clone`, `Debug`, `PartialEq` and `Eq`. Builder methods take `self` and are `#[must_use]`.

| Method | Description |
| --- | --- |
| `pub fn new(name: impl Into<String>) -> Self` | A manifest named `name`, `start_url` `./`, `display` `standalone`, theme `#000000`, background `#ffffff`. |
| `pub fn short_name(self, name: impl Into<String>) -> Self` | The name under the icon. |
| `pub fn description(self, description: impl Into<String>) -> Self` | What the app does. |
| `pub fn id(self, id: impl Into<String>) -> Self` | The identity of the app (defaults to `start_url`). |
| `pub fn start_url(self, url: impl Into<String>) -> Self` | The page opened by the icon. |
| `pub fn scope(self, scope: impl Into<String>) -> Self` | The URLs that belong to the app (defaults to the manifest's folder). |
| `pub fn display(self, display: impl Into<String>) -> Self` | `standalone`, `fullscreen`, `minimal-ui` or `browser`. |
| `pub fn orientation(self, orientation: impl Into<String>) -> Self` | `portrait`, `landscape`, `any`… |
| `pub fn theme_color(self, color: impl Into<String>) -> Self` | Status bar and window title color. |
| `pub fn background_color(self, color: impl Into<String>) -> Self` | Splash screen color. |
| `pub fn lang(self, lang: impl Into<String>) -> Self` | Language of the texts. |
| `pub fn categories<I, S>(self, categories: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | Store categories; replaces previous ones. |
| `pub fn icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | An icon of purpose `any`. |
| `pub fn maskable_icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | An icon of purpose `maskable` (content inside the central 80%). |
| `pub fn name(&self) -> &str` | The name. |
| `pub fn theme(&self) -> &str` | The theme color. |
| `pub fn icons(&self) -> &[Icon]` | The icons. |
| `pub fn to_json(&self) -> String` | The manifest as JSON (strings escaped, `<` as `\u003c`). |

### Icon

```rust
pub struct Icon { pub src: String, pub sizes: String, pub mime: String, pub purpose: String }
```

An icon of the manifest: URL, `"192x192"` or `"any"`, MIME type, and `"any"`, `"maskable"` or `"monochrome"`.

### ServiceWorker

```rust
pub struct ServiceWorker { /* … */ }
```

The offline service worker. Implements `Clone`, `Debug`, `PartialEq` and `Eq`. Builder methods take `self` and are `#[must_use]`.

| Method | Description |
| --- | --- |
| `pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self` | A worker whose cache is `{name}-{version}`; a new version replaces the caches of older ones. |
| `pub fn precache<I, S>(self, urls: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | URLs saved at installation; adds to previous ones. |
| `pub fn offline_page(self, url: impl Into<String>) -> Self` | The page shown offline for a page never saved; also precached. |
| `pub fn network_only(self, prefix: impl Into<String>) -> Self` | Paths starting with `prefix` always go to the network. |
| `pub fn cache_name(&self) -> String` | `{name}-{version}`. |
| `pub fn to_js(&self) -> String` | The script: pages from the network first (saved copy, then offline page, when offline), other same-origin `GET` files from the cache, refreshed in the background. |

### register_script

```rust
pub fn register_script(url: &str) -> String
```

The inline script written by `Document::service_worker`: registers the worker at `url` on `load`. Hash it to allow it in a Content-Security-Policy.

### version

```rust
pub fn version<I, B>(parts: I) -> String
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
```

A short hash of `parts` (10 hexadecimal characters, FNV-1a), for `ServiceWorker::new`: hash what the worker saves, and every deployment that changes it installs a new worker.

## Routing

Available on the server and in the browser. See [Client-side apps](../client-app/).

### match_path

```rust
pub fn match_path(pattern: &str, path: &str) -> Option<Params>
```

Matches `path` (its query string and fragment ignored, as well as extra slashes) against `pattern`: `posts` matches that segment, `:id` one segment, `:id?` an optional segment, `*rest` (last) everything left (`*` alone captures nothing). Values are percent-decoded.

### Params

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Params { /* … */ }
```

| Method | Description |
| --- | --- |
| `pub fn get(&self, name: &str) -> Option<&str>` | The value of a parameter. |
| `pub fn parse<T: FromStr>(&self, name: &str) -> Option<T>` | The value parsed; `None` if missing or invalid. |
| `pub fn iter(&self) -> impl Iterator<Item = (&str, &str)>` | The parameters, in pattern order. |
| `pub fn is_empty(&self) -> bool` | Whether there is none. |

### Routes

```rust
#[derive(Clone, Default)]
pub struct Routes { /* … */ }
```

| Method | Description |
| --- | --- |
| `pub fn new() -> Self` | An empty table. |
| `pub fn route(self, pattern: impl Into<String>, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | Adds a route; the first matching pattern wins. |
| `pub fn fallback(self, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | The page shown when nothing matches; its `path` parameter holds the whole path. |
| `pub fn resolve(&self, path: &str) -> Option<Params>` | The parameters of the route that matches (or of the fallback). |
| `pub fn matches(&self, path: &str) -> bool` | Whether a route other than the fallback matches. |
| `pub fn render(&self, path: &str) -> Node` | The page of one path. |
| `pub fn view(self, path: impl Fn() -> String + 'static) -> Node` | The page of a changing path: kept while the same route matches (its `State<Params>` is updated), rebuilt when another does. |

### before_navigate and Navigation

```rust
pub fn before_navigate(guard: impl Fn(&str) -> Navigation + 'static)

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation { Continue, Cancel, Redirect(String) }
```

Registers a guard asked before the browser router goes to an address (links, `navigate`, `navigate_replace`, back and forward). Guards run in registration order; a redirect is checked again (ten in a row cancel). Removed with the component that registered it. Never called on the server. See [Client-side apps](../client-app/#navigation-guards).

## Islands

### island

```rust
pub fn island(name: &'static str, props: impl Into<Str>, component: IslandFn) -> Node
```

Calls `component(&props)` and wraps its view in `<active-island data-island="name" data-props="props">`, to be hydrated in the browser by the component registered under `name`.

### island_with and Hydrate

```rust
pub fn island_with(name: &'static str, props: impl Into<Str>, component: IslandFn, when: Hydrate) -> Node

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hydrate { #[default] Load, Idle, Visible, Media(Str), Interaction }
```

Like `island`, hydrated when `when` says: at once, when the browser is idle, when visible, while a media query matches, or at the first interaction. Adds `data-hydrate="idle|visible|media:…|interaction"` (nothing for `Load`). See [Islands](../islands/#when-islands-hydrate).

### IslandFn

```rust
pub type IslandFn = fn(&str) -> Node;
```

An island component: builds its view from a props string, identically on the server and in the browser.

## Browser functions

Available with the `client` feature, at the crate root.

### hydrate

```rust
pub fn hydrate(islands: &[(&str, IslandFn)])
```

Hydrates every `<active-island>` of the page whose name is in `islands`: runs the component with its props and binds listeners, dynamic text, attributes and classes to the existing HTML. Already hydrated islands and unknown names are skipped. On a structure mismatch, prints a console warning and renders the island from scratch. Never changes the `<head>`.

### mount

```rust
pub fn mount(view: &Node, parent: &web_sys::Element)
```

Creates the DOM for `view` after the existing children of `parent`. Merges the view's `Seo` values and writes them to the head with `update_head`; dynamic regions inside the view update the head when they are rebuilt.

### mount_to

```rust
pub fn mount_to(selector: &str, view: impl Into<Node>)
```

Replaces the content of the first element matching the CSS `selector` with `view` (using `mount`). Prints a console warning if nothing matches. The usual entry point of a [client-side app](../client-app/).

### location

```rust
pub fn location() -> State<String>
```

The current path (`window.location.pathname`, e.g. `"/search"`), as a state. The first call to `location`, `query` or `query_param` starts the router: back and forward update it, and clicks on same-origin `<a href>` links call `navigate` instead of loading a page (except links with a `target` other than `_self`, a `download` attribute, a modifier key or a non-primary button, and anchors on the current page).

### query

```rust
pub fn query() -> State<String>
```

The current query string without the `?` (`"q=rust&page=2"`, empty when there is none), as a state updated by the router like `location`.

### query_param

```rust
pub fn query_param(name: &str) -> Option<String>
```

One parameter of the query string, decoded (`query_param("q")` on `/search?q=caf%C3%A9` gives `Some("café")`), or `None`. Reading it in a view, a dynamic region or an effect subscribes to `query`.

### navigate

```rust
pub fn navigate(to: &str)
```

Adds a history entry for `to` (`"/about"`, `"/search?q=rust"`, `"/docs#install"`), updates `location()` and `query()` (only the ones that changed, so only the views that depend on them are rebuilt), then scrolls to the top, or to the element whose id is the `#fragment`. The guards registered with [`before_navigate`](#before-navigate-and-navigation) are asked first; the scroll position of the page left is kept for the back button.

### navigate_replace

```rust
pub fn navigate_replace(to: &str)
```

Like `navigate`, replacing the current history entry instead of adding one.

### update_head

```rust
pub fn update_head(seo: &Seo)
```

Writes `seo` into the page's `<head>`: sets `document.title` (to an empty title when `seo` has neither a title nor a site name) and replaces the description, robots, canonical, `hreflang` alternates, Open Graph, Twitter and JSON-LD tags, and any meta tag with the same name or property as one of the extra meta tags. Other head elements are left alone.

## HTML markers

The server output contains a few markers that hydration relies on. They are part of the format: do not strip HTML comments from pages with islands.

| Marker | Meaning |
| --- | --- |
| `<!--#-->` | start of a dynamic text |
| `<!---->` | separator between two adjacent texts |
| `<!--[-->` … `<!--]-->` | a dynamic region |
| `<active-island data-island data-props>` | an island; gets `data-hydrated` once hydrated |
| `data-hydrate="…"` on an island | its hydration strategy (`island_with`) |

## Panics

- Using a `State` after it was released panics with "this state was disposed". In the browser this happens after its dynamic region was rebuilt or its scope disposed; on the server, after two top-level renders that did not use it.
- Reading a state inside its own `update` closure panics with "a state was read inside its own `update`; read it before updating". Calling `set` or `update` on a state inside its own `with` panics with "a state was updated while being read or updated".
