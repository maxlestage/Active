---
title: Îlots
description: Rendez interactives certaines parties d'une page rendue côté serveur avec des îlots WebAssembly : déclaration, props, hydratation et incohérences à éviter.
---

Un îlot est une partie interactive d'une page rendue côté serveur : un compteur, un champ de recherche, un configurateur de produit. Le serveur le rend en HTML comme le reste de la page et le marque. Dans le navigateur, le bundle WebAssembly trouve les parties marquées et rend uniquement celles-ci interactives. Tout ce qui se trouve hors des îlots reste du HTML simple et ne coûte rien.

## Déclarer un îlot

Un composant îlot est une fonction qui reçoit ses props sous forme de chaîne et renvoie un `Node` :

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

pub fn page() -> Element {
    div()
        .child(h1().text("Hello"))
        .child(island("counter", "5", counter))
}

assert_eq!(
    Node::from(page()).render(),
    r#"<div><h1>Hello</h1><active-island data-island="counter" data-props="5"><button><!--#-->5</button></active-island></div>"#
);
```

`island(name, props, component)` appelle le composant avec les props, rend sa vue à l'intérieur d'un élément `<active-island>` et enregistre le nom et les props dans les attributs `data-island` et `data-props`.

Le type du composant est `active::IslandFn`, c'est-à-dire `fn(&str) -> Node` : une simple fonction (ou une closure qui ne capture rien). Tout ce dont l'îlot a besoin doit provenir de ses props.

## Enregistrer les îlots dans le navigateur

Le bundle du navigateur appelle `active::hydrate` une seule fois, avec tous les îlots qu'utilisent les pages, sous les mêmes noms :

```rust
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button().on_click(move |_| count.update(|n| *n += 1)).text_dyn(move || count.get().to_string()).into()
}

pub fn search(_props: &str) -> Node {
    input().attr("type", "search").into()
}

/// Tous les îlots, par nom : utilisés par les pages du serveur et par le bundle du navigateur.
pub const ISLANDS: [(&str, active::IslandFn); 2] = [("counter", counter), ("search", search)];

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&ISLANDS);
}
```

`#[wasm_bindgen(start)]` fait s'exécuter `start` dès que le module est chargé. La page [Compiler le client](../client-build/) montre comment le bundle est compilé et chargé ; [`Document::client`](../documents/#charger-le-bundle-du-navigateur) écrit le code de chargement dans la page.

Un îlot dont le nom n'est pas enregistré reste du HTML statique, sans la moindre erreur. Vérifiez l'orthographe des noms si un îlot ne réagit pas.

## Props

Les props voyagent dans le HTML sous forme de chaîne, échappée dans l'attribut `data-props`. Utilisez le format que vous savez relire :

- un nombre ou un mot : `island("counter", "5", counter)`, `island("search", "en", search)` ;
- du JSON pour des données structurées, avec [`serde_json`](https://docs.rs/serde_json).

```rust
use active::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct CartProps {
    pub product: String,
    pub price_cents: u32,
}

pub fn add_to_cart(props: &str) -> Node {
    let props: CartProps = serde_json::from_str(props).unwrap_or_default();
    let price = props.price_cents;
    let quantity = use_state(1u32);
    div()
        .class("cart")
        .child(strong().text(props.product))
        .child(
            input()
                .attr("type", "number")
                .attr("min", "1")
                .attr_dyn("value", move || quantity.get().to_string())
                .on_input(move |e| quantity.set(e.value().parse().unwrap_or(1))),
        )
        .child(span().text_dyn(move || {
            let total = quantity.get() * price;
            format!("{}.{:02} €", total / 100, total % 100)
        }))
        .into()
}

fn product_page() -> Element {
    let props = serde_json::to_string(&CartProps { product: "Oak chair".into(), price_cents: 24900 })
        .unwrap();
    section().child(h1().text("Oak chair")).child(island("add-to-cart", props, add_to_cart))
}
```

Gardez des props légères : elles font partie du HTML de la page.

## Ce que fait l'hydratation

Quand `hydrate` s'exécute, pour chaque `<active-island>` de la page qui n'est pas encore hydraté :

1. Elle lit le nom et trouve le composant enregistré. Les noms inconnus sont ignorés.
2. Elle appelle le composant avec les props, dans le navigateur. Cela crée les états de l'îlot.
3. Elle parcourt le HTML déjà présent dans la page en parallèle de la vue, et **récupère** les éléments et nœuds texte existants au lieu d'en créer de nouveaux.
4. Elle attache les écouteurs d'événements, lie le texte, les attributs et les classes dynamiques à leurs états, et exécute les callbacks `on_mount`.
5. Les régions dynamiques (`children_dyn`, `fragment_dyn`) sont reconstruites à partir de l'état du navigateur.
6. Elle marque l'élément avec `data-hydrated` : appeler de nouveau `hydrate` ne fait donc rien.

Le visiteur voit déjà le contenu de l'îlot avant le chargement du bundle ; l'hydratation ne fait que le rendre réactif. Le `<head>` écrit par le serveur n'est jamais modifié par les îlots.

### L'élément îlot

`<active-island>` est un élément personnalisé. `Document` ajoute le style `active-island{display:contents}` à chaque page, si bien que le conteneur n'affecte pas votre mise en page : ses enfants se comportent comme s'ils étaient directement dans le parent. Si vous rendez des pages sans `Document`, ajoutez cette règle à votre CSS.

## Incohérences d'hydratation

L'hydratation s'attend à ce que le HTML corresponde à la vue que le composant construit dans le navigateur. Si ce n'est pas le cas (un élément là où un texte était attendu, une balise différente), `active` affiche un avertissement dans la console du navigateur :

```text
active: re-rendering island (tag mismatch)
```

et reconstruit cet îlot de zéro. La page fonctionne toujours, mais l'îlot clignote et le travail du serveur est perdu. Causes courantes :

- **Des entrées différentes de chaque côté.** La vue dépend de quelque chose dont le navigateur ne dispose pas : l'heure courante, un nombre aléatoire, une valeur lue dans une base de données. Mettez dans ses props tout ce dont l'îlot a besoin.
- **Du code qui varie selon la cible.** Un `#[cfg(target_arch = "wasm32")]` qui modifie le balisage. Réservez-le au comportement (gestionnaires, effets), jamais à ce qui est rendu.
- **Du HTML modifié après le rendu.** Un proxy, une extension de navigateur ou un script qui réécrit la page.

Le contenu texte n'est pas comparé, seulement la structure. Le HTML brut défini avec `.html(...)` n'est jamais touché par l'hydratation, ce qui est utile pour du contenu que seul le serveur peut produire, comme du code avec coloration syntaxique : rendez-le sur le serveur et passez une chaîne vide dans le navigateur. Les onglets de la page d'accueil de ce site fonctionnent ainsi (voir `site/src/islands.rs`).

## Des îlots dans des îlots

Un îlot peut contenir d'autres îlots. L'îlot extérieur hydrate les îlots intérieurs en même temps que lui-même, en utilisant les vues intérieures construites par son propre code : ils ne sont donc hydratés qu'une fois.

## Le SEO dans les îlots

Un `Seo` placé dans la vue d'un îlot est fusionné dans le `<head>` de la page sur le serveur, comme n'importe quel autre (voir [SEO](../seo/#fusion)). Dans le navigateur, les îlots ne modifient jamais le `<head>`.

## Les îlots sont indépendants

Chaque îlot a ses propres états, créés lors de son hydratation, et vit aussi longtemps que la page. Deux îlots ne peuvent pas partager un état. Si deux parties de la page doivent réagir l'une à l'autre (un filtre et une liste), faites-en un seul îlot. Pour des données qui doivent survivre à un rechargement de page, utilisez le stockage du navigateur depuis un [effet](../state/#effets).

## Îlots ou application côté client ?

| | Îlots | [Application côté client](../client-app/) |
| --- | --- | --- |
| HTML envoyé par le serveur | des pages complètes | une coquille vide |
| SEO | complet, sur chaque page | limité à ce que fournissent la coquille et le navigateur |
| Code dans le navigateur | uniquement les parties interactives | toute l'interface |
| Navigation | chargements de page classiques | dans le navigateur, sans rechargement |
| Idéal pour | sites de contenu, boutiques, documentation | outils et tableaux de bord derrière une connexion |

Les deux utilisent les mêmes composants, états et événements : passer un composant de l'un à l'autre ne demande aucune réécriture.
