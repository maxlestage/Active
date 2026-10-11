---
title: Balises SEO
description: Décrivez chaque page aux moteurs de recherche et aux réseaux sociaux avec Seo : titre, description, canonique, hreflang, robots, Open Graph, Twitter, JSON-LD.
---

Les moteurs de recherche et les réseaux sociaux lisent le `<head>` de vos pages : le titre et la description affichés dans les résultats, l'adresse canonique, les versions linguistiques, l'image d'aperçu d'un lien partagé. `active::Seo` écrit toutes ces balises à partir d'un seul builder, et une page peut contenir plusieurs valeurs `Seo`, [fusionnées](#fusion) en un `<head>` unique et sans doublons.

## Un exemple complet

```rust
use active::Seo;

let head = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com")
    .title("Oak chair")
    .description("Solid oak chair, built to last and delivered in 48 hours.")
    .canonical("/products/oak-chair")
    .image("/images/oak-chair.jpg")
    .og_type("product")
    .locale("en_US")
    .twitter("@acme")
    .alternate("en", "/products/oak-chair")
    .alternate("fr", "/fr/produits/chaise-chene")
    .to_html();

assert!(head.starts_with("<title>Oak chair · Acme</title>"));
assert!(head.contains(r#"<link rel="canonical" href="https://acme.com/products/oak-chair"/>"#));
```

Le résultat, avec ici une balise par ligne pour la lecture (la vraie sortie ne contient aucun saut de ligne) :

```html
<title>Oak chair · Acme</title>
<meta name="description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<link rel="canonical" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="en" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="fr" href="https://acme.com/fr/produits/chaise-chene"/>
<meta property="og:title" content="Oak chair"/>
<meta property="og:description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<meta property="og:type" content="product"/>
<meta property="og:url" content="https://acme.com/products/oak-chair"/>
<meta property="og:image" content="https://acme.com/images/oak-chair.jpg"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
```

Dans une page, vous appelez rarement `to_html()` vous-même : placez le `Seo` dans la vue et laissez [`Document`](../documents/) le rendre dans le `<head>`.

## Le builder

`Seo::new()` (ou `Seo::default()`) part d'une valeur vide. Chaque setter reçoit sa valeur sous forme d'`impl Into<String>` (un `bool` pour `noindex` et `nofollow`) et renvoie le `Seo` mis à jour : les appels s'enchaînent.

| Méthode | Balises produites |
| --- | --- |
| `.title(title)` | `<title>` et `og:title` |
| `.title_template(template)` | `<title>` uniquement : le premier `%s` est remplacé par le titre, par exemple `"%s · Acme"` |
| `.site_name(name)` | `og:site_name`, et le titre quand aucun titre n'est défini |
| `.description(text)` | `<meta name="description">` et `og:description` |
| `.canonical(url)` | `<link rel="canonical">` et `og:url` |
| `.image(url)` | `og:image` ; fait aussi passer `twitter:card` à `summary_large_image` |
| `.og_type(type)` | `og:type` : `website` s'il n'est pas défini, `article`, `product`… |
| `.locale(locale)` | `og:locale`, par exemple `en_US` |
| `.twitter(handle)` | `twitter:site`, par exemple `@acme` |
| `.base_url(origin)` | aucune balise : résout les URL relatives de `canonical`, `image` et `alternate` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang="…">`, une par appel |
| `.json_ld(json)` | `<script type="application/ld+json">`, un par appel (voir [Données structurées](../structured-data/)) |
| `.meta_name(name, content)` | toute autre balise `<meta name="…" content="…">` |
| `.meta_property(property, content)` | toute autre balise `<meta property="…" content="…">` |

Pour le rendu : `.to_html()` renvoie une nouvelle `String`, `.write_html(&mut out)` ajoute à la fin d'une chaîne existante, et `Seo` implémente `Display`, donc `format!("<head>{seo}</head>")` fonctionne aussi.

## Règles de rendu

Ces règles rendent la sortie prévisible :

- **Ordre fixe.** Les balises apparaissent toujours dans l'ordre de l'exemple ci-dessus, puis viennent les balises supplémentaires `meta_name`/`meta_property` dans l'ordre des appels, puis le JSON-LD.
- **Vide veut dire non défini.** Une valeur absente ou une chaîne vide ne produit aucune balise. `og:type` et `twitter:card` sont toujours présentes.
- **Titre et modèle.** Avec un titre et un modèle, `<title>` est le modèle dont le `%s` est remplacé ; `og:title` est le titre seul, plus lisible dans les aperçus de liens. Sans titre, le nom du site est utilisé tel quel, sans le modèle. Un titre explicitement vide (`.title("")`) ne produit aucune balise `<title>`.
- **Robots.** La balise robots n'apparaît que si `noindex` ou `nofollow` vaut `true` : `noindex,follow`, `index,nofollow` ou `noindex,nofollow`. Les pages sont indexables par défaut : il n'y a rien à écrire pour les pages normales.
- **URL.** Avec `base_url("https://acme.com")`, une URL relative comme `/contact` ou `contact` devient `https://acme.com/contact`. Les URL qui ont un schéma (`https:`, `mailto:`…) ou qui commencent par `//` sont conservées telles quelles. Un unique `/` final sur la base est ignoré.
- **Alternatives.** Appeler de nouveau `.alternate` avec la même langue remplace son URL sur place. Une URL vide est ignorée.
- **Échappement.** Chaque valeur est échappée pour le HTML (`&`, `<`, `>`, `"`, `'`). Dans le JSON-LD, `<` est écrit `\u003c`, si bien que les données ne peuvent jamais fermer leur balise `<script>`.

## Valeurs par défaut du site

La plupart des valeurs sont les mêmes sur toutes les pages : nom du site, modèle de titre, URL de base, image de partage par défaut, locale, compte Twitter. Construisez-les une fois et clonez-les, ou confiez-les au document :

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us, we answer within a day.")
        .canonical("/contact")
        .to_html()
}
```

Quand l'adresse du site n'est connue qu'à l'exécution (un réglage `SITE_URL`, l'en-tête `Host` de la requête), construisez les valeurs par défaut dans une fonction qui reçoit l'origine, comme le fait ce site dans `site/src/server/pages.rs`.

## Le SEO dans les vues

Un `Seo` se convertit en `Node` : il peut donc être un enfant n'importe où dans une vue. Il ne rend rien dans le corps : [`Document`](../documents/) le récupère et écrit les balises dans le `<head>`.

```rust
use active::prelude::*;

fn product_page(name: &str, slug: &str) -> Element {
    article()
        .child(
            Seo::new()
                .title(name.to_string())
                .description(format!("{name}: solid oak, delivered in 48 hours."))
                .canonical(format!("/products/{slug}"))
                .og_type("product"),
        )
        .child(h1().text(name.to_string()))
}

let html = Document::new(product_page("Oak chair", "oak-chair"))
    .lang("en")
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
    .render();

assert!(html.contains("<title>Oak chair · Acme</title>"));
assert!(html.contains(r#"<meta property="og:type" content="product"/>"#));
```

La description d'une page reste ainsi à côté de son contenu : le composant qui connaît le produit le décrit aussi.

## Fusion

Une page peut contenir plusieurs valeurs `Seo` : les valeurs par défaut du site passées à `Document::seo`, la description d'une mise en page, le titre de la page, le JSON-LD d'un composant. `Document` les fusionne en un seul jeu de balises :

1. Il part du `Seo` passé à `Document::seo` (vide s'il n'y en a pas).
2. Il parcourt le corps dans l'ordre du document (l'ordre dans lequel les balises apparaissent dans le HTML) et applique chaque `Seo` rencontré par-dessus le résultat précédent.
3. Une valeur définie dans un `Seo` ultérieur remplace la précédente. Les valeurs qu'un `Seo` ultérieur ne définit pas sont conservées.
4. Les alternatives sont remplacées par langue, les balises meta supplémentaires par nom ou propriété, et les blocs JSON-LD s'additionnent.

Dans un arbre typique, la mise en page vient en premier et la page est imbriquée à l'intérieur : **la valeur la plus profonde, donc la plus spécifique, l'emporte** :

```rust
use active::prelude::*;

fn layout(content: Element) -> Element {
    div()
        .child(Seo::new().description("Solid oak furniture.").noindex(true))
        .child(content)
}

let page = layout(section().child(Seo::new().title("Chairs").noindex(false)));
let html = Document::new(page)
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
    .render();

assert_eq!(html.matches("<title>").count(), 1);
assert!(html.contains("<title>Chairs · Acme</title>"));
assert!(html.contains(r#"content="Solid oak furniture.""#));
assert!(!html.contains("robots")); // the page's noindex(false) wins
```

Où la fusion regarde :

- les enfants de chaque élément, les fragments et les îlots ;
- le contenu courant des enfants dynamiques (`children_dyn`, `fragment_dyn`), si bien qu'une page choisie par un routeur se décrit comme n'importe quelle autre ;
- pas à l'intérieur des chaînes de HTML brut (`.html(…)`), qui ne sont que du texte pour `active`.

> **Astuce :** Comme « le dernier l'emporte », placez le `Seo` d'une mise en page avant le contenu qu'elle englobe, comme ci-dessus. Un `Seo` ajouté après le contenu écraserait les valeurs de la page.

Vous pouvez aussi fusionner à la main : `a.merge(&b)` applique `b` par-dessus `a`, et `a.merged(&b)` renvoie une nouvelle valeur. `Document::final_seo()` renvoie le résultat fusionné d'une page entière, ce qui est pratique dans les tests.

### Dans le navigateur

- Les **îlots** ne touchent jamais au `<head>` : le serveur l'a déjà écrit.
- Les **applications côté client** le tiennent à jour. Quand une vue est montée avec `active::mount_to` ou `active::mount`, ses valeurs `Seo` sont fusionnées et écrites dans le `<head>` ; chaque fois qu'une région dynamique est reconstruite (par exemple quand le routeur affiche une autre page), les valeurs `Seo` de la région sont fusionnées par-dessus celles de ses ancêtres et le `<head>` est mis à jour. Placez les valeurs par défaut du site dans un `Seo` de la vue racine. Voir [Applications côté client, Titres et balises meta](../client-app/#titres-et-balises-meta).
- `active::update_head(&seo)` écrit vous-même un `Seo` dans le `<head>`, par exemple depuis un gestionnaire d'événement. Un `Seo` sans titre ni nom de site vide le titre de la page.

## Langues et hreflang

Pour un site en plusieurs langues, donnez à chaque version la même liste d'alternatives, plus `x-default` pour les visiteurs dont vous ne proposez pas la langue :

```rust
use active::Seo;

fn alternates(seo: Seo, path: &str) -> Seo {
    ["en", "fr", "es"]
        .into_iter()
        .fold(seo, |seo, lang| seo.alternate(lang, format!("/{lang}{path}")))
        .alternate("x-default", format!("/en{path}"))
}

let seo = alternates(Seo::new().base_url("https://acme.com").title("Contact"), "/contact/");
assert!(seo.to_html().contains(r#"hreflang="x-default" href="https://acme.com/en/contact/""#));
```

Définissez aussi `Document::lang` sur la langue de la page, et `locale` sur la locale Open Graph correspondante (`en_US`, `fr_FR`…).

## Pages à ne pas indexer

Les pages d'erreur, les résultats de recherche et les espaces privés doivent rester en dehors des résultats des moteurs de recherche :

```rust
use active::prelude::*;

fn not_found() -> Element {
    section()
        .child(Seo::new().title("Page not found").noindex(true))
        .child(h1().text("Page not found"))
}
```

Renvoyez aussi le bon code HTTP (404) depuis votre serveur ; voir [Vitesse](../vitesse/#erreurs-et-404).

## Utiliser Seo sans les vues

`Seo` n'a pas besoin du reste de la bibliothèque. Il fonctionne avec n'importe quel moteur de templates, framework web ou générateur statique, sur le serveur comme en WebAssembly :

```rust
use active::Seo;

let seo = Seo::new().title("Hello").description("A page.");
let page = format!(
    r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>{seo}</head><body>…</body></html>"#
);
assert!(page.contains("<title>Hello</title>"));
```

## Liste de vérification

- Un titre clair par page, d'environ 50 à 60 caractères, et une description de 120 à 160 caractères.
- Une URL canonique sur chaque page, absolue (définissez `base_url`).
- Une image de partage (`image`) de 1200 × 630 pixels pour de beaux aperçus de liens.
- Des alternatives `hreflang` sur chaque version linguistique, y compris `x-default`.
- `noindex` sur les pages d'erreur, de recherche et privées.
- Un `sitemap.xml` et un `robots.txt` (voir [Vitesse](../vitesse/) et [Sites statiques](../static-site/#sitemap-et-robots-txt)).
- Des [données structurées](../structured-data/) là où elles sont utiles : organisation, fil d'Ariane, articles, produits.
