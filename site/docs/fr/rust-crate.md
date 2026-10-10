---
title: Le crate Rust
description: Le crate active, jumeau Rust du composant Seo, génère les mêmes balises head depuis des serveurs Rust, des générateurs de sites statiques et WebAssembly.
---

Toutes les pages ne sont pas rendues par React. `active` est un crate Rust qui produit les mêmes balises SEO que `<Seo>`, avec les mêmes options et les mêmes règles, pour les serveurs web Rust (Axum, Actix Web…), les générateurs de sites statiques, les edge workers et WebAssembly.

- **Aucune dépendance**, pas d'`unsafe`, pas de macros.
- **Même résultat que React**, octet pour octet, aux deux petites différences près listées plus bas.
- **Échappement identique à React** (`&`, `<`, `>`, `"`, `'`), avec un JSON-LD protégé contre l'injection de `</script>`.
- Compile pour `wasm32-unknown-unknown`. Nécessite Rust 1.85 ou plus récent.

## Installer

```sh
cargo add active
```

## Exemple rapide

```rust
use active::Seo;

let head = Seo::new()
    .title("Oslo chair")
    .description("Solid oak chair, delivered in 48 h.")
    .canonical("https://acme.com/products/oslo")
    .to_html();
```

`head` contient maintenant, sur une seule ligne :

```html
<title>Oslo chair</title>
<meta name="description" content="Solid oak chair, delivered in 48 h."/>
<link rel="canonical" href="https://acme.com/products/oslo"/>
<meta property="og:title" content="Oslo chair"/>
<meta property="og:description" content="Solid oak chair, delivered in 48 h."/>
<meta property="og:type" content="website"/>
<meta property="og:url" content="https://acme.com/products/oslo"/>
<meta name="twitter:card" content="summary"/>
```

## Valeurs par défaut du site et pages

`Seo` est un builder : chaque setter le prend par valeur et le renvoie. Construisez une fois les valeurs communes à tout le site, puis clonez-les pour chaque page, exactement comme [`createSeo`](../site-defaults/) en React. Les setters de la page surchargent les valeurs par défaut.

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com") // resolves relative canonical, image and alternates
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us.")
        .canonical("/contact")
        .og_type("article")
        .alternate("en", "/contact")
        .alternate("fr", "/fr/contact")
        .alternate("x-default", "/contact")
        .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#)
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "The Acme logo")
        .to_html()
}
```

Chaque setter accepte `impl Into<String>` : les littéraux de chaîne, `String` et `&String` fonctionnent tous. `noindex` et `nofollow` prennent un `bool`.

## Méthodes

| Méthode | Balises | Prop React |
| --- | --- | --- |
| `Seo::new()` | Seulement `og:type` (`website`) et `twitter:card` (`summary`) | |
| `.title(title)` | `<title>`, `og:title` (sans template) | `title` |
| `.title_template(template)` | `<title>` : le premier `%s` est remplacé par le titre | `titleTemplate` |
| `.site_name(name)` | `og:site_name` ; le titre lorsqu'aucun n'est défini | `siteName` |
| `.description(text)` | `<meta name="description">`, `og:description` | `description` |
| `.canonical(url)` | `<link rel="canonical">`, `og:url` | `canonical` |
| `.image(url)` | `og:image` ; `twitter:card` devient `summary_large_image` | `image` |
| `.og_type(type)` | `og:type`, `website` par défaut | `type` |
| `.locale(locale)` | `og:locale` | `locale` |
| `.twitter(handle)` | `twitter:site` | `twitter` |
| `.base_url(url)` | Préfixe des URL relatives de `canonical`, `image` et des alternates | `baseUrl` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` | `noindex`, `nofollow` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang>`, une fois par appel, dans l'ordre des appels | `alternates` |
| `.json_ld(json)` | `<script type="application/ld+json">`, une fois par appel | `jsonLd` |
| `.meta_name(name, content)` | `<meta name content>` supplémentaire | `meta` |
| `.meta_property(property, content)` | `<meta property content>` supplémentaire | `meta` |

Les règles sont celles du composant React : même ordre des balises, même titre de repli, mêmes valeurs `robots`, même résolution des URL. Une chaîne vide compte comme « non définie ». Appeler `.alternate()` deux fois avec la même langue remplace l'URL.

`json_ld` prend du JSON déjà sérialisé, par exemple avec serde_json :

```rust
let ld = serde_json::json!({
    "@context": "https://schema.org",
    "@type": "Product",
    "name": product.name,
});
let seo = SITE.clone().title(&product.name).json_ld(ld.to_string());
```

Le JSON est inséré tel quel, sauf que chaque `<` devient `\u003c` : c'est le même JSON, mais il ne peut jamais fermer la balise `<script>`.

## Rendu

Trois façons d'obtenir le HTML, qui produisent toutes les mêmes balises, sans espace entre elles :

```rust
// 1. A new String.
let head: String = seo.to_html();

// 2. Appended to a buffer you are already writing.
let mut page = String::from("<!doctype html><html><head><meta charset=\"utf-8\">");
seo.write_html(&mut page);
page.push_str("</head><body>…</body></html>");

// 3. Display: straight into format! or write!, with no intermediate String.
let page = format!("<!doctype html><html><head>{seo}</head><body>…</body></html>");
```

## Différences avec React

- **`hreflang` est en minuscules.** React écrit l'attribut `hrefLang`. Les noms d'attributs HTML ne sont pas sensibles à la casse : les deux signifient la même chose.
- **Le JSON-LD arrive en dernier dans le head.** React laisse le `<script>` là où `<Seo>` est rendu, dans le `<body>`. Le crate n'a pas de body : il ajoute donc le script après les autres balises. Les moteurs de recherche acceptent les deux.
- `json_ld` produit un script par appel, alors que la prop React produit un script par valeur (un objet, ou un tableau dans un seul script).

## Exemple avec Axum

```rust
use std::sync::LazyLock;

use active::Seo;
use axum::{Router, response::Html, routing::get};

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
});

async fn page() -> Html<String> {
    let seo = SITE
        .clone()
        .title("Pricing")
        .description("Simple plans for teams of every size.")
        .canonical("/pricing");
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">{seo}</head><body><h1>Pricing</h1></body></html>"#
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/pricing", get(page));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Dépendances : `cargo add active axum` et `cargo add tokio --features full`. Actix Web, Rocket, Leptos ou un moteur de templates fonctionnent de la même façon : le crate vous donne une chaîne, que vous placez dans votre `<head>`. Le dépôt contient aussi un exemple sans framework : `cargo run --example axum_like` dans `crates/active`.

## Générateur de site statique

```rust
use std::fs;

fn main() -> std::io::Result<()> {
    let pages = [
        ("/", "Home", "Furniture made to last."),
        ("/about/", "About", "Who we are and how we work."),
    ];
    for (path, title, description) in pages {
        let seo = SITE.clone().title(title).description(description).canonical(path);
        let dir = format!("dist{path}");
        fs::create_dir_all(&dir)?;
        fs::write(
            format!("{dir}index.html"),
            format!("<!doctype html><html><head><meta charset=\"utf-8\">{seo}</head><body><h1>{title}</h1></body></html>"),
        )?;
    }
    Ok(())
}
```

## Dans le navigateur avec wasm-bindgen

Comme il n'a aucune dépendance, `active` se compile tel quel en WebAssembly. Enveloppez-le dans votre propre crate wasm-bindgen pour générer des balises head dans le navigateur ou dans un edge worker. C'est exactement ce que fait le playground interactif de la page d'accueil de ce site : chaque frappe appelle le crate, compilé en WebAssembly.

```toml
[dependencies]
active = "0.1"
wasm-bindgen = "0.2"
```

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn seo_head(title: &str, description: &str, path: &str, origin: &str) -> String {
    active::Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url(origin)
        .title(title)
        .description(description)
        .canonical(path)
        .to_html()
}
```

Chargez-le ensuite avec active-react comme n'importe quel autre module (voir [charger WebAssembly](../loading-wasm/)) :

```tsx
import { useWasm, wasm } from "active-react";
import url from "./pkg/seo_wasm_bg.wasm?url";

const seoWasm = wasm(() => import("./pkg/seo_wasm.js"), { url });

export function HeadPreview({ title, description }: { title: string; description: string }) {
  const rust = useWasm(seoWasm);
  return <pre>{rust ? rust.seo_head(title, description, "/products/oak-chair", location.origin) : "…"}</pre>;
}
```

> **Remarque :** Dans une app React, vous n'avez pas besoin du crate pour définir les balises de vos propres pages : `<Seo>` s'en charge. Le crate s'adresse au code Rust qui produit du HTML, ou à l'affichage des balises sous forme de texte, comme dans le playground.

## Référence de l'API

La documentation complète de l'API Rust se trouve sur [docs.rs/active](https://docs.rs/active).
