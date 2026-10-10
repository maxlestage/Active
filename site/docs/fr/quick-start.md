---
title: Démarrage rapide
description: Créez en cinq minutes environ une app React 19 avec des balises SEO complètes et une fonction Rust compilée en WebAssembly, avec Vite et wasm-pack.
---

Dans ce tutoriel, vous allez créer une app Vite + React 19, lui donner de vraies balises SEO, puis appeler une fonction Rust compilée en WebAssembly. Comptez environ cinq minutes si Rust est déjà installé.

## 1. Créer l'app

```sh
npm create vite@latest my-app -- --template react-ts
cd my-app
npm install
npm i github:maxlestage/Active
```

Le template React de Vite utilise React 19, dont active-react a besoin. Vous pouvez le vérifier avec `npm ls react`.

## 2. Écrire les valeurs par défaut du site

Créez `src/seo.ts`. Ces valeurs sont partagées par toutes les pages : vous ne les écrivez qu'une fois.

```ts
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "My App",
  titleTemplate: "%s · My App",
  baseUrl: "https://my-app.example", // relative URLs become absolute
  image: "/og.png",
  locale: "en_US",
});
```

`createSeo` renvoie un composant `<Seo>` qui intègre ces valeurs par défaut. Pour en savoir plus, consultez [les valeurs par défaut du site](../site-defaults/).

## 3. Ajouter le SEO à une page

Remplacez `src/App.tsx` :

```tsx
import { Seo } from "./seo";

export default function App() {
  return (
    <main>
      <Seo title="Home" description="A React 19 app with SEO tags and Rust." canonical="/" />
      <h1>Hello</h1>
    </main>
  );
}
```

Lancez le serveur de développement avec `npm run dev` et inspectez la page avec les outils de développement de votre navigateur. Le `<head>` contient maintenant aussi :

```html
<title>Home · My App</title>
<meta name="description" content="A React 19 app with SEO tags and Rust."/>
<link rel="canonical" href="https://my-app.example/"/>
<meta property="og:title" content="Home"/>
<meta property="og:description" content="A React 19 app with SEO tags and Rust."/>
<meta property="og:type" content="website"/>
<meta property="og:url" content="https://my-app.example/"/>
<meta property="og:image" content="https://my-app.example/og.png"/>
<meta property="og:site_name" content="My App"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
```

Vous avez rendu `<Seo>` à l'intérieur de `<main>`, et pourtant les balises sont dans le `<head>` : React 19 les y a déplacées.

> **Remarque :** `index.html` contient toujours un `<title>` statique. React insère son propre titre devant, si bien que l'onglet du navigateur affiche le bon. Vous pouvez supprimer le titre statique si vous préférez un seul élément.

## 4. Créer un crate Rust

Installez les outils une fois pour toutes (voir [Configurer Rust](../rust-setup/) pour les détails) :

```sh
# Rust itself: https://rustup.rs
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Créez ensuite un crate de bibliothèque à côté de `src/` :

```sh
cargo new --lib hello --vcs none
```

Remplacez `hello/Cargo.toml` :

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Puis `hello/src/lib.rs` :

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello {name}, from Rust!")
}

/// Number of primes below `n` (sieve of Eratosthenes).
#[wasm_bindgen]
pub fn count_primes(n: u32) -> u32 {
    let n = n as usize;
    if n < 2 {
        return 0;
    }
    let mut sieve = vec![true; n];
    sieve[0] = false;
    sieve[1] = false;
    let mut i = 2;
    while i * i < n {
        if sieve[i] {
            let mut j = i * i;
            while j < n {
                sieve[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    sieve.iter().filter(|&&p| p).count() as u32
}
```

## 5. Le compiler en WebAssembly

```sh
wasm-pack build hello --target web --out-dir ../src/pkg
```

Le chemin `--out-dir` est relatif au crate : le résultat arrive donc dans `src/pkg/` :

- `hello.js` : le code JavaScript de liaison généré par wasm-bindgen ;
- `hello_bg.wasm` : le Rust compilé ;
- `hello.d.ts` : les types TypeScript de vos fonctions.

Enregistrez la commande comme script dans `package.json` et lancez-la avant chaque build :

```json
{
  "scripts": {
    "wasm": "wasm-pack build hello --target web --out-dir ../src/pkg",
    "build": "npm run wasm && tsc -b && vite build"
  }
}
```

## 6. Déclarer le module

Créez `src/wasm.ts` :

```ts
import { wasm } from "active-react";
import url from "./pkg/hello_bg.wasm?url";

export const hello = wasm(() => import("./pkg/hello.js"), { url });
```

Rien n'est encore téléchargé : `wasm()` se contente de déclarer le module. L'import `?url` demande à Vite l'URL finale du binaire, ce qui permet à active-react de le précharger pendant le rendu serveur.

## 7. Appeler Rust depuis un composant

Modifiez `src/App.tsx` :

```tsx
import { useWasm } from "active-react";
import { Seo } from "./seo";
import { hello } from "./wasm";

export default function App() {
  const rust = useWasm(hello); // undefined while loading

  return (
    <main>
      <Seo title="Home" description="A React 19 app with SEO tags and Rust." canonical="/" />
      <h1>Hello</h1>
      <p>{rust ? rust.greet("you") : "Loading the Rust module…"}</p>
      <p>{rust && `${rust.count_primes(1_000_000)} primes below one million`}</p>
    </main>
  );
}
```

`rust` est entièrement typé grâce à `hello.d.ts` : faites une faute de frappe dans `count_primes` et TypeScript la détectera.

## 8. Lancer l'app

```sh
npm run dev
```

La page affiche d'abord « Loading the Rust module… », puis le message de bienvenue et 78 498 nombres premiers. Lancez `npm run build` puis `npm run preview` pour essayer le build de production.

## Pour aller plus loin

Votre app est une single-page app (SPA) : les balises SEO sont ajoutées par JavaScript dans le navigateur. Google exécute JavaScript, mais beaucoup de robots d'indexation et de bots d'aperçu de liens ne le font pas. Pour un SEO optimal, pré-rendez vos pages en HTML :

- [Rendu serveur](../server-rendering/) explique le SSR, la génération statique et `renderWithHead`.
- L'[exemple vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg) du dépôt est le même genre d'app, avec la génération statique en plus : chaque page est livrée en HTML avec son `<head>` rempli et le binaire `.wasm` préchargé, puis s'hydrate et appelle Rust.
- [Charger WebAssembly](../loading-wasm/) traite des erreurs, de Suspense et du chargement en dehors de React.
