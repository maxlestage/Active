# Benchmarks

Compare active-react aux alternatives sur la même page, puis réécrit [`RESULTS.md`](RESULTS.md).

```sh
npm run setup   # build de la lib + dépendances du bench
npm run bench   # tout (≈ 5 min) ; `node run.mjs seo` ou `node run.mjs wasm` pour une partie
```

Prérequis : Node ≥ 22, Rust avec la cible `wasm32-unknown-unknown`, `wasm-bindgen-cli` 0.2.129 (`cargo install wasm-bindgen-cli --version 0.2.129`) et Chromium pour Playwright (`PLAYWRIGHT_BROWSERS_PATH` ou `npx playwright install chromium`).

## Ce qui est mesuré

**SEO** : React 19 natif (référence), active-react, react-helmet, react-helmet-async, @unhead/react, react-head.

- `seo/shared.tsx` : la page commune (11 balises + JSON-LD + 30 éléments) ; `seo/libs/*` : la même page écrite avec chaque lib.
- Poids : imports réellement utilisés, minifiés par esbuild, React exclu.
- SSR (`seo/ssr/*`, `seo/measure.ts`) : document HTML complet, un processus Node par lib, médiane de 3 runs.
- Navigateur (`seo/client.tsx`) : Chromium headless en production. On mesure le chargement, la latence d'un changement de route et une rafale de 1000 navigations, puis on vérifie le `<head>` final : pas de doublons, valeurs correctes.

**WASM** : crates `wasm/rust/raw` (`extern "C"`) et `wasm/rust/bindgen` (wasm-bindgen).

- Profils Cargo (`opt-level` 3/s/z, avec ou sans `wasm-opt`) : taille et vitesse.
- Chargement (`wasm/pages/*`) : active-react, react-wasm et un `useEffect` maison, en local et avec 100 ms de RTT émulés. On compare aussi le SSR avec et sans preload.
- Coût d'appel : JS, wasm brut et wasm-bindgen (`add`, `fib(30)`, chaînes).
