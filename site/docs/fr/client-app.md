---
title: Applications côté client
description: Créez une application monopage exécutée dans le navigateur : montage avec mount_to, routage avec location et navigate, titres et balises meta tenus à jour.
---

Une application côté client (souvent appelée application monopage) est rendue entièrement dans le navigateur. Le serveur, ou n'importe quel hébergeur statique, envoie une petite page HTML ; le bundle WebAssembly construit ensuite toute l'interface, change de page sans rechargement et met à jour le titre et les balises meta au fil de la navigation du visiteur.

C'est le bon choix pour les outils, les tableaux de bord et tout ce qui se trouve derrière une connexion, là où les moteurs de recherche ne comptent pas et où l'interface est interactive partout. Pour du contenu public, préférez des [pages rendues côté serveur avec des îlots](../islands/) ou un [site statique](../static-site/) : ils envoient un HTML complet que tous les robots d'indexation peuvent lire. Les trois approches utilisent les mêmes composants : vous pouvez donc les combiner.

## Ce que vous allez construire

Le dépôt contient un exemple complet, `examples/spa` : une liste de tâches avec deux pages, un formulaire, des filtres conservés dans l'URL (`/?show=done`), une liste enregistrée dans `localStorage` et un titre de page par route. Lancez-le depuis la racine du dépôt :

```sh
cargo build -p spa --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/spa/pkg target/wasm32-unknown-unknown/wasm-release/spa.wasm
cargo run -p spa
```

Les sections suivantes expliquent chaque partie. L'organisation du crate est la même que dans [Installation](../installation/#un-crate-deux-cibles).

## La coquille

La page envoyée par le serveur est un [`Document`](../documents/) ordinaire dont le corps est un conteneur vide, accompagné du bundle :

```rust
use active::prelude::*;

/// SEO partagé par toutes les pages.
pub fn site_seo() -> Seo {
    Seo::new()
        .site_name("Todos")
        .title_template("%s · Todos")
        .description("A to-do list written in Rust, running as WebAssembly.")
}

/// La même page pour toutes les URL : l'application remplit #app dans le navigateur.
pub fn shell() -> String {
    Document::new(div().id("app").child(p().text("Loading…")))
        .lang("en")
        .seo(site_seo())
        .stylesheet("/app.css")
        .client("/pkg/spa.js", "/pkg/spa_bg.wasm")
        .render()
}
```

Le texte « Loading… » s'affiche jusqu'au démarrage du bundle. Comme la coquille porte déjà le titre et la description du site, les aperçus de liens et les robots d'indexation qui n'exécutent pas le WebAssembly voient tout de même quelque chose de pertinent.

## Monter l'application

Le point d'entrée du navigateur monte la vue racine dans le conteneur :

```rust
use active::prelude::*;

fn app() -> Node {
    h1().text("Hello from the browser").into()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app());
}
```

`mount_to(selector, view)` trouve le premier élément correspondant au sélecteur CSS, supprime son contenu et construit la vue à l'intérieur. Si aucun élément ne correspond, un avertissement s'affiche dans la console.

Pour ajouter une vue à une page sans rien remplacer (un widget dans une page existante), utilisez `active::mount(&node, &element)`, qui ajoute la vue après les enfants actuels de l'élément.

## Routage

`active::location()` renvoie un `State<String>` qui contient le chemin courant (`"/"`, `"/about"`). Lisez-le dans une [région dynamique](../state/#listes-et-conditions-qui-changent) pour afficher la page correspondante :

```rust
use active::prelude::*;

fn home() -> Node {
    section().child(Seo::new().title("Home")).child(h1().text("Home")).into()
}

fn about() -> Node {
    section()
        .child(Seo::new().title("About").description("Who we are."))
        .child(h1().text("About"))
        .into()
}

fn not_found() -> Node {
    section().child(Seo::new().title("Not found").noindex(true)).child(h1().text("Not found")).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let link = move |href: &'static str, label: &'static str| {
        a().href(href).class_if("current", move || path.get() == href).text(label)
    };
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .child(nav().child(link("/", "Home")).child(link("/about", "About")))
        .child(el("main").children_dyn(move || match path.get().as_str() {
            "/" => vec![home()],
            "/about" => vec![about()],
            _ => vec![not_found()],
        }))
        .into()
}
```

Le premier appel à `location()`, `query()` ou `query_param()` démarre le routeur :

- **Les liens fonctionnent tout seuls.** Les clics sur des éléments `<a href>` qui pointent vers le même site changent le chemin sans recharger la page. Les liens avec un `target` (autre que `_self`), un attribut `download`, `rel="external"` (une page que le serveur rend lui-même, un fichier), un clic avec une touche de modification (Ctrl, ⌘, Maj, Alt) ou avec le bouton du milieu, les liens vers d'autres sites et les liens vers une ancre de la page courante (`#section`) sont laissés au navigateur.
- **Précédent et Suivant fonctionnent.** Les boutons d'historique du navigateur mettent à jour le chemin et la chaîne de requête.
- **Seule la région est reconstruite.** Quand le chemin change, la région qui le lit est reconstruite ; la barre de navigation et tout le reste restent en place.

`location()`, `query()`, `query_param()` et `navigate()` n'existent qu'avec la fonctionnalité `client` : le code qui les appelle est donc compilé uniquement pour le navigateur (ici derrière `#[cfg(target_arch = "wasm32")]`). Les composants de page, eux, compilent partout.

### Changer de page depuis le code

`active::navigate(to)` mène à une autre page depuis un gestionnaire, par exemple après l'enregistrement d'un formulaire :

```rust
use active::prelude::*;

fn save_button() -> Element {
    button().text("Save").on_click(|_| {
        // … enregistrer, puis :
        #[cfg(target_arch = "wasm32")]
        active::navigate("/saved");
    })
}
```

La fonction ajoute une entrée à l'historique du navigateur, met à jour `location()` et `query()`, puis fait défiler la page jusqu'en haut, ou jusqu'à l'élément dont l'`id` correspond à un `#fragment` (`navigate("/docs#install")`).

### Chemins avec paramètres

Faites un `match` sur les segments du chemin pour en extraire des paramètres :

```rust
use active::prelude::*;

fn post(slug: &str) -> Node {
    article().child(Seo::new().title(slug.to_string())).child(h1().text(slug.to_string())).into()
}

fn route(path: &str) -> Vec<Node> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => vec![h1().text("Blog").into()],
        ["posts", slug] => vec![post(slug)],
        _ => vec![h1().text("Not found").into()],
    }
}

assert!(Node::from(div().children(route("/posts/hello-rust"))).render().contains("<h1>hello-rust</h1>"));
```

Dans l'application : `.children_dyn(move || route(&path.get()))`.

### Garder une page quand ses paramètres changent

Avec `children_dyn`, passer de `/posts/1` à `/posts/2` reconstruit la page de l'article de zéro : ses états repartent de leur valeur initiale et tout ce qu'elle avait chargé est rechargé. `switch(value, kind, view)` ne reconstruit la vue que quand le **genre** de la valeur change, et sinon donne la nouvelle valeur à la page déjà affichée, par le `State` avec lequel elle a été construite :

```rust
use active::prelude::*;

#[derive(Clone, PartialEq)]
enum Route {
    Blog,
    Post(String),
    NotFound,
}

fn parse(path: &str) -> Route {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => Route::Blog,
        ["posts", slug] => Route::Post(slug.to_string()),
        _ => Route::NotFound,
    }
}

/// Gardée d'un article à l'autre : seul ce qui lit `slug` suit.
fn post(route: State<Route>) -> Node {
    let slug = move || route.with(|r| match r {
        Route::Post(slug) => slug.clone(),
        _ => String::new(),
    });
    article().child(h1().text_dyn(slug)).into()
}

fn pages(path: State<String>) -> Node {
    switch(
        move || parse(&path.get()),
        |route| std::mem::discriminant(route),
        |route| match untrack(|| route.get()) {
            Route::Blog => h1().text("Blog").into(),
            Route::Post(_) => post(route),
            Route::NotFound => h1().text("Not found").into(),
        },
    )
}

let path = use_state("/posts/hello-rust".to_string());
assert!(pages(path).render().contains("hello-rust</h1>"));
```

Dans l'application, passez `active::location()` comme `path`. L'état donné à une page ne contient jamais que des routes de ce genre de page : la page lit ses paramètres sans avoir à gérer les autres routes. Dans la page, suivez les paramètres comme n'importe quel état : `use_state`/`effect` pour les données à recharger, `text_dyn` pour ce qui les affiche.

### Chaînes de requête

`active::query()` renvoie la chaîne de requête sans le `?` (`"q=rust&page=2"`) sous forme d'état, et `active::query_param(name)` renvoie un paramètre, décodé, ou `None`. Les deux suivent le routeur : les liens, `navigate` et les boutons d'historique les mettent à jour. Passer de `/search?q=a` à `/search?q=b` reconstruit les régions qui lisent la requête, tandis que celles qui ne lisent que le chemin restent telles quelles.

Gardez dans l'URL ce qui doit survivre à un rechargement ou être partagé dans un lien : un filtre, une recherche, un numéro de page. `examples/spa` y conserve son filtre :

```rust
use active::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

/// Le filtre fait partie de l'URL : `/`, `/?show=active`, `/?show=done`.
#[cfg(target_arch = "wasm32")]
fn filter() -> Filter {
    match active::query_param("show").as_deref() {
        Some("active") => Filter::Active,
        Some("done") => Filter::Done,
        _ => Filter::All,
    }
}

#[cfg(target_arch = "wasm32")]
fn filter_links() -> Element {
    let link = |value: Filter, href: &'static str, label: &'static str| {
        a().href(href).class_if("on", move || filter() == value).text(label)
    };
    nav()
        .child(link(Filter::All, "/", "All"))
        .child(link(Filter::Active, "/?show=active", "Active"))
        .child(link(Filter::Done, "/?show=done", "Done"))
}
```

La liste lit `filter()` dans son `children_dyn` : cliquer sur un lien ne reconstruit donc que la liste. Le fragment de l'URL (`#…`) ne fait partie d'aucun des deux états : le navigateur fait défiler jusqu'aux ancres de lui-même.

## Titres et balises meta

Chaque page place un `Seo` dans sa vue, exactement comme sur le serveur. Dans une application montée, `active` garde le `<head>` synchronisé :

- au montage de l'application, les valeurs `Seo` de la vue sont fusionnées et écrites dans le `<head>` ;
- chaque fois qu'une région dynamique est reconstruite (un changement de page), ses valeurs `Seo` sont fusionnées **par-dessus celles de ses ancêtres** et le `<head>` est mis à jour : le titre, la description, robots, l'URL canonique, les alternatives, Open Graph, Twitter, le JSON-LD et les balises meta supplémentaires ;
- quand le nouveau contenu n'a pas de `Seo`, le `<head>` revient aux valeurs des ancêtres ;
- quand les valeurs fusionnées n'ont ni titre ni nom de site, le titre de la page est vidé plutôt que de conserver celui de la page précédente.

Placez les valeurs par défaut du site (nom du site, modèle de titre, description par défaut) dans un `Seo` de la **vue racine**, comme le fait l'exemple de routeur ci-dessus, afin que le titre de chaque page reçoive le modèle. Le `Seo` passé à `Document::seo` dans la coquille n'est utilisé que par le serveur.

Les autres éléments du `<head>` (feuilles de style, charset, vos propres balises `<meta>` que `Seo` ne gère pas) ne sont pas touchés.

Pour modifier le `<head>` depuis un gestionnaire, appelez `active::update_head(&seo)` avec un `Seo` complet ; la fonction remplace les balises des types énumérés ci-dessus.

## Conserver des données d'une page à l'autre

Les états créés dans une page sont libérés quand le routeur quitte cette page : une page affichée de nouveau repart de zéro. Les données qui doivent survivre aux changements de page vont dans un état créé **au-dessus** du routeur, puis transmis aux pages :

```rust
use active::prelude::*;

#[derive(Clone)]
struct Todo {
    text: String,
    done: bool,
}

fn todo_page(todos: State<Vec<Todo>>) -> Node {
    let left = move || todos.with(|list| list.iter().filter(|t| !t.done).count());
    section().child(p().text_dyn(move || format!("{} left", left()))).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let todos = use_state(Vec::<Todo>::new()); // survives page changes
    div()
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![todo_page(todos)],
            _ => vec![h1().text("Not found").into()],
        })
        .into()
}
```

Pour conserver des données d'une visite à l'autre, enregistrez-les dans `localStorage` depuis un [effet](../state/#effets), comme le fait `examples/spa`.

## Servir l'application

Chaque URL de l'application doit renvoyer la coquille, pour que recharger `/about` ou ouvrir un lien partagé fonctionne. Les requêtes vers le bundle (`/pkg/…`) renvoient les fichiers.

Avec Axum, servez `/pkg` sous forme de fichiers et répondez avec la coquille pour tout le reste :

```rust
use axum::{Router, response::Html, routing::get};
use tower_http::services::ServeDir;

fn shell() -> String {
    String::from("<!DOCTYPE html>…") // the shell() function above
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/pkg", ServeDir::new("pkg"))
        .fallback(get(|| async { Html(shell()) }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`examples/spa` fait la même chose avec un serveur écrit uniquement avec la bibliothèque standard. Sur un hébergeur statique, écrivez la coquille dans `index.html` (voir [Sites statiques](../static-site/)) et configurez une redirection de repli :

| Hébergeur | Repli vers `index.html` |
| --- | --- |
| Netlify | un fichier `_redirects` contenant `/* /index.html 200` |
| Cloudflare Pages | automatique quand le site n'a pas de `404.html` |
| GitHub Pages | pas de réécriture : copiez `index.html` vers `404.html` (les pages sont alors servies avec un statut 404) |

## Rendu de la première page côté serveur

Les pages d'un routeur peuvent aussi être rendues sur le serveur : l'application reçoit le chemin sous forme de `State`, créé à partir de la requête sur le serveur et à partir de `location()` dans le navigateur.

```rust
use active::prelude::*;

pub fn app(path: State<String>) -> Node {
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![section().child(Seo::new().title("Home")).text("Welcome").into()],
            _ => vec![section().child(Seo::new().title("Not found").noindex(true)).into()],
        })
        .into()
}

/// Serveur : le HTML complet de `path`, avec son titre et ses balises meta.
pub fn page(path: &str) -> String {
    Document::new(div().id("app").child(app(use_state(path.to_string()))))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app(active::location()));
}

assert!(page("/").contains("<title>Home · Acme</title>"));
```

Sur le serveur, `Document` trouve le `Seo` de la page affichée par la région dynamique : chaque URL obtient donc son propre titre et sa propre description. Dans le navigateur, `mount_to` remplace le HTML du serveur par l'application vivante. Contrairement aux [îlots](../islands/), le contenu est reconstruit plutôt qu'hydraté, ce qui est simple et convient à la plupart des applications.

## Taille

L'application `examples/spa` complète (routage, formulaires, filtres, stockage) compile en environ 120 ko de WebAssembly, 47 ko avec gzip, avec le profil `wasm-release`. Voir [Performances](../performance/) pour savoir comment la mesurer et la réduire.
