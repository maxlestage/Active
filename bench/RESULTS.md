# Résultats des benchmarks

Générés par `npm run bench` (voir `bench/README.md`). Toutes les libs sont rendues en mode production, avec exactement la même page : `<title>` avec template, description, canonical, 6 balises Open Graph, 2 balises Twitter, un JSON-LD `Product` et un corps de 30 éléments.

Machine : Intel(R) Xeon(R) Processor @ 2.80GHz (4 cœurs), Node v22.22.0, Chromium 141.0.7390.37.

## SEO

### Poids côté client

JS ajouté au bundle pour les imports réellement utilisés (minifié, react/react-dom exclus).

| Lib | min | gzip | brotli |
| --- | ---: | ---: | ---: |
| React 19 natif (sans lib) | 0 | 0 | 0 |
| **active-react** | 1,53 kB | 0,74 kB | 0,62 kB |
| react-helmet-async 3.0.0 | 17,30 kB | 6,29 kB | 5,59 kB |
| @unhead/react 3.4.2 | 15,17 kB | 6,07 kB | 5,55 kB |
| react-head 3.4.2 | 3,20 kB | 1,35 kB | 1,21 kB |
| react-helmet 6.1.0 | 18,46 kB | 6,76 kB | 5,99 kB |

### Rendu serveur (Node)

Temps pour produire le document HTML complet (head + body), médiane de 3 processus × ~2,5 s chacun.

| Lib | µs / page | pages / s | vs React natif |
| --- | ---: | ---: | ---: |
| React 19 natif (sans lib) | 39,7 | 25 199 | 0 % |
| **active-react** | 42,4 | 23 565 | +7 % |
| **active-react** (`renderWithHead`, template HTML) | 44,1 | 22 665 | +11 % |
| @unhead/react 3.4.2 | 44,9 | 22 260 | +13 % |
| react-head 3.4.2 | 60,7 | 16 474 | +53 % |
| react-helmet-async 3.0.0 | 64,9 | 15 405 | +64 % |
| react-helmet 6.1.0 | 66,5 | 15 031 | +68 % |

### Navigateur (Chromium headless)

- **Head prêt** : du début de la navigation jusqu'à ce que title/description/canonical soient corrects (médiane de 15 chargements).
- **Latence** : entre le rendu d'une nouvelle page (changement de route) et le `<head>` à jour (300 navigations).
- **Rafale** : 1000 changements de route d'affilée, jusqu'au `<head>` final correct.

| Lib | Head prêt | Latence médiane | Latence p95 | Rafale (nav/s) | `<head>` final |
| --- | ---: | ---: | ---: | ---: | ---: |
| React 19 natif (sans lib) | 20,9 ms | 0,3 ms | 0,5 ms | 2 267 | ✅ correct |
| **active-react** | 23,8 ms | 0,3 ms | 0,6 ms | 2 144 | ✅ correct |
| react-helmet-async 3.0.0 | 22,7 ms | 0,3 ms | 0,5 ms | 2 402 | ✅ correct |
| @unhead/react 3.4.2 | 24,2 ms | 0,4 ms | 0,7 ms | 4 753 | ✅ correct |
| react-head 3.4.2 | 23,5 ms | 1,4 ms | 5,7 ms | 1 682 | ✅ correct |
| react-helmet 6.1.0 | 31,1 ms | 16,7 ms | 18,6 ms | 3 362 | ✅ correct |

> Les approches basées sur le hoisting natif de React 19 (natif, active-react, react-helmet-async 3, react-head) laissent le `<title>` statique de `index.html` en place : React insère le sien **avant**, donc `document.title` est correct. Supprimez le `<title>` statique de `index.html` si vous voulez un seul élément.

## WASM (Rust)

### Profils Cargo

Même crate (`bench/wasm/rust`), `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. `fib(30)` récursif mesuré dans Node (V8).

| Profil | wasm brut | wasm-bindgen | wasm-bindgen gzip | fib(30) |
| --- | ---: | ---: | ---: | ---: |
| opt-level = 3 | 144 o | 15,87 kB | 6,76 kB | 3,76 ms |
| opt-level = 3 + wasm-opt -O3 | 112 o | 14,46 kB | 6,37 kB | 3,89 ms |
| opt-level = "s" | 144 o | 14,71 kB | 6,51 kB | 4,17 ms |
| opt-level = "z" | 137 o | 14,72 kB | 6,52 kB | 4,32 ms |
| opt-level = "z" + wasm-opt -Oz | 109 o | 13,58 kB | 6,19 kB | 3,88 ms |

### Poids

| Élément | min | gzip |
| --- | ---: | ---: |
| **active-react** `wasm` + `useWasm` | 1,03 kB | 0,60 kB |
| react-wasm 1.0.1 `useWasm` | 5,27 kB | 2,07 kB |
| Glue JS wasm-bindgen (3 fonctions) | 1,89 kB | 0,93 kB |
| Binaire brut `extern "C"` (add + fib) | 144 o | 139 o |
| Binaire wasm-bindgen (add + fib + greet) | 15,87 kB | 6,76 kB |

### Temps jusqu'au résultat WASM affiché

Du début de la navigation jusqu'au rendu du résultat d'un appel Rust (médiane). « RTT 100 ms » émule une latence réseau réaliste (mobile/4G) via Chrome DevTools Protocol.

| Scénario | localhost | RTT 100 ms |
| --- | ---: | ---: |
| active-react `wasm("/x.wasm")` (brut) | 28,8 ms | 331,5 ms |
| react-wasm 1.0.1 (brut) | 28,5 ms | 327,1 ms |
| `useEffect` écrit à la main (brut) | 25,8 ms | 328,9 ms |
| active-react + wasm-bindgen | 23,5 ms | 328,4 ms |
| `useEffect` écrit à la main + wasm-bindgen | 24,6 ms | 332,9 ms |
| SSR + hydratation, **avec** preload auto (`url`) | 20,2 ms | 224,2 ms |
| SSR + hydratation, sans preload | 24,5 ms | 330,0 ms |

### Coût des appels JS → WASM (Chromium)

| Mesure | JS pur | wasm brut | wasm-bindgen |
| --- | ---: | ---: | ---: |
| `add(a, b)` par appel | 0,55 ns | 4,05 ns | 4,81 ns |
| `fib(30)` récursif | 11,4 ms | 2,1 ms | 1,9 ms |
| `greet(str) -> String` par appel | 4,7 ns | — | 553,0 ns |

> `performance.now()` est arrondi à 0,1 ms dans Chromium hors isolation cross-origin : les petites durées sont approximatives.
