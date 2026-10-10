---
title: Démarrage rapide
description: Lancez en cinq minutes une page rendue côté serveur avec un îlot compteur en WebAssembly, d'abord avec l'exemple fourni, puis dans votre propre projet.
---

Cette page affiche une vraie page sur votre écran : du HTML rendu par un serveur Rust, avec des balises SEO complètes, et un compteur interactif compilé en WebAssembly. Elle suppose que les outils de la page [Installation](../installation/) sont installés (Rust, la cible `wasm32-unknown-unknown` et `wasm-bindgen-cli`).

## Lancer l'exemple

Le dépôt contient une application minimale, [`examples/hello`](https://github.com/maxlestage/Active/tree/master/examples/hello). Clonez-le, compilez le bundle du navigateur, puis démarrez le serveur :

```sh
git clone https://github.com/maxlestage/Active
cd Active
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
cargo run -p hello
```

Ouvrez [http://localhost:3000](http://localhost:3000) et cliquez sur les boutons. Le serveur de l'exemple n'utilise que la bibliothèque standard : il se compile donc rapidement.

## Créer votre propre projet

La même application, dans un nouveau crate, avec un serveur [Axum](../axum/).

### 1. Créer le crate

```sh
cargo new my-site
cd my-site
```

### 2. Déclarer les dépendances

Remplacez `Cargo.toml` par :

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
axum = "0.8"
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread"] }
tower-http = { version = "0.7", features = ["fs"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

> **Remarque :** Tant que le crate n'est pas publié sur crates.io, remplacez les deux valeurs `active = …` par `{ git = "https://github.com/maxlestage/Active" }` (en ajoutant `features = ["client"]` à la seconde).

### 3. Écrire la page

Créez `src/lib.rs`. Ce fichier est partagé par le serveur et le navigateur :

```rust
use active::prelude::*;

/// Un îlot : rendu sur le serveur, interactif dans le navigateur.
pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    div()
        .class("counter")
        .child(button().on_click(move |_| count.update(|n| *n -= 1)).text("−"))
        .child(span().text_dyn(move || count.get().to_string()))
        .child(button().on_click(move |_| count.update(|n| *n += 1)).text("+"))
        .into()
}

/// La page entière. Tout, sauf le compteur, est du HTML simple.
pub fn page() -> String {
    let body = main()
        .child(Seo::new().title("Home").description("My first page with active."))
        .child(h1().text("Hello from Rust"))
        .child(p().text("This paragraph is plain HTML. The counter below is an island:"))
        .child(island("counter", "0", counter));
    Document::new(body)
        .lang("en")
        .seo(Seo::new().site_name("My site").title_template("%s · My site"))
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}

/// Point d'entrée du navigateur : s'exécute une fois le module WebAssembly chargé.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

### 4. Écrire le serveur

Remplacez `src/main.rs` :

```rust
use axum::{Router, response::Html, routing::get};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { Html(my_site::page()) }))
        .nest_service("/pkg", ServeDir::new("pkg"));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
```

### 5. Compiler le bundle du navigateur

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

Ces commandes écrivent `pkg/my_site.js` (le chargeur) et `pkg/my_site_bg.wasm` (votre code). Relancez-les chaque fois que vous modifiez du code qui s'exécute dans le navigateur.

### 6. Lancer l'application

```sh
cargo run
```

Ouvrez [http://localhost:3000](http://localhost:3000). Le compteur fonctionne, et le code source de la page contient tout son texte.

## Ce qui vient de se passer

Affichez le code source de la page dans votre navigateur (en général `Ctrl+U` ou `⌘+⌥+U`). Raccourci et indenté pour la lecture, il ressemble à ceci :

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Home · My site</title>
  <meta name="description" content="My first page with active."/>
  <meta property="og:title" content="Home"/>
  <meta property="og:description" content="My first page with active."/>
  <meta property="og:type" content="website"/>
  <meta property="og:site_name" content="My site"/>
  <meta name="twitter:card" content="summary"/>
  <style>active-island{display:contents}</style>
  <link rel="modulepreload" href="/pkg/my_site.js"/>
  <link rel="preload" href="/pkg/my_site_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>
  <script type="module">import init from "/pkg/my_site.js";init({module_or_path:"/pkg/my_site_bg.wasm"});</script>
</head>
<body>
  <main>
    <h1>Hello from Rust</h1>
    <p>This paragraph is plain HTML. The counter below is an island:</p>
    <active-island data-island="counter" data-props="0">
      <div class="counter"><button>−</button><span><!--#-->0</span><button>+</button></div>
    </active-island>
  </main>
</body>
</html>
```

Étape par étape :

1. **Le serveur rend tout.** `Document::render()` écrit le doctype, le `<head>` et le corps dans une seule `String`. Le `Seo` placé dans le corps et celui passé à `Document::seo` sont fusionnés en un seul jeu de balises (voir [SEO](../seo/#fusion)).
2. **L'îlot est balisé.** `island("counter", "0", counter)` a rendu le HTML du compteur à l'intérieur d'un élément `<active-island>` qui enregistre son nom et ses props. Le commentaire `<!--#-->` marque le texte dynamique pour que le navigateur puisse le retrouver.
3. **Le bundle se charge en parallèle.** Les deux liens `preload` lancent le téléchargement du chargeur et du fichier `.wasm` pendant l'analyse de la page. Le script de module d'une ligne démarre le module WebAssembly.
4. **L'îlot se réveille.** `start()` appelle `active::hydrate`, qui trouve l'élément `<active-island data-island="counter">`, exécute de nouveau `counter("0")` dans le navigateur et attache les écouteurs de clic et le texte dynamique au HTML déjà présent. Rien n'est recréé.

Le titre et le paragraphe n'exécutent jamais le moindre code dans le navigateur.

## Prochaines étapes

- Construisez des vues avec les [composants](../components/), puis ajoutez de l'[état](../state/) et des [événements](../events/).
- Décrivez chaque page aux moteurs de recherche avec le [SEO](../seo/).
- Découvrez comment les [îlots](../islands/) sont hydratés, ou livrez les mêmes composants sous forme de [site statique](../static-site/).
- Mettez le site en ligne avec [Déployer sur Heroku](../deploy-heroku/).
