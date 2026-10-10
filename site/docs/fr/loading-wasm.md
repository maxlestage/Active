---
title: Charger WebAssembly
description: Déclarez un module Rust avec wasm(), utilisez-le avec useWasm(), préchargez-le en SSR, et gérez erreurs, Suspense et chargement hors de React.
---

Deux fonctions couvrent WebAssembly dans active-react : `wasm()` déclare un module, `useWasm()` l'utilise dans un composant. Cette page présente les deux, ainsi que le préchargement, les erreurs et les façons moins courantes de charger un module.

## Déclarer un module

Déclarez chaque module une seule fois, dans son propre fichier, et importez-le partout où vous en avez besoin :

```ts
// src/wasm.ts
import { wasm } from "active-react";
import url from "./pkg/hello_bg.wasm?url";

export const hello = wasm(() => import("./pkg/hello.js"), { url });
```

- Le premier argument est une fonction qui importe le code de liaison généré par wasm-bindgen ou wasm-pack. Comme il s'agit d'un `import()` dynamique, votre bundler place ce code dans un chunk séparé.
- `url` est l'URL publique du binaire `.wasm`. Elle est facultative, mais elle active le preload décrit plus bas.

**Rien n'est téléchargé à ce stade.** Déclarer des modules ne coûte rien : vous pouvez en déclarer autant que vous voulez.

## L'utiliser dans un composant

```tsx
import { useWasm } from "active-react";
import { hello } from "./wasm";

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(hello);
  if (!rust) return <p>Loading…</p>;
  return <p>Mean: {rust.mean(data)}</p>;
}
```

`useWasm(mod)` renvoie les exports du module, ou `undefined` pendant son chargement :

- Le premier composant qui l'appelle lance le téléchargement. Tous les autres composants partagent la même requête : le module est téléchargé, compilé et initialisé **une seule fois** pour toute l'app.
- Quand le chargement se termine, chaque composant qui l'utilise est rendu de nouveau avec les exports.
- Une fois le module chargé, les composants suivants obtiennent les exports dès leur premier rendu, sans état de chargement.
- Il renvoie toujours `undefined` sur le serveur et pendant l'hydratation, pour que les balisages serveur et client correspondent. Voir [rendu serveur](../server-rendering/#hydratation-et-webassembly).

## Exports typés

Les exports sont typés à partir du fichier `.d.ts` que wasm-bindgen génère à côté du code de liaison. Si votre fonction Rust est `pub fn mean(values: &[f64]) -> f64`, alors `rust.mean` est de type `(values: Float64Array) => number`, et TypeScript détecte les fautes de frappe et les mauvais arguments. Il n'y a rien à configurer : le type découle de l'`import()`.

## Préchargement

Quand `url` est connue, `useWasm()` demande à React de précharger le binaire. Pendant le rendu serveur, cela ajoute une balise dans le `<head>` :

```html
<link rel="preload" href="/assets/hello_bg-CSLcOxPe.wasm" as="fetch" crossorigin="" type="application/wasm"/>
```

Le navigateur commence à télécharger le fichier `.wasm` dès qu'il lit le `<head>`, en parallèle de votre JavaScript, au lieu d'attendre que React s'exécute. Dans les benchmarks, avec 100 ms de latence réseau, le premier résultat Rust est apparu au bout de **224 ms au lieu de 330 ms (−32 %)**.

`as="fetch"` et `crossorigin` correspondent à la façon dont le binaire est réellement demandé ensuite (avec `fetch()`) : le navigateur réutilise donc la réponse préchargée au lieu de télécharger le fichier deux fois.

Dans une app rendue uniquement côté client, le preload est ajouté pendant le premier rendu : une légère avance seulement, puisqu'il n'y a pas de HTML serveur dans lequel le placer.

### Obtenir l'URL

Avec Vite, importez le binaire avec le suffixe `?url`. Vite copie le fichier dans `dist/assets`, ajoute un hash de contenu à son nom et vous donne l'URL finale, la même dans les builds client et serveur :

```ts
import url from "./pkg/hello_bg.wasm?url";
```

Avec webpack 5 ou Rspack, le motif `new URL()` émet le fichier et vous donne son URL (vérifiez que votre build serveur produit la même URL publique) :

```ts
const url = new URL("./pkg/hello_bg.wasm", import.meta.url).href;
```

Sans `url`, tout fonctionne quand même ; le code de liaison de wasm-bindgen trouve le binaire à côté de lui, mais il n'y a pas de preload.

## Gérer les erreurs

Si le module ne parvient pas à se charger (erreur réseau, 404, binaire invalide), `useWasm()` lève l'erreur pendant le rendu, pour qu'elle atteigne l'**error boundary** la plus proche. Appeler à nouveau `mod.load()` relance le chargement. Faites-le quand l'error boundary se réinitialise :

```tsx
import { ErrorBoundary } from "react-error-boundary";
import { hello } from "./wasm";

<ErrorBoundary
  onReset={() => void hello.load().catch(() => {})}
  fallbackRender={({ resetErrorBoundary }) => (
    <p>
      The Rust module could not load. <button onClick={resetErrorBoundary}>Try again</button>
    </p>
  )}
>
  <Stats data={data} />
</ErrorBoundary>;
```

`react-error-boundary` est facultatif ; n'importe quelle error boundary fait l'affaire. L'essentiel est d'appeler `load()` avant que les enfants soient rendus de nouveau.

## Suspense à la place

Vous préférez Suspense à la vérification de `undefined` ? Le `use()` de React fonctionne avec `mod.load()`, qui renvoie toujours la même promesse :

```tsx
"use client";
import { Suspense, use } from "react";
import { hello } from "./wasm";

function Stats({ data }: { data: Float64Array }) {
  const rust = use(hello.load()); // suspends until loaded
  return <p>Mean: {rust.mean(data)}</p>;
}

export function StatsPanel({ data }: { data: Float64Array }) {
  return (
    <Suspense fallback={<p>Loading…</p>}>
      <Stats data={data} />
    </Suspense>
  );
}
```

> **Attention :** N'utilisez ce motif que dans des composants rendus dans le navigateur. Pendant le rendu serveur, `load()` essaierait de télécharger le binaire sur le serveur. `useWasm()` est le bon choix pour les composants rendus côté serveur.

## Hors de React

Un module déclaré est un simple objet, utilisable partout :

```ts
import { hello } from "./wasm";

// Load on demand, for example in an event handler.
button.addEventListener("click", async () => {
  const rust = await hello.load();
  output.textContent = rust.greet("you");
});

// Read synchronously: the exports, or undefined if not loaded yet.
const maybeRust = hello.get();
```

`load()` et `useWasm()` partagent le même état : charger le module à un endroit le rend disponible partout.

## Charger tôt

Par défaut, le chargement démarre avec le premier composant qui a besoin du module. Pour le lancer dès que le JavaScript de votre app s'exécute, appelez `load()` au niveau du module, dans le navigateur uniquement :

```ts
export const hello = wasm(() => import("./pkg/hello.js"), { url });

if (typeof window !== "undefined") {
  hello.load().catch(() => {}); // a failure still reaches useWasm() and your error boundary
}
```

## Cibles wasm-bindgen

- **`--target web`** (recommandée) : le code de liaison exporte par défaut une fonction asynchrone `init`. active-react l'appelle pour vous, avec `{ module_or_path: url }` si vous avez fourni une `url`.
- **`--target bundler`** : le bundler instancie lui-même le binaire (Vite a besoin d'un plugin pour cela). Le module est utilisé tel quel, sans `init`, et il n'y a pas de preload.

Pour un fichier `.wasm` sans code de liaison, passez son URL au lieu d'une fonction : voir [WebAssembly brut](../raw-wasm/).

## Tests

Dans les tests unitaires, déclarez un faux module avec de simples fonctions. Un loader qui renvoie un objet sans export `init` par défaut est utilisé tel quel :

```ts
import { wasm } from "active-react";

export const fakeHello = wasm(async () => ({
  greet: (name: string) => `Hello ${name}`,
  mean: (values: Float64Array) => values.reduce((a, b) => a + b, 0) / values.length,
}));
```

## L'objet Wasm

`wasm()` renvoie un `Wasm<T>`, où `T` est le type des exports :

```ts
interface Wasm<T> {
  /** URL of the .wasm binary when known; preloaded by useWasm. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, undefined before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies listener when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}
```

`subscribe` et `get` sont ce que `useWasm()` passe au `useSyncExternalStore` de React. Vous pouvez vous en servir pour connecter un module à une autre bibliothèque de gestion d'état.
