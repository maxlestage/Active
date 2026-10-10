---
title: Introduction
description: active-react apporte les balises SEO et Rust/WebAssembly à React 19 en 1,3 kB gzip environ, sans dépendance, compatible SSR, SSG et RSC.
---

active-react est une toute petite bibliothèque pour React 19 qui fait bien deux choses : elle rend les balises SEO dont chaque page a besoin, et elle permet à vos composants d'appeler du Rust compilé en WebAssembly (WASM).

La bibliothèque entière pèse environ **1,3 kB gzip**. Elle n'a **aucune dépendance** (React est une peer dependency), n'exécute **aucun effet**, et fonctionne avec le rendu serveur (SSR), la génération de site statique (SSG) et les React Server Components (RSC).

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="The Acme website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

## Ce que vous obtenez

- **`<Seo>` et `createSeo()`** : titre, description, URL canonique, Open Graph, cartes Twitter, robots, alternatives hreflang et données structurées JSON-LD, depuis un seul composant.
- **`wasm()` et `useWasm()`** : déclarez un module Rust une fois, chargez-le une fois, utilisez-le depuis n'importe quel composant. Le binaire est préchargé pendant le rendu serveur.
- **`renderWithHead()`** : un utilitaire serveur pour les templates HTML (Vite SSR, génération statique) qui renvoie séparément les balises du head et le body.
- **Le crate `active`** : le jumeau Rust de `<Seo>`, qui produit les mêmes balises depuis des serveurs Rust, des générateurs statiques ou WebAssembly.

## Comment ça marche

### SEO : React se charge de la remontée

Depuis React 19, les éléments `<title>`, `<meta>` et `<link>` rendus n'importe où dans votre arbre de composants sont déplacés (« remontés », le hoisting) dans le `<head>` du document par React lui-même, côté serveur comme dans le navigateur.

Il ne reste donc presque rien à faire pour une bibliothèque SEO. `<Seo>` se contente de rendre les bonnes balises. Il n'y a ni provider, ni contexte, ni store, ni effet, ni seconde passe de rendu. Quand l'utilisateur navigue, React retire les balises de l'ancienne page et insère les nouvelles : le `<head>` correspond toujours à la page affichée.

### WASM : déclarer, charger une fois, précharger

- `wasm()` **déclare** un module. Rien n'est encore téléchargé.
- `useWasm()` le **charge** la première fois qu'un composant en a besoin, avec une seule requête partagée, quel que soit le nombre de composants qui l'utilisent. Il renvoie les exports du module, ou `undefined` pendant le chargement.
- Quand l'URL du binaire est connue, `useWasm()` demande à React de le **précharger**. Pendant le SSR, cela place un `<link rel="preload">` dans le `<head>` : le navigateur télécharge le fichier `.wasm` en parallèle de votre JavaScript, et non après.

## Chiffres clés

D'après [les benchmarks](../benchmarks/) (même page pour chaque bibliothèque, builds de production) :

| Mesure | active-react | Pour comparaison |
| --- | ---: | --- |
| Code SEO dans le bundle client | 0,74 kB gzip | react-helmet 6,76 kB, react-helmet-async 6,29 kB, @unhead/react 6,07 kB |
| Temps de SSR par page | 42,4 µs | 39,7 µs avec des balises écrites à la main, 66,5 µs avec react-helmet |
| Mise à jour du head après un changement de route | 0,3 ms en médiane | 16,7 ms avec react-helmet |
| Chargeur WASM dans le bundle client | 0,60 kB gzip | react-wasm 2,07 kB |
| Premier résultat Rust à l'écran, RTT de 100 ms | 224 ms avec le preload SSR | 330 ms sans preload |

En bref : le SEO coûte quelques pourcents de plus que des balises écrites à la main, le rendu serveur est 1,5× plus rapide qu'avec react-helmet, et le preload automatique réduit de 32 % le temps nécessaire pour obtenir le premier résultat Rust.

## Quand l'utiliser

active-react est un bon choix si :

- vous développez avec **React 19** et voulez des balises SEO correctes et complètes avec très peu de code : Vite (SPA, SSR ou SSG), React Router v7, TanStack Start, Waku ou votre propre serveur ;
- vous voulez confier les traitements lourds (parsing, calculs, traitement d'images ou de données) à **Rust** sans écrire de code de chargement répétitif ;
- la **taille du bundle** compte pour vous.

## Quand ne pas l'utiliser

- **React 18 et les versions antérieures** ne sont pas pris en charge. La remontée des balises dans `<head>` est une fonctionnalité de React 19, et la bibliothèque repose dessus.
- **L'App Router de Next.js** dispose de sa propre API Metadata (`metadata` et `generateMetadata`), intégrée à son routage et à son streaming. Vous la préférerez peut-être pour le SEO. `wasm()` et `useWasm()` fonctionnent toujours dans les composants client.
- **Les tout petits calculs** n'ont pas besoin de Rust : chaque appel vers WebAssembly a un petit coût fixe. Consultez le [guide des performances](../performance/).

## Plan de la documentation

**Bien démarrer**

- [Installation](../installation/) : prérequis, commandes d'installation et points d'entrée.
- [Démarrage rapide](../quick-start/) : une app Vite avec SEO et Rust en cinq minutes.

**SEO**

- [Le composant `<Seo>`](../seo-component/) : chaque prop et le HTML qu'elle produit.
- [Valeurs par défaut du site](../site-defaults/) : `createSeo`, templates de titre, URL, robots et hreflang.
- [Données structurées](../structured-data/) : recettes JSON-LD pour les résultats enrichis.
- [Rendu serveur](../server-rendering/) : SSR, génération statique et `renderWithHead`.

**Rust et WebAssembly**

- [Configurer Rust](../rust-setup/) : toolchain, profil Cargo et premier build.
- [Charger WebAssembly](../loading-wasm/) : `wasm()`, `useWasm()`, preload, erreurs et Suspense.
- [WebAssembly brut](../raw-wasm/) : des modules minuscules sans wasm-bindgen.
- [Guide des performances](../performance/) : ce que nous apprennent les benchmarks.
- [Le crate Rust](../rust-crate/) : les mêmes balises SEO, générées depuis Rust.

**Déployer et publier**

- [Déployer sur Heroku](../deploy-heroku/) : depuis votre téléphone, sans ordinateur.
- [Publier une version](../publishing/) : publier sur npm et crates.io depuis une GitHub Release.

**Référence**

- [Référence de l'API](../api/), [Benchmarks](../benchmarks/) et [FAQ](../faq/).
