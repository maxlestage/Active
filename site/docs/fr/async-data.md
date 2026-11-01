---
title: Données asynchrones et erreurs
description: Charger des données avec resource, afficher un repli avec suspense pendant le chargement, garder les anciennes données pendant un rechargement, et remplacer une partie de la page en échec avec error_boundary.
---

Les pages attendent des choses : un profil venu d'une API, des résultats de recherche, un fichier lu dans IndexedDB. `active` donne une forme à cette attente : une **ressource** contient des données chargées de façon asynchrone, **suspense** affiche un repli tant que les données d'une vue ne sont pas là, et une **error boundary** affiche autre chose quand une partie de la page échoue. Les idées viennent de Solid (`createResource`, `<Suspense>`, `<ErrorBoundary>`) et de React (Suspense, error boundaries, transitions).

Tous les exemples de cette page commencent par :

```rust
use active::prelude::*;
use active::{error_boundary, resource, suspense, ErrorBoundary, Resource};
```

## resource

`resource(source, fetch)` appelle `fetch` avec la valeur de `source` et garde ce que donne le future renvoyé :

```rust
#[derive(Clone, PartialEq)]
struct User {
    name: String,
}

async fn load_user(id: u32) -> Result<User, String> {
    // Dans le navigateur : gloo-net, reqwest ou le fetch de web-sys.
    Ok(User { name: format!("Utilisateur {id}") })
}

fn profile(id: State<u32>) -> Element {
    let user = resource(move || id.get(), load_user);
    div().children_dyn(move || match user.get() {
        None => vec![p().text("Chargement…").into()],
        Some(Ok(user)) => vec![h2().text(user.name).into()],
        Some(Err(error)) => vec![p().class("error").text(error).into()],
    })
}
```

- `source` est suivie comme un effet : quand `id` change, `fetch` est rappelée avec le nouvel id. Si une requête plus ancienne se termine après une plus récente, son résultat est ignoré : une réponse lente n'écrase jamais une réponse récente.
- La valeur vaut `None` jusqu'à l'arrivée du premier résultat, puis le dernier résultat. Faites de `T` un `Result` pour garder les erreurs, comme ci-dessus.
- Une `Resource` est `Copy`, comme un état : déplacez-la dans autant de closures que nécessaire.

| Méthode | Ce qu'elle fait |
| --- | --- |
| `get()` / `with(f)` | La dernière valeur (`None` avant la première) ; la lire abonne la vue. |
| `loading()` | Si un chargement est en cours, le premier ou un rechargement. |
| `refetch()` | Recharge avec la source actuelle : un bouton « Actualiser » ou « Réessayer ». |
| `set(value)` | Remplace la valeur tout de suite, pour une mise à jour optimiste. |
| `state()` | Le `State<Option<T>>` qui contient la valeur, pour un `memo` ou un `watch`. |

### Dans le navigateur et sur le serveur

Dans le navigateur (fonctionnalité `client`), le future s'exécute sur la boucle d'événements du navigateur grâce à `wasm-bindgen-futures`. Le serveur n'a pas de boucle d'événements : le future y est interrogé **une fois**. Une valeur déjà disponible (un cache, `async { value }`, des données avec lesquelles la page est rendue) est rendue tout de suite ; une vraie requête reste en chargement, et le serveur rend l'état de chargement ou le repli de suspense. Le navigateur charge ensuite les données après l'hydratation.

## Requêtes partagées : use_query et use_mutation

Le meilleur de TanStack Query (React Query) et de SWR. `use_query(clé, fetch)` garde les données dans un cache partagé par clé : dix composants qui demandent `"users/7"` font une seule requête et affichent la même valeur. Les données déjà en cache s'affichent tout de suite et se rechargent en arrière-plan quand elles sont plus vieilles que leur durée de fraîcheur (`use_query_with(clé, QueryOptions::default().stale_time(..), fetch)` ; 0 par défaut : à chaque montage). `invalidate("users")` recharge chaque requête affichée dont la clé commence par `users`, `set_query_data` / `Query::set` font des mises à jour optimistes, et `use_mutation(envoi).invalidates(préfixe)` envoie un changement puis recharge ce qu'il touche. Dans un `suspense`, le repli s'affiche jusqu'aux premières données.

```rust
use active::prelude::*;

fn user_card(id: u32) -> Node {
    // Shared by every component asking for the same key: one request, one value.
    let user = use_query(move || format!("users/{id}"), |key| async move { api::get(&key).await });
    let rename = use_mutation(move |name: String| async move { api::rename(id, name).await })
        .invalidates("users"); // reloads "users", "users/7"… once saved
    div().child(p().text_dyn(move || match user.get() {
        None => "Loading…".into(),
        Some(Ok(u)) => u.name,
        Some(Err(e)) => format!("Error: {e}"),
    }))
    .child(button().on_click(move |_| rename.mutate("Ada".into())).text("Rename"))
    .into()
}
```

Sur le serveur, chaque rendu a son propre cache : les données d'un visiteur n'atteignent jamais un autre ; `provide(QueryClient::new(), || page())` en partage un entre les composants d'une page. Écrivez les clés comme des chemins, pour qu'un préfixe désigne une famille.

## Garder les anciennes données pendant un rechargement

Pendant un rechargement, la valeur précédente reste lisible : la page continue de l'afficher au lieu de revenir à « Chargement… ». Affichez un indicateur discret avec `loading()`, comme le fait `useTransition` de React :

```rust
fn results(query: State<String>) -> Element {
    let found = resource(move || query.get(), |q| async move { vec![format!("Résultat pour {q}")] });
    div()
        .class_if("dimmed", move || found.loading())
        .children_dyn(move || {
            found
                .get()
                .unwrap_or_default()
                .into_iter()
                .map(|r| p().text(r).into())
                .collect()
        })
}
```

## suspense

`suspense(fallback, view)` affiche `fallback` jusqu'à ce que chaque ressource créée par `view`, à n'importe quelle profondeur, ait sa première valeur, puis affiche la vue :

```rust
fn dashboard() -> Node {
    suspense(p().class("skeleton").text("Chargement du tableau de bord…"), || {
        let stats = resource(|| (), |()| async { 42 });
        let news = resource(|| (), |()| async { String::from("Version 1.0") });
        div()
            .child(p().text_dyn(move || format!("{} visites", stats.get().unwrap_or(0))))
            .child(p().text_dyn(move || news.get().unwrap_or_default()))
            .into()
    })
}
```

- `view` est construite une fois, tout de suite, pour que ses ressources se chargent en parallèle.
- Seuls les premiers chargements comptent : un rechargement ultérieur garde la vue à l'écran avec ses données précédentes (utilisez `loading()` là).
- Les suspense s'imbriquent : une ressource compte dans la plus proche.
- Sur le serveur, une ressource qui n'est pas prête tout de suite laisse le repli dans le HTML.

## error_boundary

`error_boundary(view, fallback)` construit `view`, et la remplace par `fallback` dès qu'une erreur est signalée à l'intérieur :

```rust
fn price(raw: &str) -> Result<Node, std::num::ParseFloatError> {
    let value: f64 = raw.parse()?;
    Ok(span().text(format!("{value:.2} €")).into())
}

fn product() -> Node {
    error_boundary(
        || div().child(price("neuf")).into(),
        |boundary| {
            div()
                .class("error")
                .child(p().text(format!("Une erreur est survenue : {}", boundary.errors().join(", "))))
                .child(button().text("Réessayer").on_click(move |_| boundary.reset()))
                .into()
        },
    )
}
```

Les erreurs atteignent la boundary la plus proche de trois façons :

- **Un `Result` dans la vue.** `Result<T, E>` se convertit en `Node` quand `T` le fait et que `E: Display` : un `Ok` affiche son nœud, un `Err` est signalé et n'affiche rien. Un composant peut simplement renvoyer `Result<Node, MonErreur>` et utiliser `?`.
- **`report_error(error)`** pendant la construction de la vue.
- **`ErrorBoundary::report(error)`** plus tard, depuis un gestionnaire d'événement ou la fin d'une requête : récupérez la boundary avec `use_context::<ErrorBoundary>()` pendant la construction, puis déplacez-la dans la closure.

```rust
fn save_button() -> Element {
    let boundary = use_context::<ErrorBoundary>();
    button().text("Enregistrer").on_click(move |_| {
        if let Some(boundary) = boundary {
            boundary.report("Le serveur n'a pas répondu");
        }
    })
}
```

Le repli reçoit l'`ErrorBoundary` : `errors()` liste les messages, `has_errors()` dit s'il y en a, et `reset()` les oublie et reconstruit la vue. Tout ce que la vue en échec avait créé (états, effets, écouteurs) est libéré quand le repli la remplace.

Sans boundary, `report_error` écrit l'erreur dans la console du navigateur (ou sur la sortie d'erreur du serveur) et le `Err` n'affiche rien.

> Les composants Rust ne lèvent pas d'exceptions. Une **panique** n'est pas rattrapée : dans le navigateur, elle arrête le module WebAssembly. Renvoyez des `Result` pour les erreurs qui peuvent arriver.

## Grouper des écritures : batch

Quand un résultat modifie plusieurs états, entourez les écritures de `active::batch` pour que les vues qui les lisent ne se mettent à jour qu'une fois (voir [État et effets](../state/)) :

```rust
fn load_more(items: State<Vec<String>>, page: State<u32>, more: Vec<String>) {
    active::batch(|| {
        items.update(|list| list.extend(more));
        page.update(|p| *p += 1);
    });
}
```
