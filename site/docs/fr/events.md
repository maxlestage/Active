---
title: Événements et formulaires
description: Gérez clics, saisie, touches et envois de formulaires avec on, on_click et on_input, créez des formulaires contrôlés et accédez au DOM avec on_mount et web-sys.
---

Les gestionnaires d'événements rendent une vue interactive : un clic met à jour un état, la saisie filtre une liste, l'envoi d'un formulaire transmet des données. Les gestionnaires sont des closures Rust attachées avec `.on(...)`. Ils s'exécutent dans le navigateur une fois la vue [hydratée](../islands/) ou [montée](../client-app/) ; sur le serveur, ils sont acceptés puis ignorés, si bien que le même composant compile des deux côtés.

## Écouter des événements

```rust
use active::prelude::*;

fn like_button() -> Element {
    let likes = use_state(0);
    button()
        .attr("type", "button")
        .on_click(move |_| likes.update(|n| *n += 1))
        .text_dyn(move || format!("♥ {}", likes.get()))
}
```

| Méthode | Écoute |
| --- | --- |
| `.on(name, handler)` | n'importe quel événement DOM : `"submit"`, `"change"`, `"keydown"`, `"focus"`, `"mouseenter"`… |
| `.on_click(handler)` | `"click"` |
| `.on_input(handler)` | `"input"`, déclenché à chaque frappe dans un champ |

Un gestionnaire est n'importe quelle closure `Fn(Event)`. Comme elle est `Fn` (elle peut s'exécuter de nombreuses fois), elle modifie la page au travers d'[états](../state/), dont les handles `Copy` peuvent y être déplacés librement.

## La valeur Event

| Méthode | Renvoie |
| --- | --- |
| `e.value()` | la valeur courante de l'`<input>`, du `<textarea>` ou du `<select>` qui a déclenché l'événement (`""` sinon) |
| `e.key()` | la touche d'un événement clavier : `"Enter"`, `"Escape"`, `"ArrowDown"`, `"k"`… (`""` sinon) |
| `e.prevent_default()` | annule l'action par défaut du navigateur : suivre un lien, envoyer un formulaire… |
| `e.current_target()` | l'élément auquel l'écouteur est attaché, sous forme d'`Option<Dom>` |
| `e.raw()` | le `web_sys::Event` sous-jacent (fonctionnalité `client` uniquement) |

`Dom` est `web_sys::Element` dans le navigateur et un simple substitut vide sur le serveur, où `current_target()` renvoie `None`.

## Champs de texte

Lisez ce que l'utilisateur tape avec `on_input` et `e.value()`. Pour aussi piloter le champ depuis votre code (le vider, le remplir), liez sa valeur avec `attr_dyn("value", …)` : dans le navigateur, cela définit la valeur courante du champ, et pas seulement l'attribut.

```rust
use active::prelude::*;

fn name_field() -> Element {
    let name = use_state(String::new());
    div()
        .child(
            input()
                .attr("type", "text")
                .attr("placeholder", "Your name")
                .attr_dyn("value", move || name.get())
                .on_input(move |e| name.set(e.value())),
        )
        .child(button().attr("type", "button").on_click(move |_| name.set(String::new())).text("Clear"))
        .child(p().text_dyn(move || format!("Hello, {}!", name.get())))
}
```

`<textarea>` fonctionne de la même façon.

## Cases à cocher et listes déroulantes

Liez une case à cocher avec `bool_attr("checked", …)`, qui définit aussi son état coché dans le navigateur, et un `<select>` avec `attr_dyn("value", …)` :

```rust
use active::prelude::*;

fn preferences() -> Element {
    let newsletter = use_state(true);
    let size = use_state("m".to_string());
    form()
        .child(
            label()
                .child(
                    input()
                        .attr("type", "checkbox")
                        .bool_attr("checked", move || newsletter.get())
                        .on("change", move |_| newsletter.update(|on| *on = !*on)),
                )
                .text(" Send me the newsletter"),
        )
        .child(
            select()
                .attr_dyn("value", move || size.get())
                .on("change", move |e| size.set(e.value()))
                .children([("s", "Small"), ("m", "Medium"), ("l", "Large")].map(|(value, label)| {
                    option()
                        .attr("value", value)
                        .bool_attr("selected", move || size.get() == value)
                        .text(label)
                })),
        )
}
```

En HTML, `<select>` n'a pas d'attribut `value` : le serveur marque donc l'`<option>` choisie avec `selected` ; dans le navigateur, la liaison `value` garde ensuite le menu synchronisé.

## Soumettre un formulaire

Gérez `"submit"` sur le `<form>` et appelez `prevent_default()` pour que le navigateur ne charge pas une nouvelle page :

```rust
use active::prelude::*;

fn newsletter_form() -> Element {
    let email = use_state(String::new());
    let sent = use_state(false);
    form()
        .attr("action", "/subscribe")
        .attr("method", "post")
        .on("submit", move |e| {
            e.prevent_default();
            if email.with(|v| v.contains('@')) {
                sent.set(true);
                email.set(String::new());
            }
        })
        .child(
            input()
                .attr("type", "email")
                .attr("name", "email")
                .attr_dyn("value", move || email.get())
                .on_input(move |e| email.set(e.value())),
        )
        .child(button().text("Subscribe"))
        .child(p().bool_attr("hidden", move || !sent.get()).text("Thanks, see you soon!"))
}
```

> **Astuce :** Gardez de vrais attributs `action` et `method` sur les formulaires rendus par le serveur. Avant le chargement du bundle WebAssembly (ou s'il échoue), le formulaire fonctionne toujours comme un formulaire HTML classique ; une fois hydraté, votre gestionnaire prend le relais.

## Clavier

```rust
use active::prelude::*;

fn search_field(open: State<bool>) -> Element {
    input().attr("type", "search").on("keydown", move |e| {
        if e.key() == "Escape" {
            e.prevent_default();
            open.set(false);
        }
    })
}
```

## Accéder au DOM

### on_mount

`.on_mount(f)` exécute `f` avec l'élément une fois celui-ci présent dans la page, dans le navigateur uniquement : quand un îlot est hydraté ou quand une vue est montée. Servez-vous-en pour donner le focus à un champ, dessiner sur un canvas ou lancer une animation.

```rust
use active::prelude::*;

fn focused_input() -> Element {
    input().attr("type", "text").on_mount(|_element| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(input) = _element.dyn_ref::<web_sys::HtmlElement>() {
                let _ = input.focus();
            }
        }
    })
}
```

### web-sys

`active` couvre ce dont les composants ont généralement besoin. Pour le reste des API du navigateur, ajoutez [`web-sys`](https://docs.rs/web-sys) aux dépendances `wasm32` avec les fonctionnalités que vous utilisez, et gardez ce code derrière `#[cfg(target_arch = "wasm32")]` pour que la compilation serveur l'ignore :

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = ["HtmlElement", "Storage", "Window"] }
```

`e.raw()` donne accès au `web_sys::Event` complet, par exemple pour arrêter la propagation :

```rust
use active::prelude::*;

fn inner_button() -> Element {
    button().on_click(|_e| {
        #[cfg(target_arch = "wasm32")]
        _e.raw().stop_propagation();
    })
}
```

Le tiret bas de `_e` évite un avertissement « unused variable » dans la compilation serveur.

## Travail asynchrone dans les gestionnaires

Un gestionnaire ne peut pas être `async`, mais il peut lancer une future avec [`wasm-bindgen-futures`](https://docs.rs/wasm-bindgen-futures) et écrire le résultat dans un état quand elle se termine :

```rust
use active::prelude::*;

#[cfg(target_arch = "wasm32")]
fn load_text(url: &'static str, target: State<String>) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::{JsFuture, spawn_local};

    spawn_local(async move {
        let Some(window) = web_sys::window() else { return };
        let Ok(response) = JsFuture::from(window.fetch_with_str(url)).await else { return };
        let Ok(response) = response.dyn_into::<web_sys::Response>() else { return };
        let Ok(body) = response.text() else { return };
        if let Ok(text) = JsFuture::from(body).await {
            target.set(text.as_string().unwrap_or_default());
        }
    });
}

fn quote() -> Element {
    let text = use_state(String::from("Click to load a quote."));
    div()
        .child(p().text_dyn(move || text.get()))
        .child(button().on_click(move |_| {
            #[cfg(target_arch = "wasm32")]
            load_text("/api/quote", text);
        }).text("Load"))
}
```

Il faut pour cela `wasm-bindgen-futures = "0.4"` et les fonctionnalités web-sys `Response` et `Window`. La recherche dans la documentation de ce site fonctionne ainsi : voir `site/src/client/search.rs`.

> **Attention :** Quand la future se termine, l'état doit toujours exister. Si le composant risque d'être retiré entre-temps (il se trouve dans une région dynamique susceptible d'être reconstruite), créez l'état au-dessus de cette région.

## Liens

Les liens sont de simples éléments `a().href(...)`. Dans une [application côté client](../client-app/), le routeur démarré par `active::location()` suit les liens internes au site sans recharger la page, et `active::navigate("/path")` change de page depuis un gestionnaire.
