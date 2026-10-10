---
title: Installation
description: Installez active-react depuis GitHub ou, une fois publié, depuis npm. Types TypeScript inclus, et une app purement SEO n'embarque jamais le code WASM.
---

active-react est un paquet unique avec deux points d'entrée. L'installation tient en une commande ; la toolchain Rust n'est nécessaire que si vous utilisez la partie WebAssembly.

## Prérequis

| Outil | Version | Nécessaire pour |
| --- | --- | --- |
| `react` et `react-dom` | 19 ou plus récent | Tout (peer dependencies) |
| Node.js | 20 ou plus récent | Rendu serveur et génération statique |
| Rust | Stable, avec la cible `wasm32-unknown-unknown` | La partie WebAssembly uniquement |
| TypeScript | Facultatif | Les types sont inclus |

Pourquoi React 19 ? Depuis la version 19, React déplace lui-même `<title>`, `<meta>` et `<link>` dans le `<head>`. active-react repose sur cette fonctionnalité : React 18 n'est donc pas pris en charge.

Vérifiez vos versions :

```sh
npm ls react react-dom
```

Si vous êtes encore sur React 18, commencez par mettre à jour :

```sh
npm i react@latest react-dom@latest
npm i -D @types/react@latest @types/react-dom@latest
```

## Installer

### Depuis GitHub (dès aujourd'hui)

Le paquet n'est pas encore sur npm. Installez-le directement depuis le dépôt :

```sh
npm i github:maxlestage/Active
```

npm clone le dépôt et exécute son script `prepare`, qui compile la bibliothèque dans `dist/`. Le paquet s'appelle toujours `active-react` : vos imports seront donc les mêmes qu'avec la future version npm. Votre `package.json` reçoit cette ligne :

```json
{
  "dependencies": {
    "active-react": "github:maxlestage/Active"
  }
}
```

> **Astuce :** Ajoutez `#` suivi d'un hash de commit (ou d'un tag) pour figer une version exacte : `npm i github:maxlestage/Active#<commit-hash>`.

### Depuis npm (une fois publié)

```sh
npm i active-react
```

La même chose avec d'autres gestionnaires de paquets :

```sh
pnpm add active-react
yarn add active-react
bun add active-react
```

> **Remarque :** pnpm, Yarn et Bun acceptent aussi `github:maxlestage/Active`. Certains d'entre eux bloquent par défaut les scripts d'installation des dépendances ; si `node_modules/active-react/dist` est absent après l'installation, autorisez le script de build du paquet ou utilisez npm.

## Points d'entrée

| Importer depuis | Exports | À utiliser dans |
| --- | --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` | Vos composants, rendus sur le serveur ou dans le navigateur |
| `active-react/server` | `renderWithHead` | Le code serveur uniquement |

```tsx
import { Seo, createSeo, wasm, useWasm, type SeoProps, type Wasm } from "active-react";
import { renderWithHead } from "active-react/server";
```

> **Attention :** N'importez jamais `active-react/server` depuis du code navigateur. Il embarque `react-dom/server`, que vous ne voulez pas dans votre bundle client.

## TypeScript

Les types sont fournis avec le paquet : il n'y a pas de `@types/active-react` à installer. Survolez n'importe quelle prop dans votre éditeur pour lire sa documentation.

Le paquet utilise le champ `exports` du `package.json` : TypeScript a donc besoin d'une résolution de modules moderne pour trouver `active-react/server` :

```json
{
  "compilerOptions": {
    "moduleResolution": "bundler"
  }
}
```

`"bundler"` est la valeur par défaut des templates Vite. `"node16"` et `"nodenext"` fonctionnent aussi. L'ancien réglage `"node"` ne sait pas résoudre le point d'entrée `/server`.

Le paquet est uniquement ESM (`"type": "module"`) : utilisez `import`, pas `require`.

> **Remarque :** Importer un fichier `.wasm` avec le suffixe `?url` de Vite nécessite les types client de Vite. Les templates Vite les incluent déjà (`vite/client`).

## Tree-shaking

Le paquet déclare `"sideEffects": false` : votre bundler ne conserve que ce que vous importez :

| Vous importez | Ajouté à votre bundle client |
| --- | ---: |
| `Seo` et/ou `createSeo` | 0,74 kB gzip |
| `wasm` et `useWasm` | 0,60 kB gzip |
| Tout | environ 1,3 kB gzip |

Une app qui ne fait que du SEO n'embarque jamais le chargeur WebAssembly, et une app qui n'utilise que Rust n'embarque jamais le code SEO. Ces tailles proviennent des [benchmarks](../benchmarks/) : minifiées avec esbuild, sans React ni React DOM, puisque votre app les contient déjà.

## Étapes suivantes

- Suivez le [démarrage rapide](../quick-start/) pour construire une petite app avec SEO et Rust.
- Découvrez [le composant `<Seo>`](../seo-component/).
- Installez la [toolchain Rust](../rust-setup/) si vous prévoyez d'utiliser WebAssembly.
