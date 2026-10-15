---
title: Streaming et données serveur
description: Envoyer une page en streaming avec ssr::stream pour que le visiteur la lise pendant que les parties lentes chargent, et transmettre les valeurs des ressources avec ssr::resource pour que le navigateur hydrate sans les recharger.
---

Une page attend souvent une seule requête lente (des recommandations, des commentaires, un prix venu d'un autre service) alors que tout le reste est prêt. `active::ssr` supprime cette attente deux fois :

- **Le streaming** (`ssr::stream`) : le serveur envoie tout de suite la page entière avec le fallback de chaque `suspense` encore en chargement, puis, à mesure que chaque requête se termine, le contenu de cette frontière et quelques octets de JavaScript qui le mettent en place. Le visiteur lit, fait défiler et clique pendant l'attente. Les frontières arrivent dans l'ordre où arrivent leurs données (dans le désordre), comme avec React 18, Solid et Marko.
- **Les données serveur** (`ssr::resource`) : les valeurs chargées sur le serveur voyagent avec le HTML en JSON ; quand le navigateur hydrate la page, la ressource démarre avec cette valeur au lieu de la charger une seconde fois (les payloads de Next et Nuxt).

```rust
use active::prelude::*;
use active::{ssr, suspense};
```

## Données serveur : ssr::resource

`ssr::resource(clé, source, fetch)` est une [ressource](../async-data/) avec un nom. Construisez la page dans `ssr::render`, qui rend le `Document` et écrit à la fin du body la valeur de chaque `ssr::resource` prête :

```rust
async fn load_product(id: u32) -> Result<Product, String> { /* base de données, API… */ }

fn product_page(id: u32) -> Node {
    suspense(p().text("Chargement…"), move || {
        let product = ssr::resource(format!("product:{id}"), move || id, load_product);
        div().children_dyn(move || match product.get() {
            Some(Ok(p)) => vec![h1().text(p.name).into()],
            Some(Err(e)) => vec![p().class("error").text(e).into()],
            None => vec![],
        }).into()
    })
}

let html = ssr::render(|| Document::new(product_page(7)).client("/pkg/app.js", "/pkg/app_bg.wasm"));
// …<script type="application/json" data-active-data="product:7">{"Ok":{…}}</script></body></html>
```

Dans le navigateur, le même composant tourne pendant l'hydratation : le premier chargement de `product:7` lit le JSON dans la page (et l'en retire), donc `suspense` affiche le contenu tout de suite et aucune requête ne part. Une source qui change ou `refetch` chargent normalement.

Donnez à chaque ressource d'une page sa propre clé. Une ressource dont la valeur n'est pas là quand la page est rendue n'est simplement pas envoyée : le navigateur la charge.

## Le trait Data

Les valeurs sont écrites en JSON avec le trait `active::Data`, implémenté pour les chaînes, les nombres, `bool`, `char`, `Option`, `Vec`, `Result` (`{"Ok": …}`), les tuples et les maps à clés texte, dans le même JSON que `serde_json`. Pour vos propres types, écrivez les deux méthodes, à la main ou avec `serde_json` :

```rust
impl active::Data for Product {
    fn write_data(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).unwrap_or_default());
    }
    fn read_data(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}
```

## Streaming : ssr::stream

`ssr::stream(build)` construit et rend la page tout de suite et renvoie un `Stream` de morceaux de HTML :

1. la page, avec le fallback de chaque `suspense` encore en chargement entouré de `<active-suspense>` ;
2. pour chaque frontière dont les données sont arrivées, un `<template>` avec son contenu et `<script>__activeSwap(n)</script>`, qui le met à sa place ;
3. la fin de la page.

Les futures des ressources (`resource` simple et `ssr::resource`) avancent pendant que le stream est interrogé, et une frontière part dès que tout ce qu'elle attend est là. Les frontières à l'intérieur d'une frontière envoyée sont envoyées à leur tour. Les valeurs de `ssr::resource` partent juste avant le contenu qui les utilise.

```rust
let mut stream = ssr::stream(|| Document::new(page()).client("/pkg/app.js", "/pkg/app_bg.wasm"));
while let Some(chunk) = stream.next_chunk().await {
    body.send(chunk).await?; // text/html; charset=utf-8
}
```

Un stream appartient au thread qui l'a créé (ses états vivent dans l'arène de ce thread) : faites-le avancer sur ce thread (un `tokio::task::LocalSet`, un runtime mono-thread) et transmettez les morceaux au body de la réponse par un canal. `poll_chunk` sert aux adaptateurs vers n'importe quel trait `Stream`.

Pour borner l'attente, appelez `stream.now_or_never_all()` quand votre délai est atteint : il envoie ce qui est prêt et ferme la page ; les frontières encore en chargement gardent leur fallback, et le navigateur charge leurs données en hydratant.

## Bon à savoir

- Le `<head>` part avec le premier morceau : placez le `Seo` de la page hors des frontières envoyées en streaming.
- Le bundle du navigateur démarre une fois le document entier arrivé (les scripts module l'attendent) : l'hydratation voit toujours le HTML final.
- Sans streaming, `Document::render` et `ssr::render` rendent le fallback de ce qui n'est pas prêt, comme avant.
