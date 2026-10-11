---
title: Sites statiques
description: Générez les pages d'un site active en fichiers HTML à la compilation, avec sitemap et robots.txt, hébergez-les gratuitement et gardez les îlots interactifs.
---

Quand vos pages ne dépendent pas de la requête (un blog, une documentation, un portfolio, une page de présentation), vous n'avez besoin d'aucun serveur. Un petit programme Rust rend chaque page dans un fichier HTML, une seule fois, au moment de la compilation, et n'importe quel hébergeur statique sert les fichiers. Les pages conservent leurs balises SEO complètes, et les [îlots](../islands/) sont toujours hydratés dans le navigateur, car l'hydratation n'a besoin que du HTML et du bundle.

## Fonctionnement

```text
cargo build --lib --target wasm32-unknown-unknown …   →  pkg/            le bundle des îlots
cargo run --release                                    →  dist/index.html
                                                          dist/posts/hello-rust/index.html
                                                          dist/pkg/…, dist/sitemap.xml, dist/robots.txt
```

L'organisation du crate est l'organisation habituelle ([Installation](../installation/#un-crate-deux-cibles)) : les pages et les îlots dans `src/lib.rs`, et `src/main.rs` devient un générateur au lieu d'un serveur. Il n'a besoin d'aucune dépendance en dehors d'`active` (et de `wasm-bindgen` pour le navigateur).

## Les pages

```rust
use active::prelude::*;

/// Adresse publique du site, sans barre oblique finale.
pub const ORIGIN: &str = "https://example.com";
/// Chemin sous lequel le site est publié : "" à la racine d'un domaine, "/my-blog" pour
/// le site d'un projet sur GitHub Pages (https://<user>.github.io/my-blog/).
pub const BASE: &str = "";

pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

pub static POSTS: [Post; 2] = [
    Post { slug: "hello-rust", title: "Hello, Rust", summary: "Why this blog is written in Rust." },
    Post { slug: "islands", title: "Islands", summary: "Interactive parts on a static page." },
];

/// Un lien interne au site, avec le chemin de base.
fn url(path: &str) -> String {
    format!("{BASE}{path}")
}

/// Îlot : un bouton « j'aime », interactif une fois le bundle chargé.
pub fn likes(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(0));
    button().on_click(move |_| n.update(|n| *n += 1)).text_dyn(move || format!("♥ {}", n.get())).into()
}

fn document(content: Element) -> String {
    let body = div()
        .child(header().child(a().href(url("/")).text("My blog")))
        .child(main().child(content));
    Document::new(body)
        .lang("en")
        .seo(
            Seo::new()
                .site_name("My blog")
                .title_template("%s · My blog")
                .base_url(format!("{ORIGIN}{BASE}")),
        )
        .stylesheet(url("/static/app.css"))
        .client(url("/pkg/my_blog.js"), url("/pkg/my_blog_bg.wasm"))
        .render()
}

fn home() -> String {
    document(
        section()
            .child(Seo::new().title("Home").description("Notes about Rust and the web.").canonical("/"))
            .child(h1().text("My blog"))
            .child(ul().children(POSTS.iter().map(|post| {
                li().child(a().href(url(&format!("/posts/{}/", post.slug))).text(post.title))
            }))),
    )
}

fn post_page(post: &Post) -> String {
    document(
        article()
            .child(
                Seo::new()
                    .title(post.title)
                    .description(post.summary)
                    .canonical(format!("/posts/{}/", post.slug))
                    .og_type("article"),
            )
            .child(h1().text(post.title))
            .child(p().text(post.summary))
            .child(island("likes", "0", likes)),
    )
}

/// La page 404 que les hébergeurs statiques affichent pour les URL inconnues.
pub fn not_found() -> String {
    document(
        section()
            .child(Seo::new().title("Page not found").noindex(true))
            .child(h1().text("Page not found")),
    )
}

/// Toutes les pages du site, sous forme de couples (chemin, HTML).
pub fn pages() -> Vec<(String, String)> {
    let mut pages = vec![("/".to_string(), home())];
    for post in &POSTS {
        pages.push((format!("/posts/{}/", post.slug), post_page(post)));
    }
    pages
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("likes", likes)]);
}
```

Chaque page est rendue avec son propre `Document` : chacune obtient donc son propre titre, sa propre description et sa propre URL canonique. Les chemins se terminent par `/` et sont écrits sous forme de fichiers `index.html` dans un dossier du même nom, que tous les hébergeurs statiques servent à cette URL propre.

## Le générateur

`src/main.rs` écrit les pages, copie le bundle et les fichiers statiques, puis ajoute le sitemap et `robots.txt` :

```rust
use std::fs;
use std::io;
use std::path::Path;

use my_blog::{BASE, ORIGIN};

fn main() -> io::Result<()> {
    let out = Path::new("dist");
    if out.exists() {
        fs::remove_dir_all(out)?;
    }

    // 1. Un index.html par page.
    let pages = my_blog::pages();
    for (path, html) in &pages {
        let dir = out.join(path.trim_start_matches('/'));
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }
    fs::write(out.join("404.html"), my_blog::not_found())?;

    // 2. Le bundle des îlots et les fichiers statiques.
    copy_dir(Path::new("pkg"), &out.join("pkg"))?;
    copy_dir(Path::new("static"), &out.join("static"))?;

    // 3. Sitemap et robots.txt, avec des URL absolues.
    let mut sitemap = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for (path, _) in &pages {
        sitemap.push_str(&format!("<url><loc>{ORIGIN}{BASE}{path}</loc></url>"));
    }
    sitemap.push_str("</urlset>\n");
    fs::write(out.join("sitemap.xml"), sitemap)?;
    fs::write(
        out.join("robots.txt"),
        format!("User-agent: *\nAllow: /\n\nSitemap: {ORIGIN}{BASE}/sitemap.xml\n"),
    )?;

    println!("{} pages written to dist/", pages.len());
    Ok(())
}

/// Copie (récursivement) les fichiers de `from` dans `to`. Un `from` absent est ignoré.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(from) else {
        return Ok(());
    };
    fs::create_dir_all(to)?;
    for entry in entries {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
```

## Compilation

Compilez d'abord le bundle, puis lancez le générateur :

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
cargo run --release
```

`dist/` contient maintenant tout le site. Pour le consulter en local, servez le dossier avec n'importe quel serveur de fichiers statiques, par exemple :

```sh
python3 -m http.server --directory dist 8000
```

puis ouvrez [http://localhost:8000](http://localhost:8000). (Ouvrir directement les fichiers avec `file://` ne fonctionne pas : les navigateurs refusent de charger des modules WebAssembly depuis des fichiers locaux.)

Un site sans îlots n'a pas besoin de bundle : supprimez `.client(...)`, la fonction `start` et les deux premières commandes.

## Sitemap et robots.txt

Les moteurs de recherche découvrent les pages grâce à `sitemap.xml`, qui liste chaque URL avec son adresse absolue, et trouvent le sitemap grâce à `robots.txt`. Le générateur ci-dessus écrit les deux à partir de la même liste de pages : ils ne sont donc jamais désynchronisés. Soumettez l'adresse du sitemap dans [Google Search Console](https://search.google.com/search-console) et dans Bing Webmaster Tools pour accélérer l'indexation.

Pour un site multilingue, ajoutez dans chaque `<url>` un élément `<xhtml:link rel="alternate" hreflang="…">` par langue, comme le fait le serveur de ce site dans `site/src/server/mod.rs`.

## Hébergement

`dist/` ne contient que des fichiers : n'importe quel hébergeur statique convient, souvent gratuitement.

### GitHub Pages

1. Dans le dépôt, ouvrez **Settings**, puis **Pages**, et réglez **Source** sur **GitHub Actions**.
2. Donnez à `BASE` la valeur `"/<repository>"` (ou `""` avec un domaine personnalisé ou un dépôt `<user>.github.io`), et à `ORIGIN` la valeur `https://<user>.github.io`.
3. Ajoutez ce workflow sous le nom `.github/workflows/pages.yml` :

```yaml
name: Pages

on:
  push:
    branches: [master]

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
      - run: |
          cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
          wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
          cargo run --release
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Chaque push sur `master` recompile et publie le site. Cela fonctionne aussi depuis l'application GitHub : modifiez un fichier, faites un commit, et le site est mis à jour quelques minutes plus tard.

### Netlify, Cloudflare Pages et les autres

Compilez `dist/` en CI (comme ci-dessus) et déployez le dossier avec l'action ou l'outil en ligne de commande de l'hébergeur, ou glissez-déposez `dist/` dans le tableau de bord de l'hébergeur pour un déploiement ponctuel. Ces hébergeurs servent `404.html` pour les URL inconnues et définissent automatiquement le type de contenu `application/wasm`.

## Installable et hors ligne

Un site statique peut aussi s'installer sur l'écran d'accueil et continuer de fonctionner hors ligne : écrivez `manifest.webmanifest` et `sw.js` à côté des pages avec `active::pwa`, et ajoutez `.manifest(...)` et `.service_worker(...)` à chaque `Document`. Voir [Applications installables (PWA)](../pwa/#sites-statiques-et-chemin-de-base).

## Mettre à jour le contenu

Le site ne change que lorsque vous le recompilez. Le contenu se trouve généralement dans le dépôt (des données Rust comme ci-dessus, ou des fichiers Markdown lus par le générateur avec un crate comme [`pulldown-cmark`](https://docs.rs/pulldown-cmark) et insérés avec `.html(...)`) : un commit déclenche donc une recompilation via la CI. Quand les pages doivent changer à chaque requête ou pour chaque visiteur, utilisez plutôt un serveur : voir [Axum](../axum/).
