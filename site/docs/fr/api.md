---
title: Référence de l'API
description: Chaque élément public du crate Active et sa signature : vues, éléments, état, effets, événements, Seo, Document, PWA, îlots, fonctions navigateur, fonctionnalités.
---

Cette page liste chaque élément public du crate `active`, regroupé par thème, avec sa signature exacte et une description d'une ligne. Les guides expliquent comment s'en servir ; la documentation générée sur [docs.rs/active](https://docs.rs/active) contient les mêmes éléments, avec des exemples.

## Fonctionnalités

| Fonctionnalité | Par défaut | Ajoute |
| --- | --- | --- |
| `client` | désactivée | Le runtime du navigateur : [`hydrate`](#hydrate), [`mount`](#mount), [`mount_to`](#mount-to), [`location`](#location), [`query`](#query), [`query_param`](#query-param), [`navigate`](#navigate), [`navigate_replace`](#navigate-replace), [`update_head`](#update-head), `Event::raw`, de vrais écouteurs d'événements, les effets, les callbacks `on_mount`, les transitions, les portails, les futures des ressources et les stratégies d'hydratation des îlots. Dépend de `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys` et `web-sys`. |

Sans `client`, le crate n'a aucune dépendance. Les gestionnaires d'événements, les effets et les callbacks `on_mount` sont acceptés puis ignorés, si bien que les mêmes composants compilent sur le serveur. N'activez `client` que pour la compilation `wasm32` (voir [Installation](../installation/#un-crate-deux-cibles)).

Le crate utilise l'édition 2024 et nécessite Rust 1.85 ou plus récent. Il ne contient aucun code `unsafe`.

## Organisation du crate

| Chemin | Contenu |
| --- | --- |
| `active::*` | tous les éléments ci-dessous |
| `active::html` | une fonction par balise HTML et SVG |
| `active::forms` | [`Form`](#form), [`Field`](#field), [`Rule`](#rule) et les règles : formulaires validés |
| `active::pwa` | [`Manifest`](#manifest), [`ServiceWorker`](#serviceworker), [`Icon`](#icon) et [`register_script`](#register-script) et [`version`](#version) : sites installables et hors ligne |
| `active::prelude` | `html::*`, `Document`, `Element`, `Event`, `Node`, `Seo`, `State`, `effect`, `el`, `fragment`, `fragment_dyn`, `indexed`, `island`, `keyed`, `memo`, `on_cleanup`, `switch`, `text`, `text_dyn`, `untrack`, `use_state`, `watch`, `watch_immediate`, `provide`, `use_context`, `expect_context`, `match_path` ; avec `client`, aussi `location`, `navigate` et `query_param` |

Absents du prélude : `Str`, `IslandFn`, `Dom`, `Scope`, `scope`, `Watcher`, `Params`, `Routes`, `hydrate`, `mount`, `mount_to`, `query`, `update_head`, et les éléments ajoutés d'après les points forts des autres frameworks : `batch`, `resource`, `Resource`, `suspense`, `error_boundary`, `ErrorBoundary`, `report_error`, `portal`, `island_with`, `Hydrate`, `before_navigate`, `Navigation`, `navigate_replace` et le module `forms`. Utilisez-les sous la forme `active::hydrate`, etc.

## Vues

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

Un morceau de vue. Vous construisez rarement les variantes à la main : les fonctions de balise, `text`, `fragment`, `island` et les conversions `From` s'en chargent.

| Variante | Signification |
| --- | --- |
| `Empty` | rien (la valeur `Default`) |
| `Element` | un élément HTML |
| `Text` | du texte échappé |
| `DynText` | du texte calculé à partir d'états ([`text_dyn`](#fonctions-libres)) |
| `Fragment` | plusieurs nœuds côte à côte |
| `Seo` | des balises SEO, écrites dans le `<head>` par `Document` |
| `Island` | un îlot ([`island`](#island)) |
| `Dynamic` | une région reconstruite quand ses états changent ([`fragment_dyn`](#fonctions-libres)) |
| `Keyed` | une liste dont les lignes sont gardées par clé ([`keyed`](#fonctions-libres)) |

| Méthode | Description |
| --- | --- |
| `pub fn render(&self) -> String` | Rend la vue en HTML. Peut être appelée autant de fois que nécessaire. |
| `pub fn render_into(&self, out: &mut String)` | Idem, en ajoutant le résultat à `out`. |

Implémente `Clone`, `Default` et `Debug` (qui affiche le HTML rendu).

Conversions en `Node` (`From`) : `Element`, `&'static str`, `String`, `Str`, `Seo`, `Vec<T>`, `Option<T>` et `[T; N]` pour tout `T: Into<Node>`.

### Element

Un élément HTML en cours de construction, créé par une [fonction de balise](#fonctions-de-balise) ou par [`el`](#fonctions-libres). Chaque méthode prend `self` et renvoie l'`Element` mis à jour (`#[must_use]`) : les appels s'enchaînent. Implémente `Clone` et `Into<Node>`.

**Attributs et classes**

| Méthode | Description |
| --- | --- |
| `pub fn attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Définit un attribut ; redéfinir le même nom remplace la valeur. `attr("class", …)` ajoute des classes comme `class`. |
| `pub fn attr_dyn(self, name: impl Into<Str>, f: impl Fn() -> String + 'static) -> Self` | Attribut recalculé à partir d'états (remplace un attribut précédent de même nom). Pour `value`, définit aussi la valeur courante du champ dans le navigateur. |
| `pub fn bool_attr(self, name: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Attribut booléen, présent tant que `f()` est vrai (remplace un attribut précédent de même nom). Pour `checked`, définit aussi l'état de la case à cocher dans le navigateur. |
| `pub fn id(self, id: impl Into<Str>) -> Self` | `id="…"` (remplace un id précédent). |
| `pub fn class(self, class: impl Into<Str>) -> Self` | Ajoute des classes (séparées par des espaces) ; les chaînes vides sont ignorées. |
| `pub fn class_if(self, class: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self` | Ajoute `class` tant que `f()` est vrai. |
| `pub fn style(self, style: impl Into<Str>) -> Self` | `style="…"` (remplace un style précédent). |
| `pub fn style_dyn(self, property: &'static str, f: impl Fn() -> String + 'static) -> Self` | Une propriété CSS recalculée à partir d'états, ajoutée au `style` posé avant ; une valeur vide l'omet. Un `style` posé après la remplace. |
| `pub fn href(self, href: impl Into<Str>) -> Self` | `href="…"` (remplace un href précédent). |

**Contenu**

| Méthode | Description |
| --- | --- |
| `pub fn child(self, child: impl Into<Node>) -> Self` | Ajoute un enfant. Après `html`/`html_dyn`, commence une nouvelle liste d'enfants. |
| `pub fn children(self, children: impl IntoIterator<Item = impl Into<Node>>) -> Self` | Ajoute plusieurs enfants. |
| `pub fn text(self, value: impl Into<Str>) -> Self` | Ajoute du texte échappé. |
| `pub fn text_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Ajoute du texte recalculé à partir d'états. |
| `pub fn children_dyn(self, f: impl Fn() -> Vec<Node> + 'static) -> Self` | Ajoute une région dynamique, reconstruite quand les états lus par `f` changent. Équivaut à `.child(fragment_dyn(f))`. |
| `pub fn children_indexed<T>(self, items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Self` (`T: Clone + PartialEq`) | Ajoute une liste gardée par position. Équivaut à `.child(indexed(items, row))`. |
| `pub fn children_keyed<T, K>(self, items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Self` (`T: PartialEq`, `K: Hash`) | Ajoute une liste gardée par clé. Équivaut à `.child(keyed(items, key, row))`. |
| `pub fn html(self, html: impl Into<Str>) -> Self` | Remplace le contenu par du HTML de confiance, non échappé. |
| `pub fn html_dyn(self, f: impl Fn() -> String + 'static) -> Self` | Idem, recalculé à partir d'états. |

**Événements et accès au DOM** (navigateur uniquement ; ignorés sans `client`)

| Méthode | Description |
| --- | --- |
| `pub fn on(self, event: &'static str, handler: impl Fn(Event) + 'static) -> Self` | Écoute un événement DOM. |
| `pub fn on_click(self, handler: impl Fn(Event) + 'static) -> Self` | `on("click", …)`. |
| `pub fn on_input(self, handler: impl Fn(Event) + 'static) -> Self` | `on("input", …)`. |
| `pub fn on_submit(self, handler: impl Fn(Event) + 'static) -> Self` | `on("submit", …)` qui appelle d'abord `prevent_default`. |
| `pub fn on_key(self, key: &'static str, handler: impl Fn(Event) + 'static) -> Self` | Écouteur `keydown` appelé seulement pour `key` (`"Enter"`, `"Escape"`…). |
| `pub fn on_mount(self, f: impl Fn(&Dom) + 'static) -> Self` | Exécute `f` avec l'élément du DOM une fois celui-ci présent dans la page. |
| `pub fn transition(self, name: &'static str, when: impl Fn() -> bool + 'static) -> Self` | Affiché tant que `when` est vrai, caché (`hidden`) sinon, avec les classes `{name}-enter-*` / `{name}-leave-*` pendant chaque changement ; le serveur rend `hidden` quand c'est faux. Voir [Composants](../components/#transitions). |
| `pub fn transition_in(self, name: &'static str) -> Self` | Joue `{name}-enter-*` quand l'élément est ajouté à la page (pas à l'hydratation). |

**Liaison bidirectionnelle** (la valeur est rendue côté serveur ; l'écouteur n'existe qu'avec `client`)

| Méthode | Description |
| --- | --- |
| `pub fn bind_value(self, state: State<String>) -> Self` | Valeur du champ ⇄ état : `<input>` et `<textarea>` écrits sur `input`, `<select>` sur `change`. Un `<textarea>` rend l'état comme texte. |
| `pub fn bind_checked(self, state: State<bool>) -> Self` | Case à cocher ⇄ état, écrit sur `change`. |
| `pub fn bind_parsed<T>(self, state: State<T>) -> Self` (`T: FromStr + Display + PartialEq`) | Champ ⇄ état d'un type converti (nombres) : écrit quand le texte se convertit, le texte tapé est gardé sinon. |
| `pub fn bind_field(self, field: forms::Field) -> Self` | `bind_value` sur la valeur du champ, touché au `blur`, `aria-invalid="true"` tant que son erreur est affichée. Voir [Formulaires](../forms/). |

### Fonctions libres

| Signature | Description |
| --- | --- |
| `pub fn el(tag: &'static str) -> Element` | Un élément avec n'importe quel nom de balise, par exemple `el("my-widget")`. |
| `pub fn text(value: impl Into<Str>) -> Node` | Un nœud texte. |
| `pub fn text_dyn(f: impl Fn() -> String + 'static) -> Node` | Du texte recalculé à partir d'états. |
| `pub fn fragment(nodes: impl IntoIterator<Item = impl Into<Node>>) -> Node` | Plusieurs nœuds sans élément englobant. |
| `pub fn fragment_dyn(f: impl Fn() -> Vec<Node> + 'static) -> Node` | Une région dynamique : reconstruite d'un bloc dans le navigateur quand les états lus par `f` changent ; les états, effets et écouteurs créés pour les nœuds précédents sont libérés, et le focus revient à l'élément correspondant des nouveaux nœuds. Rendue entre `<!--[-->` et `<!--]-->` sur le serveur. |
| `pub fn keyed<T, K>(items: impl Fn() -> Vec<T> + 'static, key: impl Fn(&T) -> K + 'static, row: impl Fn(&T) -> Node + 'static) -> Node` (`T: PartialEq`, `K: Hash`) | Une liste dont les lignes sont gardées par clé : dans le navigateur, les lignes dont la clé est toujours là et dont l'élément est identique gardent leur DOM, leurs états, leurs écouteurs et le focus (et se déplacent si l'ordre a changé) ; les lignes nouvelles ou modifiées sont construites, les lignes retirées libérées. Rendue comme `fragment_dyn` sur le serveur. Voir [État](../state/#longues-listes-lignes-par-cle). |
| `pub fn indexed<T>(items: impl Fn() -> Vec<T> + 'static, row: impl Fn(State<T>) -> Node + 'static) -> Node` (`T: Clone + PartialEq`) | Une liste dont les lignes sont gardées par position : la ligne `i` est construite une fois avec un `State` du `i`-ième élément et mise à jour sur place ; les lignes sont ajoutées ou retirées à la fin. Voir [État](../state/#lignes-par-position-indexed). |
| `pub fn switch<V, K>(value: impl Fn() -> V + 'static, kind: impl Fn(&V) -> K + 'static, view: impl Fn(State<V>) -> Node + 'static) -> Node` (`V: Clone + PartialEq`, `K: PartialEq`) | Affiche `view` pour `value()`, reconstruite seulement quand `kind` change ; sinon le `State<V>` de la vue reçoit la nouvelle valeur. Voir [Applications côté client](../client-app/#garder-une-page-quand-ses-parametres-changent). |
| `pub fn portal(target: &'static str, view: impl Into<Node>) -> Node` | Rend `view` à la fin de l'élément qui correspond à `target` (navigateur seulement ; rien côté serveur), retiré avec le portail. Voir [Composants](../components/#portails). |

### Str

```rust
pub type Str = std::borrow::Cow<'static, str>;
```

Le type de chaîne des noms et valeurs d'attributs, du texte et des URL : un `&'static str` ou une `String`.

### Fonctions de balise

`active::html` contient une fonction par balise, chacune de la forme `pub fn name() -> Element` :

`a` `abbr` `address` `area` `article` `aside` `audio` `b` `bdi` `blockquote` `br` `button` `canvas` `caption` `cite` `code` `col` `colgroup` `data` `datalist` `dd` `del` `details` `dfn` `dialog` `div` `dl` `dt` `em` `embed` `fieldset` `figcaption` `figure` `footer` `form` `h1` `h2` `h3` `h4` `h5` `h6` `header` `hgroup` `hr` `i` `iframe` `img` `input` `ins` `kbd` `label` `legend` `li` `main` `map` `mark` `menu` `meter` `nav` `noscript` `object` `ol` `optgroup` `option` `output` `p` `picture` `pre` `progress` `q` `s` `samp` `search` `section` `select` `small` `source` `span` `strong` `sub` `summary` `sup` `table` `tbody` `td` `template` `textarea` `tfoot` `th` `thead` `time` `tr` `track` `u` `ul` `var` `video` `wbr`

SVG : `svg` `circle` `ellipse` `g` `line` `path` `polygon` `polyline` `rect` `tspan` `defs` `stop`, plus `text_svg` (`<text>`), `use_svg` (`<use>`) et `linear_gradient` (`<linearGradient>`).

Les éléments vides (`area`, `br`, `col`, `embed`, `hr`, `img`, `input`, `source`, `track`, `wbr`, ainsi que `base`, `link` et `meta` via `el`) sont écrits sous la forme `<tag …/>` et ignorent leurs enfants. Il n'existe pas de fonction `html`, `head` ou `body` : [`Document`](#document) les écrit.

## État

### use_state

```rust
pub fn use_state<T: 'static>(initial: T) -> State<T>
```

Crée une valeur réactive appartenant au composant en cours de construction. Dans le navigateur, elle est libérée avec sa région dynamique ; sur le serveur, à la fin du deuxième rendu de premier niveau qui ne la lit ni ne la crée.

### State

```rust
pub struct State<T> { /* index into the state store */ }
```

Un handle vers une valeur réactive. Implémente `Copy`, `Clone` et `Debug` pour tout `T`. Il n'est ni `Send` ni `Sync` : la valeur vit dans le registre du thread qui l'a créée, si bien qu'un handle ne peut ni passer à un autre thread, ni être conservé au-delà d'un `.await` dans une future multithread (`Send`).

| Méthode | Description |
| --- | --- |
| `pub fn get(self) -> T where T: Clone` | Un clone de la valeur. Dans un effet ou une liaison, abonne celui-ci à l'état. |
| `pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R` | Lit la valeur par référence. Abonne comme `get`. |
| `pub fn set(self, value: T)` | Remplace la valeur et réexécute ce qui l'a lue. |
| `pub fn update(self, f: impl FnOnce(&mut T))` | Modifie la valeur sur place et réexécute ce qui l'a lue. |
| `pub fn is_alive(self) -> bool` | `false` une fois l'état libéré (sa région s'est reconstruite). À vérifier avant d'écrire depuis du code asynchrone. |

### effect

```rust
pub fn effect(f: impl Fn() + 'static)
```

Exécute `f` immédiatement, puis de nouveau chaque fois qu'un état lu lors de sa dernière exécution change, une fois les memos qu'il lit à jour. Navigateur uniquement : sans `client`, ne fait rien. Un état écrit dans un effet prévient ses lecteurs une fois l'effet terminé ; un effet qui écrit un état qu'il lit ne se redéclenche pas lui-même.

### on_cleanup

```rust
pub fn on_cleanup(f: impl FnOnce() + 'static)
```

Exécute `f` quand le composant en cours de construction est libéré : sa région dynamique se reconstruit, le routeur affiche une autre page, ou son `Scope` est libéré. Les nettoyages s'exécutent du dernier enregistré au premier, avant la libération des états de la portée. Sur le serveur, ou hors de toute portée, `f` ne s'exécute jamais.

### memo

```rust
pub fn memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> State<T>
```

Un état qui contient `f()`, recalculé quand les états lus par `f` changent — avant l'exécution des effets qui le lisent, et seulement si une entrée a vraiment changé — et écrit seulement si le résultat diffère. Sur le serveur, il garde la valeur calculée à sa création.

### untrack

```rust
pub fn untrack<R>(f: impl FnOnce() -> R) -> R
```

Exécute `f` sans abonner l'effet ou la région dynamique en cours aux états que `f` lit.

### scope et Scope

```rust
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Scope)
pub struct Scope(/* … */);
impl Scope { pub fn dispose(self) }
```

`scope` exécute `f` et renvoie son résultat avec un `Scope` qui possède les états, effets et écouteurs d'événements créés à l'intérieur. `dispose` les libère. `Scope` est `#[must_use]`. Sur le serveur, un rendu effectué à l'intérieur d'une portée ouverte ne vide pas le registre des états.

### watch et watch_immediate

```rust
pub fn watch<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, &T) + 'static) -> Watcher
pub fn watch_immediate<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, callback: impl Fn(&T, Option<&T>) + 'static) -> Watcher
```

Appelle `callback(&nouvelle, &ancienne)` chaque fois que la valeur renvoyée par `source` change (`PartialEq`). `source` est suivie comme un effet, `callback` ne l'est pas. `watch` ignore la première valeur ; `watch_immediate` appelle tout de suite `callback(&premiere, None)`. Navigateur uniquement, comme `effect` ; libéré avec le composant.

### Watcher

```rust
#[derive(Clone, Copy, Debug)]
pub struct Watcher { /* … */ }
impl Watcher { pub fn stop(self) }
```

Arrête un watcher avant que son composant soit libéré. L'arrêter deux fois ne fait rien.

### batch

```rust
pub fn batch<R>(f: impl FnOnce() -> R) -> R
```

Exécute `f` en retenant les effets que ses écritures déclenchent jusqu'à son retour : chacun s'exécute alors une fois. Les batch imbriqués attendent le plus externe. Voir [État](../state/#grouper-des-ecritures-batch).

### provide, use_context et expect_context

```rust
pub fn provide<T: 'static, R>(value: T, f: impl FnOnce() -> R) -> R
pub fn use_context<T: Clone + 'static>() -> Option<T>
pub fn expect_context<T: Clone + 'static>() -> T
```

`provide` donne `value` à tout ce que construit `f`, régions dynamiques et listes par clé comprises (elles la gardent quand elles se reconstruisent). `use_context` renvoie la valeur de type `T` fournie la plus proche, clonée ; `expect_context` panique, en nommant le type, s'il n'y en a pas. Appelez-les pendant la construction de la vue. Voir [État](../state/).

## Données asynchrones et erreurs

Voir [Données asynchrones et erreurs](../async-data/).

### resource et Resource

```rust
pub fn resource<S, T, F>(source: impl Fn() -> S + 'static, fetch: impl Fn(S) -> F + 'static) -> Resource<T>
where S: Clone + PartialEq + 'static, T: 'static, F: Future<Output = T> + 'static

pub struct Resource<T> { /* … */ } // Copy
```

Charge `fetch(source())` et recharge quand `source` change ; un résultat plus ancien que la dernière requête est ignoré. Dans le navigateur, le future s'exécute sur la boucle d'événements ; sur le serveur, il est interrogé une fois (une valeur prête est rendue, sinon elle reste en chargement).

| Méthode | Description |
| --- | --- |
| `pub fn get(self) -> Option<T>` (`T: Clone`) | La dernière valeur, `None` avant la première. |
| `pub fn with<R>(self, f: impl FnOnce(Option<&T>) -> R) -> R` | La lit par référence. |
| `pub fn loading(self) -> bool` | Si un chargement est en cours. |
| `pub fn refetch(self)` | Recharge avec la source actuelle. |
| `pub fn set(self, value: T)` | Remplace la valeur tout de suite (mise à jour optimiste). |
| `pub fn state(self) -> State<Option<T>>` | L'état qui contient la valeur. |

### suspense

```rust
pub fn suspense(fallback: impl Into<Node>, view: impl FnOnce() -> Node) -> Node
```

Construit `view` une fois et affiche `fallback` à sa place tant qu'une ressource créée à l'intérieur n'a pas chargé sa première valeur. Les rechargements suivants gardent la vue.

### error_boundary, ErrorBoundary et report_error

```rust
pub fn error_boundary(view: impl Fn() -> Node + 'static, fallback: impl Fn(ErrorBoundary) -> Node + 'static) -> Node
pub fn report_error(error: impl Display) -> bool

#[derive(Clone, Copy, Debug)]
pub struct ErrorBoundary { /* … */ }
impl<T: Into<Node>, E: Display> From<Result<T, E>> for Node
```

Affiche `fallback` au lieu de `view` dès qu'une erreur est signalée à l'intérieur : un `Err` converti en `Node`, `report_error` pendant la construction (renvoie `false` et journalise s'il n'y a pas de boundary), ou `ErrorBoundary::report` plus tard. `ErrorBoundary` a `report(error)`, `errors() -> Vec<String>`, `has_errors() -> bool` et `reset()` (oublie les erreurs et reconstruit la vue) ; la vue l'obtient avec `use_context::<ErrorBoundary>()`.

## Formulaires

Le module `active::forms`. Voir [Formulaires et validation](../forms/).

### Form

```rust
#[derive(Clone, Copy, Debug)]
pub struct Form { /* … */ }
```

| Méthode | Description |
| --- | --- |
| `pub fn new() -> Self` | Un formulaire sans champ (aussi `Default`). |
| `pub fn field(self, value: State<String>, rules: impl IntoIterator<Item = Rule>) -> Field` | Ajoute un champ vérifié par `rules`, dans l'ordre. |
| `pub fn is_valid(self) -> bool` | Si tous les champs sont valides. |
| `pub fn submit(self) -> bool` | Affiche désormais toutes les erreurs ; renvoie si le formulaire est valide. |
| `pub fn submitted(self) -> bool` | Si `submit` a été appelé depuis la dernière réinitialisation. |
| `pub fn reset(self)` | Cache à nouveau les erreurs (les valeurs ne changent pas). |

### Field

```rust
#[derive(Clone, Copy, Debug)]
pub struct Field { /* … */ }
```

| Méthode | Description |
| --- | --- |
| `pub fn value(self) -> State<String>` | L'état du champ. |
| `pub fn error(self) -> Option<String>` | L'erreur actuelle. |
| `pub fn is_valid(self) -> bool` | S'il n'y a pas d'erreur. |
| `pub fn message(self) -> Option<String>` | L'erreur à afficher : une fois touché ou soumis. |
| `pub fn touched(self) -> bool` / `pub fn touch(self)` | Si le visiteur a quitté le champ ; le marquer comme quitté. |

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

`check` renvoie le message quand `value` n'est pas valide. Toutes les règles sauf `required` acceptent une valeur vide.

## Événements

### Event

Un événement DOM passé aux gestionnaires. Implémente `Debug` et `Clone`.

| Méthode | Description |
| --- | --- |
| `pub fn value(&self) -> String` | Valeur courante de l'`<input>`, du `<textarea>` ou du `<select>` qui a déclenché l'événement ; vide sinon. |
| `pub fn key(&self) -> String` | La touche d'un événement clavier (`"Enter"`, `"k"`…) ; vide sinon. |
| `pub fn prevent_default(&self)` | Annule l'action par défaut du navigateur. |
| `pub fn current_target(&self) -> Option<Dom>` | L'élément auquel l'écouteur est attaché. `None` sur le serveur. |
| `pub fn raw(&self) -> &web_sys::Event` | L'événement sous-jacent. Uniquement avec `client`. |
| `pub fn stop_propagation(&self)` | Empêche l'événement d'atteindre les éléments parents. |
| `pub fn checked(&self) -> bool` | Si la case à cocher ou le bouton radio qui a déclenché l'événement est coché ; `false` sinon. |

### Dom

```rust
pub type Dom = web_sys::Element; // with `client`
pub struct Dom;                  // without: an empty placeholder (Debug, Clone, Copy, Default)
```

Le type d'élément passé à `on_mount` et renvoyé par `Event::current_target`.

## SEO

### Seo

```rust
pub struct Seo { /* … */ }
```

Les balises du `<head>` d'une page. Implémente `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`, `Display` (même sortie que `to_html`) et `Into<Node>`. `#[must_use]` : les setters renvoient la valeur mise à jour. Règles et exemples : [Balises SEO](../seo/).

| Méthode | Description |
| --- | --- |
| `pub fn new() -> Self` | Vide ; ne rend que `og:type` (`website`) et `twitter:card` (`summary`). |
| `pub fn title(self, title: impl Into<String>) -> Self` | `<title>` et `og:title`. |
| `pub fn title_template(self, template: impl Into<String>) -> Self` | Modèle pour `<title>` ; le premier `%s` est remplacé par le titre. |
| `pub fn description(self, description: impl Into<String>) -> Self` | Meta description et `og:description`. |
| `pub fn canonical(self, url: impl Into<String>) -> Self` | `<link rel="canonical">` et `og:url`. |
| `pub fn image(self, url: impl Into<String>) -> Self` | `og:image` ; fait passer `twitter:card` à `summary_large_image`. |
| `pub fn og_type(self, og_type: impl Into<String>) -> Self` | `og:type`. |
| `pub fn site_name(self, site_name: impl Into<String>) -> Self` | `og:site_name`, et le titre quand aucun n'est défini. |
| `pub fn locale(self, locale: impl Into<String>) -> Self` | `og:locale`. |
| `pub fn twitter(self, handle: impl Into<String>) -> Self` | `twitter:site`. |
| `pub fn base_url(self, url: impl Into<String>) -> Self` | Origine des URL relatives de canonical, image et alternate. |
| `pub fn noindex(self, noindex: bool) -> Self` | `noindex` dans `<meta name="robots">`. |
| `pub fn nofollow(self, nofollow: bool) -> Self` | `nofollow` dans `<meta name="robots">`. |
| `pub fn alternate(self, lang: impl Into<String>, url: impl Into<String>) -> Self` | `<link rel="alternate" hreflang>` ; la même langue une seconde fois remplace l'URL. |
| `pub fn json_ld(self, json: impl Into<String>) -> Self` | Un `<script>` JSON-LD, un par appel ; `<` est échappé en `\u003c`. |
| `pub fn meta_name(self, name: impl Into<String>, content: impl Into<String>) -> Self` | `<meta name content>` supplémentaire. |
| `pub fn meta_property(self, property: impl Into<String>, content: impl Into<String>) -> Self` | `<meta property content>` supplémentaire. |
| `pub fn merge(&mut self, other: &Seo)` | Applique `other` par-dessus : ses valeurs définies l'emportent, les alternatives et les meta supplémentaires sont remplacées par clé, le JSON-LD est ajouté. |
| `pub fn merged(&self, other: &Seo) -> Seo` | `merge` dans une nouvelle valeur. |
| `pub fn to_html(&self) -> String` | Rend les balises. |
| `pub fn write_html(&self, out: &mut String)` | Ajoute les balises à `out`. |

## Documents

### Document

```rust
pub struct Document { /* … */ }
```

Une page HTML complète. Implémente `Clone` et `Default`. Les méthodes du builder prennent `self` et sont `#[must_use]`. Guide : [Documents](../documents/).

| Méthode | Description |
| --- | --- |
| `pub fn new(body: impl Into<Node>) -> Self` | Une page dont le `<body>` est `body`. |
| `pub fn lang(self, lang: impl Into<Str>) -> Self` | `<html lang>`. |
| `pub fn seo(self, seo: Seo) -> Self` | SEO de base, fusionné avec chaque `Seo` du corps ; remplace une base précédente. |
| `pub fn stylesheet(self, href: impl Into<Str>) -> Self` | `<link rel="stylesheet">`, dans l'ordre des appels. |
| `pub fn head(self, html: impl Into<Str>) -> Self` | HTML de confiance ajouté à la fin du `<head>`. |
| `pub fn client(self, js: impl Into<Str>, wasm: impl Into<Str>) -> Self` | Précharge et démarre le bundle `wasm-bindgen --target web --no-typescript`. |
| `pub fn manifest(self, href: &str, manifest: &pwa::Manifest) -> Self` | `<link rel="manifest">` vers `href`, `theme-color`, le titre de l'écran d'accueil et l'icône Apple (la plus grande icône PNG de type `any`). |
| `pub fn service_worker(self, url: &str) -> Self` | Un script en ligne qui enregistre le service worker servi sur `url` une fois la page chargée. |
| `pub fn viewport(self, content: impl Into<Str>) -> Self` | `content` de `<meta name="viewport">` (par défaut `width=device-width, initial-scale=1`). |
| `pub fn html_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Attribut sur `<html>`. |
| `pub fn body_attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self` | Attribut sur `<body>`. |
| `pub fn final_seo(&self) -> Seo` | Le SEO fusionné de la page, îlots et contenu courant des régions dynamiques compris (évalués séparément de `render`). |
| `pub fn render(&self) -> String` | La page entière. Rend d'abord le corps en récupérant ses valeurs `Seo` au passage, si bien que chaque région dynamique n'est évaluée qu'une fois. Peut être appelée autant de fois que nécessaire. |

## PWA

Le module `active::pwa`. Guide : [Applications installables (PWA)](../pwa/).

### Manifest

```rust
pub struct Manifest { /* … */ }
```

Le manifeste d'application web. Implémente `Clone`, `Debug`, `PartialEq` et `Eq`. Les méthodes du builder prennent `self` et sont `#[must_use]`.

| Méthode | Description |
| --- | --- |
| `pub fn new(name: impl Into<String>) -> Self` | Un manifeste nommé `name`, `start_url` `./`, `display` `standalone`, thème `#000000`, fond `#ffffff`. |
| `pub fn short_name(self, name: impl Into<String>) -> Self` | Le nom sous l'icône. |
| `pub fn description(self, description: impl Into<String>) -> Self` | Ce que fait l'application. |
| `pub fn id(self, id: impl Into<String>) -> Self` | L'identité de l'application (`start_url` par défaut). |
| `pub fn start_url(self, url: impl Into<String>) -> Self` | La page qu'ouvre l'icône. |
| `pub fn scope(self, scope: impl Into<String>) -> Self` | Les URL qui font partie de l'application (par défaut le dossier du manifeste). |
| `pub fn display(self, display: impl Into<String>) -> Self` | `standalone`, `fullscreen`, `minimal-ui` ou `browser`. |
| `pub fn orientation(self, orientation: impl Into<String>) -> Self` | `portrait`, `landscape`, `any`… |
| `pub fn theme_color(self, color: impl Into<String>) -> Self` | Couleur de la barre d'état et du titre de la fenêtre. |
| `pub fn background_color(self, color: impl Into<String>) -> Self` | Couleur de l'écran de démarrage. |
| `pub fn lang(self, lang: impl Into<String>) -> Self` | Langue des textes. |
| `pub fn categories<I, S>(self, categories: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | Catégories des stores ; remplace les précédentes. |
| `pub fn icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | Une icône de type `any`. |
| `pub fn maskable_icon(self, src: impl Into<String>, sizes: impl Into<String>, mime: impl Into<String>) -> Self` | Une icône de type `maskable` (contenu dans les 80 % du centre). |
| `pub fn name(&self) -> &str` | Le nom. |
| `pub fn theme(&self) -> &str` | La couleur du thème. |
| `pub fn icons(&self) -> &[Icon]` | Les icônes. |
| `pub fn to_json(&self) -> String` | Le manifeste en JSON (chaînes échappées, `<` en `\u003c`). |

### Icon

```rust
pub struct Icon { pub src: String, pub sizes: String, pub mime: String, pub purpose: String }
```

Une icône du manifeste : URL, `"192x192"` ou `"any"`, type MIME, et `"any"`, `"maskable"` ou `"monochrome"`.

### ServiceWorker

```rust
pub struct ServiceWorker { /* … */ }
```

Le service worker hors ligne. Implémente `Clone`, `Debug`, `PartialEq` et `Eq`. Les méthodes du builder prennent `self` et sont `#[must_use]`.

| Méthode | Description |
| --- | --- |
| `pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self` | Un worker dont le cache est `{name}-{version}` ; une nouvelle version remplace les caches des anciennes. |
| `pub fn precache<I, S>(self, urls: I) -> Self` (`I: IntoIterator<Item = S>`, `S: Into<String>`) | Les URL enregistrées à l'installation ; s'ajoutent aux précédentes. |
| `pub fn offline_page(self, url: impl Into<String>) -> Self` | La page affichée hors ligne pour une page jamais enregistrée ; aussi mise en cache d'avance. |
| `pub fn network_only(self, prefix: impl Into<String>) -> Self` | Les chemins qui commencent par `prefix` passent toujours par le réseau. |
| `pub fn cache_name(&self) -> String` | `{name}-{version}`. |
| `pub fn to_js(&self) -> String` | Le script : les pages d'abord depuis le réseau (hors ligne, la copie enregistrée puis la page hors ligne), les autres fichiers `GET` du site depuis le cache, rafraîchis en arrière-plan. |

### register_script

```rust
pub fn register_script(url: &str) -> String
```

Le script en ligne écrit par `Document::service_worker` : enregistre le worker de `url` à l'événement `load`. Calculez son hash pour l'autoriser dans une Content-Security-Policy.

### version

```rust
pub fn version<I, B>(parts: I) -> String
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
```

Un hash court de `parts` (10 caractères hexadécimaux, FNV-1a), pour `ServiceWorker::new` : hachez ce que le worker enregistre, et chaque déploiement qui le modifie installe un nouveau worker.

## Routage

Disponible côté serveur et dans le navigateur. Voir [Applications côté client](../client-app/).

### match_path

```rust
pub fn match_path(pattern: &str, path: &str) -> Option<Params>
```

Compare `path` (sans sa chaîne de requête, son fragment ni les barres obliques en trop) à `pattern` : `posts` correspond à ce segment, `:id` à un segment, `:id?` à un segment facultatif, `*rest` (en dernier) à tout le reste (`*` seul ne capture rien). Les valeurs sont décodées.

### Params

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Params { /* … */ }
```

| Méthode | Description |
| --- | --- |
| `pub fn get(&self, name: &str) -> Option<&str>` | La valeur d'un paramètre. |
| `pub fn parse<T: FromStr>(&self, name: &str) -> Option<T>` | La valeur convertie ; `None` si absente ou invalide. |
| `pub fn iter(&self) -> impl Iterator<Item = (&str, &str)>` | Les paramètres, dans l'ordre du motif. |
| `pub fn is_empty(&self) -> bool` | S'il n'y en a aucun. |

### Routes

```rust
#[derive(Clone, Default)]
pub struct Routes { /* … */ }
```

| Méthode | Description |
| --- | --- |
| `pub fn new() -> Self` | Une table vide. |
| `pub fn route(self, pattern: impl Into<String>, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | Ajoute une route ; le premier motif qui correspond l'emporte. |
| `pub fn fallback(self, view: impl Fn(State<Params>) -> Node + 'static) -> Self` | La page affichée quand rien ne correspond ; son paramètre `path` contient tout le chemin. |
| `pub fn resolve(&self, path: &str) -> Option<Params>` | Les paramètres de la route qui correspond (ou du fallback). |
| `pub fn matches(&self, path: &str) -> bool` | Si une route autre que le fallback correspond. |
| `pub fn render(&self, path: &str) -> Node` | La page d'un chemin. |
| `pub fn view(self, path: impl Fn() -> String + 'static) -> Node` | La page d'un chemin qui change : gardée tant que la même route correspond (son `State<Params>` est mis à jour), reconstruite quand une autre correspond. |

### before_navigate et Navigation

```rust
pub fn before_navigate(guard: impl Fn(&str) -> Navigation + 'static)

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation { Continue, Cancel, Redirect(String) }
```

Enregistre une garde consultée avant que le routeur du navigateur aille à une adresse (liens, `navigate`, `navigate_replace`, précédent et suivant). Les gardes s'exécutent dans l'ordre d'enregistrement ; une redirection est revérifiée (dix d'affilée annulent). Retirée avec le composant qui l'a enregistrée. Jamais appelée côté serveur. Voir [Applications côté client](../client-app/#gardes-de-navigation).

## Îlots

### island

```rust
pub fn island(name: &'static str, props: impl Into<Str>, component: IslandFn) -> Node
```

Appelle `component(&props)` et enveloppe sa vue dans `<active-island data-island="name" data-props="props">`, pour qu'elle soit hydratée dans le navigateur par le composant enregistré sous le nom `name`.

### island_with et Hydrate

```rust
pub fn island_with(name: &'static str, props: impl Into<Str>, component: IslandFn, when: Hydrate) -> Node

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hydrate { #[default] Load, Idle, Visible, Media(Str), Interaction }
```

Comme `island`, hydraté quand `when` le dit : tout de suite, quand le navigateur est inoccupé, quand il est visible, tant qu'une media query correspond, ou à la première interaction. Ajoute `data-hydrate="idle|visible|media:…|interaction"` (rien pour `Load`). Voir [Îlots](../islands/#quand-les-ilots-s-hydratent).

### IslandFn

```rust
pub type IslandFn = fn(&str) -> Node;
```

Un composant îlot : construit sa vue à partir d'une chaîne de props, de façon identique sur le serveur et dans le navigateur.

## Fonctions du navigateur

Disponibles avec la fonctionnalité `client`, à la racine du crate.

### hydrate

```rust
pub fn hydrate(islands: &[(&str, IslandFn)])
```

Hydrate chaque `<active-island>` de la page dont le nom figure dans `islands` : exécute le composant avec ses props et lie les écouteurs, le texte, les attributs et les classes dynamiques au HTML existant. Les îlots déjà hydratés et les noms inconnus sont ignorés. En cas d'incohérence de structure, affiche un avertissement dans la console et rend l'îlot de zéro. Ne modifie jamais le `<head>`.

### mount

```rust
pub fn mount(view: &Node, parent: &web_sys::Element)
```

Crée le DOM de `view` après les enfants existants de `parent`. Fusionne les valeurs `Seo` de la vue et les écrit dans le `<head>` avec `update_head` ; les régions dynamiques de la vue mettent le `<head>` à jour quand elles sont reconstruites.

### mount_to

```rust
pub fn mount_to(selector: &str, view: impl Into<Node>)
```

Remplace le contenu du premier élément correspondant au sélecteur CSS `selector` par `view` (au moyen de `mount`). Affiche un avertissement dans la console si aucun élément ne correspond. Le point d'entrée habituel d'une [application côté client](../client-app/).

### location

```rust
pub fn location() -> State<String>
```

Le chemin courant (`window.location.pathname`, par exemple `"/search"`), sous forme d'état. Le premier appel à `location`, `query` ou `query_param` démarre le routeur : Précédent et Suivant le mettent à jour, et les clics sur des liens `<a href>` de même origine appellent `navigate` au lieu de charger une page (sauf pour les liens avec un `target` autre que `_self`, un attribut `download`, une touche de modification ou un bouton autre que le principal, et pour les ancres de la page courante).

### query

```rust
pub fn query() -> State<String>
```

La chaîne de requête courante sans le `?` (`"q=rust&page=2"`, vide s'il n'y en a pas), sous forme d'état mis à jour par le routeur comme `location`.

### query_param

```rust
pub fn query_param(name: &str) -> Option<String>
```

Un paramètre de la chaîne de requête, décodé (`query_param("q")` sur `/search?q=caf%C3%A9` donne `Some("café")`), ou `None`. Le lire dans une vue, une région dynamique ou un effet abonne celui-ci à `query`.

### navigate

```rust
pub fn navigate(to: &str)
```

Ajoute une entrée d'historique pour `to` (`"/about"`, `"/search?q=rust"`, `"/docs#install"`), met à jour `location()` et `query()` (seulement ceux qui ont changé, pour que seules les vues qui en dépendent soient reconstruites), puis fait défiler jusqu'en haut, ou jusqu'à l'élément dont l'id est le `#fragment`. Les gardes enregistrées avec [`before_navigate`](#before-navigate-et-navigation) sont consultées d'abord ; la position de défilement de la page quittée est gardée pour le bouton retour.

### navigate_replace

```rust
pub fn navigate_replace(to: &str)
```

Comme `navigate`, en remplaçant l'entrée actuelle de l'historique au lieu d'en ajouter une.

### update_head

```rust
pub fn update_head(seo: &Seo)
```

Écrit `seo` dans le `<head>` de la page : définit `document.title` (avec un titre vide quand `seo` n'a ni titre ni nom de site) et remplace les balises de description, robots, canonical, alternatives `hreflang`, Open Graph, Twitter et JSON-LD, ainsi que toute balise meta portant le même nom ou la même propriété que l'une des balises meta supplémentaires. Les autres éléments du `<head>` ne sont pas touchés.

## Streaming, routage, i18n et outils

Aucun de ces éléments n'est dans le prélude : utilisez-les comme `active::ssr::stream`, `active::persisted`, `active::i18n::I18n`, etc. Les guides : [Streaming et données serveur](../streaming/), [Layouts, chargement des données et préchargement](../routing/), [Listes animées et actions](../animations/), [Formulaires et validation](../forms/), [État persistant et débogage](../stores/), [Traductions et formatage](../i18n/).

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

Une page envoyée en streaming ajoute deux marqueurs : `<active-suspense data-suspense="n">` autour du fallback d'une frontière envoyée plus tard, et `<script type="application/json" data-active-data="clé">` qui contient la valeur d'une `ssr::resource`.

## Marqueurs HTML

La sortie du serveur contient quelques marqueurs sur lesquels s'appuie l'hydratation. Ils font partie du format : ne supprimez pas les commentaires HTML des pages qui contiennent des îlots.

| Marqueur | Signification |
| --- | --- |
| `<!--#-->` | début d'un texte dynamique |
| `<!---->` | séparateur entre deux textes adjacents |
| `<!--[-->` … `<!--]-->` | une région dynamique |
| `<active-island data-island data-props>` | un îlot ; reçoit `data-hydrated` une fois hydraté |
| `data-hydrate="…"` sur un îlot | sa stratégie d'hydratation (`island_with`) |

## Paniques

- Utiliser un `State` après sa libération provoque une panique avec le message « this state was disposed ». Dans le navigateur, cela arrive après la reconstruction de sa région dynamique ou la libération de sa portée ; sur le serveur, après deux rendus de premier niveau qui ne l'ont pas utilisé.
- Lire un état dans sa propre closure `update` provoque une panique avec le message « a state was read inside its own `update`; read it before updating ». Appeler `set` ou `update` sur un état à l'intérieur de son propre `with` provoque une panique avec le message « a state was updated while being read or updated ».
