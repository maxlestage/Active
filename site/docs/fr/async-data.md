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
