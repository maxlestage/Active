---
title: FAQ
description: Réponses aux questions fréquentes sur active-react : React 18, Next.js, erreurs wasm-bindgen, types MIME, navigateurs pris en charge et licence.
---

## Est-ce que ça fonctionne avec React 18 ?

Non. active-react repose sur une fonctionnalité de React 19 : les balises `<title>`, `<meta>` et `<link>` rendues n'importe où dans l'arbre sont remontées dans le `<head>` par React lui-même. C'est ce qui permet à `<Seo>` d'être un simple composant, sans provider ni effet. Avec React 18, les balises resteraient dans le body. Mettez à jour avec `npm i react@latest react-dom@latest`.

## Puis-je l'utiliser avec Next.js ?

Avec l'**App Router**, Next.js dispose de sa propre API Metadata (`export const metadata` et `generateMetadata`), intégrée à son routage, à son streaming et à ses conventions de fichiers. Préférez-la pour le SEO, afin que les deux systèmes ne produisent pas de balises en double.

La partie WebAssembly reste utilisable dans Next.js : déclarez le module avec `wasm()` et appelez `useWasm()` dans des composants client (fichiers commençant par `"use client"`). Obtenez l'URL du binaire avec `new URL("./pkg/my_crate_bg.wasm", import.meta.url).href`, car le suffixe `?url` est propre à Vite.

## Est-ce que ça fonctionne avec d'autres frameworks ?

Oui. React Router v7 (mode framework), TanStack Start et Waku ne demandent aucune configuration. Ces frameworks rendent tout le document avec React 19 : les balises rendues par `<Seo>` arrivent donc automatiquement dans le `<head>` du HTML serveur. Voir [rendu serveur](../server-rendering/).

## Une page peut-elle avoir deux `<Seo>` ?

Vous obtenez alors deux jeux de balises : deux titres, deux descriptions, deux liens canoniques. React remonte tout ce qu'on lui donne et ne fusionne pas les balises. Rendez un seul `<Seo>` par page, dans le composant de la page, et placez les valeurs partagées par toutes les pages dans [`createSeo`](../site-defaults/) plutôt que dans un second `<Seo>` au sein d'un layout.

## Pourquoi y a-t-il deux balises `<title>` ?

C'est le `<title>` statique de votre `index.html`. React insère le titre de la page **devant** lui : `document.title`, l'onglet du navigateur et les moteurs de recherche utilisent donc le bon. Vous pouvez supprimer le titre statique d'`index.html` pour ne garder qu'un seul élément.

En revanche, retirez bien les balises statiques `<meta name="description">` et `og:*` d'`index.html` : elles seraient dupliquées.

## Google lit-il le JSON-LD placé dans le body ?

Oui. Google lit les scripts JSON-LD n'importe où dans le document, dans le `<head>` comme dans le `<body>`. React 19 ne remonte pas les scripts ordinaires : le `<script type="application/ld+json">` reste donc là où `<Seo>` est rendu. Vérifiez vos pages avec le [test des résultats enrichis](https://search.google.com/test/rich-results). Pour en savoir plus, consultez [données structurées](../structured-data/).

## Puis-je utiliser active-react sans Rust ?

Bien sûr. La partie SEO n'a absolument pas besoin de Rust et, grâce au tree-shaking, une app qui ne fait que du SEO n'embarque que 0,74 kB gzip : le chargeur WebAssembly n'est jamais inclus si vous n'importez ni `wasm` ni `useWasm`.

## Versions de wasm-bindgen incompatibles

L'erreur complète commence par :

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary
```

L'outil en ligne de commande `wasm-bindgen` doit être exactement dans la même version que le crate `wasm-bindgen` de votre `Cargo.lock`. Installez le CLI correspondant :

```sh
cargo install -f wasm-bindgen-cli --version 0.2.129
```

ou figez le crate sur la version de votre CLI dans `Cargo.toml` avec `wasm-bindgen = "=0.2.129"`. wasm-pack évite complètement le problème en téléchargeant pour vous le bon CLI. Voir [Configurer Rust](../rust-setup/).

## Mon serveur n'envoie pas application/wasm

Votre module se charge quand même : quand le `Content-Type` n'est pas `application/wasm`, active-react télécharge le fichier en entier, puis le compile, au lieu de le compiler pendant le téléchargement. C'est seulement un peu plus lent. Pour corriger l'en-tête :

```text
# nginx (inside http, server or location)
types { application/wasm wasm; }

# Apache (.htaccess or server config)
AddType application/wasm .wasm
```

Les serveurs de développement et de preview de Vite, ainsi que `express.static` d'Express, envoient déjà le bon type. Avec un serveur Node écrit à la main, définissez `content-type: application/wasm` pour les fichiers `.wasm`.

## Puis-je exécuter Rust sur le serveur pendant le SSR ?

`useWasm()` renvoie volontairement `undefined` sur le serveur et pendant l'hydratation, pour que le HTML serveur et le premier rendu client correspondent toujours. Le résultat Rust apparaît juste après l'hydratation.

Si vous avez besoin d'un résultat Rust dans le HTML serveur, calculez-le sur le serveur avant le rendu et passez-le en prop. Avec la sortie `--target web` de wasm-bindgen, Node peut charger le binaire de façon synchrone :

```ts
// server only
import { readFileSync } from "node:fs";
import { count_primes, initSync } from "./pkg/hello.js";

initSync({ module: readFileSync(new URL("./pkg/hello_bg.wasm", import.meta.url)) });
const primes = count_primes(1_000_000); // pass it to your page as a prop
```

Sur un serveur Rust, appelez directement votre code Rust, et utilisez le [crate `active`](../rust-crate/) pour les balises SEO.

## Quels navigateurs sont pris en charge ?

Tous les navigateurs actuels : Chrome, Edge, Firefox et Safari, sur ordinateur comme sur mobile. WebAssembly lui-même est pris en charge partout depuis 2017. La compilation en streaming (`WebAssembly.instantiateStreaming`), utilisée pour les modules `.wasm` bruts servis en `application/wasm`, nécessite Safari 15 ou plus récent. Les navigateurs qui ne prennent pas en charge les liens `preload` les ignorent simplement et chargent le module normalement.

## Quelle est sa taille, concrètement ?

Environ 1,3 kB gzip si vous utilisez tout : 0,74 kB pour la partie SEO et 0,60 kB pour le chargeur WebAssembly, mesurés avec esbuild, sans React. Grâce au tree-shaking, vous ne payez que ce que vous importez. Les [benchmarks](../benchmarks/) la comparent aux alternatives.

## Quelle est la licence ?

MIT, pour le paquet npm comme pour le crate Rust. Vous pouvez l'utiliser dans des projets commerciaux et propriétaires ; conservez la mention de copyright.

## Comment contribuer ?

Les issues et les pull requests sont les bienvenues sur [GitHub](https://github.com/maxlestage/Active). Pour travailler sur le code :

```sh
git clone https://github.com/maxlestage/Active.git
cd Active
npm install
npm test            # vitest: SSR, client (jsdom) and wasm tests
npm run typecheck
npm run build       # dist/ (ESM + .d.ts)
```

Le crate Rust se trouve dans `crates/active` (`cargo test`), ce site web dans `site/`, et les benchmarks dans `bench/` (`npm run setup && npm run bench`). Lancez les tests avant d'ouvrir une pull request, et joignez une exécution des benchmarks si votre modification peut affecter les performances.
