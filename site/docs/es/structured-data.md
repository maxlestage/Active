---
title: Datos estructurados (JSON-LD)
description: Añade JSON-LD de schema.org a cualquier página con la prop jsonLd: describe productos, artículos, migas de pan y más, y obtén resultados enriquecidos.
---

Los datos estructurados describen de qué trata una página en un formato que las máquinas entienden: esta página es un producto que cuesta 249 euros, aquella es un artículo escrito por Ada el 1 de octubre. Los buscadores los usan para mostrar **resultados enriquecidos**: precios y disponibilidad, migas de pan, fechas de artículos, nombres de sitios y más.

El formato estándar es **JSON-LD**: un objeto JSON que usa el vocabulario de [schema.org](https://schema.org), colocado en una etiqueta `<script type="application/ld+json">`.

## La prop jsonLd

Pasa un objeto a `jsonLd`:

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

O un array, cuando la página describe varias cosas. Se renderiza como un único script que contiene un array JSON, algo que los buscadores admiten:

```tsx
<Seo title="Oslo chair" jsonLd={[productLd, breadcrumbLd]} />
```

Pasa objetos simples: active-react los serializa con `JSON.stringify` y los escapa por ti.

## Recetas

Copia, adapta y valida. Todas las recetas necesitan `"@context": "https://schema.org"`.

### Organization

En tu página de inicio, describe quién eres. Los buscadores pueden usar tu logotipo y tus perfiles sociales.

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

Indica a los buscadores el nombre de tu sitio, que puede aparecer encima de tus resultados.

```tsx
const website = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  name: "Acme",
  url: "https://acme.com",
};

<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" jsonLd={[organization, website]} />;
```

### Product con Offer

El precio, la moneda y la disponibilidad pueden aparecer directamente en los resultados de búsqueda.

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

> **Nota:** `baseUrl` no afecta al JSON-LD: escribe URLs absolutas dentro de tus datos estructurados.

### Article

Usa `Article`, o los más específicos `BlogPosting` o `NewsArticle`.

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

Muestra la posición de la página dentro de tu sitio en lugar de una URL en bruto.

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

El último elemento, la página actual, no necesita una URL `item`.

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

> **Nota:** Actualmente, Google solo muestra resultados enriquecidos de preguntas frecuentes para un conjunto limitado de sitios de referencia. El marcado sigue siendo válido y otros consumidores todavía pueden leerlo, pero no cuentes con el resultado visual.

## Tipar tus datos

`jsonLd` acepta cualquier objeto. Si quieres autocompletado y comprobaciones para los tipos de schema.org, el paquete `schema-dts` de Google proporciona definiciones de TypeScript:

```ts
import type { Product, WithContext } from "schema-dts";

const productLd: WithContext<Product> = {
  "@context": "https://schema.org",
  "@type": "Product",
  name: "Oslo chair",
};
```

Es un paquete que solo contiene tipos: no añade nada a tu bundle.

## Seguridad

Los datos estructurados suelen contener contenido de los usuarios: nombres de productos, reseñas, títulos de artículos. Si un valor contuviera `</script>`, un serializador ingenuo cerraría la etiqueta script antes de tiempo y dejaría que el resto se ejecutara como HTML.

active-react escapa cada `<` como `\u003c` dentro del JSON. Para los parsers sigue siendo exactamente el mismo JSON, pero nunca puede cerrar la etiqueta `<script>`:

```html
<script type="application/ld+json">{"@type":"Thing","name":"\u003c/script>\u003cscript>alert(1)\u003c/script>"}</script>
```

Pasa siempre objetos, nunca cadenas JSON ya construidas, para que se aplique el escape.

## Dónde se renderiza

React 19 traslada `<title>`, `<meta>` y `<link>` al `<head>`, pero no los scripts normales. Por eso, el `<script>` JSON-LD se queda donde se renderiza `<Seo>`, dentro del `<body>`. Google lee el JSON-LD en cualquier parte del documento, así que no hay problema.

Con renderizado en servidor, el script forma parte del HTML que recibe el rastreador. Con `renderWithHead`, está en la parte `html`, no en la parte `head`. Consulta [renderizado en servidor](../server-rendering/).

## Validar

Antes de publicar, prueba tus páginas:

- [Prueba de resultados enriquecidos de Google](https://search.google.com/test/rich-results): muestra para qué resultados enriquecidos es apta una página. Pega una URL pública, o pega el HTML de una página local.
- [Schema Markup Validator](https://validator.schema.org): comprueba el marcado con todo el vocabulario de schema.org.

> **Consejo:** Después del lanzamiento, los informes de resultados enriquecidos de Google Search Console enumeran los errores de datos estructurados de todo tu sitio.
