---
title: Introduction
description: active est une petite bibliothèque front-end en Rust : des composants-fonctions, rendus en HTML avec toutes les balises SEO, et du WebAssembly là où il sert.
---

`active` est une petite bibliothèque Rust pour construire le front-end d'un site web. Vous écrivez vos composants comme de simples fonctions Rust, vous gardez l'état dans des handles `use_state`, vous décrivez chaque page aux moteurs de recherche avec `Seo`, et vous compilez les parties interactives en WebAssembly. Le même code s'exécute sur le serveur et dans le navigateur, et vous n'écrivez jamais de JavaScript.

## Premier aperçu

Un composant est une fonction qui renvoie une vue. Les vues se construisent avec une fonction par balise HTML et des méthodes chaînées :

```rust
use active::prelude::*;

/// Une carte produit : du HTML simple, rendu sur le serveur.
fn card(name: &str, price: u32) -> Element {
    article()
        .class("card")
        .child(h2().text(name.to_string()))
        .child(p().class("price").text(format!("{price} €")))
}

/// Un compteur : rendu sur le serveur, interactif dans le navigateur.
fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

fn page() -> String {
    let body = div()
        .child(Seo::new().title("Oak chair").description("Solid oak, built to last."))
        .child(card("Oak chair", 249))
        .child(island("counter", "0", counter));
    Document::new(body).lang("en").render()
}
```

`page()` renvoie un document HTML complet : le doctype, un `<head>` avec le titre, la description et les balises Open Graph, puis le corps de la page. La carte est du HTML statique. Le compteur est un *îlot* : son HTML est lui aussi rendu sur le serveur, et une fois le bundle WebAssembly chargé, la même fonction `counter` prend le relais sur le bouton existant.

## Trois façons de livrer

Les mêmes composants fonctionnent dans trois types de projets. Choisissez celui qui vous convient, ou combinez-les.

| Mode | Fonctionnement | Idéal pour |
| --- | --- | --- |
| Pages rendues côté serveur avec des îlots | Un serveur Rust (Vitesse ou un autre) rend chaque requête avec `Document::render()`. Le navigateur n'hydrate que les îlots. | Sites avec des données dynamiques, une connexion, une recherche, de l'e-commerce |
| Site statique | Un petit programme rend chaque page dans un fichier HTML au moment de la compilation. Les îlots sont toujours hydratés. | Documentation, blogs, pages de présentation, portfolios |
| Application côté client | Toute l'interface est montée dans le navigateur depuis le WebAssembly. | Tableaux de bord et outils derrière une connexion, là où le SEO ne compte pas |

Consultez [Vitesse](../vitesse/), [Sites statiques](../static-site/) et [Applications côté client](../client-app/) pour chacun d'eux.

## Pourquoi le rendu côté serveur passe en premier

La plupart des sites web sont faits surtout de texte et d'images, avec quelques parties interactives. `active` est conçu autour de ce constat :

- **Les moteurs de recherche et les aperçus de liens lisent du HTML.** Chaque page arrive complète depuis le serveur, avec son titre, sa description, son URL canonique, ses liens `hreflang`, ses balises Open Graph et son JSON-LD. Rien ne dépend d'un robot d'indexation qui exécuterait votre code.
- **Le contenu s'affiche avant l'exécution du moindre code.** Les visiteurs voient la page dès que le HTML arrive. Le bundle WebAssembly se télécharge en parallèle et ne fait que réveiller les îlots.
- **Les parties statiques ne coûtent rien dans le navigateur.** Un paragraphe rendu sur le serveur n'est jamais rendu une seconde fois, et aucun code n'est envoyé pour lui.

Quand une page n'a besoin d'aucune interactivité, elle n'envoie aucun WebAssembly.

## Choix de conception

- **Les composants sont des fonctions.** Ni macro ni syntaxe spéciale : rust-analyzer, rustfmt et les messages d'erreur du compilateur fonctionnent comme d'habitude.
- **Un état à granularité fine.** `use_state` renvoie un petit handle `Copy` que vous déplacez dans des closures. Quand un état change, seuls le texte, l'attribut ou la classe qui le lisent sont mis à jour. Il n'y a ni DOM virtuel ni nouveau rendu du composant.
- **Un seul langage.** Composants, serveur, compilation et tests sont en Rust. Le seul JavaScript que reçoit le navigateur est le chargeur généré par `wasm-bindgen` et une ligne `<script type="module">` écrite par `Document::client`.
- **Léger.** La version serveur n'a aucune dépendance. La fonctionnalité `client` ajoute `wasm-bindgen` et `web-sys`. Une page avec un îlot compteur charge environ 22 ko de WebAssembly compressé avec gzip (voir [Performances](../performance/)).

L'API reprend des idées familières (les composants sous forme de fonctions, `use_state`, les effets), mais `active` est une bibliothèque Rust de bout en bout.

## Comment les pièces s'assemblent

```text
              votre crate : src/lib.rs (composants et îlots)
                  /                                              \
     compilation native (serveur ou générateur)    compilation wasm32 (fonctionnalité "client")
     Document::new(view).render() -> HTML          active::hydrate(&[("counter", counter)])
                  \                                              /
        navigateur : le HTML arrive en premier, puis les îlots se réveillent
```

Un seul crate contient vos composants. Il est compilé deux fois : une fois pour le serveur (ou le générateur de site statique), une fois en WebAssembly pour le navigateur. La page [Installation](../installation/) montre le `Cargo.toml` qui rend cela possible.

## Ce que `active` ne fait pas

`active` reste volontairement minimaliste : pas de serveur web, pas de couche d'accès aux bases de données, pas de bundler, pas de solution CSS. Utilisez le serveur Rust de votre choix ([Vitesse](../vitesse/) est présenté dans cette documentation), de simples fichiers CSS, et `cargo` avec `wasm-bindgen` pour compiler.

## Prochaines étapes

1. [Installez](../installation/) la chaîne d'outils et le crate.
2. Suivez le [Démarrage rapide](../quick-start/) pour lancer une page avec un îlot en cinq minutes.
3. Découvrez les [composants](../components/), l'[état](../state/) et les [événements](../events/).
4. Rendez vos pages faciles à référencer grâce au [SEO](../seo/).
