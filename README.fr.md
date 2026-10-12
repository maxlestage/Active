[![Active : front-end, vrai SEO, tout en Rust. Un îlot compteur écrit en Rust](https://raw.githubusercontent.com/maxlestage/Active/master/site/static/og.png)](https://maxlestage.github.io/Active/)

# Active

[English](README.md) · **Français** · [Español](README.es.md)

**Une bibliothèque front-end simple et rapide pour Rust : composants, état, routage et balises SEO complètes, rendus sur le serveur et exécutés dans le navigateur en WebAssembly.**

```rust
use active::prelude::*;

fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || format!("Clicked {} times", count.get()))
        .into()
}

fn page() -> String {
    let body = div()
        .child(Seo::new().title("Home").description("My site, in Rust."))
        .child(h1().text("Hello"))
        .child(island("counter", "0", counter)); // the only part sent to the browser
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}
```

Le serveur envoie un HTML terminé, avec toutes les balises SEO. Dans le navigateur, la même fonction `counter`, compilée en WebAssembly, prend le relais sur le bouton déjà affiché. Aucun JavaScript à écrire, pas de DOM virtuel, pas de second rendu du contenu statique.

Les mêmes composants servent aussi à créer des **applications côté client** : `active::mount_to("#app", app())` rend toute l'application dans le navigateur, `active::location()` et `active::navigate()` assurent le routage sans rechargement, et le `Seo` de chaque page met à jour le titre et les balises meta ([`examples/spa`](examples/spa)). Vous pouvez aussi les écrire dans des fichiers pour obtenir un **site statique**.

📚 **Documentation en anglais, en français et en espagnol** : le site web dans [`site/`](site), lui-même construit avec `active`. Lancez-le avec `cargo run -p active-site` ou déployez votre propre copie sur Heroku depuis un téléphone (voir plus bas). Les pages se lisent aussi sur GitHub dans [`site/docs/`](site/docs).

## Pourquoi

- **Simple.** Les composants sont des fonctions Rust qui renvoient une vue : les idées des hooks React, sans le JavaScript. L'état passe par `use_state`, un handle `Copy` que vous déplacez dans des closures. Aucune macro à apprendre, aucun plugin de build.
- **Le SEO d'abord.** Le serveur rend toujours la page entière. `Seo` couvre le titre, la description, robots, l'URL canonique, hreflang, Open Graph, Twitter et JSON-LD ; placez-en un dans une mise en page, une page et un composant : ils sont fusionnés sans aucune balise en double.
- **Rapide.** Le rendu côté serveur écrit directement dans une `String`. Dans le navigateur, seuls les îlots sont hydratés, avec des liaisons à granularité fine : un changement d'état met à jour exactement le texte, l'attribut ou la classe qui en dépend. Voir les [benchmarks](#benchmarks).
- **Léger.** La version serveur n'a aucune dépendance. Le bundle du navigateur utilise `wasm-bindgen` et `web-sys`, rien d'autre.
- **Installable.** `active::pwa` écrit le manifeste d'application web et un service worker hors ligne, et `Document::manifest` / `Document::service_worker` ajoutent leurs balises : votre site s'installe sur l'écran d'accueil et fonctionne sans réseau, sans une ligne de JavaScript. Le site web du projet est une PWA ([guide](site/docs/fr/pwa.md)).

## Benchmarks

Mesuré face aux principaux frameworks Rust, sur les mêmes pages, le tout en Rust ([méthode et données brutes](bench/RESULTS.md)) :

| | Rendu serveur d'une page produit | WebAssembly + glue pour un compteur (gzip) | Interactif avec 100 ms de latence |
| --- | ---: | ---: | ---: |
| **active** | **5,6 µs** | **36,8 ko** | **224 ms** |
| Leptos 0.8 + leptos_meta | 13,4 µs | 60,9 ko | 227 ms |
| Leptos 0.8, `<head>` écrit à la main | 5,2 µs | — | — |
| Sycamore 0.9 | 33,0 µs | 45,8 ko | 227 ms |
| Yew 0.23 | 17,2 µs | 96,3 ko | 228 ms |
| Dioxus 0.7 | 24,2 µs | 170,4 ko | 339 ms |
| maud 0.27 (template seul) | 1,4 µs | — | — |

Le plus petit bundle de tous, et un rendu côté serveur 2,4 fois plus rapide que celui de l'autre framework qui fusionne les balises SEO venant des composants (Leptos avec leptos_meta). Intel Xeon 2,8 GHz, Rust 1.97, Chromium 141 ; lancez-les vous-même avec `cd bench && cargo run --release -p runner`.

## Installation

```sh
cargo add active --git https://github.com/maxlestage/Active                      # server
cargo add active --git https://github.com/maxlestage/Active --features client    # browser bundle, for wasm32-unknown-unknown
```

Deux applications complètes : [`examples/hello`](examples/hello), une page serveur avec un îlot (un seul crate pour les deux côtés), et [`examples/spa`](examples/spa), une liste de tâches côté client avec routage. Pour lancer la première :

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello   # http://localhost:3000
```

## Contenu du dépôt

| Dossier | Contenu |
| --- | --- |
| [`crates/active`](crates/active) | Le crate `active` : vues, état, SEO, documents, îlots, hydratation |
| [`examples/hello`](examples/hello) | La plus petite application : une page et un îlot compteur, serveur sans autre dépendance que std |
| [`examples/spa`](examples/spa) | Une application côté client : liste de tâches, routage, formulaires, `localStorage` |
| [`site`](site) | Site de présentation et documentation (en/fr/es) : serveur Vitesse, animations WebAssembly, recherche et bac à sable interactif, installable et hors ligne (PWA) |
| [`bench`](bench) | Benchmarks face à Leptos, Dioxus, Yew, Sycamore, maud et à du HTML écrit à la main |

## Publier le site sur GitHub Pages (gratuit)

1. Sur GitHub, ouvrez **Settings → Pages** et réglez **Source** sur **GitHub Actions** (une seule fois ; l'application GitHub ou le site mobile suffisent).
2. C'est tout. Après chaque CI verte, qui fusionne la branche dans `master`, le workflow [`pages.yml`](.github/workflows/pages.yml) exporte le site en fichiers statiques avec `active-site export` et le publie à l'adresse `https://maxlestage.github.io/Active/`. Pour publier tout de suite : **Actions → Pages → Run workflow**.

Guide : [`site/docs/fr/deploy-pages.md`](site/docs/fr/deploy-pages.md).

## Déployer le site sur Heroku depuis un téléphone

1. Ouvrez `https://heroku.com/deploy?template=https://github.com/maxlestage/Active` dans le navigateur de votre téléphone, choisissez un nom et touchez **Deploy app**. Heroku lit [`app.json`](app.json) et construit le [`Dockerfile`](Dockerfile) (voir [`heroku.yml`](heroku.yml)) : le bundle WebAssembly, puis le serveur, entièrement à partir des sources Rust.
2. Pour redéployer à chaque changement : dans l'onglet **Deploy** de l'application, connectez le dépôt et activez **Enable Automatic Deploys** depuis `master`.

Heroku n'a pas d'offre gratuite : un dyno Eco coûte environ 5 $ par mois. Guide pas à pas : [`site/docs/fr/deploy-heroku.md`](site/docs/fr/deploy-heroku.md).

## Publier sur crates.io depuis un téléphone

1. Créez un jeton sur [crates.io/settings/tokens](https://crates.io/settings/tokens) et ajoutez-le comme secret du dépôt, sous le nom `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Ouvrez **Releases → Draft a new release**, créez un tag comme `v0.1.0`, puis touchez **Publish release**.

Le workflow [`release.yml`](.github/workflows/release.yml) définit la version du crate à partir du tag, lance les tests et publie `active`. Guide : [`site/docs/fr/publishing.md`](site/docs/fr/publishing.md).

## Développement

```sh
cargo test --workspace                     # library, site and examples
cargo run -p active --example page         # print a complete page
cargo run -p active-site                   # the website on http://localhost:3000
```

Le bundle navigateur du site web se compile comme l'exemple ci-dessus (`-p active-site`, sortie dans `site/pkg`) ; voir le [`Dockerfile`](Dockerfile). Chaque branche qui passe la CI est automatiquement fusionnée dans `master`, sauf les branches nommées `wip/…` ([`ci.yml`](.github/workflows/ci.yml)) ; `master` est ensuite publié sur GitHub Pages ([`pages.yml`](.github/workflows/pages.yml)).

## Licence

Active est un logiciel propriétaire : © 2026 Maxime Nathan Lestage, tous droits réservés. Ce n'est pas un logiciel open source. Le code source est publié sur crates.io et GitHub pour pouvoir être lu, audité et compilé, et vous pouvez l'utiliser librement comme dépendance de vos propres programmes, y compris commerciaux. Le modifier ou redistribuer son code source demande l'accord écrit de l'auteur. Conditions complètes : [LICENSE](LICENSE).

## Crédits

Conçu et développé par Maxime Nathan Lestage, au sein d'une même famille de projets Rust : [Active](https://maxlestage.github.io/Active/), [Vitesse](https://maxlestage.github.io/Vitesse/), [Securite](https://maxlestage.github.io/Securite/), [Memoire](https://maxlestage.github.io/Memoire/), [Paquets](https://maxlestage.github.io/Paquets/), [FerroSwift](https://maxlestage.github.io/FerroSwift/) et [Solidify](https://maxlestage.github.io/Solidify/).
