---
title: État et effets
description: Gardez les valeurs changeantes dans des handles use_state, liez-les au texte, aux attributs, aux classes et aux listes, et lancez des effets côté client.
---

Les parties interactives d'une page ont besoin de valeurs qui changent : un compteur, le texte d'un champ de recherche, les éléments d'une liste. Dans `active`, une telle valeur vit dans un **état**, créé avec `use_state`. La vue le lit au travers de petites closures, et quand l'état change, seules les parties de la page qui le lisent sont mises à jour.

## use_state

```rust
use active::prelude::*;

let count = use_state(0);
count.set(5);
count.update(|n| *n += 1);
assert_eq!(count.get(), 6);
```

`use_state(initial)` stocke la valeur et renvoie un `State<T>`, un handle vers celle-ci. N'importe quel type `'static` convient : nombres, `String`, `Vec`, vos propres structures et énumérations.

| Méthode | Sert à |
| --- | --- |
| `.get()` | obtenir une copie de la valeur (exige `T: Clone`) |
| `.with(\|v\| …)` | lire au travers d'une référence, sans cloner (`items.with(Vec::len)`) |
| `.set(value)` | remplacer la valeur |
| `.update(\|v\| …)` | modifier la valeur sur place (`list.push(…)`, `*n += 1`) |

`set` et `update` notifient tout ce qui a lu l'état, ce qui met la page à jour.

## Pourquoi les handles sont Copy

Un `State<T>` n'est qu'un index dans un registre : il implémente donc `Copy`, quel que soit `T`. Vous pouvez déplacer le même handle dans autant de closures que nécessaire, sans `Rc`, sans `clone()` ni durées de vie :

```rust
use active::prelude::*;

fn counter() -> Element {
    let count = use_state(0);
    div()
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
}
```

Chaque closure `move` reçoit sa propre copie du handle ; toutes les copies désignent la même valeur.

## Lier l'état à la vue

Les méthodes qui se terminent par `_dyn` (ainsi que `class_if` et `bool_attr`) reçoivent une closure au lieu d'une valeur. Sur le serveur, la closure s'exécute une fois pour produire le HTML. Dans le navigateur, elle s'exécute de nouveau chaque fois qu'un état qu'elle a lu change, et seul ce nœud texte, cet attribut ou cette classe est mis à jour.

| Méthode | Met à jour |
| --- | --- |
| `.text_dyn(f)` ou `text_dyn(f)` | un nœud texte |
| `.attr_dyn(name, f)` | un attribut (`value` met aussi à jour la valeur courante du champ) |
| `.bool_attr(name, f)` | un attribut booléen comme `disabled`, `hidden`, `open` (`checked` met aussi à jour la case à cocher) |
| `.class_if(class, f)` | une classe, présente tant que `f()` vaut `true` |
| `.html_dyn(f)` | le HTML interne de l'élément (HTML de confiance uniquement) |
| `.children_dyn(f)` ou `fragment_dyn(f)` | un groupe de nœuds, reconstruit (voir plus bas) |

```rust
use active::prelude::*;

let stock = use_state(3);
let view = div()
    .class("product")
    .class_if("sold-out", move || stock.get() == 0)
    .attr_dyn("data-stock", move || stock.get().to_string())
    .child(button().bool_attr("disabled", move || stock.get() == 0).text("Buy"))
    .child(span().text_dyn(move || format!("{} left", stock.get())));

assert_eq!(
    Node::from(view).render(),
    r#"<div class="product" data-stock="3"><button>Buy</button><span><!--#-->3 left</span></div>"#
);
```

Le commentaire `<!--#-->` marque le texte dynamique pour que le navigateur puisse le retrouver lorsqu'il [hydrate](../islands/) la page.

> **Astuce :** Pour afficher ou masquer quelque chose, `bool_attr("hidden", …)` ou `class_if(…)` est l'option la moins coûteuse : l'élément reste dans la page et seul un attribut change.

## Valeurs dérivées

Une valeur calculée à partir d'états n'est qu'une closure. Comme elle capture des handles `Copy`, la closure est elle aussi `Copy` et peut servir à plusieurs endroits :

```rust
use active::prelude::*;

let price = use_state(120u32);
let quantity = use_state(2u32);
let total = move || price.get() * quantity.get();

let view = p()
    .text_dyn(move || format!("Total: {} €", total()))
    .class_if("expensive", move || total() > 200);
assert!(Node::from(view).render().contains("Total: 240 €"));
```

Tout ce qui lit `total()` est mis à jour quand l'un ou l'autre des états change.

Quand les entrées changent souvent mais le résultat rarement, gardez le résultat dans un **memo** : `memo(f)` renvoie un `State` qui n'est écrit que si le nouveau résultat de `f` diffère du précédent, si bien que ce qui le lit ne se réexécute qu'à ce moment-là.

```rust
use active::prelude::*;

let positions = use_state(vec![("VER", 1), ("NOR", 2)]);
// Mis à jour chaque seconde alors que les noms (donc ce menu) changent rarement.
let names = memo(move || positions.with(|p| p.iter().map(|(n, _)| *n).collect::<Vec<_>>()));
let menu = select().children_dyn(move || names.get().into_iter().map(|n| option().text(n).into()).collect());
```

## Listes et conditions qui changent

`.children_dyn(f)` ajoute une **région dynamique** à un élément : `f` renvoie les nœuds à afficher, et chaque fois qu'un état qu'elle a lu change, la région est reconstruite. Les enfants ajoutés avant ou après la région restent en place. `fragment_dyn(f)` crée la même région sous forme de nœud autonome.

```rust
use active::prelude::*;

let items = use_state(vec!["Milk".to_string(), "Bread".to_string()]);
let view = div()
    .child(ul().child(li().text("Always first")).children_dyn(move || {
        items.with(|list| list.iter().map(|item| li().text(item.clone()).into()).collect())
    }))
    .child(button().on_click(move |_| items.update(|list| list.push("Eggs".into()))).text("Add eggs"));

assert_eq!(
    Node::from(view).render(),
    "<div><ul><li>Always first</li><!--[--><li>Milk</li><li>Bread</li><!--]--></ul><button>Add eggs</button></div>"
);
```

Sur le serveur, la région est écrite entre les marqueurs `<!--[-->` et `<!--]-->`. Dans le navigateur :

- seule la région est reconstruite, le reste de la page n'est pas touché ;
- les états, effets et écouteurs d'événements créés lors de la construction du contenu précédent sont libérés ;
- les états créés **en dehors** de la région (comme `items` ci-dessus) sont conservés : c'est ainsi qu'une liste garde ses données pendant que ses éléments sont reconstruits.

Une condition qui change est une région qui renvoie soit des nœuds, soit aucun :

```rust
use active::prelude::*;

fn details() -> Element {
    let open = use_state(false);
    div()
        .child(button().on_click(move |_| open.update(|o| *o = !*o)).text("Details"))
        .children_dyn(move || {
            if open.get() {
                vec![p().text("Delivered in 48 hours.").into()]
            } else {
                Vec::new()
            }
        })
}
```

### Longues listes : lignes par clé

Une région `children_dyn` est reconstruite d'un bloc. C'est rapide pour les listes d'une page typique (quelques dizaines d'éléments) et c'est le choix le plus simple. Pour les listes longues ou interactives, `children_keyed` (ou la fonction libre `keyed`) garde chaque ligne par clé : quand les éléments changent, une ligne dont la clé est toujours là et dont l'élément est identique garde son DOM, ses états, ses écouteurs et le focus, et se déplace si l'ordre a changé. Seules les lignes nouvelles ou modifiées sont construites, et les lignes retirées sont libérées.

```rust
use active::prelude::*;

#[derive(Clone, PartialEq)]
struct Todo {
    id: u32,
    text: String,
    done: bool,
}

fn list(todos: State<Vec<Todo>>) -> Element {
    ul().children_keyed(
        move || todos.get(),
        |todo| todo.id,
        |todo| {
            li().class(if todo.done { "done" } else { "" })
                .text(todo.text.clone())
                .into()
        },
    )
}
```

- La première closure renvoie les éléments (lire des états abonne la liste), la deuxième une clé qui identifie un élément dans le temps (un identifiant, pas une position), la troisième construit une ligne.
- Le type des éléments implémente `PartialEq` : une ligne dont l'élément a changé est reconstruite, elle affiche donc toujours les valeurs actuelles.
- Faites de chaque ligne un seul élément (`li`, `tr`…).
- Avec `children_dyn`, gardez les champs de texte **en dehors** de la région : ils sont recréés à chaque reconstruction et perdent le focus. Une ligne par clé qui ne change pas garde son champ.

### Lignes par position : indexed

`children_keyed` suit chaque élément : quand un élément change, sa ligne est reconstruite. Pour les listes dont les éléments changent **sur place** — un classement rafraîchi chaque seconde, les résultats d'une autre saison affichés dans le même tableau —, `children_indexed` (ou la fonction libre `indexed`) garde chaque ligne par **position** et ne la reconstruit jamais : la ligne `i` est construite une fois avec un `State` qui contient le `i`-ième élément, et seuls les textes, attributs et classes liés à cet état sont mis à jour. Les lignes sont ajoutées ou retirées à la fin quand la liste s'allonge ou raccourcit.

```rust
use active::prelude::*;

#[derive(Clone, PartialEq)]
struct Line {
    driver: String,
    gap: String,
}

fn timing(lines: State<Vec<Line>>) -> Element {
    ol().children_indexed(
        move || lines.get(),
        |line| {
            li().child(span().text_dyn(move || line.with(|l| l.driver.clone())))
                .child(span().class("gap").text_dyn(move || line.with(|l| l.gap.clone())))
                .into()
        },
    )
}
```

Les lignes gardent leur DOM : les animations CSS d'entrée ne se rejouent pas et un champ garde le focus. Choisissez `indexed` quand les positions comptent plus que les identités, `keyed` quand les lignes doivent suivre leur élément (une liste de tâches qu'on réordonne).

## Effets

`effect(f)` exécute `f` une fois, puis de nouveau chaque fois qu'un état qu'elle a lu change. Utilisez-le pour le travail extérieur à la vue : enregistrer dans `localStorage`, programmer un minuteur, journaliser, appeler une API du navigateur.

```rust
use active::prelude::*;

/// Enregistre le brouillon dans le navigateur. Les effets ne s'exécutent jamais sur le
/// serveur, mais le code doit aussi y compiler, d'où la version vide.
#[cfg(target_arch = "wasm32")]
fn save(text: &str) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("draft", text);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn save(_text: &str) {}

fn editor() -> Element {
    let draft = use_state(String::new());
    effect(move || draft.with(|text| save(text)));
    textarea().on_input(move |e| draft.set(e.value()))
}
```

Le code propre au navigateur utilise [`web-sys`](https://docs.rs/web-sys), déclaré avec les fonctionnalités dont il a besoin : `web-sys = { version = "0.3", features = ["Storage", "Window"] }` dans les dépendances `wasm32`.

Comportement des effets :

- **Navigateur uniquement.** Sur le serveur, `effect` ne fait absolument rien : un effet peut donc utiliser librement les API du navigateur. Ne calculez jamais dans un effet quelque chose dont le HTML a besoin ; calculez-le dans la vue.
- **Dépendances automatiques.** Un effet dépend des états qu'il a lus lors de sa dernière exécution. S'il ne lit `a` que lorsque `flag` est vrai, il cesse de réagir à `a` dès que `flag` est faux.
- **Pas d'incohérence.** Quand un état change, les memos calculés à partir de lui sont mis à jour avant l'exécution des effets qui les lisent : un effet ne voit jamais une valeur et un memo qui en dérive en décalage. Chaque effet s'exécute une fois par changement, et un memo n'est recalculé que si l'une de ses entrées a vraiment changé.
- **Un à la fois.** Un état écrit dans un effet prévient ses lecteurs une fois cet effet terminé (et avant que le `set` à l'origine de tout ne rende la main).
- **Pas de boucle infinie.** Un effet qui écrit un état qu'il lit ne se redéclenche pas lui-même ; deux effets qui écrivent chacun ce que l'autre lit sont arrêtés après mille exécutions.
- **Libérés avec leur région.** Les effets créés dans une région dynamique s'arrêtent quand la région est reconstruite.

## Nettoyer

Un composant lance parfois quelque chose qui vit hors de la vue : un minuteur, une WebSocket, une boucle d'animation, un écouteur sur `window`. `on_cleanup(f)` exécute `f` quand le composant est libéré, c'est-à-dire quand la région dynamique qui l'a construit se reconstruit ou que le routeur affiche une autre page :

```rust
use active::prelude::*;

fn clock() -> Element {
    let seconds = use_state(0u32);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, closure::Closure};
        let tick = Closure::<dyn Fn()>::new(move || seconds.update(|s| *s += 1));
        let window = web_sys::window().expect("window");
        let id = window
            .set_interval_with_callback_and_timeout_and_arguments_0(tick.as_ref().unchecked_ref(), 1000)
            .unwrap_or(0);
        on_cleanup(move || {
            window.clear_interval_with_handle(id);
            drop(tick);
        });
    }
    p().text_dyn(move || format!("{} s sur cette page", seconds.get()))
}
```

Les nettoyages s'exécutent avant la libération des états du composant : ils peuvent encore les lire. Sur le serveur, `f` ne s'exécute jamais.

## Lire sans suivre : untrack

Dans une région dynamique, chaque lecture d'état abonne **toute la région**. Un composant construit dans la région qui lit directement (hors closure) un de ses propres états serait reconstruit, avec des états neufs, à chaque changement de cet état. Lisez les états dans des closures (`text_dyn`, `children_dyn`…), et entourez d'`untrack` une lecture dont on n'a besoin qu'une fois :

```rust
use active::prelude::*;

let (page, size) = (use_state(1), use_state(20));
let view = ul().children_dyn(move || {
    let size = untrack(|| size.get()); // lu une fois, pas suivi
    (0..size).map(|i| li().text(format!("Élément {}", (page.get() - 1) * size + i + 1)).into()).collect()
});
```

Seul ce que lit la closure de la région décide de sa reconstruction : la construction et le montage de ses nœuds (rappels `on_mount`, composants imbriqués) ne sont déjà pas suivis.

## Résultats asynchrones : is_alive

Une requête réseau peut se terminer après que l'utilisateur a quitté la page. `State::is_alive()` vaut `false` une fois l'état libéré : vérifiez-le avant d'écrire un résultat.

```rust,ignore
fn weather(city: String) -> Element {
    let forecast = use_state(None::<String>);
    wasm_bindgen_futures::spawn_local(async move {
        let text = fetch_forecast(&city).await; // votre propre fonction asynchrone
        if forecast.is_alive() {
            forecast.set(Some(text));
        }
    });
    p().text_dyn(move || forecast.get().unwrap_or_else(|| "Chargement…".into()))
}
```

## Partager un état

Pour partager une valeur entre composants, créez l'état dans leur parent commun et transmettez le handle en argument :

```rust
use active::prelude::*;

fn search_box(query: State<String>) -> Element {
    input().attr("type", "search").on_input(move |e| query.set(e.value()))
}

fn result_count(query: State<String>) -> Element {
    p().text_dyn(move || format!("Searching for “{}”", query.get()))
}

fn search() -> Element {
    let query = use_state(String::new());
    div().child(search_box(query)).child(result_count(query))
}
```

Des [îlots](../islands/) distincts ne partagent pas d'états : chaque îlot est sa propre petite application. Si deux parties d'une page doivent communiquer, placez-les dans le même îlot.

## Côté serveur

Sur le serveur, plus rien ne change une fois la page rendue, et les effets comme les gestionnaires d'événements ne sont même pas stockés. Les états occupent tout de même de la mémoire, alors `active` les libère pour vous :

- les états vivent dans un registre propre au thread courant ;
- à la fin de chaque rendu de premier niveau (`Node::render`, `Node::render_into` ou `Document::render`), les états que ni ce rendu ni le précédent n'ont lus ou créés sont libérés.

Les états de la page en cours de rendu restent disponibles : vous pouvez rendre plusieurs fois la même vue, lire ses états après le rendu ou afficher un `Node` avec `{:?}`. Les états des requêtes terminées sont libérés après les rendus suivants sur le même thread : un serveur qui rend des millions de pages ne garde ainsi qu'une poignée d'états vivants.

Il en découle une règle : **construisez et rendez la vue sans `.await` entre les deux.** Dans un gestionnaire asynchrone, chargez d'abord vos données, puis construisez la vue et rendez-la en une seule étape synchrone. Un `State` appartient au thread qui l'a créé, il n'est donc pas `Send` : dans un serveur multithread comme Axum, en conserver un au-delà d'un `.await` provoque une erreur de compilation, pas une surprise à l'exécution. Voir [Axum, Données asynchrones](../axum/#donnees-asynchrones).

## Portées

Pour aller plus loin : `active::scope(f)` exécute `f` et renvoie son résultat accompagné d'un `Scope` qui possède tous les états et effets créés à l'intérieur. `Scope::dispose()` les libère. Les régions dynamiques s'en servent en interne ; vous n'en avez besoin que si vous montez et retirez des vues à la main dans le navigateur.

```rust
use active::{scope, use_state};

let (count, owner) = scope(|| use_state(1));
assert_eq!(count.get(), 1);
owner.dispose(); // `count` can no longer be used
```

Sur le serveur, un rendu effectué à l'intérieur d'un `scope` ouvert ne libère aucun état : libérez la portée vous-même.

## Erreurs courantes

### Lire un état dans sa propre mise à jour

`count.update(|n| *n = count.get() + 1)` provoque une panique avec le message « a state was read inside its own `update`; read it before updating ». Utilisez l'argument de la closure, ou lisez d'abord, puis écrivez :

```rust
use active::use_state;

let count = use_state(1);
let step = use_state(10);

count.update(|n| *n += step.get()); // the argument, plus another state: fine
let doubled = count.get() * 2;      // read first…
count.set(doubled);                 // …then write
assert_eq!(count.get(), 22);
```

De même, appeler `set` ou `update` sur un état à l'intérieur de son propre `with` provoque une panique (« a state was updated while being read or updated »).

### Utiliser un état libéré

Un état créé dans une région dynamique est libéré quand la région est reconstruite ; l'utiliser ensuite provoque une panique avec le message « this state was disposed ». Gardez les données durables dans un état créé au-dessus de la région.

### Calculer du HTML dans un effet

Les effets ne s'exécutent pas sur le serveur : tout ce qu'ils produisent est absent du HTML que voient les moteurs de recherche. Calculez-le plutôt dans la vue.
