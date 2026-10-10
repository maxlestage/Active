---
title: El componente <Seo>
description: Renderiza las etiquetas title, description, canonical, Open Graph, Twitter, hreflang, robots y JSON-LD con un componente que React 19 traslada al head.
---

`<Seo>` renderiza todas las etiquetas que una página necesita para los buscadores y las redes sociales. Lo renderizas en cualquier parte de tu página y React 19 traslada las etiquetas al `<head>`, en el servidor y en el navegador.

## Uso básico

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

En una app real, rara vez pasas `titleTemplate`, `siteName` o URLs completas en cada página. Escríbelos una sola vez con [`createSeo`](../site-defaults/) y mantén cada página breve:

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

## HTML generado

Con los valores por defecto anteriores, esta página:

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

produce, en el `<head>`:

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

El script JSON-LD se queda donde se renderiza `<Seo>`, en el `<body>`, algo que los buscadores aceptan (consulta [datos estructurados](../structured-data/)):

```html
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"Oslo chair"}</script>
```

Las etiquetas siempre aparecen en este orden. Una etiqueta cuyo valor falta se omite, excepto `og:type` y `twitter:card`, que siempre están presentes.

## Props

Todas las props son opcionales.

| Prop | Tipo | Produce |
| --- | --- | --- |
| `title` | `string` | `<title>` (con la plantilla) y `og:title` (sin ella) |
| `titleTemplate` | `string` | Plantilla para `<title>`: `%s` se reemplaza por `title` |
| `description` | `string` | `<meta name="description">` y `og:description` |
| `canonical` | `string` | `<link rel="canonical">` y `og:url` |
| `image` | `string` | `og:image`; cambia `twitter:card` a `summary_large_image` |
| `type` | `string` | `og:type`, `"website"` por defecto |
| `siteName` | `string` | `og:site_name`; el título cuando falta `title` |
| `locale` | `string` | `og:locale`, por ejemplo `"en_US"` |
| `twitter` | `string` | `twitter:site`, por ejemplo `"@acme"` |
| `baseUrl` | `string` | Prefijo para las URLs relativas de `canonical`, `image` y `alternates` |
| `noindex` | `boolean` | `<meta name="robots">` con `noindex` |
| `nofollow` | `boolean` | `<meta name="robots">` con `nofollow` |
| `alternates` | `Record<string, string>` | Un `<link rel="alternate" hrefLang>` por idioma |
| `jsonLd` | `object \| object[]` | `<script type="application/ld+json">` |
| `meta` | `Array<{ name?, property?, content }>` | Etiquetas `<meta>` adicionales |

### El título y su valor de respaldo

El `<title>` se construye así:

1. `title` y `titleTemplate` están definidos: el primer `%s` de la plantilla se reemplaza por el título (`"%s · Acme"` da `Oslo chair · Acme`).
2. Solo `title` está definido: se usa tal cual.
3. Sin `title`: se usa `siteName` tal cual, sin la plantilla. Práctico para una página de inicio.
4. Ninguno de los dos: no hay etiqueta `<title>`.

`og:title` sigue la misma lógica, pero nunca usa la plantilla: las tarjetas de las redes sociales ya muestran el nombre del sitio junto al título.

### Tarjeta de Twitter

`twitter:card` vale `summary_large_image` cuando hay una `image`, y `summary` en caso contrario. X (Twitter) también lee las etiquetas `og:*`, así que no hace falta ninguna otra etiqueta `twitter:*`.

### Robots

`<meta name="robots">` solo aparece cuando `noindex` o `nofollow` es true:

| Props | `content` |
| --- | --- |
| `noindex` | `noindex,follow` |
| `nofollow` | `index,nofollow` |
| `noindex` y `nofollow` | `noindex,nofollow` |
| Ninguna | Ninguna etiqueta (los buscadores indexan y siguen los enlaces por defecto) |

### Etiquetas meta adicionales

`meta` es una vía de escape para cualquier etiqueta que `<Seo>` no cubra. Usa `name` para las etiquetas meta normales y `property` para las de Open Graph:

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

## Reglas prácticas

### Un solo `<Seo>` por página

Renderiza un único `<Seo>` por página, en el componente de la página. Dos `<Seo>` montados al mismo tiempo producen etiquetas duplicadas: React traslada todo lo que recibe y no fusiona las etiquetas. Los valores que comparten todas las páginas van en [`createSeo`](../site-defaults/), no en un segundo `<Seo>` en tu layout.

### Funciona en Server Components

`<Seo>` no usa ningún hook, ni contexto, ni efecto: es una simple función que devuelve etiquetas. Funciona igual en componentes cliente, en componentes renderizados en el servidor y en React Server Components.

### La navegación en el cliente es automática

Cuando cambia la ruta, la página anterior se desmonta y React elimina sus etiquetas; la nueva página se monta y React inserta las suyas. Los benchmarks midieron una mediana de 0,3 ms entre el renderizado de una nueva página y la actualización del `<head>`. No hay nada que configurar.

### Etiquetas estáticas en `index.html`

- Un `<title>` estático en `index.html` puede quedarse: React inserta el `<title>` de la página antes que él, así que `document.title` es correcto.
- Elimina de `index.html` las etiquetas estáticas `description` y `og:*`. React no las deduplica, así que los rastreadores verían dos descripciones.

### Datos de una API

`<Seo>` es un componente como cualquier otro, así que puede usar los datos que tu página ya cargó:

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

Para que los rastreadores que no ejecutan JavaScript vean estas etiquetas, renderiza la página en el servidor: consulta [renderizado en servidor](../server-rendering/).
