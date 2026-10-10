# active-react

[English](README.md) · **Français** · [Español](README.es.md)

**Le SEO et Rust/WebAssembly pour React 19, en ~1,3 kB gzip.** Zéro dépendance, zéro effet. Fonctionne avec le SSR, la génération statique et les Server Components. Un jumeau en Rust, le crate [`active`](crates/active), génère les mêmes balises depuis Rust.

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="Acme's website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

📚 **Documentation complète (anglais, français, espagnol) :** le site de présentation dans [`site/`](site). Déployez votre propre copie avec le bouton [Deploy to Heroku](#mettre-le-site-en-ligne-sur-heroku-depuis-un-smartphone), ou lisez directement les pages dans [`site/docs/`](site/docs).

## Pourquoi c'est rapide

La conception découle des benchmarks. Les détails et la méthode se trouvent dans [`bench/RESULTS.md`](bench/RESULTS.md).

- **Du SEO sans gestionnaire de `<head>`.** Depuis React 19, les balises `<title>`, `<meta>` et `<link>` rendues n'importe où sont remontées dans `<head>` par React lui-même, côté serveur comme dans le navigateur. `<Seo>` se contente de rendre les bonnes balises : pas de contexte, pas de store, pas d'effet, pas de seconde passe. Le coût reste à quelques pourcents de celui de balises écrites à la main, et le SSR est 1,5× plus rapide qu'avec react-helmet.
- **WASM : un seul fetch, partagé et préchargé.** `wasm()` déclare un module sans rien télécharger. `useWasm()` le charge une seule fois (`instantiateStreaming`), quel que soit le nombre de composants qui l'utilisent. Quand l'URL du `.wasm` est connue, le rendu serveur émet `<link rel="preload">` : le navigateur télécharge alors le binaire **en parallèle** du JavaScript. Avec 100 ms de latence réseau, le premier résultat Rust arrive **32 % plus tôt**.

| SEO (même page : 11 balises + JSON-LD) | JS client (gzip) | SSR (µs/page) | Latence de navigation (médiane) |
| --- | ---: | ---: | ---: |
| React 19, sans bibliothèque (référence) | 0 | 39,7 | 0,3 ms |
| **active-react** | **0,74 kB** | **42,4** | **0,3 ms** |
| @unhead/react 3.4.2 | 6,07 kB | 44,9 | 0,4 ms |
| react-head 3.4.2 | 1,35 kB | 60,7 | 1,4 ms |
| react-helmet-async 3.0.0 | 6,29 kB | 64,9 | 0,3 ms |
| react-helmet 6.1.0 | 6,76 kB | 66,5 | 16,7 ms |

| WASM (crate wasm-bindgen) | JS (gzip) | Résultat Rust à l'écran, RTT de 100 ms |
| --- | ---: | ---: |
| **active-react**, SSR + preload automatique | **0,60 kB** | **224 ms** |
| active-react, SSR sans preload | 0,60 kB | 330 ms |
| active-react, rendu client | 0,60 kB | 328 ms |
| react-wasm 1.0.1, rendu client (`.wasm` brut) | 2,07 kB | 327 ms |
| `useEffect` écrit à la main, rendu client | 0 | 333 ms |

Intel Xeon 2,8 GHz, Node 22, Chromium 141, React 19.3. Les chiffres du SSR varient d'environ ±5 % d'une exécution à l'autre.

## Installation

```sh
npm i github:maxlestage/Active   # until the first npm release, then: npm i active-react
cargo add active                 # optional: the Rust crate, once published
```

Peer dependencies : `react >= 19`, `react-dom >= 19`.

## SEO

```tsx
// seo.ts — your site's defaults, written once
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com", // relative URLs (canonical, image, alternates) are resolved against it
  image: "/og.png",
  locale: "en_US",
  twitter: "@acme",
});
```

```tsx
// any page
<Seo
  title="Oak chair"
  description="Solid oak, delivered in 48 hours."
  canonical="/products/oak-chair"
  type="product"
  alternates={{ en: "/products/oak-chair", fr: "/fr/produits/chaise-chene" }}
  jsonLd={{ "@context": "https://schema.org", "@type": "Product", name: "Oak chair" }}
/>
```

Ce composant rend `<title>`, la description, les liens canonical et hreflang, la balise robots, ainsi que les balises Open Graph et Twitter. Le JSON-LD est échappé contre l'injection de `</script>`. `<Seo>` n'utilise aucun hook : il fonctionne donc aussi dans les Server Components. Rendez un seul `<Seo>` par page.

**Rendu serveur.** Si React rend `<html>` (React Router, TanStack Start, Waku, un serveur sur mesure), il n'y a rien à faire. Pour les templates HTML (Vite SSR, génération statique), utilisez `active-react/server` :

```tsx
import { renderWithHead } from "active-react/server";

const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

## WebAssembly (Rust)

```sh
wasm-pack build --target web --out-dir ../src/pkg
```

```tsx
import { wasm, useWasm } from "active-react";
import url from "./pkg/my_crate_bg.wasm?url"; // Vite; optional, enables the SSR preload

export const myCrate = wasm(() => import("./pkg/my_crate.js"), { url });

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(myCrate);
  if (!rust) return <Spinner />;
  return <p>Mean: {rust.mean(data)}</p>; // typed from the generated .d.ts
}
```

- `useWasm(mod)` renvoie les exports, ou `undefined` pendant le chargement. Il renvoie toujours `undefined` pendant le SSR et l'hydratation, pour que les HTML serveur et client correspondent.
- Une erreur de chargement est levée vers l'error boundary la plus proche, et `mod.load()` relance le chargement.
- Pour Suspense, appelez `use(mod.load())` dans un composant client. Hors de React, utilisez `await mod.load()`.
- Un module brut `extern "C"` fonctionne sans aucun code de liaison : `wasm<{ add(a: number, b: number): number }>("/math.wasm")`.

Profil Cargo recommandé (validé par les benchmarks) : `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.

## Le crate Rust `active`

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

Il produit les mêmes balises que `<Seo>`, pour Axum, Actix, les générateurs statiques ou WebAssembly. Il n'a aucune dépendance. Voir [`crates/active`](crates/active).

## Dépôt

| Dossier | Contenu |
| --- | --- |
| `src/`, `test/` | La bibliothèque `active-react` et ses tests (vitest) |
| `crates/active/` | Le crate Rust `active` (crates.io) |
| `site/` | Site de présentation + documentation (en/fr/es) : Vite, React 19, active-react, hero et playground en Rust/WASM |
| `bench/` | Benchmarks face à react-helmet, react-helmet-async, @unhead/react, react-head et react-wasm |
| `examples/vite-ssg/` | Exemple minimal Vite + Rust + génération statique |

## Mettre le site en ligne sur Heroku depuis un smartphone

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/maxlestage/Active)

1. Ouvrez ce README sur votre téléphone et touchez **Deploy to Heroku**. Connectez-vous, choisissez un nom d'app, puis touchez **Deploy app**.
2. Pour redéployer à chaque modification : dans le tableau de bord Heroku, ouvrez **Deploy → GitHub**, connectez le dépôt, puis touchez **Enable Automatic Deploys** pour la branche `master`.

Heroku n'a pas d'offre gratuite. Un dyno Eco coûte environ 5 $ par mois. Le build n'utilise que Node : le module WASM du site est commité dans le dépôt. Guide pas à pas : `site/docs/fr/deploy-heroku.md`.

## Publier sur npm et crates.io depuis un smartphone

1. Ajoutez les secrets de dépôt `NPM_TOKEN` et `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Ouvrez **Releases → Draft a new release**, créez un tag comme `v0.1.0`, puis touchez **Publish release**.

Le workflow `release.yml` lit la version dans le tag, lance les tests et publie le paquet `active-react` et le crate `active`. Guide : `site/docs/fr/publishing.md`.

## Développement

```sh
npm install && npm test && npm run build      # library
cd crates/active && cargo test                # Rust crate
cd site && npm install && npm run dev         # site (npm run build && npm start for production)
cd bench && npm run setup && npm run bench    # benchmarks (needs Rust, wasm-bindgen-cli, Chromium)
```

Chaque branche `claude/*` qui passe la CI est fusionnée automatiquement dans `master` (`.github/workflows/ci.yml`).

## Licence

[MIT](LICENSE)
