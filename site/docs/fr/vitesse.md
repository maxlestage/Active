---
title: Vitesse
description: Servez des pages active avec Vitesse : routes renvoyant du Html, îlots, fichiers statiques, page 404, sitemap, données asynchrones, compression, tests et rendu par requête sûr.
---

[Vitesse](https://maxlestage.github.io/Vitesse/) est un framework web Rust qui reprend l'API d'Express : `app.get`, `req.param`, middlewares, routeurs, son propre moteur HTTP/1.1 sur tokio. Rendre une page `active` dans un gestionnaire Vitesse tient en une ligne, `Html(Document::new(view).render())` ; cette page construit autour une petite boutique complète : des pages avec leurs balises SEO, un îlot, des fichiers statiques, une page 404, un sitemap et des données chargées de manière asynchrone. Ce site web est lui-même un serveur Vitesse, dans `site/src/server`.

## Dépendances

L'organisation du crate est celle de la page [Installation](../installation/#un-crate-deux-cibles) : les composants dans `src/lib.rs`, le serveur dans `src/main.rs`. Vitesse apporte son propre runtime : aucun tokio à ajouter.

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
active = "0.1"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
vitesse = { version = "0.1", features = ["compress"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"
```

## Les pages

`src/lib.rs` contient les composants, l'îlot et une fonction par page. Chaque fonction de page renvoie le HTML final :

```rust
use active::prelude::*;

pub struct Product {
    pub slug: &'static str,
    pub name: &'static str,
    pub price: u32,
}

pub static PRODUCTS: [Product; 2] = [
    Product { slug: "oak-chair", name: "Oak chair", price: 249 },
    Product { slug: "pine-table", name: "Pine table", price: 590 },
];

pub fn find_product(slug: &str) -> Option<&'static Product> {
    PRODUCTS.iter().find(|p| p.slug == slug)
}

/// Îlot : un sélecteur de quantité, interactif dans le navigateur.
pub fn quantity(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(1));
    div()
        .class("quantity")
        .child(button().on_click(move |_| n.update(|n| *n = n.saturating_sub(1).max(1))).text("−"))
        .child(output().text_dyn(move || n.get().to_string()))
        .child(button().on_click(move |_| n.update(|n| *n += 1)).text("+"))
        .into()
}

pub const ISLANDS: [(&str, active::IslandFn); 1] = [("quantity", quantity)];

/// Commun à toutes les pages. `origin` est l'adresse publique, par exemple "https://acme.com".
fn document(origin: &str, content: Element) -> String {
    let body = div()
        .child(header().child(a().href("/").text("Acme")))
        .child(main().child(content))
        .child(footer().text("© Acme"));
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url(origin.to_string()))
        .stylesheet("/static/app.css")
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}

pub fn home_page(origin: &str) -> String {
    let content = section()
        .child(Seo::new().title("Furniture").description("Solid wood furniture.").canonical("/"))
        .child(h1().text("Our furniture"))
        .child(ul().children(PRODUCTS.iter().map(|p| {
            li().child(a().href(format!("/products/{}", p.slug)).text(p.name))
        })));
    document(origin, content)
}

pub fn product_page(origin: &str, product: &Product) -> String {
    let content = article()
        .child(
            Seo::new()
                .title(product.name)
                .description(format!("{}, {} €, delivered in 48 hours.", product.name, product.price))
                .canonical(format!("/products/{}", product.slug))
                .og_type("product"),
        )
        .child(h1().text(product.name))
        .child(p().text(format!("{} €", product.price)))
        .child(island("quantity", "1", quantity));
    document(origin, content)
}

pub fn not_found_page(origin: &str) -> String {
    let content = section()
        .child(Seo::new().title("Page not found").noindex(true))
        .child(h1().text("Page not found"))
        .child(a().href("/").text("Back to the shop"));
    document(origin, content)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&ISLANDS);
}
```

## Le serveur

`src/main.rs` associe les URL aux fonctions de page et sert les fichiers :

```rust
use vitesse::prelude::*;

/// Commun à toutes les requêtes.
struct Shop {
    /// Adresse publique, utilisée dans les URL canoniques et le sitemap.
    origin: String,
}

async fn home(req: Request) -> Html<String> {
    let shop = req.state::<Shop>();
    Html(my_site::home_page(&shop.origin))
}

async fn product(req: Request) -> Response {
    let shop = req.state::<Shop>();
    match my_site::find_product(req.param("slug").unwrap_or("")) {
        Some(product) => Html(my_site::product_page(&shop.origin, product)).into_response(),
        None => not_found(req).await,
    }
}

async fn not_found(req: Request) -> Response {
    let shop = req.state::<Shop>();
    res::status(404).html(my_site::not_found_page(&shop.origin))
}

async fn sitemap(req: Request) -> Response {
    let shop = req.state::<Shop>();
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    let paths = std::iter::once("/".to_string())
        .chain(my_site::PRODUCTS.iter().map(|p| format!("/products/{}", p.slug)));
    for path in paths {
        xml.push_str(&format!("<url><loc>{}{path}</loc></url>", shop.origin));
    }
    xml.push_str("</urlset>");
    res::send(xml).header("content-type", "application/xml; charset=utf-8")
}

async fn robots(req: Request) -> String {
    let shop = req.state::<Shop>();
    format!("User-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n", shop.origin)
}

fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let origin = std::env::var("SITE_URL").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let mut app = App::new();
    app.state(Shop { origin });
    app.middleware(middleware::compress());
    app.get("/", home)
        .get("/products/:slug", product)
        .get("/sitemap.xml", sitemap)
        .get("/robots.txt", robots)
        .static_dir("/pkg", "pkg")
        .static_dir("/static", "static")
        .fallback(not_found);
    println!("http://localhost:{port}");
    app.run(port)
}
```

Compilez le bundle dans `pkg/` ([Compiler le client](../client-build/)), placez `app.css` dans `static/`, puis lancez `cargo run`.

Le rôle de chaque partie :

- **`Html(...)`** définit `Content-Type: text/html; charset=utf-8`.
- **`static_dir`** sert le bundle et les fichiers statiques avec les bons types de contenu, y compris `application/wasm`, et les en-têtes `ETag` et `Last-Modified` grâce auxquels les navigateurs les gardent en cache.
- **`middleware::compress()`** compresse les pages, le chargeur et le fichier `.wasm` avec Brotli ou gzip, selon ce qu'accepte le navigateur (la fonctionnalité `compress` de Vitesse).
- **`app.state(...)`** partage une valeur avec tous les gestionnaires, qui la lisent avec `req.state()`.
- **`PORT` et `SITE_URL`** viennent de l'environnement : c'est ainsi que des hébergeurs comme [Heroku](../deploy-heroku/) configurent une application. `app.run` écoute sur toutes les interfaces, démarre le runtime sur tous les cœurs et s'arrête proprement sur Ctrl+C ou SIGTERM.

## Erreurs et 404

Renvoyez le bon code de statut avec la page : les moteurs de recherche indexent une page servie avec un `200`, même si elle indique « page introuvable ». Dans l'exemple, les produits inconnus et les URL inconnues (`fallback`) répondent tous deux `404` avec une page marquée `noindex`.

## Données asynchrones

Les vraies pages chargent leurs données depuis une base de données ou une API. Faites toute l'attente d'abord, puis construisez et rendez la vue **sans aucun `.await` entre les deux** :

```rust
use active::prelude::*;
use vitesse::prelude::*;

/// Représente une requête en base de données ou un appel HTTP.
async fn fetch_reviews(slug: &str) -> Vec<String> {
    vec![format!("Great {slug}, five stars.")]
}

fn reviews_page(slug: &str, reviews: &[String]) -> String {
    let body = section()
        .child(Seo::new().title(format!("Reviews of {slug}")))
        .child(ul().children(reviews.iter().map(|r| li().text(r.clone()))));
    Document::new(body).render()
}

async fn reviews(req: Request) -> Html<String> {
    let slug = req.param("slug").unwrap_or("").to_string();
    // 1. Attendre tout ce dont la page a besoin.
    let reviews = fetch_reviews(&slug).await;
    // 2. Construire et rendre en un seul appel synchrone.
    Html(reviews_page(&slug, &reviews))
}
```

Le compilateur fait respecter cette règle. Vitesse exécute les gestionnaires sur plusieurs threads, leurs futures doivent donc être `Send`. Les vues (qui contiennent des closures à comptage de références) et les états (qui appartiennent au thread qui les a créés) ne sont pas `Send` : un gestionnaire qui conserve l'un ou l'autre au-delà d'un `.await` ne compile pas. La solution est toujours la même : déplacer le `.await` avant le code qui construit la vue.

> **Astuce :** L'erreur indique que la future « cannot be sent between threads safely », ou que la fonction n'est pas un `Handler` ; dans les deux cas, elle désigne la vue ou l'état conservé au-delà du `.await`.

## Pourquoi le rendu par requête est sûr

`active` garde les états dans un registre propre au thread courant. Avec le modèle ci-dessus, chaque requête construit sa vue et la rend en un seul appel synchrone, sur un seul thread, du début à la fin : rien d'une autre requête ne peut s'exécuter entre-temps, et comme les vues et les états ne peuvent pas franchir un `.await`, rien ne peut fuir d'une requête à l'autre. À la fin de chaque rendu, les états que les deux derniers rendus n'ont pas utilisés sont libérés : un serveur peut ainsi rendre des pages en parallèle sur tous ses threads, aussi longtemps qu'il tourne, en ne gardant qu'une poignée d'états vivants. Les détails se trouvent dans [État, Côté serveur](../state/#cote-serveur).

## Intégrer le bundle au binaire

Pour déployer un fichier unique, intégrez le bundle avec `include_bytes!` au lieu de servir `pkg/` depuis le disque. Compilez le bundle avant le serveur, car les fichiers sont lus à la compilation :

```rust
use vitesse::prelude::*;

static JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site.js"));
static WASM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/my_site_bg.wasm"));

/// Les routes du bundle intégré, à monter dans n'importe quelle application.
fn bundle() -> Router {
    let mut router = Router::new();
    router
        .get("/my_site.js", |_| async { res::send(JS).header("content-type", "text/javascript") })
        .get("/my_site_bg.wasm", |_| async { res::send(WASM).header("content-type", "application/wasm") });
    router
}
```

Montez-les avec `app.mount("/pkg", bundle())`. Avec un hachage du contenu dans les URL (voir [Documents, Mise en cache longue durée](../documents/#mise-en-cache-longue-duree)), vous pouvez aussi ajouter un en-tête `Cache-Control: public, max-age=31536000, immutable`.

## Tests

Le client de test de Vitesse appelle l'application sans réseau, ce qui rend les tests de pages rapides :

```rust
use vitesse::prelude::*;
use vitesse::test::TestClient;

#[tokio::test]
async fn home_has_a_title() {
    let mut app = App::new();
    app.get("/", |_| async { Html("<title>Home</title>") });
    let client = TestClient::new(app);
    let response = client.get("/").await;
    assert_eq!(response.status(), 200);
    assert!(response.text().contains("<title>Home</title>"));
}
```

Il faut pour cela `tokio = { version = "1", features = ["macros", "rt"] }` dans `[dev-dependencies]`. Les tests de ce site, dans `site/tests/server.rs`, vérifient ainsi ses balises SEO, ses redirections et sa mise en cache.

## Autres serveurs

Rien dans `active` n'est propre à Vitesse : `Document::render()` renvoie une `String`. Avec Axum, Actix Web, Rocket, Warp, Hyper ou un serveur écrit à la main, envoyez cette chaîne avec le type de contenu `text/html; charset=utf-8`. `examples/hello` le fait avec la seule bibliothèque standard.
