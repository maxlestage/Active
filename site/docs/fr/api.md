---
title: Référence de l'API
description: Chaque export d'active-react avec sa signature TypeScript, ses props, ses valeurs par défaut et son comportement, plus un résumé du crate Rust active.
---

active-react a deux points d'entrée :

| Point d'entrée | Exports |
| --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` |
| `active-react/server` | `renderWithHead` |

## Seo

```ts
function Seo(props: SeoProps): JSX.Element;
```

Rend les balises SEO d'une page : `<title>`, meta description, robots, canonical, alternates hreflang, Open Graph, Twitter et JSON-LD. React 19 remonte les balises `<title>`, `<meta>` et `<link>` dans le `<head>`, côté serveur comme dans le navigateur. Le `<script>` JSON-LD reste là où le composant est rendu.

N'utilise ni hook, ni contexte, ni effet : il peut donc être rendu dans des Server Components. Rendez un seul `<Seo>` par page. Guide : [le composant `<Seo>`](../seo-component/).

## SeoProps

```ts
interface SeoProps {
  title?: string;
  titleTemplate?: string;
  description?: string;
  canonical?: string;
  image?: string;
  type?: string;
  siteName?: string;
  locale?: string;
  twitter?: string;
  baseUrl?: string;
  noindex?: boolean;
  nofollow?: boolean;
  alternates?: Record<string, string>;
  jsonLd?: object | object[];
  meta?: Array<{ name?: string; property?: string; content: string }>;
}
```

| Prop | Valeur par défaut | Description |
| --- | --- | --- |
| `title` | aucune | Titre de la page. `titleTemplate` lui est appliqué pour `<title>` ; utilisé tel quel pour `og:title`. Se rabat sur `siteName` |
| `titleTemplate` | aucune | Template dans lequel le premier `%s` est remplacé par `title`, par exemple `"%s · Acme"`. Inutilisé sans `title` |
| `description` | aucune | `<meta name="description">` et `og:description` |
| `canonical` | aucune | `<link rel="canonical">` et `og:url`. Les chemins relatifs sont résolus avec `baseUrl` |
| `image` | aucune | `og:image`. Les chemins relatifs sont résolus avec `baseUrl`. Fait passer `twitter:card` à `summary_large_image` |
| `type` | `"website"` | `og:type` |
| `siteName` | aucune | `og:site_name`, et le titre lorsque `title` est absent |
| `locale` | aucune | `og:locale`, par exemple `"en_US"` |
| `twitter` | aucune | `twitter:site`, par exemple `"@acme"` |
| `baseUrl` | aucune | Origine (ou préfixe) des URL relatives de `canonical`, `image` et `alternates`, par exemple `"https://acme.com"` |
| `noindex` | `false` | Ajoute `noindex` à `<meta name="robots">` |
| `nofollow` | `false` | Ajoute `nofollow` à `<meta name="robots">` |
| `alternates` | aucune | Alternates hreflang : `{ en: "/en", fr: "/fr", "x-default": "/en" }` |
| `jsonLd` | aucune | Données structurées JSON-LD, un objet ou un tableau, sérialisées avec `<` échappé |
| `meta` | aucune | Balises meta supplémentaires : `[{ name: "theme-color", content: "#000" }]` |

Toujours rendues : `og:type` et `twitter:card` (`summary`, ou `summary_large_image` avec une image). `<meta name="robots">` n'apparaît que lorsque `noindex` ou `nofollow` vaut true.

Résolution des URL : les URL avec un schéma (`https:`, `mailto:`…) ou commençant par `//` sont conservées ; sinon, `baseUrl`, privée d'une éventuelle barre oblique finale, est ajoutée devant, avec un `/` entre les deux quand le chemin ne commence pas par une barre oblique.

## createSeo

```ts
function createSeo(defaults: SeoProps): (props: SeoProps) => ReactNode;
```

Renvoie un composant `<Seo>` doté de valeurs par défaut communes à tout le site. Les props de la page sont fusionnées par-dessus les valeurs par défaut avec un spread superficiel (`{ ...defaults, ...props }`) : les props de type objet ou tableau remplacent la valeur par défaut, et un `undefined` explicite la supprime.

```ts
export const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
```

Guide : [valeurs par défaut du site](../site-defaults/).

## wasm

```ts
function wasm<T>(source: () => Promise<T>, options?: { url?: string }): Wasm<T>;

function wasm<T = WebAssembly.Exports>(
  source: string | URL,
  options?: { imports?: WebAssembly.Imports },
): Wasm<T>;
```

Déclare un module WebAssembly. **Rien n'est téléchargé** avant `load()` ou `useWasm()`.

- **Surcharge avec loader**, pour le résultat de wasm-bindgen ou wasm-pack : `wasm(() => import("./pkg/my_crate.js"), { url })`. Si le module a une fonction comme export par défaut (`--target web`), elle est appelée une fois avec `{ module_or_path: url }`, ou sans argument quand `url` est absente. Sinon (`--target bundler`, ou un simple objet), le module est utilisé tel quel. `url` est l'URL publique du binaire `.wasm` et active le préchargement.
- **Surcharge avec URL**, pour un module brut sans code de liaison : `wasm<Exports>("/my_crate.wasm", { imports })`. Le fichier est téléchargé et compilé avec `WebAssembly.instantiateStreaming` quand la réponse a le type de contenu `application/wasm`, ou sinon avec `WebAssembly.instantiate` sur ses octets. Une réponse hors de la plage 2xx est rejetée avec `wasm: <url> responded <status>`. Se résout avec les exports de l'instance.

Guides : [charger WebAssembly](../loading-wasm/), [WebAssembly brut](../raw-wasm/).

## Wasm

```ts
interface Wasm<T> {
  /** URL of the `.wasm` binary when known; preloaded from `<head>` by `useWasm`. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, `undefined` before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies `listener` when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}
```

| Membre | Comportement |
| --- | --- |
| `url` | L'option `url` de la surcharge avec loader, ou l'URL passée à la surcharge avec URL |
| `load()` | Lance le chargement au premier appel. Les appels suivants renvoient la même promesse. Après un échec, l'appel suivant lance une nouvelle tentative |
| `get()` | Lecture synchrone : les exports, `undefined` tant que le module n'est pas chargé, ou lève l'erreur de chargement |
| `subscribe(listener)` | Appelle `listener` quand le chargement réussit ou échoue, et lance le chargement si nécessaire. Renvoie une fonction de désabonnement |

## useWasm

```ts
function useWasm<T>(mod: Wasm<T>): T | undefined;
```

Renvoie les exports du module, ou `undefined` pendant le chargement. Repose sur `useSyncExternalStore` :

- le premier composant qui utilise un module lance son chargement ; tous les composants partagent le même chargement ;
- il renvoie toujours `undefined` pendant le rendu serveur et l'hydratation, pour que les balisages serveur et client correspondent ;
- quand le chargement échoue, l'erreur est levée pendant le rendu, vers l'error boundary la plus proche ; appelez `mod.load()` pour réessayer ;
- tant que le module n'est pas chargé et que `mod.url` est connue, il appelle le `preload(url, { as: "fetch", crossOrigin: "anonymous", type: "application/wasm" })` de React DOM. Pendant le rendu serveur, cela émet `<link rel="preload" as="fetch" crossorigin type="application/wasm">` dans le `<head>`.

Pour Suspense, utilisez le `use(mod.load())` de React dans un composant qui n'est rendu que dans le navigateur.

## renderWithHead

```ts
import { renderWithHead } from "active-react/server";

function renderWithHead(node: ReactNode): { head: string; html: string };
```

Pour les templates HTML dans lesquels React ne rend pas lui-même `<html>`, comme le SSR de Vite avec les emplacements `<!--app-head-->` et `<!--app-html-->`, ou la génération statique. Rend `node` avec `renderToString` et découpe le résultat :

- `head` : les balises remontées par React (title, meta, link, preloads), à insérer dans le `<head>` du template ;
- `html` : le reste du balisage, à insérer dans le conteneur de l'app.

```tsx
const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

Synchrone : les frontières Suspense encore en attente rendent leur fallback. Si votre arbre rend `<html>`, utilisez directement `renderToString` ou `renderToPipeableStream` de `react-dom/server`. Guide : [rendu serveur](../server-rendering/).

## Crate Rust

Le crate `active` ([crates.io](https://crates.io/crates/active)) génère les mêmes balises depuis Rust :

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head: String = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

| Élément | Description |
| --- | --- |
| `Seo::new()` | Builder vide ; `Seo` implémente aussi `Default`, `Clone`, `Debug`, `PartialEq`, `Eq` et `Hash` |
| `title`, `title_template`, `description`, `canonical`, `image`, `og_type`, `site_name`, `locale`, `twitter`, `base_url` | Setters qui prennent un `impl Into<String>` |
| `noindex(bool)`, `nofollow(bool)` | Indicateurs robots |
| `alternate(lang, url)` | Ajoute ou remplace un alternate hreflang |
| `json_ld(json)` | Ajoute un script JSON-LD à partir d'une chaîne JSON sérialisée |
| `meta_name(name, content)`, `meta_property(property, content)` | Balises meta supplémentaires |
| `to_html(&self) -> String` | Rend les balises |
| `write_html(&self, out: &mut String)` | Ajoute les balises à la fin de `out` |
| `impl Display` | `format!("{seo}")` rend les balises |

Guide : [le crate Rust](../rust-crate/). Documentation complète : [docs.rs/active](https://docs.rs/active).
