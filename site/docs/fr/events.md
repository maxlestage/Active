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
| `e.stop_propagation()` | empêche l'événement d'atteindre les écouteurs des éléments parents |
| `e.checked()` | si la case à cocher ou le bouton radio qui a déclenché l'événement est coché (`false` sinon) |
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

## Liaison bidirectionnelle

Écrire `attr_dyn("value", …)` et `on_input` pour chaque champ devient vite répétitif. `bind_value(etat)` fait les deux, comme `v-model` dans Vue : le champ affiche l'état, et ce que tape l'utilisateur y est réécrit.

```rust
use active::prelude::*;

fn profile() -> Element {
    let name = use_state(String::from("Ada"));
    let bio = use_state(String::new());
    let newsletter = use_state(true);
    let age = use_state(36u32);
    form()
        .child(input().attr("type", "text").bind_value(name))
        .child(textarea().bind_value(bio))
        .child(input().attr("type", "checkbox").bind_checked(newsletter))
        .child(input().attr("type", "number").bind_parsed(age))
        .child(p().text_dyn(move || format!("{}, {}", name.get(), age.get())))
}

assert!(Node::from(profile()).render().contains(r#"<input type="number" value="36"/>"#));
```

| Méthode | Lie | Vue |
| --- | --- | --- |
| `.bind_value(State<String>)` | le texte d'un `<input>` ou d'un `<textarea>`, la valeur d'un `<select>` (écrit sur `input`, sur `change` pour un `<select>`) | `v-model` |
| `.bind_checked(State<bool>)` | une case à cocher (écrit sur `change`) | `v-model` sur une case à cocher |
| `.bind_parsed(State<T>)` | un nombre, ou tout `T: FromStr + Display + PartialEq` : écrit quand le texte se convertit ; sinon l'état garde sa dernière valeur et le texte tapé n'est pas touché | `v-model.number` |

Le serveur rend la valeur courante (`value="…"`, `checked`, le texte d'un `<textarea>`), le formulaire est donc rempli avant que le WebAssembly ne se charge. Pour un `<select>`, marquez aussi l'`<option>` choisie avec `bool_attr("selected", …)`, comme ci-dessus.

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

## Modificateurs

Les modificateurs d'événements de Vue ont des équivalents courts :

| Vue | active |
| --- | --- |
| `@submit.prevent="save"` | `.on_submit(save)` : annule le chargement de page, puis appelle `save` |
| `@keydown.enter="send"` | `.on_key("Enter", send)`, avec n'importe quel nom de touche : `"Escape"`, `"ArrowDown"`, `"k"`… |
| `@click.stop="…"` | `.on_click(move \|e\| { e.stop_propagation(); … })` |
| `@click.prevent="…"` | `.on_click(move \|e\| { e.prevent_default(); … })` |

```rust
use active::prelude::*;

fn chat() -> Element {
    let message = use_state(String::new());
    let sent = use_state(Vec::<String>::new());
    let send = move || {
        let text = message.get();
        if !text.trim().is_empty() {
            sent.update(|list| list.push(text));
            message.set(String::new());
        }
    };
    form()
        .on_submit(move |_| send())
        .child(
            textarea()
                .bind_value(message)
                .on_key("Escape", move |_| message.set(String::new())),
        )
        .child(button().text("Send"))
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

## Validation et résultats asynchrones

- Pour vérifier des champs (obligatoire, email, longueur…) et afficher une erreur par champ, voir [Formulaires et validation](../forms/).
- Pour charger des données, afficher un repli pendant le chargement et gérer les échecs, préférez `resource`, `suspense` et `error_boundary` aux futures écrits à la main : voir [Données asynchrones et erreurs](../async-data/).

## Liens

Les liens sont de simples éléments `a().href(...)`. Dans une [application côté client](../client-app/), le routeur démarré par `active::location()` suit les liens internes au site sans recharger la page, et `active::navigate("/path")` change de page depuis un gestionnaire.
