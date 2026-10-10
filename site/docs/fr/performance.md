---
title: Guide des performances
description: Les leçons des benchmarks pour charger, appeler et alléger Rust/WebAssembly dans React, et pourquoi le SEO d'active-react ne coûte presque rien.
---

Chaque choix de conception d'active-react découle de mesures. Cette page transforme les [benchmarks](../benchmarks/) en conseils pratiques : ce qui rend un module WebAssembly rapide à charger, rapide à appeler et léger, et ce que coûte la partie SEO.

## Chargement : le réseau décide

Les benchmarks ont mesuré le temps écoulé entre la navigation et l'affichage du premier résultat Rust, pour chaque façon de charger un module dans une page rendue côté client :

| Scénario (rendu client) | localhost | RTT de 100 ms |
| --- | ---: | ---: |
| active-react, `.wasm` brut | 28,8 ms | 331,5 ms |
| react-wasm 1.0.1, `.wasm` brut | 28,5 ms | 327,1 ms |
| `useEffect` écrit à la main, `.wasm` brut | 25,8 ms | 328,9 ms |
| active-react + wasm-bindgen | 23,5 ms | 328,4 ms |
| `useEffect` écrit à la main + wasm-bindgen | 24,6 ms | 332,9 ms |

Avec une latence réaliste, ils arrivent tous autour de **330 ms**. Le code de chargement n'a pas d'importance : le temps part dans les allers-retours réseau (le HTML, puis le JavaScript, puis le fichier `.wasm`) et dans la compilation.

Deux choses font donc une vraie différence :

1. **Lancer le téléchargement plus tôt.** Avec le rendu serveur, active-react émet un preload pour le binaire, qui se télécharge alors en parallèle du JavaScript. Même page, même module : **224,2 ms avec preload contre 330,0 ms sans, soit 32 % plus rapide**.
2. **Livrer un binaire plus petit.** Moins d'octets à télécharger et à compiler. Voir [taille du binaire](#taille-du-binaire) plus bas.

> **Astuce :** Donnez l'option `url` à `wasm()` et faites le rendu sur le serveur ou au moment du build. Vous obtenez le preload gratuitement. Voir [charger WebAssembly](../loading-wasm/#prechargement).

## Appels : regroupez le travail

Appeler WebAssembly est rapide, mais pas gratuit. Mesures dans Chromium :

| Mesure | JS pur | wasm brut | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)`, par appel | 0,55 ns | 4,05 ns | 4,81 ns |
| `fib(30)`, récursif | 11,4 ms | 2,1 ms | 1,9 ms |
| `greet(str) -> String`, par appel | 4,7 ns | — | 553,0 ns |

Trois leçons :

- **Rust est environ 5× plus rapide sur du vrai travail.** `fib(30)` prend 2,1 ms en WebAssembly contre 11,4 ms en JavaScript.
- **Chaque appel coûte environ 4 ns**, contre 0,6 ns pour une fonction JavaScript. Dix millions de tout petits appels perdent du temps à franchir la frontière.
- **Les chaînes coûtent cher : environ 550 ns par appel.** wasm-bindgen copie et réencode chaque chaîne dans les deux sens.

La règle : **envoyez de gros blocs de travail, pas une multitude de petits appels.** Passez un tableau typé entier en une fois au lieu d'appeler Rust pour chaque élément :

```rust
use wasm_bindgen::prelude::*;

// One call processes the whole dataset.
#[wasm_bindgen]
pub fn smooth(values: &[f64], window: usize) -> Vec<f64> {
    let window = window.max(1);
    values
        .windows(window)
        .map(|w| w.iter().sum::<f64>() / window as f64)
        .collect()
}
```

```ts
// Good: one call, one copy in, one copy out.
const smoothed = rust.smooth(new Float64Array(samples), 5);

// Slow: one boundary crossing per item (scale being a tiny Rust function).
const scaled = samples.map((x) => rust.scale(x, 2));
```

Réservez les chaînes aux extrémités (un titre, un résultat à afficher), pas aux boucles internes.

## Quand Rust en vaut la peine

Bons candidats : boucles lourdes sur des nombres, parsing, compression, traitement d'images et d'audio, simulations, cryptographie, recherche dans de gros volumes de données en mémoire. Une tâche qui prend plusieurs millisecondes en JavaScript et s'exécute en un seul appel est le cas idéal.

Mauvais candidats : petites fonctions appelées très souvent depuis JavaScript, mise en forme de chaînes, manipulation du DOM (WebAssembly ne peut pas toucher au DOM sans passer par JavaScript).

## Taille du binaire

| Élément | Minifié | Gzip |
| --- | ---: | ---: |
| active-react `wasm` + `useWasm` | 1,03 kB | 0,60 kB |
| react-wasm 1.0.1 `useWasm` | 5,27 kB | 2,07 kB |
| Code JS de liaison wasm-bindgen (3 fonctions) | 1,89 kB | 0,93 kB |
| Binaire brut `extern "C"` (add + fib) | 144 B | 139 B |
| Binaire wasm-bindgen (add + fib + greet) | 15,87 kB | 6,76 kB |

Le binaire wasm-bindgen est environ 100 fois plus gros que le binaire brut, et presque tout vient de la fonction `greet` : `format!` et la machinerie de `String` (allocateur, gestion de l'UTF-8, formatage) pèsent environ 15 kB. Les fonctions purement numériques se compilent en quelques dizaines d'octets.

Pour garder des binaires légers :

- Évitez `format!`, `println!` et `to_string()` dans le code livré au navigateur quand vous pouvez renvoyer des nombres à la place.
- Surveillez les dépendances lourdes : `serde_json`, `regex` ou `chrono` ajoutent chacune beaucoup de poids.
- Pour les modules purement numériques, passez-vous complètement de wasm-bindgen : voir [WebAssembly brut](../raw-wasm/).

## Profils Cargo

Même crate, mesuré en taille et en vitesse (`lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true` sur chaque ligne ; `fib(30)` mesuré dans Node) :

| Profil | wasm brut | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15,87 kB | 6,76 kB | 3,76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14,46 kB | 6,37 kB | 3,89 ms |
| `opt-level = "s"` | 144 B | 14,71 kB | 6,51 kB | 4,17 ms |
| `opt-level = "z"` | 137 B | 14,72 kB | 6,52 kB | 4,32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13,58 kB | 6,19 kB | 3,88 ms |

## Recommandations

- **Par défaut, utilisez `opt-level = 3`** avec `lto`, `codegen-units = 1`, `panic = "abort"` et `strip`. C'est le profil le plus rapide, et l'écart de taille avec les autres est faible.
- **Si chaque kilo-octet compte**, utilisez `opt-level = "z"` avec `wasm-opt -Oz` (fourni par binaryen). Le résultat est environ 8 % plus petit une fois compressé en gzip (6,19 kB contre 6,76 kB), pour une vitesse similaire dans ces mesures.
- `opt-level = "s"` ou `"z"` seuls sont plus lents et à peine plus petits : ils n'en valent pas la peine à eux seuls.
- **Préchargez** le binaire grâce au rendu serveur et à l'option `url`.
- **Regroupez** les appels et évitez les chaînes dans les chemins critiques.

## Le SEO ne coûte presque rien

La partie SEO a été mesurée, sur la même page, face à des balises écrites à la main (React 19 sans bibliothèque) et aux alternatives populaires :

- **Rendu serveur :** 42,4 µs par page, contre 39,7 µs pour des balises écrites à la main, soit +7 %. C'est **1,5× plus rapide que react-helmet** (66,5 µs). Avec `renderWithHead` et un template HTML : 44,1 µs.
- **Mise à jour du head lors de la navigation :** une médiane de **0,3 ms** entre le rendu d'une nouvelle page et un `<head>` à jour, comme avec des balises écrites à la main.
- **Taille du bundle :** 0,74 kB gzip, contre 6,07 à 6,76 kB pour @unhead/react, react-helmet-async et react-helmet.

La raison est simple : React 19 fait le travail. `<Seo>` rend des balises et rien d'autre : ni contexte, ni effet, ni seconde passe sur l'arbre.
