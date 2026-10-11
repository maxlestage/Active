---
title: Benchmarks
description: active comparé à Leptos, Dioxus, Yew, Sycamore, maud et à du HTML écrit à la main : rendu côté serveur, taille du WebAssembly et délai avant interactivité.
---

`active` a été comparé aux principaux frameworks web Rust sur trois questions : à quelle vitesse le serveur rend-il une page complète, combien de WebAssembly le navigateur télécharge-t-il pour un petit composant interactif, et au bout de combien de temps ce composant est-il utilisable. Tout est en Rust : les pages, les serveurs et le harnais qui pilote Chromium. L'ensemble des benchmarks se trouve dans `bench/`.

## Résumé

| | active | Meilleur des autres |
| --- | ---: | ---: |
| Rendu serveur d'une page produit | 5,58 µs | 5,19 µs (Leptos, `<head>` écrit à la main), 13,44 µs avec leptos_meta |
| WebAssembly + glue pour un compteur (gzip) | **36,8 ko** | 45,8 ko (Sycamore) |
| Interactif, latence réseau de 100 ms | **224,2 ms** | 226,7 ms (Sycamore) |

- **Le plus petit bundle.** L'îlot compteur pèse 36,8 ko gzip au total : 20 % de moins que Sycamore, 40 % de moins que Leptos et 4,6 fois moins que Dioxus.
- **Un rendu serveur rapide avec un vrai SEO.** Parmi les frameworks qui fusionnent dans le `<head>` les balises SEO provenant des composants (le `Document` d'`active`, Leptos avec leptos_meta), `active` est 2,4 fois plus rapide. Seul Leptos, quand tout le `<head>` est écrit à la main dans un unique template, est légèrement plus rapide (7 %).
- **Interactif dès que le bundle s'exécute.** Les quatre frameworks qui hydratent sont à quelques millisecondes les uns des autres ; le téléchargement domine, si bien qu'un bundle plus petit aide sur les réseaux lents.

> **Remarque :** ces chiffres proviennent d'une seule machine (Intel Xeon 2,8 GHz, 4 cœurs, Rust 1.97, Chromium 141). Comparez les frameworks entre eux, pas avec votre propre matériel.

## Rendu côté serveur

Chaque implémentation rend le même document complet pour la page produit `i`, `i` changeant à chaque itération : `<html lang="en">`, un `<head>` avec 11 balises SEO (titre avec modèle, description, URL canonique, Open Graph, Twitter) et un script JSON-LD, puis un `<body>` avec un `<h1>` et une liste de 30 éléments. Chaque sortie est vérifiée balise par balise avant la mesure. Chaque implémentation s'exécute dans son propre processus, effectue un préchauffage de 1 s, puis rend des pages pendant 3 s par lots d'environ 10 ms ; le tableau donne la médiane de trois exécutions.

| Implémentation | µs / page | Pages / s | Par rapport au code écrit à la main |
| --- | ---: | ---: | ---: |
| maud 0.27 | 1,37 | 729 288 | ×0,68 |
| Écrit à la main (`push_str`/`write!`) | 2,01 | 497 191 | ×1,00 |
| Leptos 0.8, `<head>` écrit à la main | 5,19 | 192 686 | ×2,58 |
| **active** | **5,58** | **179 083** | **×2,78** |
| Leptos 0.8 + leptos_meta | 13,44 | 74 406 | ×6,68 |
| Yew 0.23 | 17,17 | 58 257 | ×8,53 |
| Dioxus 0.7 | 24,22 | 41 281 | ×12,04 |
| Sycamore 0.9 | 33,01 | 30 298 | ×16,41 |

maud et la page écrite à la main sont des templates : ils ne font qu'écrire des chaînes. `active` construit un arbre de composants, récupère le `Seo` de chaque composant et les fusionne dans le `<head>`, ce qui correspond au travail dont un vrai site a besoin. Il écrit directement dans une seule `String`, sans DOM virtuel, et la plupart des éléments ne demandent aucune allocation en dehors de leur texte.

## Taille du WebAssembly

Le même compteur dans chaque framework : un bouton qui incrémente le nombre affiché à côté de lui, rendu par le serveur puis repris par le navigateur. Compilé pour `wasm32-unknown-unknown` avec le profil optimisé pour la taille (`opt-level = "s"`, `lto`, `panic = "abort"`), puis passé à `wasm-bindgen --target web --no-typescript`, sans `wasm-opt`. Le total inclut la glue JavaScript générée par wasm-bindgen.

| Framework | Mode dans le navigateur | .wasm | .wasm gzip | Glue gzip | Total gzip |
| --- | --- | ---: | ---: | ---: | ---: |
| **active** | hydratation d'îlot | 81,6 ko | 32,4 ko | 4,3 ko | **36,8 ko** |
| Sycamore | hydratation | 105,8 ko | 41,1 ko | 4,7 ko | 45,8 ko |
| Leptos | hydratation | 144,9 ko | 54,8 ko | 6,2 ko | 60,9 ko |
| Yew | hydratation | 241,8 ko | 90,0 ko | 6,3 ko | 96,3 ko |
| Dioxus | rendu côté client | 426,9 ko | 155,4 ko | 15,0 ko | 170,4 ko |

> **Astuce :** le compteur du benchmark enregistre des durées avec un nombre à virgule flottante, ce qui ajoute le formatage des flottants de Rust à chaque bundle. Sans cela, l'exemple `hello` pèse environ 22 ko gzip. Voir [Performances](../performance/) pour la taille de chaque type d'application et les moyens de réduire encore un bundle.

## Délai avant interactivité

Chromium en mode headless charge chaque page de compteur 21 fois par profil réseau, avec le cache HTTP désactivé, en alternant entre les frameworks. Chaque application écrit `performance.now()` dans la page quand son WebAssembly démarre et quand le bouton devient interactif ; le harnais lit ces repères au moyen du protocole DevTools, puis clique réellement sur le bouton (le nombre doit passer de 3 à 4).

| Framework | Localhost : médiane (min–max) | Latence de 100 ms : médiane (min–max) | Démarrage → interactif |
| --- | ---: | ---: | ---: |
| Yew | 25,3 ms (21–38) | 227,8 ms (225–239) | 0,8 ms |
| **active** | 25,5 ms (21–42) | **224,2 ms (221–232)** | 0,6 ms |
| Leptos | 25,5 ms (22–38) | 227,2 ms (222–239) | 0,5 ms |
| Sycamore | 25,7 ms (21–33) | 226,7 ms (222–392) | 0,5 ms |
| Dioxus | 41,3 ms (32–58) | 339,3 ms (332–351) | 1,0 ms |

Les plages de mesure des frameworks qui hydratent se chevauchent : considérez-les comme à égalité. Dioxus fait le rendu côté client (son hydratation exige son propre serveur full-stack) et sa glue importe des fichiers supplémentaires, ce qui coûte un aller-retour réseau de plus.

## Lancer les benchmarks vous-même

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129
cd bench
cargo run --release -p runner                    # everything, about 3 minutes after the first build
cargo run --release -p runner -- ssr             # one part: ssr, wasm, browser or report
BENCH_PRINT=1 cargo run --release -p ssr-active  # print the page an implementation renders
```

Le runner trouve Chromium dans le `PATH` (ou définissez `CHROME=/path/to/chrome`). Il écrit `bench/RESULTS.md` et les données brutes dans `bench/results/`. La méthode, la version de chaque framework et les choix faits pour chacun sont décrits dans `bench/README.md`.
