---
title: Données structurées
description: Ajoutez des données structurées JSON-LD à vos pages avec Seo::json_ld : organisations, fils d'Ariane, articles, produits et FAQ, via serde_json ou à la main.
---

Les données structurées indiquent aux moteurs de recherche, sous une forme lisible par une machine, de quoi parle une page : cette page est un produit avec un prix, celle-ci un article écrit par un auteur, ces liens forment un fil d'Ariane. Google s'en sert pour les résultats enrichis (étoiles, prix, fils d'Ariane, questions de FAQ). Le format standard est [JSON-LD](https://json-ld.org) avec le vocabulaire [schema.org](https://schema.org), dans une balise `<script type="application/ld+json">`.

`active` les ajoute au moyen du même builder `Seo` que les autres balises.

## Seo::json_ld

`.json_ld(json)` reçoit une chaîne de JSON déjà sérialisée et ajoute une balise `<script type="application/ld+json">` par appel, après les autres balises du `<head>` :

```rust
use active::Seo;

let seo = Seo::new().json_ld(
    r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}"#,
);
assert!(seo.to_html().ends_with(
    r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}</script>"#
));
```

Le JSON est écrit tel quel, à une modification près : chaque `<` devient `\u003c`. C'est toujours la même valeur JSON, mais une chaîne comme `</script>` dans vos données ne peut jamais fermer la balise prématurément et injecter du HTML. Une chaîne vide n'ajoute rien.

## Construire le JSON avec serde_json

Le moyen le plus sûr de produire le JSON est [`serde_json`](https://docs.rs/serde_json), qui échappe correctement chaque valeur :

```toml
[dependencies]
serde_json = "1"
```

```rust
use active::Seo;
use serde_json::json;

fn product_seo(name: &str, price: &str, image: &str) -> Seo {
    let data = json!({
        "@context": "https://schema.org",
        "@type": "Product",
        "name": name,
        "image": image,
        "offers": {
            "@type": "Offer",
            "price": price,
            "priceCurrency": "EUR",
            "availability": "https://schema.org/InStock"
        }
    });
    Seo::new().title(name.to_string()).og_type("product").json_ld(data.to_string())
}

let html = product_seo("Oak chair", "249.00", "https://acme.com/oak.jpg").to_html();
assert!(html.contains(r#""@type":"Product""#));
```

`json!` construit une `serde_json::Value`, et `to_string()` la sérialise sans espaces. Avec vos propres types, dérivez `serde::Serialize` et appelez `serde_json::to_string(&value)`.

## Sans dépendance

Pour quelques champs, `format!` fait l'affaire, à condition que chaque valeur texte soit échappée comme une chaîne JSON :

```rust
use active::Seo;

/// `s` sous forme de littéral de chaîne JSON, guillemets compris.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

let title = r#"Say "hello" to Rust"#;
let seo = Seo::new().json_ld(format!(
    r#"{{"@context":"https://schema.org","@type":"Article","headline":{}}}"#,
    json_string(title)
));
assert!(seo.to_html().contains(r#""headline":"Say \"hello\" to Rust""#));
```

Pensez à doubler les accolades (`{{` et `}}`) du JSON lui-même à l'intérieur de `format!`.

## Types courants

Chaque exemple ci-dessous est un objet JSON-LD ; passez-le à `.json_ld(...)` comme montré plus haut.

### Organisation et site web

Une fois par site, généralement dans la mise en page ou sur la page d'accueil :

```text
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "Acme",
  "url": "https://acme.com",
  "logo": "https://acme.com/logo.png",
  "sameAs": ["https://github.com/acme"]
}
```

### Fil d'Ariane

Affiche la position de la page dans le site au sein des résultats de recherche. Ce site en ajoute un à chaque page de documentation :

```text
{
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    { "@type": "ListItem", "position": 1, "name": "Docs", "item": "https://acme.com/docs/" },
    { "@type": "ListItem", "position": 2, "name": "SEO", "item": "https://acme.com/docs/seo/" }
  ]
}
```

### Article

Pour les billets de blog et les actualités. Associez-le à `.og_type("article")` :

```text
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Shipping a Rust website",
  "datePublished": "2026-03-01",
  "author": { "@type": "Person", "name": "Ada" },
  "image": "https://acme.com/posts/rust-site.png"
}
```

### FAQ

```text
{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "mainEntity": [
    {
      "@type": "Question",
      "name": "Do you ship abroad?",
      "acceptedAnswer": { "@type": "Answer", "text": "Yes, to the whole European Union." }
    }
  ]
}
```

Le JSON est présenté ici sur plusieurs lignes pour la lecture ; `serde_json` produit une seule ligne, et les deux formes sont valides.

## Plusieurs blocs sur une page

Chaque appel à `json_ld` ajoute sa propre balise `<script>`, et lorsque [les valeurs `Seo` sont fusionnées](../seo/#fusion), les blocs JSON-LD **s'additionnent** au lieu de se remplacer. Une mise en page peut déclarer l'organisation pendant que chaque page ajoute son fil d'Ariane :

```rust
use active::prelude::*;

let organization = r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#;
let breadcrumbs = r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#;

let page = div()
    .child(Seo::new().json_ld(organization))
    .child(article().child(Seo::new().title("Chairs").json_ld(breadcrumbs)));

let html = Document::new(page).render();
assert_eq!(html.matches(r#"<script type="application/ld+json">"#).count(), 2);
```

> **Attention :** Comme les blocs s'additionnent, un bloc déclaré à la fois par la mise en page et par une page apparaît deux fois. Déclarez chaque bloc commun à tout le site à un seul endroit.

## Vérifier votre balisage

- Testez une page publique avec le [test des résultats enrichis](https://search.google.com/test/rich-results) de Google (Rich Results Test) ou avec le [Schema Markup Validator](https://validator.schema.org).
- Dans vos propres tests, rendez la page et examinez les balises fusionnées avec `Document::final_seo()` :

```rust
use active::prelude::*;

let page = main().child(Seo::new().json_ld(r#"{"@type":"Thing"}"#));
let seo = Document::new(page).final_seo();
assert!(seo.to_html().contains(r#"{"@type":"Thing"}"#));
```
