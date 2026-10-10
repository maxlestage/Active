---
title: Valores por defecto del sitio con createSeo
description: Escribe una sola vez los valores SEO de todo tu sitio con createSeo y sobrescríbelos por página, por sección, por entorno o por idioma.
---

La mayoría de los valores SEO son iguales en todas las páginas: el nombre del sitio, la plantilla de título, el dominio, la imagen por defecto para compartir. `createSeo` te permite escribirlos una sola vez.

## Escribe los valores por defecto una sola vez

Crea `src/seo.ts`:

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

Después importa este `Seo` en lugar del de `active-react`:

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

`createSeo(defaults)` devuelve un componente que renderiza `<Seo>` combinando tus valores por defecto con las props de la página. Al igual que `<Seo>`, no usa ningún hook ni contexto, así que no hay ningún provider que añadir a tu app.

## Sobrescribir por página

Las props de la página tienen prioridad sobre los valores por defecto. La fusión es superficial, exactamente como `{ ...defaults, ...props }`:

```tsx
// Uses the default image, replaces the default type.
<Seo title="Our story" type="article" canonical="/about" />

// Replaces the default image for this page only.
<Seo title="Oslo chair" image="/img/oslo.jpg" canonical="/products/oslo" />
```

Dos consecuencias de la fusión superficial:

- Las props de tipo objeto y array (`alternates`, `jsonLd`, `meta`) **reemplazan** el valor por defecto; no se fusionan elemento por elemento.
- Pasar `undefined` explícitamente **anula** un valor por defecto: `<Seo image={undefined} />` no renderiza ningún `og:image`.

> **Advertencia:** Ten cuidado con los datos opcionales. `<Seo image={post.cover} />` elimina la imagen por defecto cada vez que `post.cover` es `undefined`. Pasa la prop solo cuando tengas un valor: `<Seo {...(post.cover ? { image: post.cover } : {})} />`.

## Plantilla de título

`titleTemplate` contiene `%s`, que se reemplaza por el título de la página:

| Plantilla | `title` | `<title>` |
| --- | --- | --- |
| `"%s · Acme"` | `"Contact"` | `Contact · Acme` |
| `"%s · Acme"` | ninguno | `Acme` (el `siteName`) |
| `"%s"` | `"Acme, furniture made to last"` | `Acme, furniture made to last` |

Solo se reemplaza el primer `%s`, y la plantilla solo se aplica cuando la página tiene título. `og:title` nunca usa la plantilla.

### Página de inicio sin plantilla

En la página de inicio, a menudo quieres un título completo y personalizado en lugar de `Home · Acme`. Sobrescribe la plantilla para esa página:

```tsx
<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" />
```

Omitir `title` también funciona: la página usa entonces solo el `siteName`.

## baseUrl y las URLs

Las URLs de Open Graph y la canónica deben ser absolutas. Con `baseUrl`, puedes escribir rutas relativas y dejar que active-react las complete. Se aplica a `canonical` (y por tanto a `og:url`), a `image` y a todas las URLs de `alternates`.

| `baseUrl` | Valor | Resultado |
| --- | --- | --- |
| `https://acme.com` | `/contact` | `https://acme.com/contact` |
| `https://acme.com/` | `/contact` | `https://acme.com/contact` |
| `https://acme.com` | `contact` | `https://acme.com/contact` |
| `https://acme.com` | `https://cdn.acme.com/a.png` | Sin cambios |
| `https://acme.com` | `//cdn.acme.com/a.png` | Sin cambios |
| ninguno | `/contact` | `/contact` |

Las reglas:

- Las URLs con esquema (`https:`, `http:`, `mailto:`, `data:`…) y las URLs relativas al protocolo (`//host/…`) se conservan tal cual.
- Se ignora una barra final en `baseUrl`, así que `https://acme.com` y `https://acme.com/` se comportan igual.
- `baseUrl` es un **prefijo**, no una resolución de URL al estilo del navegador: con `https://acme.com/blog`, la ruta `/post` da `https://acme.com/blog/post`.

Lee el dominio desde tu entorno para que las previsualizaciones y la producción obtengan cada una el suyo:

```ts
export const Seo = createSeo({
  siteName: "Acme",
  baseUrl: import.meta.env.VITE_SITE_URL ?? "https://acme.com",
});
```

## Valores por defecto por sección

Las distintas secciones pueden tener sus propios valores por defecto. Guarda los valores compartidos en un objeto simple y propágalo con spread:

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

## Robots según el entorno

Un sitio de staging o de previsualización nunca debería acabar en los resultados de búsqueda. Define `noindex` a partir de una variable de entorno:

```ts
const production = import.meta.env.VITE_DEPLOY_ENV === "production";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  noindex: !production,
});
```

Fuera de producción, cada página recibe `<meta name="robots" content="noindex,follow"/>`. Una página concreta todavía puede activarlo o desactivarlo: `<Seo noindex={false} />` o `<Seo noindex />`.

> **Consejo:** Después de desplegar en producción, mira una vez el código fuente de la página y comprueba que ninguna etiqueta `robots` diga `noindex`. Es el accidente de SEO más común.

## Sitios multilingües

`alternates` informa a los buscadores de las otras versiones de idioma de una página. Cada clave es un código de idioma (`en`, `fr`, `en-GB`…), más `x-default` para la versión que se muestra cuando ningún idioma coincide:

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

Los buscadores esperan que cada versión de idioma enumere todas las versiones, incluida ella misma. Un pequeño helper mantiene esa coherencia cuando las rutas solo difieren por el prefijo:

```ts
const LANGS = ["en", "fr", "de"] as const;

export const alternatesFor = (path: string) => ({
  ...Object.fromEntries(LANGS.map((lang) => [lang, `/${lang}${path}`])),
  "x-default": `/en${path}`,
});

// <Seo canonical={`/fr${path}`} alternates={alternatesFor(path)} locale="fr_FR" />
```

También puedes crear un `Seo` por idioma, con su propio `locale` y su propia plantilla de título:

```ts
export const SeoFr = createSeo({ ...base, locale: "fr_FR", titleTemplate: "%s · Acme France" });
```
