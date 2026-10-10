---
title: Le composant <Seo>
description: Rendez titre, description, canonical, Open Graph, Twitter, hreflang, robots et JSON-LD avec un seul composant, que React 19 remonte dans le head.
---

`<Seo>` rend toutes les balises dont une page a besoin pour les moteurs de recherche et les réseaux sociaux. Vous le rendez n'importe où dans votre page, et React 19 déplace les balises dans le `<head>`, côté serveur comme dans le navigateur.

## Utilisation de base

```tsx
import { Seo } from "active-react";

export function ProductPage() {
  return (
    <main>
      <Seo
        title="Oslo chair"
        titleTemplate="%s · Acme"
        description="Solid oak chair, delivered in 48 hours."
        canonical="https://acme.com/products/oslo"
        image="https://acme.com/img/oslo.jpg"
        type="product"
        siteName="Acme"
      />
      <h1>Oslo chair</h1>
    </main>
  );
}
```

Dans une vraie app, vous passez rarement `titleTemplate`, `siteName` ou des URL complètes sur chaque page. Écrivez-les une fois avec [`createSeo`](../site-defaults/) et gardez chaque page concise :

```tsx
// src/seo.ts
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  image: "/og.png",
  locale: "en_US",
  twitter: "@acme",
});
```

## HTML généré

Avec les valeurs par défaut ci-dessus, cette page :

```tsx
<Seo
  title="Oslo chair"
  description="Solid oak chair, delivered in 48 hours."
  canonical="/products/oslo"
  image="/img/oslo.jpg"
  type="product"
  alternates={{ en: "/products/oslo", fr: "/fr/produits/oslo", "x-default": "/products/oslo" }}
  jsonLd={{ "@context": "https://schema.org", "@type": "Product", name: "Oslo chair" }}
  meta={[{ name: "theme-color", content: "#0f172a" }]}
/>
```

produit, dans le `<head>` :

```html
<title>Oslo chair · Acme</title>
<meta name="description" content="Solid oak chair, delivered in 48 hours."/>
<link rel="canonical" href="https://acme.com/products/oslo"/>
<link rel="alternate" hrefLang="en" href="https://acme.com/products/oslo"/>
<link rel="alternate" hrefLang="fr" href="https://acme.com/fr/produits/oslo"/>
<link rel="alternate" hrefLang="x-default" href="https://acme.com/products/oslo"/>
<meta property="og:title" content="Oslo chair"/>
<meta property="og:description" content="Solid oak chair, delivered in 48 hours."/>
<meta property="og:type" content="product"/>
<meta property="og:url" content="https://acme.com/products/oslo"/>
<meta property="og:image" content="https://acme.com/img/oslo.jpg"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
<meta name="theme-color" content="#0f172a"/>
```

Le script JSON-LD reste là où `<Seo>` est rendu, dans le `<body>`, ce que les moteurs de recherche acceptent (voir [données structurées](../structured-data/)) :

```html
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"Oslo chair"}</script>
```

Les balises apparaissent toujours dans cet ordre. Une balise dont la valeur manque est omise, sauf `og:type` et `twitter:card`, qui sont toujours présentes.

## Props

Toutes les props sont facultatives.

| Prop | Type | Produit |
| --- | --- | --- |
| `title` | `string` | `<title>` (avec le template) et `og:title` (sans) |
| `titleTemplate` | `string` | Template du `<title>` : `%s` est remplacé par `title` |
| `description` | `string` | `<meta name="description">` et `og:description` |
| `canonical` | `string` | `<link rel="canonical">` et `og:url` |
| `image` | `string` | `og:image` ; fait passer `twitter:card` à `summary_large_image` |
| `type` | `string` | `og:type`, `"website"` par défaut |
| `siteName` | `string` | `og:site_name` ; le titre lorsque `title` est absent |
| `locale` | `string` | `og:locale`, par exemple `"en_US"` |
| `twitter` | `string` | `twitter:site`, par exemple `"@acme"` |
| `baseUrl` | `string` | Préfixe des URL relatives de `canonical`, `image` et `alternates` |
| `noindex` | `boolean` | `<meta name="robots">` avec `noindex` |
| `nofollow` | `boolean` | `<meta name="robots">` avec `nofollow` |
| `alternates` | `Record<string, string>` | Un `<link rel="alternate" hrefLang>` par langue |
| `jsonLd` | `object \| object[]` | `<script type="application/ld+json">` |
| `meta` | `Array<{ name?, property?, content }>` | Balises `<meta>` supplémentaires |

### Titre et valeur de repli

Le `<title>` est construit ainsi :

1. `title` et `titleTemplate` sont définis : le premier `%s` du template est remplacé par le titre (`"%s · Acme"` donne `Oslo chair · Acme`).
2. Seul `title` est défini : il est utilisé tel quel.
3. Pas de `title` : `siteName` est utilisé tel quel, sans le template. Pratique pour une page d'accueil.
4. Ni l'un ni l'autre : pas de balise `<title>`.

`og:title` suit la même logique mais n'utilise jamais le template : les cartes sociales affichent déjà le nom du site à côté du titre.

### Carte Twitter

`twitter:card` vaut `summary_large_image` quand il y a une `image`, et `summary` sinon. X (Twitter) lit aussi les balises `og:*` : aucune autre balise `twitter:*` n'est nécessaire.

### Robots

`<meta name="robots">` n'apparaît que lorsque `noindex` ou `nofollow` vaut true :

| Props | `content` |
| --- | --- |
| `noindex` | `noindex,follow` |
| `nofollow` | `index,nofollow` |
| `noindex` et `nofollow` | `noindex,nofollow` |
| Aucune des deux | Pas de balise (par défaut, les moteurs de recherche indexent et suivent les liens) |

### Balises meta supplémentaires

`meta` est une porte de sortie pour toute balise que `<Seo>` ne couvre pas. Utilisez `name` pour les balises meta classiques et `property` pour celles d'Open Graph :

```tsx
<Seo
  title="Release notes"
  type="article"
  meta={[
    { name: "theme-color", content: "#0f172a" },
    { name: "author", content: "Ada Lovelace" },
    { property: "article:published_time", content: "2026-10-01T09:00:00Z" },
    { property: "og:image:alt", content: "The Acme logo on a dark background" },
  ]}
/>
```

## Règles pratiques

### Un seul `<Seo>` par page

Rendez un seul `<Seo>` par page, dans le composant de la page. Deux `<Seo>` montés en même temps produisent des balises en double : React remonte tout ce qu'on lui donne et ne fusionne pas les balises. Les valeurs partagées par toutes les pages ont leur place dans [`createSeo`](../site-defaults/), pas dans un second `<Seo>` placé dans votre layout.

### Compatible avec les Server Components

`<Seo>` n'utilise ni hook, ni contexte, ni effet : c'est une simple fonction qui renvoie des balises. Il fonctionne aussi bien dans les composants client que dans les composants rendus côté serveur et dans les React Server Components.

### La navigation côté client est automatique

Quand la route change, l'ancienne page est démontée et React retire ses balises ; la nouvelle page est montée et React insère les siennes. Les benchmarks ont mesuré une médiane de 0,3 ms entre le rendu d'une nouvelle page et la mise à jour du `<head>`. Il n'y a rien à configurer.

### Balises statiques dans `index.html`

- Un `<title>` statique dans `index.html` peut rester : React insère le `<title>` de la page devant lui, donc `document.title` est correct.
- Retirez les balises statiques `description` et `og:*` de `index.html`. React ne les dédoublonne pas : les robots d'indexation verraient deux descriptions.

### Données issues d'une API

`<Seo>` est un composant comme les autres : il peut donc utiliser les données que votre page a déjà chargées.

```tsx
function ProductPage({ product }: { product: Product }) {
  return (
    <>
      <Seo title={product.name} description={product.summary} canonical={`/products/${product.slug}`} />
      <h1>{product.name}</h1>
    </>
  );
}
```

Pour que les robots d'indexation qui n'exécutent pas JavaScript voient ces balises, rendez la page côté serveur : voir [rendu serveur](../server-rendering/).
