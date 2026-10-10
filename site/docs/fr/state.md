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
- **Pas de boucle infinie.** Un effet qui écrit un état qu'il lit ne se redéclenche pas lui-même.
- **Libérés avec leur région.** Les effets créés dans une région dynamique s'arrêtent quand la région est reconstruite.

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
