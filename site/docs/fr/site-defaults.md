---
title: Valeurs par défaut avec createSeo
description: Écrivez une seule fois les valeurs SEO communes à tout le site avec createSeo, puis surchargez-les par page, par section, par environnement ou par langue.
---

La plupart des valeurs SEO sont identiques sur toutes les pages : le nom du site, le template de titre, le domaine, l'image de partage par défaut. `createSeo` vous permet de les écrire une seule fois.

## Écrire les valeurs par défaut une fois

Créez `src/seo.ts` :

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

Importez ensuite ce `Seo` au lieu de celui d'`active-react` :

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

`createSeo(defaults)` renvoie un composant qui rend `<Seo>` en combinant vos valeurs par défaut et les props de la page. Comme `<Seo>`, il n'utilise ni hook ni contexte : il n'y a aucun provider à ajouter à votre app.

## Surcharger par page

Les props de la page l'emportent sur les valeurs par défaut. La fusion est superficielle, exactement comme `{ ...defaults, ...props }` :

```tsx
// Uses the default image, replaces the default type.
<Seo title="Our story" type="article" canonical="/about" />

// Replaces the default image for this page only.
<Seo title="Oslo chair" image="/img/oslo.jpg" canonical="/products/oslo" />
```

Deux conséquences de cette fusion superficielle :

- Les props de type objet ou tableau (`alternates`, `jsonLd`, `meta`) **remplacent** la valeur par défaut ; elles ne sont pas fusionnées élément par élément.
- Passer `undefined` explicitement **supprime** une valeur par défaut : `<Seo image={undefined} />` ne rend aucun `og:image`.

> **Attention :** Méfiez-vous des données facultatives. `<Seo image={post.cover} />` supprime l'image par défaut chaque fois que `post.cover` vaut `undefined`. Ne passez la prop que si vous avez une valeur : `<Seo {...(post.cover ? { image: post.cover } : {})} />`.

## Template de titre

`titleTemplate` contient `%s`, qui est remplacé par le titre de la page :

| Template | `title` | `<title>` |
| --- | --- | --- |
| `"%s · Acme"` | `"Contact"` | `Contact · Acme` |
| `"%s · Acme"` | aucun | `Acme` (le `siteName`) |
| `"%s"` | `"Acme, furniture made to last"` | `Acme, furniture made to last` |

Seul le premier `%s` est remplacé, et le template ne s'applique que si la page a un titre. `og:title` n'utilise jamais le template.

### Page d'accueil sans template

Sur la page d'accueil, vous voulez souvent un titre entièrement personnalisé plutôt que `Home · Acme`. Surchargez le template pour cette page :

```tsx
<Seo title="Acme, furniture made to last" titleTemplate="%s" canonical="/" />
```

Omettre `title` fonctionne aussi : la page utilise alors le `siteName` seul.

## baseUrl et URL

Les URL Open Graph et canoniques doivent être absolues. Avec `baseUrl`, vous pouvez écrire des chemins relatifs et laisser active-react les compléter. Cela s'applique à `canonical` (et donc à `og:url`), à `image` et à chaque URL d'`alternates`.

| `baseUrl` | Valeur | Résultat |
| --- | --- | --- |
| `https://acme.com` | `/contact` | `https://acme.com/contact` |
| `https://acme.com/` | `/contact` | `https://acme.com/contact` |
| `https://acme.com` | `contact` | `https://acme.com/contact` |
| `https://acme.com` | `https://cdn.acme.com/a.png` | Inchangée |
| `https://acme.com` | `//cdn.acme.com/a.png` | Inchangée |
| aucun | `/contact` | `/contact` |

Les règles :

- Les URL avec un schéma (`https:`, `http:`, `mailto:`, `data:`…) et les URL relatives au protocole (`//host/…`) sont conservées telles quelles.
- Une barre oblique finale sur `baseUrl` est ignorée : `https://acme.com` et `https://acme.com/` se comportent de la même façon.
- `baseUrl` est un **préfixe**, pas une résolution d'URL à la manière d'un navigateur : avec `https://acme.com/blog`, le chemin `/post` donne `https://acme.com/blog/post`.

Lisez le domaine depuis votre environnement pour que les previews et la production obtiennent chacune le bon :

```ts
export const Seo = createSeo({
  siteName: "Acme",
  baseUrl: import.meta.env.VITE_SITE_URL ?? "https://acme.com",
});
```

## Valeurs par défaut par section

Chaque section peut avoir ses propres valeurs par défaut. Gardez les valeurs partagées dans un simple objet et décomposez-le avec l'opérateur spread :

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

## Robots selon l'environnement

Un site de staging ou de preview ne doit jamais se retrouver dans les résultats de recherche. Définissez `noindex` à partir d'une variable d'environnement :

```ts
const production = import.meta.env.VITE_DEPLOY_ENV === "production";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  noindex: !production,
});
```

En dehors de la production, chaque page reçoit `<meta name="robots" content="noindex,follow"/>`. Une page donnée peut toujours choisir explicitement : `<Seo noindex={false} />` ou `<Seo noindex />`.

> **Astuce :** Après un déploiement en production, affichez une fois le code source de la page et vérifiez qu'aucune balise `robots` n'indique `noindex`. C'est l'accident SEO le plus courant.

## Sites multilingues

`alternates` indique aux moteurs de recherche les autres versions linguistiques d'une page. Chaque clé est un code de langue (`en`, `fr`, `en-GB`…), plus `x-default` pour la version à afficher quand aucune langue ne correspond :

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

Les moteurs de recherche attendent que chaque version linguistique liste toutes les versions, y compris elle-même. Un petit utilitaire garantit cette cohérence quand les chemins ne diffèrent que par leur préfixe :

```ts
const LANGS = ["en", "fr", "de"] as const;

export const alternatesFor = (path: string) => ({
  ...Object.fromEntries(LANGS.map((lang) => [lang, `/${lang}${path}`])),
  "x-default": `/en${path}`,
});

// <Seo canonical={`/fr${path}`} alternates={alternatesFor(path)} locale="fr_FR" />
```

Vous pouvez aussi créer un `Seo` par langue, avec sa propre `locale` et son propre template de titre :

```ts
export const SeoFr = createSeo({ ...base, locale: "fr_FR", titleTemplate: "%s · Acme France" });
```
