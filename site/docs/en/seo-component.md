---
title: The <Seo> component
description: Render title, description, canonical, Open Graph, Twitter, hreflang, robots and JSON-LD tags with one component that React 19 hoists into the head.
---

`<Seo>` renders every tag a page needs for search engines and social networks. You render it anywhere in your page, and React 19 moves the tags into the `<head>`, on the server and in the browser.

## Basic usage

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

In a real app you rarely pass `titleTemplate`, `siteName` or full URLs on every page. Write them once with [`createSeo`](../site-defaults/) and keep each page short:

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

## Generated HTML

With the defaults above, this page:

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

produces, in the `<head>`:

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

The JSON-LD script stays where `<Seo>` is rendered, in the `<body>`, which search engines accept (see [structured data](../structured-data/)):

```html
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"Oslo chair"}</script>
```

Tags always come in this order. A tag whose value is missing is left out, except `og:type` and `twitter:card`, which are always present.

## Props

Every prop is optional.

| Prop | Type | Produces |
| --- | --- | --- |
| `title` | `string` | `<title>` (with the template) and `og:title` (without) |
| `titleTemplate` | `string` | Template for `<title>`: `%s` is replaced by `title` |
| `description` | `string` | `<meta name="description">` and `og:description` |
| `canonical` | `string` | `<link rel="canonical">` and `og:url` |
| `image` | `string` | `og:image`; switches `twitter:card` to `summary_large_image` |
| `type` | `string` | `og:type`, `"website"` by default |
| `siteName` | `string` | `og:site_name`; the title when `title` is missing |
| `locale` | `string` | `og:locale`, for example `"en_US"` |
| `twitter` | `string` | `twitter:site`, for example `"@acme"` |
| `baseUrl` | `string` | Prefix for relative `canonical`, `image` and `alternates` URLs |
| `noindex` | `boolean` | `<meta name="robots">` with `noindex` |
| `nofollow` | `boolean` | `<meta name="robots">` with `nofollow` |
| `alternates` | `Record<string, string>` | One `<link rel="alternate" hrefLang>` per language |
| `jsonLd` | `object \| object[]` | `<script type="application/ld+json">` |
| `meta` | `Array<{ name?, property?, content }>` | Extra `<meta>` tags |

### Title and fallback

The `<title>` is built like this:

1. `title` and `titleTemplate` are set: the first `%s` in the template is replaced by the title (`"%s · Acme"` gives `Oslo chair · Acme`).
2. Only `title` is set: it is used as is.
3. No `title`: `siteName` is used as is, without the template. Handy for a home page.
4. Neither: no `<title>` tag.

`og:title` follows the same logic but never uses the template: social cards already show the site name next to the title.

### Twitter card

`twitter:card` is `summary_large_image` when there is an `image`, and `summary` otherwise. X (Twitter) also reads the `og:*` tags, so no other `twitter:*` tag is needed.

### Robots

`<meta name="robots">` only appears when `noindex` or `nofollow` is true:

| Props | `content` |
| --- | --- |
| `noindex` | `noindex,follow` |
| `nofollow` | `index,nofollow` |
| `noindex` and `nofollow` | `noindex,nofollow` |
| Neither | No tag (search engines index and follow by default) |

### Extra meta tags

`meta` is an escape hatch for any tag `<Seo>` doesn't cover. Use `name` for regular meta tags and `property` for Open Graph ones:

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

## Rules of thumb

### One `<Seo>` per page

Render a single `<Seo>` per page, in the page component. Two `<Seo>` mounted at the same time produce duplicate tags: React hoists everything it is given and does not merge tags. Values shared by all pages belong in [`createSeo`](../site-defaults/), not in a second `<Seo>` in your layout.

### Works in Server Components

`<Seo>` uses no hook, no context and no effect: it is a plain function returning tags. It works in client components, in server-rendered components and in React Server Components alike.

### Client navigation is automatic

When the route changes, the old page unmounts and React removes its tags; the new page mounts and React inserts its own. The benchmarks measured a median of 0.3 ms between rendering a new page and the `<head>` being up to date. There is nothing to configure.

### Static tags in `index.html`

- A static `<title>` in `index.html` can stay: React inserts the page's `<title>` before it, so `document.title` is correct.
- Remove static `description` and `og:*` tags from `index.html`. React does not deduplicate them, so crawlers would see two descriptions.

### Data from an API

`<Seo>` is a component like any other, so it can use the data your page already loaded:

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

For crawlers that don't run JavaScript to see these tags, render the page on the server: see [server rendering](../server-rendering/).
