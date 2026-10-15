---
title: Layouts, chargement des données et préchargement
description: Partager des layouts entre pages, imbriquer des tables de routes, charger les données de chaque page avant de l'afficher, répondre le bon code HTTP ou rediriger, et précharger la page suivante avant le clic.
---

[`Routes`](../client-app/) associe des chemins à des pages et garde une page tant que seuls ses paramètres changent. Cette page couvre ce que les frameworks full-stack (Next, Nuxt, SvelteKit, Remix) ajoutent par-dessus : les **layouts**, les **tables imbriquées**, le **chargement des données par route**, les **codes HTTP et redirections**, et le **préchargement**.

```rust
use active::prelude::*;
use active::{redirect, set_status, Prefetch, Routes};
```

## Layouts

`layout` entoure chaque page d'une table, la page étant donnée comme `outlet` :

```rust
let routes = Routes::new()
    .layout(|outlet| {
        div().class("shell")
            .child(header().child(nav().child(a().href("/").text("Accueil"))))
            .child(main().child(outlet))
            .into()
    })
    .route("/", home)
    .route("/posts/:id", post);
```

Dans une application côté client (`routes.view(move || active::location().get())`), le layout est construit **une seule fois** : passer d'une page à l'autre ne remplace que l'outlet, donc les états du layout — un menu ouvert, un champ de recherche, un lecteur audio — sont conservés.

## Tables imbriquées

`nest(préfixe, routes)` confie chaque chemin sous `préfixe` à une autre table, dont les motifs sont relatifs au préfixe et dont le layout se place dans celui du parent. Les paramètres du préfixe arrivent jusqu'aux pages imbriquées :

```rust
let user = Routes::new()
    .layout(|outlet| section().child(user_tabs()).child(outlet).into())
    .route("/", user_overview)          // /users/:id
    .route("/posts/:post", user_post);  // /users/:id/posts/:post — `id` est là aussi

let routes = Routes::new().route("/", home).nest("/users/:id", user);
```

Le layout imbriqué reste à l'écran pendant que le visiteur passe d'une page de l'utilisateur à l'autre. `routes.matches(chemin)` regarde dans les tables imbriquées : un chemin inconnu de la table imbriquée est une 404.

## Charger les données d'une page

`route_with_data(motif, load, page)` charge d'abord les données puis construit la page avec, comme le `load` de SvelteKit ou les loaders de Remix. `loading` définit ce qui s'affiche entre-temps :

```rust
async fn load_post(id: u32) -> Option<Post> { /* … */ }

let routes = Routes::new()
    .loading(|| div().class("skeleton").into())
    .route_with_data(
        "/posts/:id",
        |params| load_post(params.parse("id").unwrap_or(0)),
        |post: State<Option<Post>>, params| match untrack(|| post.get()) {
            Some(p) => article().child(h1().text(p.title)).into(),
            None => { set_status(404); p().text("Article introuvable").into() }
        },
    );
```

La page est construite une fois ses données là, avec un état mis à jour sur place quand le visiteur va sur la même route avec d'autres paramètres. Les données passent par [`ssr::resource`](../streaming/) sous la clé `"route:/posts/:id"` : `ssr::render` et `ssr::stream` les envoient avec la page, et le navigateur ne les recharge pas. `T` implémente `Data`.

## Codes HTTP et redirections

Sur le serveur, `respond(chemin)` construit la page et renvoie une `PageResponse` avec le code, la redirection et le contenu :

```rust
let response = routes.respond(&path);
let html = Document::new(response.body).render();
// status : 200, 404 si aucune route ne correspond, ou ce que la page a demandé
// location : Some("/login") après redirect("/login") (code 302 sauf si set_status a donné un 3xx)
```

Dans une page, `set_status(410)` fixe le code et `redirect("/login")` envoie le visiteur ailleurs. Dans le navigateur, `redirect` fait aller le routeur à cette adresse en remplaçant l'entrée d'historique ; `set_status` ne fait rien.

## Préchargement

`Document::prefetch(Prefetch::Hover)` charge les pages que le visiteur s'apprête à ouvrir avant le clic. La page reçoit des speculation rules (Chrome, Edge) et un petit script inline pour les autres navigateurs, qui ajoute une fois un `<link rel="prefetch">` par lien du même site :

| `Prefetch` | Quand |
| --- | --- |
| `Hover` | le pointeur s'arrête sur le lien, ou il reçoit le focus |
| `Press` | le lien est pressé (souris ou doigt enfoncé) |
| `Visible` | le lien apparaît à l'écran |

`Document::prerender(quand)` va plus loin dans Chrome et Edge : la page suivante est rendue en arrière-plan et apparaît instantanément. Réservez-le aux pages sans effet de bord au chargement. Les liens avec `rel="nofollow"`, `download` ou dans `data-no-prefetch` sont ignorés.

Dans une application côté client, les liens ne chargent pas de document : `active::on_link_hover(|to| …)` (feature `client`) vous appelle une fois par adresse quand le visiteur s'apprête à suivre un lien, pour commencer à charger les données de cette page.
