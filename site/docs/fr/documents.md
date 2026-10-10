---
title: Documents
description: Transformez une vue en page HTML complète avec Document : langue, balises SEO fusionnées, feuilles de style, HTML du head, chargeur WebAssembly et attributs.
---

Une vue décrit le contenu d'une page. `Document` l'enveloppe dans un document HTML complet : le doctype, l'élément `<html>` avec sa langue, un `<head>` contenant les balises SEO fusionnées, vos feuilles de style et le code qui démarre le bundle WebAssembly, puis le `<body>`. Sa méthode `render()` renvoie la page sous forme de `String`, prête à être envoyée par un serveur ou écrite dans un fichier.

## Une page complète

```rust
use active::prelude::*;

fn page() -> String {
    let body = el("main")
        .child(Seo::new().title("Contact").description("Write to us, we answer within a day."))
        .child(h1().text("Contact"));

    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
        .stylesheet("/static/app.css")
        .head(r#"<link rel="icon" href="/favicon.svg" type="image/svg+xml"/>"#)
        .client("/pkg/acme.js", "/pkg/acme_bg.wasm")
        .html_attr("class", "light")
        .body_attr("class", "page-contact")
        .render()
}

let html = page();
assert!(html.starts_with(r#"<!DOCTYPE html><html lang="en" class="light"><head><meta charset="utf-8"/>"#));
assert!(html.contains("<title>Contact · Acme</title>"));
assert!(html.ends_with(r#"<body class="page-contact"><main><h1>Contact</h1></main></body></html>"#));
```

## Ce qu'écrit render

La sortie tient sur une seule ligne de HTML. Mise en forme pour la lecture, sa structure est toujours la suivante :

```html
<!DOCTYPE html>
<html lang="en" class="light">                      <!-- lang, then html_attr -->
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Contact · Acme</title>                     <!-- merged Seo tags -->
  <meta name="description" content="…"/>
  …
  <style>active-island{display:contents}</style>
  <link rel="stylesheet" href="/static/app.css"/>   <!-- stylesheet, in call order -->
  <link rel="modulepreload" href="/pkg/acme.js"/>   <!-- client -->
  <link rel="preload" href="/pkg/acme_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/acme.js";init({module_or_path:"/pkg/acme_bg.wasm"});</script>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>   <!-- head, in call order -->
</head>
<body class="page-contact">                         <!-- body_attr -->
  <main><h1>Contact</h1></main>                     <!-- your view -->
</body>
</html>
```

Les balises charset et viewport sont toujours écrites : chaque page est donc en UTF-8 et adaptée aux mobiles. Le petit style `active-island` empêche les conteneurs des [îlots](../islands/) d'influer sur la mise en page.

## Le builder

| Méthode | Effet |
| --- | --- |
| `Document::new(view)` | une page dont le `<body>` contient `view` (tout ce qui se convertit en `Node`) |
| `.lang(code)` | `<html lang="…">`, par exemple `"en"`, `"fr"` |
| `.seo(seo)` | le `Seo` de base, fusionné avec chaque `Seo` trouvé dans la vue ; un nouvel appel le remplace |
| `.stylesheet(href)` | un `<link rel="stylesheet">` ; appelez-la une fois par fichier |
| `.head(html)` | du HTML de confiance ajouté à la fin du `<head>` : icônes, polices, mesure d'audience, `theme-color` |
| `.client(js, wasm)` | charge le bundle WebAssembly (voir plus bas) |
| `.html_attr(name, value)` | un attribut sur `<html>` |
| `.body_attr(name, value)` | un attribut sur `<body>` |
| `.render()` | la page entière sous forme de `String` |
| `.final_seo()` | le `Seo` fusionné de la page, sans rendu (utile dans les tests) |

`Document` implémente `Clone` et `Default`.

> **Attention :** `.head(...)` insère le HTML tel quel. Ne passez que du balisage que vous avez écrit vous-même.

## SEO

`Document` récupère chaque `Seo` de la vue, y compris dans les îlots et dans le contenu courant des régions dynamiques, et les fusionne par-dessus la base passée à `.seo(...)`. Le résultat est écrit une seule fois : la page a donc toujours exactement un `<title>`. Les règles sont décrites dans [SEO, Fusion](../seo/#fusion).

`final_seo()` renvoie cette valeur fusionnée, ce qui rend le SEO facile à tester :

```rust
use active::prelude::*;

let doc = Document::new(div().child(Seo::new().title("Pricing")))
    .seo(Seo::new().title_template("%s · Acme"));
assert!(doc.final_seo().to_html().starts_with("<title>Pricing · Acme</title>"));
```

## Charger le bundle du navigateur

`wasm-bindgen --target web --no-typescript` produit deux fichiers : un petit module JavaScript (le chargeur) et le binaire `.wasm`. `.client(js, wasm)` reçoit leurs URL et écrit trois choses dans le `<head>` :

1. `<link rel="modulepreload">` pour le chargeur ;
2. `<link rel="preload" as="fetch" type="application/wasm">` pour le binaire, afin qu'il se télécharge **en parallèle** du HTML et du chargeur plutôt qu'après eux ;
3. une ligne `<script type="module">` qui importe le chargeur et démarre le module avec l'URL du binaire.

Au démarrage du module, sa fonction `#[wasm_bindgen(start)]` s'exécute : elle [hydrate les îlots](../islands/) ou [monte une application](../client-app/). Cette ligne est le seul JavaScript de la page, en dehors du chargeur généré par `wasm-bindgen`.

Les pages sans îlot n'ont pas besoin du bundle : omettez `.client(...)` et elles n'envoient aucun code.

Les URL sont écrites de façon sûre dans une chaîne JavaScript (les guillemets, les barres obliques inverses et `<` sont échappés) : elles peuvent donc contenir des paramètres de requête comme `?v=…`.

### Mise en cache longue durée

Les navigateurs peuvent conserver indéfiniment le bundle et les feuilles de style si leur URL change dès que leur contenu change. Une méthode courante consiste à ajouter un paramètre de version calculé à partir du contenu du fichier, comme le fait ce site dans [`site/src/server/assets.rs`](https://github.com/maxlestage/Active/blob/master/site/src/server/assets.rs) :

```rust
/// Un court hachage du contenu (FNV-1a), suffisant pour distinguer les versions d'un fichier.
fn version(bytes: &[u8]) -> String {
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")[..10].to_string()
}

fn versioned(url: &str, bytes: &[u8]) -> String {
    format!("{url}?v={}", version(bytes))
}

let url = versioned("/pkg/acme_bg.wasm", b"\0asm");
assert!(url.starts_with("/pkg/acme_bg.wasm?v="));
```

Lisez les fichiers une fois au démarrage, calculez leurs URL versionnées et passez-les à `.stylesheet(...)` et `.client(...)`. Servez les URL qui portent un `?v=` avec `Cache-Control: public, max-age=31536000, immutable`, et le HTML lui-même avec `no-cache`.

## Rendu

`render()` rend d'abord le corps, en récupérant ses valeurs `Seo` au passage, puis écrit la page autour : chaque région dynamique est évaluée une seule fois par rendu. Un document peut être rendu autant de fois que nécessaire ; sur le serveur, les états qu'utilise sa vue sont libérés automatiquement dès que les rendus suivants n'en ont plus besoin (voir [État, Côté serveur](../state/#cote-serveur)). `final_seo()`, quand vous l'appelez vous-même, évalue les régions dynamiques séparément.

Pour envoyer la page, renvoyez-la avec le type de contenu `text/html; charset=utf-8`. Avec Axum, l'envelopper dans `axum::response::Html` s'en charge pour vous (voir [Axum](../axum/)). Pour un site statique, écrivez-la dans un fichier `index.html` (voir [Sites statiques](../static-site/)).

## Sans Document

Pour maîtriser entièrement le balisage de la page (un template existant, un autre framework), rendez les morceaux vous-même : `Node::render()` donne le HTML du corps et `Seo` donne les balises du `<head>` avec `to_html()`, `write_html()` ou `format!("{seo}")`. Vous perdez alors la fusion automatique des valeurs `Seo` trouvées dans la vue ainsi que le chargeur du bundle, qu'il vous faudrait écrire à la main.
