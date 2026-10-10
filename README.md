# active-react

**SEO + Rust/WASM pour React 19, en ~1,3 kB gzip.** Zéro dépendance, zéro effet, compatible SSR, SSG et Server Components.

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined pendant le chargement
  return (
    <>
      <Seo title="Accueil" description="Le site d'Acme." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("toi") : "Chargement…"}</p>
    </>
  );
}
```

## Pourquoi c'est rapide

Les benchmarks (détails et méthode dans [`bench/RESULTS.md`](bench/RESULTS.md)) ont guidé chaque choix :

- **SEO : pas de gestionnaire de `<head>`.** Depuis React 19, `<title>`, `<meta>` et `<link>` rendus n'importe où sont remontés dans `<head>` par React lui-même, côté serveur comme côté client. `<Seo>` se contente donc de produire les bonnes balises : pas de contexte, pas de store, pas d'effet, pas de seconde passe. Résultat : un coût à quelques pourcents de balises écrites à la main, et 1,5× plus rapide que react-helmet en SSR.
- **WASM : un seul `fetch`, partagé, préchargé.** `wasm()` déclare le module sans rien télécharger ; `useWasm()` le charge une seule fois (`instantiateStreaming`), quel que soit le nombre de composants qui l'utilisent. Si l'URL du `.wasm` est connue, le rendu SSR émet `<link rel="preload">` et le navigateur télécharge le binaire **en parallèle** du JS : **−32 % de temps** jusqu'au premier résultat Rust avec 100 ms de latence réseau.

| SEO (même page : 11 balises + JSON-LD) | JS client (gzip) | SSR (µs/page) | Latence nav. (médiane) |
| --- | ---: | ---: | ---: |
| React 19 natif, sans lib (référence) | 0 | 39,7 | 0,3 ms |
| **active-react** | **0,74 kB** | **42,4** | **0,3 ms** |
| @unhead/react 3.4.2 | 6,07 kB | 44,9 | 0,4 ms |
| react-head 3.4.2 | 1,35 kB | 60,7 | 1,4 ms |
| react-helmet-async 3.0.0 | 6,29 kB | 64,9 | 0,3 ms |
| react-helmet 6.1.0 | 6,76 kB | 66,5 | 16,7 ms |

| WASM (crate wasm-bindgen) | JS (gzip) | Résultat Rust affiché, RTT 100 ms |
| --- | ---: | ---: |
| **active-react**, SSR + preload automatique | **0,60 kB** | **224 ms** |
| active-react, SSR sans preload | 0,60 kB | 330 ms |
| active-react, rendu client | 0,60 kB | 328 ms |
| react-wasm 1.0.1, rendu client (`.wasm` brut) | 2,07 kB | 327 ms |
| `useEffect` écrit à la main, rendu client | 0 | 333 ms |

Intel Xeon 2,8 GHz, Node 22, Chromium 141, React 19.3. Le SSR varie d'environ ±5 % d'un run à l'autre. Relancer : voir [`bench/`](bench/).

## Installation

```sh
npm i github:maxlestage/Active   # pas encore publié sur npm ; peer : react >= 19, react-dom >= 19
```

## SEO

### `<Seo>` et `createSeo(defaults)`

Définissez les valeurs du site une fois, utilisez `<Seo>` une fois par page :

```tsx
// seo.ts
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com", // les URL relatives (canonical, image, alternates) sont résolues avec
  image: "/og.png",
  locale: "fr_FR",
  twitter: "@acme",
});
```

```tsx
// pages/Produit.tsx
<Seo
  title="Chaise Oslo"
  description="Chaise en chêne massif, livrée en 48 h."
  canonical="/produits/oslo"
  image="/img/oslo.jpg"
  type="product"
  alternates={{ en: "/en/products/oslo", fr: "/produits/oslo" }}
  jsonLd={{ "@context": "https://schema.org", "@type": "Product", name: "Chaise Oslo" }}
/>
```

Produit dans `<head>` :

```html
<title>Chaise Oslo · Acme</title>
<meta name="description" content="Chaise en chêne massif, livrée en 48 h."/>
<link rel="canonical" href="https://acme.com/produits/oslo"/>
<link rel="alternate" hrefLang="en" href="https://acme.com/en/products/oslo"/>
<link rel="alternate" hrefLang="fr" href="https://acme.com/produits/oslo"/>
<meta property="og:title" content="Chaise Oslo"/>
<meta property="og:description" content="Chaise en chêne massif, livrée en 48 h."/>
<meta property="og:type" content="product"/>
<meta property="og:url" content="https://acme.com/produits/oslo"/>
<meta property="og:image" content="https://acme.com/img/oslo.jpg"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="fr_FR"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
```

…et un `<script type="application/ld+json">` (échappé contre l'injection `</script>`) à l'emplacement du composant, ce que Google accepte.

| Prop | Balises |
| --- | --- |
| `title`, `titleTemplate` | `<title>` (template `%s`), `og:title` (sans template) |
| `siteName` | `og:site_name`, titre par défaut |
| `description` | `description`, `og:description` |
| `canonical` | `<link rel="canonical">`, `og:url` |
| `image` | `og:image`, `twitter:card` = `summary_large_image` |
| `type` | `og:type` (défaut `website`) |
| `locale`, `twitter` | `og:locale`, `twitter:site` |
| `noindex`, `nofollow` | `<meta name="robots">` |
| `alternates` | `<link rel="alternate" hreflang>` |
| `jsonLd` | données structurées (objet ou tableau) |
| `meta` | balises supplémentaires : `[{ name: "theme-color", content: "#000" }]` |
| `baseUrl` | résolution des URL relatives |

`<Seo>` n'utilise aucun hook : il fonctionne aussi dans les **Server Components**.

### Rendu serveur

- **React rend `<html>`** (React Router, TanStack Start, Waku, serveur maison…) : rien à faire, `renderToString` / `renderToPipeableStream` placent les balises dans `<head>`.
- **Template HTML** (SSR Vite avec `<!--app-head-->` et `<!--app-html-->`, génération statique) :

```tsx
import { renderWithHead } from "active-react/server";

const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

### Bon à savoir

- Un seul `<Seo>` par page (les valeurs communes vont dans `createSeo`). Deux `<Seo>` montés en même temps produisent des balises en double.
- En SPA, retirez `description`/`og:*` de `index.html` : `<Seo>` s'en charge. Le `<title>` statique peut rester, React place le sien devant.

## WASM (Rust)

### 1. Compiler le crate

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]   # profil validé par les benchmarks
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

```sh
wasm-pack build --target web --out-dir ../src/pkg
# ou : cargo build --release --target wasm32-unknown-unknown
#      wasm-bindgen --target web --out-dir src/pkg target/wasm32-unknown-unknown/release/my_crate.wasm
```

### 2. L'utiliser dans React

```tsx
import { wasm, useWasm } from "active-react";
import url from "./pkg/my_crate_bg.wasm?url"; // Vite ; optionnel, active le preload SSR

export const myCrate = wasm(() => import("./pkg/my_crate.js"), { url });

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(myCrate);
  if (!rust) return <Spinner />;
  return <p>Moyenne : {rust.mean(data)}</p>; // typé grâce au .d.ts généré
}
```

- `useWasm(mod)` renvoie les exports, ou `undefined` pendant le chargement. Il renvoie toujours `undefined` pendant le SSR et l'hydratation : le HTML serveur et client correspondent.
- Une erreur de chargement est lancée vers l'error boundary la plus proche ; `mod.load()` relance le chargement.
- Vous préférez Suspense ? `use(mod.load())` dans un composant client.
- Hors React : `const rust = await myCrate.load()`.
- Démarrer le téléchargement au plus tôt : appelez `myCrate.load()` au niveau du module.

**Sans wasm-bindgen** (fonctions numériques `extern "C"`, binaire de quelques centaines d'octets, aucun JS de liaison) :

```rust
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 { a + b }
```

```tsx
const math = wasm<{ add(a: number, b: number): number }>("/math.wasm");
```

### Conseils tirés des benchmarks

- **Pas de différence de temps de chargement** entre active-react, react-wasm et un `useEffect` maison : c'est le réseau et la compilation qui comptent. Ce qui fait gagner du temps, c'est le **preload** (SSR) et un binaire **petit**.
- **Rust est ~5× plus rapide** que JS sur `fib(30)`, mais **chaque appel coûte ~4 ns** (contre 0,6 ns en JS) et **passer une chaîne coûte ~550 ns**. Envoyez de gros blocs de travail (tableaux, buffers) plutôt que des milliers de petits appels.
- Le binaire wasm-bindgen qui utilise `format!` et `String` pèse ~15 kB. Pour des calculs purs, l'export brut fait 144 octets.
- `opt-level = "z"` + `wasm-opt -Oz` réduit la taille d'environ 8 %, avec une vitesse équivalente sur nos mesures. `opt-level = 3` reste le bon défaut si la vitesse prime.

## API

| Export | Rôle |
| --- | --- |
| `Seo(props)` | Balises SEO, remontées dans `<head>` par React 19 |
| `createSeo(defaults)` | Renvoie un `<Seo>` avec les valeurs du site |
| `wasm(loader, { url? })` | Déclare un module wasm-bindgen (`--target web` ou `bundler`) |
| `wasm(url, { imports? })` | Déclare un `.wasm` brut |
| `mod.load()` / `mod.get()` | Charger (une seule fois) / lire de manière synchrone |
| `useWasm(mod)` | Exports ou `undefined` ; preload du binaire en SSR |
| `renderWithHead(node)` (`active-react/server`) | `{ head, html }` pour les templates HTML |

## Exemple complet

[`examples/vite-ssg`](examples/vite-ssg) : Vite + crate Rust + génération statique. Chaque page HTML contient ses balises SEO et le preload du `.wasm`, puis s'hydrate et appelle Rust.

## Développement

```sh
npm install
npm test          # vitest : SSR, client (jsdom), wasm
npm run typecheck
npm run build     # dist/ (ESM + .d.ts)
cd bench && npm run setup && npm run bench   # nécessite Rust, wasm-bindgen-cli et Chromium
```
