---
title: Données structurées (JSON-LD)
description: Ajoutez du JSON-LD schema.org à toute page avec la prop jsonLd pour décrire produits, articles ou fils d'Ariane, et obtenir des résultats enrichis.
---

Les données structurées décrivent le sujet d'une page dans un format que les machines comprennent : cette page est un produit qui coûte 249 euros, celle-ci est un article écrit par Ada le 1er octobre. Les moteurs de recherche s'en servent pour afficher des **résultats enrichis** : prix et stock, fils d'Ariane, dates d'articles, noms de sites, etc.

Le format standard est **JSON-LD** : un objet JSON qui utilise le vocabulaire [schema.org](https://schema.org), placé dans une balise `<script type="application/ld+json">`.

## La prop jsonLd

Passez un objet à `jsonLd` :

```tsx
<Seo
  title="Oslo chair"
  jsonLd={{
    "@context": "https://schema.org",
    "@type": "Product",
    name: "Oslo chair",
  }}
/>
```

Ou un tableau, quand la page décrit plusieurs choses. Il est rendu sous la forme d'un seul script contenant un tableau JSON, ce que les moteurs de recherche prennent en charge :

```tsx
<Seo title="Oslo chair" jsonLd={[productLd, breadcrumbLd]} />
```

Passez de simples objets : active-react les sérialise avec `JSON.stringify` et les échappe pour vous.

## Recettes

Copiez, adaptez et validez. Chaque recette a besoin de `"@context": "https://schema.org"`.

### Organization

Sur votre page d'accueil, décrivez qui vous êtes. Les moteurs de recherche peuvent utiliser votre logo et vos profils sociaux.

```tsx
const organization = {
  "@context": "https://schema.org",
  "@type": "Organization",
  name: "Acme",
  url: "https://acme.com",
  logo: "https://acme.com/logo.png",
  sameAs: ["https://github.com/acme", "https://www.linkedin.com/company/acme"],
};
```

### WebSite

Indique aux moteurs de recherche le nom de votre site, qui peut apparaître au-dessus de vos résultats.

```tsx
const website = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  name: "Acme",
  url: "https://acme.com",
};

<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" jsonLd={[organization, website]} />;
```

### Product avec Offer

Le prix, la devise et la disponibilité peuvent apparaître directement dans les résultats de recherche.

```tsx
function ProductSeo({ product }: { product: Product }) {
  return (
    <Seo
      title={product.name}
      description={product.summary}
      canonical={`/products/${product.slug}`}
      image={product.image}
      type="product"
      jsonLd={{
        "@context": "https://schema.org",
        "@type": "Product",
        name: product.name,
        description: product.summary,
        image: `https://acme.com${product.image}`,
        sku: product.sku,
        brand: { "@type": "Brand", name: "Acme" },
        offers: {
          "@type": "Offer",
          price: product.price,
          priceCurrency: "EUR",
          availability: "https://schema.org/InStock",
          url: `https://acme.com/products/${product.slug}`,
        },
      }}
    />
  );
}
```

> **Remarque :** `baseUrl` ne touche pas au JSON-LD : écrivez des URL absolues dans vos données structurées.

### Article

Utilisez `Article`, ou les types plus spécifiques `BlogPosting` ou `NewsArticle`.

```tsx
<Seo
  title={post.title}
  description={post.excerpt}
  canonical={`/blog/${post.slug}`}
  image={post.cover}
  type="article"
  jsonLd={{
    "@context": "https://schema.org",
    "@type": "BlogPosting",
    headline: post.title,
    image: [`https://acme.com${post.cover}`],
    datePublished: post.publishedAt, // "2026-10-01T09:00:00+02:00"
    dateModified: post.updatedAt,
    author: [{ "@type": "Person", name: post.author, url: post.authorUrl }],
  }}
/>
```

### BreadcrumbList

Affiche la position de la page dans votre site au lieu d'une URL brute.

```tsx
const breadcrumb = {
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  itemListElement: [
    { "@type": "ListItem", position: 1, name: "Shop", item: "https://acme.com/shop" },
    { "@type": "ListItem", position: 2, name: "Chairs", item: "https://acme.com/shop/chairs" },
    { "@type": "ListItem", position: 3, name: "Oslo chair" },
  ],
};
```

Le dernier élément, la page courante, n'a pas besoin d'URL `item`.

### FAQPage

```tsx
const faq = {
  "@context": "https://schema.org",
  "@type": "FAQPage",
  mainEntity: [
    {
      "@type": "Question",
      name: "How long does delivery take?",
      acceptedAnswer: { "@type": "Answer", text: "48 hours in mainland France." },
    },
    {
      "@type": "Question",
      name: "Can I return a chair?",
      acceptedAnswer: { "@type": "Answer", text: "Yes, within 30 days, for free." },
    },
  ],
};
```

> **Remarque :** Google n'affiche actuellement les résultats enrichis FAQ que pour un nombre restreint de sites faisant autorité. Le balisage reste valide et d'autres consommateurs peuvent toujours le lire, mais ne comptez pas sur le rendu visuel.

## Typer vos données

`jsonLd` accepte n'importe quel objet. Si vous voulez l'autocomplétion et des vérifications pour les types schema.org, le paquet `schema-dts` de Google fournit des définitions TypeScript :

```ts
import type { Product, WithContext } from "schema-dts";

const productLd: WithContext<Product> = {
  "@context": "https://schema.org",
  "@type": "Product",
  name: "Oslo chair",
};
```

C'est un paquet qui ne contient que des types : il n'ajoute rien à votre bundle.

## Sécurité

Les données structurées contiennent souvent du contenu utilisateur : noms de produits, avis, titres d'articles. Si une valeur contenait `</script>`, un sérialiseur naïf fermerait la balise script trop tôt et laisserait le reste s'exécuter comme du HTML.

active-react échappe chaque `<` en `\u003c` dans le JSON. Pour les parseurs, c'est toujours exactement le même JSON, mais il ne peut jamais fermer la balise `<script>` :

```html
<script type="application/ld+json">{"@type":"Thing","name":"\u003c/script>\u003cscript>alert(1)\u003c/script>"}</script>
```

Passez toujours des objets, jamais des chaînes JSON préconstruites, pour que l'échappement s'applique.

## Où il est rendu

React 19 remonte `<title>`, `<meta>` et `<link>` dans le `<head>`, mais pas les scripts ordinaires. Le `<script>` JSON-LD reste donc là où `<Seo>` est rendu, dans le `<body>`. Google lit le JSON-LD n'importe où dans le document : ce n'est donc pas un problème.

Avec le rendu serveur, le script fait partie du HTML que reçoit le robot d'indexation. Avec `renderWithHead`, il se trouve dans la partie `html`, pas dans la partie `head`. Voir [rendu serveur](../server-rendering/).

## Valider

Avant la mise en production, testez vos pages :

- [Test des résultats enrichis de Google](https://search.google.com/test/rich-results) : indique à quels résultats enrichis une page est éligible. Collez une URL publique, ou le HTML d'une page locale.
- [Schema Markup Validator](https://validator.schema.org) : vérifie le balisage par rapport à l'ensemble du vocabulaire schema.org.

> **Astuce :** Après le lancement, les rapports sur les résultats enrichis de Google Search Console listent les erreurs de données structurées sur l'ensemble de votre site.
