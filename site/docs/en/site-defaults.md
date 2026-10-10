---
title: Site defaults with createSeo
description: Write your site-wide SEO values once with createSeo, then override them per page, per section, per environment or per language.
---

Most SEO values are the same on every page: the site name, the title template, the domain, the default share image. `createSeo` lets you write them once.

## Write defaults once

Create `src/seo.ts`:

```ts
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

Then import this `Seo` instead of the one from `active-react`:

```tsx
import { Seo } from "../seo";

export function Contact() {
  return (
    <main>
      <Seo title="Contact" description="Write to us, we answer within a day." canonical="/contact" />
      <h1>Contact</h1>
    </main>
  );
}
```

`createSeo(defaults)` returns a component that renders `<Seo>` with your defaults and the page's props combined. Like `<Seo>`, it uses no hook and no context, so there is no provider to add to your app.

## Override per page

Page props win over defaults. The merge is shallow, exactly like `{ ...defaults, ...props }`:

```tsx
// Uses the default image, replaces the default type.
<Seo title="Our story" type="article" canonical="/about" />

// Replaces the default image for this page only.
<Seo title="Oslo chair" image="/img/oslo.jpg" canonical="/products/oslo" />
```

Two consequences of the shallow merge:

- Object and array props (`alternates`, `jsonLd`, `meta`) **replace** the default; they are not merged item by item.
- Passing `undefined` explicitly **unsets** a default: `<Seo image={undefined} />` renders no `og:image`.

> **Warning:** Watch out for optional data. `<Seo image={post.cover} />` removes the default image whenever `post.cover` is `undefined`. Only pass the prop when you have a value: `<Seo {...(post.cover ? { image: post.cover } : {})} />`.

## Title template

`titleTemplate` contains `%s`, which is replaced by the page title:

| Template | `title` | `<title>` |
| --- | --- | --- |
| `"%s · Acme"` | `"Contact"` | `Contact · Acme` |
| `"%s · Acme"` | none | `Acme` (the `siteName`) |
| `"%s"` | `"Acme, furniture made to last"` | `Acme, furniture made to last` |

Only the first `%s` is replaced, and the template only applies when the page has a title. `og:title` never uses the template.

### Home page without template

On the home page you often want a full custom title rather than `Home · Acme`. Override the template for that page:

```tsx
<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" />
```

Leaving out `title` also works: the page then uses the `siteName` alone.

## baseUrl and URLs

Open Graph and canonical URLs must be absolute. With `baseUrl`, you can write relative paths and let active-react complete them. It applies to `canonical` (and so `og:url`), `image` and every `alternates` URL.

| `baseUrl` | Value | Result |
| --- | --- | --- |
| `https://acme.com` | `/contact` | `https://acme.com/contact` |
| `https://acme.com/` | `/contact` | `https://acme.com/contact` |
| `https://acme.com` | `contact` | `https://acme.com/contact` |
| `https://acme.com` | `https://cdn.acme.com/a.png` | Unchanged |
| `https://acme.com` | `//cdn.acme.com/a.png` | Unchanged |
| none | `/contact` | `/contact` |

The rules:

- URLs with a scheme (`https:`, `http:`, `mailto:`, `data:`…) and protocol-relative URLs (`//host/…`) are kept as they are.
- One trailing slash on `baseUrl` is ignored, so `https://acme.com` and `https://acme.com/` behave the same.
- `baseUrl` is a **prefix**, not a browser-style URL resolution: with `https://acme.com/blog`, the path `/post` gives `https://acme.com/blog/post`.

Read the domain from your environment so previews and production each get the right one:

```ts
export const Seo = createSeo({
  siteName: "Acme",
  baseUrl: import.meta.env.VITE_SITE_URL ?? "https://acme.com",
});
```

## Per-section defaults

Different sections can have their own defaults. Keep the shared values in a plain object and spread it:

```ts
import { createSeo, type SeoProps } from "active-react";

const base: SeoProps = {
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  image: "/og.png",
};

export const Seo = createSeo(base);
export const ArticleSeo = createSeo({ ...base, type: "article", titleTemplate: "%s · Acme Blog" });
export const ShopSeo = createSeo({ ...base, type: "product" });
```

## Robots per environment

A staging or preview site should never end up in search results. Set `noindex` from an environment variable:

```ts
const production = import.meta.env.VITE_DEPLOY_ENV === "production";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  noindex: !production,
});
```

Outside production every page gets `<meta name="robots" content="noindex,follow"/>`. A single page can still opt in or out: `<Seo noindex={false} />` or `<Seo noindex />`.

> **Tip:** After deploying to production, view the page source once and check that no `robots` tag says `noindex`. It is the most common SEO accident.

## Multilingual sites

`alternates` tells search engines about the other language versions of a page. Each key is a language code (`en`, `fr`, `en-GB`…), plus `x-default` for the version to show when no language matches:

```tsx
<Seo
  title="Oslo chair"
  locale="en_US"
  canonical="/en/products/oslo"
  alternates={{
    en: "/en/products/oslo",
    fr: "/fr/produits/oslo",
    de: "/de/produkte/oslo",
    "x-default": "/en/products/oslo",
  }}
/>
```

Search engines expect every language version to list all the versions, itself included. A small helper keeps that consistent when paths only differ by prefix:

```ts
const LANGS = ["en", "fr", "de"] as const;

export const alternatesFor = (path: string) => ({
  ...Object.fromEntries(LANGS.map((lang) => [lang, `/${lang}${path}`])),
  "x-default": `/en${path}`,
});

// <Seo canonical={`/fr${path}`} alternates={alternatesFor(path)} locale="fr_FR" />
```

You can also create one `Seo` per language with its own `locale` and title template:

```ts
export const SeoFr = createSeo({ ...base, locale: "fr_FR", titleTemplate: "%s · Acme France" });
```
