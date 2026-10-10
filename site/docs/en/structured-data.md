---
title: Structured data (JSON-LD)
description: Add schema.org JSON-LD to any page with the jsonLd prop to describe products, articles, breadcrumbs and more, and unlock rich search results.
---

Structured data describes what a page is about in a format machines understand: this page is a product that costs 249 euros, that one is an article written by Ada on October 1st. Search engines use it to show **rich results**: prices and stock, breadcrumbs, article dates, site names and more.

The standard format is **JSON-LD**: a JSON object using the [schema.org](https://schema.org) vocabulary, placed in a `<script type="application/ld+json">` tag.

## The jsonLd prop

Pass an object to `jsonLd`:

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

Or an array, when the page describes several things. It is rendered as one script containing a JSON array, which search engines support:

```tsx
<Seo title="Oslo chair" jsonLd={[productLd, breadcrumbLd]} />
```

Pass plain objects: active-react serializes them with `JSON.stringify` and escapes them for you.

## Recipes

Copy, adapt and validate. Every recipe needs `"@context": "https://schema.org"`.

### Organization

On your home page, describe who you are. Search engines may use your logo and social profiles.

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

Tells search engines the name of your site, which can appear above your results.

```tsx
const website = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  name: "Acme",
  url: "https://acme.com",
};

<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" jsonLd={[organization, website]} />;
```

### Product with Offer

Price, currency and availability can show up directly in search results.

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

> **Note:** `baseUrl` does not touch JSON-LD: write absolute URLs inside your structured data.

### Article

Use `Article`, or the more specific `BlogPosting` or `NewsArticle`.

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

Shows the page's position in your site instead of a raw URL.

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

The last item, the current page, doesn't need an `item` URL.

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

> **Note:** Google currently shows FAQ rich results for a limited set of authoritative sites only. The markup stays valid and other consumers can still read it, but don't count on the visual result.

## Typing your data

`jsonLd` accepts any object. If you want autocompletion and checks for schema.org types, the `schema-dts` package from Google provides TypeScript definitions:

```ts
import type { Product, WithContext } from "schema-dts";

const productLd: WithContext<Product> = {
  "@context": "https://schema.org",
  "@type": "Product",
  name: "Oslo chair",
};
```

It is a types-only package: it adds nothing to your bundle.

## Security

Structured data often contains user content: product names, reviews, article titles. If a value contained `</script>`, a naive serializer would close the script tag early and let the rest run as HTML.

active-react escapes every `<` as `\u003c` inside the JSON. It is still exactly the same JSON for parsers, but it can never close the `<script>` tag:

```html
<script type="application/ld+json">{"@type":"Thing","name":"\u003c/script>\u003cscript>alert(1)\u003c/script>"}</script>
```

Always pass objects, never pre-built JSON strings, so the escaping applies.

## Where it is rendered

React 19 hoists `<title>`, `<meta>` and `<link>` into the `<head>`, but not regular scripts. The JSON-LD `<script>` therefore stays where `<Seo>` is rendered, inside the `<body>`. Google reads JSON-LD anywhere in the document, so this is fine.

With server rendering, the script is part of the HTML the crawler receives. With `renderWithHead`, it is in the `html` part, not the `head` part. See [server rendering](../server-rendering/).

## Validate

Before shipping, test your pages:

- [Google Rich Results Test](https://search.google.com/test/rich-results): shows which rich results a page is eligible for. Paste a public URL, or paste the HTML of a local page.
- [Schema Markup Validator](https://validator.schema.org): checks the markup against the whole schema.org vocabulary.

> **Tip:** After launch, the rich result reports in Google Search Console list structured data errors across your whole site.
