---
title: Applications installables (PWA)
description: Rendez un site active installable et utilisable hors ligne avec active::pwa : manifeste d'application web, service worker et sa stratégie de cache, méthodes de Document, service et export, icônes, tests hors ligne.
---

Une Progressive Web App est un site web que les téléphones et les ordinateurs peuvent installer : il obtient une icône sur l'écran d'accueil, s'ouvre en plein écran sans la barre du navigateur et continue de fonctionner quand le réseau est coupé. Pour faire d'un site active une PWA, il suffit de deux fichiers et de quelques balises, tous écrits en Rust par le module `active::pwa` :

- le **manifeste d'application web** (`manifest.webmanifest`) : le nom, les icônes et les couleurs de l'application installée, écrit par `Manifest` ;
- le **service worker** (`sw.js`) : un petit script que le navigateur exécute à côté des pages, qui enregistre le site sur l'appareil et répond à sa place hors ligne, généré par `ServiceWorker` ;
- dans chaque page, `Document::manifest` et `Document::service_worker` ajoutent le lien vers le manifeste, la couleur du thème, le titre et l'icône de l'écran d'accueil, et la ligne qui enregistre le worker.

Aucun JavaScript à écrire : les navigateurs n'exécutent les service workers que sous forme de scripts, alors active génère le worker à partir de vos réglages en Rust. Ce site est une PWA construite ainsi : installez-le depuis le menu de votre navigateur, ou passez en mode avion et rechargez la page.

Les service workers ne fonctionnent que dans un contexte sécurisé : HTTPS, ou `http://localhost` pendant le développement.

## Le manifeste

```rust
use active::pwa::Manifest;

fn manifest() -> Manifest {
    Manifest::new("Acme")
        .short_name("Acme")
        .description("Les recettes de la cuisine Acme, même hors ligne.")
        .start_url("/")
        .scope("/")
        .id("/")
        .theme_color("#6d4aff")
        .background_color("#0b0b10")
        .lang("fr")
        .categories(["food"])
        .icon("/icon-192.png", "192x192", "image/png")
        .icon("/icon-512.png", "512x512", "image/png")
        .maskable_icon("/icon-maskable-512.png", "512x512", "image/png")
        .icon("/favicon.svg", "any", "image/svg+xml")
}

let json = manifest().to_json();
assert!(json.starts_with(r#"{"name":"Acme","short_name":"Acme""#));
assert!(json.contains(r#""start_url":"/","scope":"/","display":"standalone""#));
```

| Méthode | Effet |
| --- | --- |
| `Manifest::new(name)` | un manifeste nommé `name`, qui s'ouvre sur `./` en mode `standalone` (plein écran, sans barre du navigateur) |
| `.short_name(name)` | le nom sous l'icône ; 12 caractères au plus pour un bon rendu |
| `.description(text)` | ce que fait l'application |
| `.start_url(url)` | la page qu'ouvre l'icône (`./` par défaut : relatif au manifeste) |
| `.scope(url)` | les URL qui font partie de l'application (par défaut le dossier du manifeste) |
| `.id(url)` | l'identité de l'application (`start_url` par défaut) ; ne la changez plus une fois publiée |
| `.display(mode)` | `standalone` (par défaut), `fullscreen`, `minimal-ui` ou `browser` |
| `.orientation(value)` | `portrait`, `landscape`, `any`… |
| `.theme_color(color)` | la couleur de la barre d'état et du titre de la fenêtre (`#000000` par défaut) |
| `.background_color(color)` | la couleur de l'écran de démarrage pendant le lancement (`#ffffff` par défaut) |
| `.lang(code)` | la langue des textes : `fr`, `en`, `fr-CA`… |
| `.categories(list)` | catégories des stores : `productivity`, `education`… |
| `.icon(src, sizes, mime)` | une icône : `"192x192"`, `"512x512"`, ou `"any"` pour un SVG |
| `.maskable_icon(src, sizes, mime)` | une icône dessinée pour les masques ronds ou arrondis (voir [Icônes](#icones)) |
| `.to_json()` | le manifeste en JSON, à servir sur `/manifest.webmanifest` |

`name()`, `theme()` et `icons()` relisent les valeurs. Toutes les chaînes sont échappées : noms et descriptions peuvent contenir des guillemets ou des `<`.

L'URL de démarrage doit être une page qui répond **200** et que le worker enregistre : une redirection ne peut pas être rejouée depuis le cache, donc une application qui démarre sur un `/` qui redirige s'ouvrirait sur la page hors ligne. C'est pour cela que ce site démarre sur `/en/` : son `/` redirige vers la langue du visiteur.

## Le service worker

```rust
use active::pwa::ServiceWorker;

fn worker() -> ServiceWorker {
    ServiceWorker::new("acme", "2026-10-10.1")
        .precache([
            "/",
            "/static/app.css",
            "/pkg/acme.js",
            "/pkg/acme_bg.wasm",
            "/icon-192.png",
            "/icon-512.png",
        ])
        .offline_page("/offline/")
        .network_only("/api/")
}

let js = worker().to_js();
assert!(js.contains(r#"const CACHE = "acme-2026-10-10.1";"#));
assert!(js.contains(r#"const OFFLINE = "/offline/";"#));
assert!(js.contains(r#"const NETWORK_ONLY = ["/api/"];"#));
```

| Méthode | Effet |
| --- | --- |
| `ServiceWorker::new(name, version)` | un worker dont le cache s'appelle `{name}-{version}` |
| `.precache(urls)` | les URL enregistrées à l'installation du worker, disponibles hors ligne dès la première visite ; appelez-la autant de fois que nécessaire |
| `.offline_page(url)` | la page affichée quand une page n'a jamais été enregistrée et que le réseau est coupé ; elle est aussi mise en cache d'avance |
| `.network_only(prefix)` | les URL dont le chemin commence par `prefix` passent toujours par le réseau (une API, un flux en direct…) |
| `.cache_name()` | `{name}-{version}` |
| `.to_js()` | le script, à servir à la racine du site |

### Stratégie

Le worker généré traite les requêtes `GET` de son propre site et laisse tout le reste au navigateur (les formulaires en `POST`, les autres sites) :

- **Les pages** (navigations) viennent **d'abord du réseau** : un visiteur en ligne voit toujours la dernière version, et chaque page reçue est enregistrée. Hors ligne, c'est la copie enregistrée qui s'affiche, sinon la page hors ligne.
- **Les autres fichiers** du site (feuille de style, bundle WebAssembly, images, polices, données chargées par la page) viennent **tout de suite du cache**, et sont rafraîchis en arrière-plan pour la visite suivante. Un fichier pas encore enregistré vient du réseau, puis est enregistré.
- **Les préfixes `network_only`** ne sont jamais mis en cache. Le préfixe est comparé au chemin de l'URL : il inclut donc le chemin de base du site s'il en a un.
- **Une nouvelle version** (une autre chaîne `version`) s'installe, télécharge sa liste de fichiers, prend aussitôt le contrôle des pages ouvertes et supprime les caches des anciennes versions du même `name`.

### Quoi mettre en cache d'avance

Mettez en cache ce dont un visiteur a besoin pour ouvrir l'application hors ligne : la page de démarrage et les pages principales, la feuille de style, le bundle WebAssembly (le chargeur JavaScript et le fichier `.wasm`), les polices et les icônes. Utilisez **exactement les URL qu'utilisent les pages** : le cache tient compte de la chaîne de requête, donc si une page charge `/static/app.css?v=3f2a…`, mettez cette URL en cache et non `/static/app.css`. Laissez de côté les fichiers qui n'existent pas : un seul téléchargement raté annule l'installation.

### Choisir la version

Changez `version` à chaque déploiement qui modifie quelque chose que le worker enregistre ; sinon les navigateurs gardent l'ancien worker et ses anciens fichiers. Un hash de ces fichiers s'en charge automatiquement, le même hash FNV-1a que ce site utilise pour ses URL en `?v=` (voir [Documents, Mise en cache longue durée](../documents/#mise-en-cache-longue-duree)) :

```rust
/// Un hash court de tout ce que le worker enregistre : nouveaux fichiers, nouveau worker.
fn version(files: &[&[u8]]) -> String {
    let hash = files.iter().flat_map(|f| f.iter()).fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")[..10].to_string()
}

let v1 = version(&[b"body{color:red}", b"\0asm"]);
assert_ne!(v1, version(&[b"body{color:blue}", b"\0asm"]));
```

Ce site hache les pages mises en cache telles qu'elles sont rendues, le manifeste et tous les fichiers statiques : le moindre changement publie un nouveau worker.

## Les balises de chaque page

```rust
use active::pwa::Manifest;
use active::prelude::*;

let manifest = Manifest::new("Acme")
    .short_name("Acme")
    .theme_color("#6d4aff")
    .icon("/icon-192.png", "192x192", "image/png")
    .icon("/icon-512.png", "512x512", "image/png");
let html = Document::new(h1().text("Recettes"))
    .manifest("/manifest.webmanifest", &manifest)
    .service_worker("/sw.js")
    .render();
assert!(html.contains(r#"<link rel="manifest" href="/manifest.webmanifest"/>"#));
assert!(html.contains(r##"<meta name="theme-color" content="#6d4aff"/>"##));
assert!(html.contains(r#"<link rel="apple-touch-icon" href="/icon-512.png"/>"#));
assert!(html.contains(r#"navigator.serviceWorker.register("/sw.js")"#));
```

`.manifest(href, &manifest)` écrit dans le `<head>` :

- `<link rel="manifest">` vers `href`, où vous servez `manifest.to_json()` ;
- `<meta name="theme-color">` avec la couleur du thème du manifeste (n'écrivez pas la vôtre) ;
- `mobile-web-app-capable`, `apple-mobile-web-app-capable` et `apple-mobile-web-app-title` (le nom court, sinon le nom), pour les navigateurs qui lisent ces balises plutôt que le manifeste ;
- `<link rel="apple-touch-icon">` avec la plus grande icône PNG de type `any` : iOS l'utilise pour l'écran d'accueil et ignore les icônes du manifeste.

`.service_worker(url)` ajoute un petit script en ligne qui enregistre le worker une fois la page chargée, pour ne jamais retarder le premier affichage :

```html
<script>if("serviceWorker"in navigator)addEventListener("load",()=>navigator.serviceWorker.register("/sw.js"))</script>
```

Appelez les deux sur **chaque** page, pour que l'application puisse s'installer depuis n'importe laquelle. Ce sont des raccourcis pour [`.head(...)`](../documents/#le-builder) ; l'URL est échappée sans risque.

### Content-Security-Policy

Si votre serveur envoie une Content-Security-Policy sans `'unsafe-inline'`, autorisez le script d'enregistrement par son hash : calculez le SHA-256 de `active::pwa::register_script(url)` (le texte exact entre les balises `<script>`), encodez-le en base64 et ajoutez `'sha256-…'` à `script-src`, comme pour vos autres scripts en ligne. Le worker et le manifeste sont des fichiers de votre propre site : gardez `'self'` dans `script-src` (ou `worker-src`) et dans `default-src` (ou `manifest-src`).

## Servir les fichiers

Le manifeste doit être servi en `application/manifest+json` et le worker en JavaScript. Servez le worker avec `Cache-Control: no-cache`, pour que les navigateurs voient une nouvelle version dès qu'elle est déployée. Avec [Axum](../axum/) :

```rust
use active::pwa::{Manifest, ServiceWorker};
use axum::{Router, http::header, response::IntoResponse, routing::get};

fn manifest() -> Manifest {
    Manifest::new("Acme").start_url("/").icon("/icon-512.png", "512x512", "image/png")
}

fn worker() -> ServiceWorker {
    ServiceWorker::new("acme", env!("CARGO_PKG_VERSION")).precache(["/", "/icon-512.png"])
}

async fn manifest_file() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/manifest+json"), (header::CACHE_CONTROL, "no-cache")],
        manifest().to_json(),
    )
}

async fn worker_file() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8"), (header::CACHE_CONTROL, "no-cache")],
        worker().to_js(),
    )
}

let app: Router = Router::new()
    .route("/manifest.webmanifest", get(manifest_file))
    .route("/sw.js", get(worker_file));
```

Un worker contrôle les pages situées sous son propre dossier : servez `sw.js` à la racine du site (`/sw.js`), pas dans `/static/` ni dans `/pkg/`, sinon il ne traiterait que les URL de ce dossier.

## Sites statiques et chemin de base

Un [site statique](../static-site/) écrit les deux fichiers à côté de ses pages :

```rust
use std::path::Path;

use active::pwa::{Manifest, ServiceWorker};

/// "" à la racine d'un domaine, "/my-blog" pour un site de projet GitHub Pages.
const BASE: &str = "/my-blog";

fn write_pwa(dist: &Path) -> std::io::Result<()> {
    let manifest = Manifest::new("Mon blog")
        .start_url(format!("{BASE}/"))
        .scope(format!("{BASE}/"))
        .id(format!("{BASE}/"))
        .icon(format!("{BASE}/icon-512.png"), "512x512", "image/png");
    let worker = ServiceWorker::new("my-blog", "1")
        .precache([format!("{BASE}/"), format!("{BASE}/app.css")])
        .offline_page(format!("{BASE}/offline/"));
    std::fs::write(dist.join("manifest.webmanifest"), manifest.to_json())?;
    std::fs::write(dist.join("sw.js"), worker.to_js())
}
```

Toutes les URL du manifeste, du worker et des balises des pages (`.manifest(&format!("{BASE}/manifest.webmanifest"), …)`, `.service_worker(&format!("{BASE}/sw.js"))`) portent le chemin de base. Écrit à la racine de l'export, `sw.js` est servi sur `/my-blog/sw.js` : sa portée est donc `/my-blog/`. GitHub Pages sert les deux fichiers avec les bons types, en HTTPS (voir [GitHub Pages](../deploy-pages/)).

## Icônes

Les téléphones veulent au moins une icône PNG de **192×192** et une de **512×512**. Ajoutez aussi une icône **maskable** de 512×512 : Android découpe les icônes en cercles ou en carrés arrondis, alors une icône maskable remplit tout le carré avec la couleur de fond et garde le logo dans le cercle central, soit 80 % du côté. Gardez le logo SVG comme icône supplémentaire de taille `any` pour les ordinateurs. La plus grande icône PNG de type `any` devient l'icône Apple.

Dessinez les PNG une fois pour toutes à partir de votre logo SVG avec n'importe quel moteur de rendu SVG (resvg, Inkscape, ou un navigateur sans interface qui fait une capture du SVG dans une page de la taille exacte) et versionnez-les avec le site : rien ne tourne au moment du build. Les icônes de ce site sont rendues ainsi à partir de `site/static/favicon.svg`.

## Tester hors ligne

- **Dans Chrome**, ouvrez les DevTools → **Application** : *Manifest* affiche le manifeste lu, ses icônes et les erreurs d'installation ; *Service workers* l'état du worker, avec **Update on reload** pendant le développement ; *Cache storage* le cache `{name}-{version}` et les URL qu'il contient. Cochez ensuite **Offline** dans l'onglet *Network*, rechargez, puis ouvrez une page jamais visitée : la page hors ligne s'affiche.
- **En automatique**, avec un navigateur sans interface comme Playwright : chargez une page, attendez `navigator.serviceWorker.ready`, puis **arrêtez le serveur** et naviguez : les pages mises en cache d'avance et celles déjà visitées s'affichent, avec leurs îlots ; toute autre page affiche la page hors ligne. Arrêter le serveur est plus fidèle que l'émulation hors ligne du navigateur, qui peut laisser passer les requêtes du worker lui-même.
- **Dans les tests Rust**, vérifiez ce que vous servez : les URL du manifeste et la liste du worker.

```rust
use active::pwa::ServiceWorker;

let js = ServiceWorker::new("acme", "1")
    .precache(["/", "/static/app.css?v=3f2a"])
    .offline_page("/offline/")
    .to_js();
assert!(js.contains(r#"const PRECACHE = ["/","/static/app.css?v=3f2a","/offline/"];"#));
```

## Ce site

Le site que vous lisez fait tout cela, dans `site/src/server/pwa.rs` :

- `/manifest.webmanifest` ouvre l'application sur `/en/`, avec les icônes PNG 192, 512 et maskable, et le logo SVG ;
- `/sw.js` met en cache d'avance les trois pages d'accueil, la feuille de style et le bundle WebAssembly avec leurs URL en `?v=`, les polices et les icônes ; sa version est un hash de tout cela ;
- `/offline/` est la page hors ligne, en trois langues puisque le worker n'en a qu'une ;
- chaque page appelle `.manifest(...)` et `.service_worker(...)`, et l'export statique pour GitHub Pages écrit `manifest.webmanifest`, `sw.js` et `offline/index.html` avec `/Active` dans chaque URL.
