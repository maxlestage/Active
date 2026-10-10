---
title: Rendu serveur et génération statique
description: Placez vos balises SEO et le preload WASM dans le HTML même, grâce au SSR de React 19, à un template HTML avec renderWithHead ou à la génération statique.
---

`<Seo>` fonctionne dans le navigateur seul, mais le SEO donne le meilleur de lui-même quand les balises sont déjà dans le HTML envoyé par le serveur. Cette page présente les deux façons d'y parvenir et la place de la génération statique.

## Pourquoi c'est important

- **Les robots d'indexation voient les balises immédiatement.** Google sait exécuter JavaScript, mais il rend les pages lors d'une seconde passe qui peut intervenir plus tard. Les autres moteurs de recherche exécutent moins de JavaScript, voire pas du tout.
- **Les aperçus de liens ont besoin de HTML.** Les bots qui construisent les aperçus pour Facebook, LinkedIn, X, Slack, Discord ou les messageries n'exécutent pas du tout JavaScript. Sans rendu serveur, ils ne voient que votre `index.html`.
- **Rust démarre plus tôt.** Quand le serveur rend un composant qui appelle `useWasm()`, il émet aussi un preload pour le binaire `.wasm`. Dans les benchmarks, le premier résultat Rust est apparu au bout de 224 ms au lieu de 330 ms (100 ms de latence réseau).

active-react prend en charge les deux configurations : React rend tout le document, ou React rend à l'intérieur d'un template HTML.

## Mode 1 : React rend tout le document

Si votre arbre React rend `<html>`, il n'y a rien à faire : le moteur de rendu serveur de React place pour vous les balises remontées dans le `<head>`. C'est le cas de React Router v7 (mode framework), de TanStack Start, de Waku et de la plupart des serveurs sur mesure.

```tsx
import { renderToPipeableStream } from "react-dom/server";
import { App } from "./App";

function Document({ url }: { url: string }) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
      </head>
      <body>
        <App url={url} />
      </body>
    </html>
  );
}

// In your request handler (Express, Fastify, node:http…)
const { pipe } = renderToPipeableStream(<Document url={req.url} />, {
  bootstrapModules: ["/assets/client.js"],
  onShellReady() {
    res.setHeader("content-type", "text/html; charset=utf-8");
    pipe(res);
  },
});
```

`renderToString(<Document url={url} />)` fonctionne de la même façon si vous n'avez pas besoin du streaming.

> **Astuce :** Rendez `<Seo>` dans le shell, en dehors des frontières Suspense qui attendent des données, et chargez les données de la page avant le rendu (les loaders de routes le font). Les balises arrivent alors dans les tout premiers octets du `<head>`.

## Mode 2 : template HTML

La configuration SSR de Vite et de nombreux générateurs statiques conservent un template `index.html` et ne laissent React rendre que l'app à l'intérieur. Le template comporte deux emplacements réservés :

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <!--app-head-->
  </head>
  <body>
    <div id="root"><!--app-html--></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Ici, React ne voit jamais le `<head>` : il ne peut donc pas y placer les balises. `renderWithHead`, fourni par `active-react/server`, résout ce problème : il rend votre app et renvoie séparément les balises remontées et le balisage du body.

```tsx
// src/entry-server.tsx
import { renderWithHead } from "active-react/server";
import { App } from "./App";

export function render(url: string) {
  return renderWithHead(<App url={url} />); // { head, html }
}
```

`head` contient les balises `<title>`, `<meta>` et `<link>`, y compris le preload du `.wasm`. `html` est le balisage de l'app, script JSON-LD compris.

### Un serveur SSR Vite

Un serveur Express minimal, adapté du template SSR de Vite :

```js
// server.js
import fs from "node:fs/promises";
import express from "express";

const prod = process.env.NODE_ENV === "production";
const app = express();
let vite;

if (prod) {
  app.use(express.static("dist/client", { index: false }));
} else {
  const { createServer } = await import("vite");
  vite = await createServer({ server: { middlewareMode: true }, appType: "custom" });
  app.use(vite.middlewares);
}

app.use(async (req, res, next) => {
  try {
    const url = req.originalUrl;
    let template, render;
    if (prod) {
      template = await fs.readFile("dist/client/index.html", "utf8");
      ({ render } = await import("./dist/server/entry-server.js"));
    } else {
      template = await vite.transformIndexHtml(url, await fs.readFile("index.html", "utf8"));
      ({ render } = await vite.ssrLoadModule("/src/entry-server.tsx"));
    }
    const { head, html } = render(url);
    const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
    res.status(200).set({ "Content-Type": "text/html" }).send(page);
  } catch (error) {
    vite?.ssrFixStacktrace(error);
    next(error);
  }
});

app.listen(process.env.PORT ?? 5173);
```

Construisez les deux côtés avec `vite build --outDir dist/client` et `vite build --ssr src/entry-server.tsx --outDir dist/server`.

Côté client, hydratez le balisage du serveur :

```tsx
// src/main.tsx
import { hydrateRoot } from "react-dom/client";
import { App } from "./App";

hydrateRoot(document.getElementById("root")!, <App url={location.pathname} />);
```

> **Remarque :** `renderWithHead` utilise `renderToString`, qui est synchrone : les frontières Suspense encore en attente rendent leur fallback. Chargez les données avant de l'appeler, ou utilisez le mode 1 avec le streaming.

## Génération statique

La génération de site statique (SSG) exécute la même fonction `render` au moment du build, une fois par route, et écrit un fichier HTML par page. Aucun serveur n'est nécessaire à l'exécution : n'importe quel hébergeur statique convient.

Voici le script `prerender.js` de l'[exemple vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg), dans lequel `entry-server.tsx` exporte aussi la liste des routes :

```js
// prerender.js — run after the client and SSR builds
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { render, urls } from "./dist-server/entry-server.js";

const template = readFileSync("dist/index.html", "utf8");
for (const url of urls) {
  const { head, html } = render(url);
  const file = `dist${url}index.html`;
  mkdirSync(file.slice(0, file.lastIndexOf("/")), { recursive: true });
  writeFileSync(file, template.replace("<!--app-head-->", head).replace("<!--app-html-->", html));
  console.log("prerendered", file);
}
rmSync("dist-server", { recursive: true });
```

```json
{
  "scripts": {
    "build": "vite build && vite build --ssr src/entry-server.tsx --outDir dist-server && node prerender.js"
  }
}
```

Chaque page générée contient ses propres balises SEO et le preload du `.wasm`, puis s'hydrate et appelle Rust.

### Ajouter un sitemap

Vous avez déjà la liste des routes : générez `sitemap.xml` et `robots.txt` dans le même script.

```js
const origin = "https://acme.com";
const entries = urls.map((url) => `  <url><loc>${origin}${url}</loc></url>`).join("\n");
writeFileSync(
  "dist/sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${entries}\n</urlset>\n`,
);
writeFileSync("dist/robots.txt", `User-agent: *\nAllow: /\nSitemap: ${origin}/sitemap.xml\n`);
```

Soumettez ensuite l'URL du sitemap dans Google Search Console.

## Server Components

`<Seo>` n'utilise aucun hook : il peut donc être rendu directement dans un React Server Component, par exemple avec des données récupérées sur le serveur.

```tsx
// A Server Component
import { Seo } from "../seo";
import { db } from "../db";

export default async function ProductPage({ slug }: { slug: string }) {
  const product = await db.products.find(slug);
  return (
    <main>
      <Seo title={product.name} description={product.summary} canonical={`/products/${slug}`} />
      <h1>{product.name}</h1>
    </main>
  );
}
```

`useWasm()` est un hook qui s'exécute dans le navigateur : les composants qui appellent Rust doivent donc être des composants client (`"use client"`). La déclaration du module avec `wasm()` peut se trouver dans n'importe quel fichier partagé.

## Single-page apps

Sans serveur, les balises sont ajoutées par JavaScript après le chargement de la page. Dans la plupart des cas, cela convient à Google, et le `<head>` reste synchronisé à chaque navigation. Mais les aperçus de liens et les robots d'indexation qui n'exécutent pas JavaScript ne verront que `index.html`.

Si vous ne pouvez pas faire tourner de serveur, la génération statique est généralement l'amélioration la plus simple : pré-rendez au moins les pages les plus partagées.

## Hydratation et WebAssembly

`useWasm()` renvoie toujours `undefined` sur le serveur **et pendant l'hydratation**, même si le module est déjà chargé dans le navigateur. Le premier rendu client produit donc exactement le balisage du serveur, et React ne signale jamais d'erreur d'hydratation (hydration mismatch). Juste après l'hydratation, le composant est rendu de nouveau avec les exports.

Écrivez vos composants comme si Rust pouvait ne pas être encore disponible :

```tsx
function Primes() {
  const rust = useWasm(hello);
  return <p>{rust ? `${rust.count_primes(1_000_000)} primes` : "Counting…"}</p>;
}
```

> **Attention :** Ne vous basez pas sur `typeof window` pour appeler Rust pendant le rendu. Le serveur et le premier rendu client divergeraient, et React devrait jeter le balisage du serveur.
