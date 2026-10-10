---
title: Benchmarks
description: Comparaison d'active-react avec react-helmet, react-head, @unhead/react et react-wasm : taille, vitesse du SSR, comportement navigateur et chargement WASM.
---

Chaque choix de conception d'active-react a été vérifié par des mesures. Cette page présente la méthode, l'ensemble des résultats et la façon de les reproduire sur votre machine. Pour les enseignements pratiques, lisez le [guide des performances](../performance/).

## Méthodologie

**Environnement :** Intel(R) Xeon(R) Processor @ 2,80 GHz (4 cœurs), Node v22.22.0, Chromium 141.0.7390.37, React 19.3.0.

**La même page pour chaque bibliothèque**, construite en mode production : un `<title>` avec template, une description, une URL canonique, 6 balises Open Graph, 2 balises Twitter, un `Product` JSON-LD et un body de 30 éléments.

**Bibliothèques SEO comparées :** React 19 avec des balises écrites à la main et sans bibliothèque (la référence), active-react, react-helmet 6.1.0, react-helmet-async 3.0.0, @unhead/react 3.4.2 et react-head 3.4.2.

- **Taille :** uniquement les imports réellement utilisés par chaque page, bundlés et minifiés avec esbuild, sans React ni React DOM. Compression gzip (niveau 9) et Brotli (qualité 11).
- **Rendu serveur :** le document HTML complet (head et body). Chaque bibliothèque tourne dans **son propre processus Node**, pour que l'état du JIT ne déborde jamais de l'une à l'autre. Après une phase de chauffe, les rendus sont chronométrés par lots pendant environ 2,5 secondes ; le résultat est la médiane de 3 processus.
- **Navigateur :** Chromium headless, builds de production, une page neuve par bibliothèque. Après les mesures, le `<head>` final est audité : pas de doublons, valeurs correctes.

**WASM :** deux crates Rust avec les mêmes fonctions, l'un avec de simples exports `extern "C"` (brut), l'autre avec wasm-bindgen, compilés avec chaque profil Cargo.

- **Chargement :** temps entre le début de la navigation et le premier résultat Rust affiché à l'écran, en localhost et avec **100 ms de latence aller-retour émulée via le Chrome DevTools Protocol** (réaliste pour les réseaux mobiles). Médiane de 15 chargements en localhost et de 7 avec latence.
- **Coût des appels :** 10 millions d'appels à `add` et 300 000 appels à `greet` par variante, chaque variante dans sa propre boucle pour que les sites d'appel restent monomorphes ; médiane de 7 exécutions après une phase de chauffe.

## Résultats SEO

### Taille du bundle client

JavaScript ajouté au bundle pour les imports réellement utilisés (minifié, sans React ni React DOM).

| Bibliothèque | min | gzip | brotli |
| --- | ---: | ---: | ---: |
| React 19 natif (sans bibliothèque) | 0 | 0 | 0 |
| **active-react** | 1,53 kB | 0,74 kB | 0,62 kB |
| react-helmet-async 3.0.0 | 17,30 kB | 6,29 kB | 5,59 kB |
| @unhead/react 3.4.2 | 15,17 kB | 6,07 kB | 5,55 kB |
| react-head 3.4.2 | 3,20 kB | 1,35 kB | 1,21 kB |
| react-helmet 6.1.0 | 18,46 kB | 6,76 kB | 5,99 kB |

### Rendu serveur (Node)

Temps nécessaire pour produire le document HTML complet (head et body), médiane de 3 processus d'environ 2,5 s chacun.

| Bibliothèque | µs / page | pages / s | par rapport à React natif |
| --- | ---: | ---: | ---: |
| React 19 natif (sans bibliothèque) | 39,7 | 25 199 | 0 % |
| **active-react** | 42,4 | 23 565 | +7 % |
| **active-react** (`renderWithHead`, template HTML) | 44,1 | 22 665 | +11 % |
| @unhead/react 3.4.2 | 44,9 | 22 260 | +13 % |
| react-head 3.4.2 | 60,7 | 16 474 | +53 % |
| react-helmet-async 3.0.0 | 64,9 | 15 405 | +64 % |
| react-helmet 6.1.0 | 66,5 | 15 031 | +68 % |

### Navigateur (Chromium headless)

- **Head prêt :** du début de la navigation jusqu'à ce que le titre, la description et l'URL canonique soient corrects (médiane de 15 chargements).
- **Latence :** entre le rendu d'une nouvelle page (changement de route) et un `<head>` à jour (300 navigations).
- **Rafale :** 1 000 changements de route d'affilée, jusqu'à ce que le `<head>` final soit correct.

| Bibliothèque | Head prêt | Latence médiane | Latence p95 | Rafale (nav/s) | `<head>` final |
| --- | ---: | ---: | ---: | ---: | ---: |
| React 19 natif (sans bibliothèque) | 20,9 ms | 0,3 ms | 0,5 ms | 2 267 | Correct |
| **active-react** | 23,8 ms | 0,3 ms | 0,6 ms | 2 144 | Correct |
| react-helmet-async 3.0.0 | 22,7 ms | 0,3 ms | 0,5 ms | 2 402 | Correct |
| @unhead/react 3.4.2 | 24,2 ms | 0,4 ms | 0,7 ms | 4 753 | Correct |
| react-head 3.4.2 | 23,5 ms | 1,4 ms | 5,7 ms | 1 682 | Correct |
| react-helmet 6.1.0 | 31,1 ms | 16,7 ms | 18,6 ms | 3 362 | Correct |

> **Remarque :** Les approches fondées sur la remontée native de React 19 (natif, active-react, react-helmet-async 3, react-head) laissent en place le `<title>` statique d'`index.html`. React insère son propre titre **devant** lui, donc `document.title` est correct. Retirez le `<title>` statique d'`index.html` si vous voulez un seul élément.

## Résultats WASM

### Profils Cargo

Même crate, avec `lto = true`, `codegen-units = 1`, `panic = "abort"` et `strip = true`. `fib(30)` récursif mesuré dans Node (V8).

| Profil | wasm brut | wasm-bindgen | wasm-bindgen gzip | `fib(30)` |
| --- | ---: | ---: | ---: | ---: |
| `opt-level = 3` | 144 B | 15,87 kB | 6,76 kB | 3,76 ms |
| `opt-level = 3` + `wasm-opt -O3` | 112 B | 14,46 kB | 6,37 kB | 3,89 ms |
| `opt-level = "s"` | 144 B | 14,71 kB | 6,51 kB | 4,17 ms |
| `opt-level = "z"` | 137 B | 14,72 kB | 6,52 kB | 4,32 ms |
| `opt-level = "z"` + `wasm-opt -Oz` | 109 B | 13,58 kB | 6,19 kB | 3,88 ms |

### Tailles

| Élément | min | gzip |
| --- | ---: | ---: |
| **active-react** `wasm` + `useWasm` | 1,03 kB | 0,60 kB |
| react-wasm 1.0.1 `useWasm` | 5,27 kB | 2,07 kB |
| Code JS de liaison wasm-bindgen (3 fonctions) | 1,89 kB | 0,93 kB |
| Binaire brut `extern "C"` (add + fib) | 144 B | 139 B |
| Binaire wasm-bindgen (add + fib + greet) | 15,87 kB | 6,76 kB |

### Temps jusqu'au premier résultat WASM

Du début de la navigation jusqu'à l'affichage du résultat d'un appel Rust (médiane). « RTT 100 ms » émule une latence réseau réaliste (mobile, 4G) via le Chrome DevTools Protocol.

| Scénario | localhost | RTT 100 ms |
| --- | ---: | ---: |
| active-react `wasm("/x.wasm")` (brut) | 28,8 ms | 331,5 ms |
| react-wasm 1.0.1 (brut) | 28,5 ms | 327,1 ms |
| `useEffect` écrit à la main (brut) | 25,8 ms | 328,9 ms |
| active-react + wasm-bindgen | 23,5 ms | 328,4 ms |
| `useEffect` écrit à la main + wasm-bindgen | 24,6 ms | 332,9 ms |
| SSR + hydratation, **avec** preload automatique (`url`) | 20,2 ms | 224,2 ms |
| SSR + hydratation, sans preload | 24,5 ms | 330,0 ms |

### Coût des appels de JS vers WASM (Chromium)

| Mesure | JS pur | wasm brut | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)` par appel | 0,55 ns | 4,05 ns | 4,81 ns |
| `fib(30)` récursif | 11,4 ms | 2,1 ms | 1,9 ms |
| `greet(str) -> String` par appel | 4,7 ns | — | 553,0 ns |

## Reproduire

Prérequis :

- Node.js 22 ou plus récent ;
- Rust avec la cible `wasm32-unknown-unknown` ;
- `wasm-bindgen-cli` 0.2.129 : `cargo install wasm-bindgen-cli --version 0.2.129` ;
- Chromium pour Playwright : définissez `PLAYWRIGHT_BROWSERS_PATH`, ou lancez `npx playwright install chromium`.

Puis, depuis la racine du dépôt :

```sh
cd bench
npm run setup   # builds the library and installs the benchmark dependencies
npm run bench   # everything, about 5 minutes
```

Lancez une seule moitié avec `node run.mjs seo` ou `node run.mjs wasm`. Le script réécrit `bench/RESULTS.md` et conserve les données brutes dans `bench/results/`. La page écrite avec chaque bibliothèque se trouve dans `bench/seo/libs/`, les pages WASM dans `bench/wasm/pages/`.

## Mises en garde

- **Les chiffres du SSR varient d'environ ±5 % d'une exécution à l'autre.** Les écarts de quelques pourcents (active-react face à @unhead/react, par exemple) relèvent du bruit de mesure ; les grands écarts (face à react-helmet), non.
- **`performance.now()` est arrondi à 0,1 ms dans Chromium** sans isolation cross-origin : les très courtes durées, comme les latences de head de 0,3 ms, sont donc approximatives.
- **Les colonnes `fib(30)` diffèrent d'un tableau à l'autre** parce qu'elles ont été mesurées dans des moteurs et des configurations différents : Node pour les profils Cargo, Chromium pour le coût des appels.
- **Une machine, une page.** Les chiffres absolus dépendent de votre matériel et de vos pages ; ce qui compte, ce sont les comparaisons entre bibliothèques.
- **Le test de « rafale »** mesure le débit brut de 1 000 changements de route consécutifs, ce qu'aucun utilisateur réel ne fait. La latence par navigation est le chiffre le plus pertinent pour de vraies apps.
