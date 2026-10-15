---
title: Formulaires et validation
description: Valider les champs d'un formulaire avec des règles comme required, min_length et email, afficher une erreur par champ une fois que le visiteur l'a quitté, et n'envoyer qu'un formulaire valide.
---

`bind_value` synchronise un champ et un état (voir [Événements et formulaires](../events/)). Le module `active::forms` ajoute ce dont tout vrai formulaire a besoin ensuite : des **règles** qui vérifient chaque champ, **une erreur par champ** affichée au bon moment, et un **formulaire** qui sait si tout est valide. C'est l'idée des formulaires réactifs et des validateurs d'Angular, ou de `useField` de VeeValidate, avec quelques handles `Copy`.

## Un formulaire validé

```rust
use active::forms::{email, min_length, required, Form};
use active::html::form as form_; // `form` est le nom d'une variable plus bas
use active::prelude::*;

fn signup() -> Element {
    let address = use_state(String::new());
    let password = use_state(String::new());
    let form = Form::new();
    let address_field = form.field(
        address,
        [required("Saisissez votre email"), email("Ce n'est pas une adresse email")],
    );
    let password_field = form.field(password, [min_length(8, "Au moins 8 caractères")]);

    form_()
        .on_submit(move |_| {
            if form.submit() {
                // Valide : envoyer address.get() et password.get().
            }
        })
        .child(label().text("Email").child(input().attr("type", "email").bind_field(address_field)))
        .child(p().class("error").text_dyn(move || address_field.message().unwrap_or_default()))
        .child(label().text("Mot de passe").child(input().attr("type", "password").bind_field(password_field)))
        .child(p().class("error").text_dyn(move || password_field.message().unwrap_or_default()))
        .child(button().text("S'inscrire"))
}
```

1. `Form::new()` crée le formulaire, possédé par le composant.
2. `form.field(state, rules)` ajoute un champ lié à un `State<String>` et vérifié par les règles, dans l'ordre : son erreur est le message de la première règle qui échoue.
3. `.bind_field(field)` sur un `<input>`, un `<textarea>` ou un `<select>` lie la valeur dans les deux sens, marque le champ comme touché quand le visiteur le quitte, et pose `aria-invalid="true"` tant que son erreur est affichée.
4. `form.submit()` dans le gestionnaire de soumission affiche toutes les erreurs et renvoie si le formulaire est valide.

## Quand les erreurs s'affichent

Personne n'aime voir « Obligatoire » en rouge avant d'avoir tapé quoi que ce soit. Un champ a deux sortes d'erreur :

- `field.error()` : l'erreur actuelle, toujours calculée (réactive).
- `field.message()` : l'erreur **à afficher** : `None` tant que le visiteur n'a pas quitté le champ (`blur`) ni tenté d'envoyer le formulaire, puis l'erreur actuelle, mise à jour pendant la frappe.

Stylez les champs invalides en CSS, sans classe à gérer :

```css
input[aria-invalid] { border-color: #d33; }
.error:empty { display: none; }
```

## Règles

| Règle | Valide quand |
| --- | --- |
| `required(message)` | La valeur n'est pas vide (des espaces seuls comptent comme vide). |
| `min_length(n, message)` | Au moins `n` caractères, ou vide. |
| `max_length(n, message)` | Au plus `n` caractères. |
| `email(message)` | Elle ressemble à `nom@domaine.tld`, ou est vide. |
| `range(min, max, message)` | Un nombre entre `min` et `max` inclus, ou vide. |
| `rule(test, message)` | `test(value)` renvoie `true`. |

Les règles autres que `required` acceptent une valeur vide : un champ facultatif peut quand même être vérifié s'il est rempli. Les messages sont des `&'static str` ou des `String` : traduisez-les comme n'importe quel texte.

`rule` couvre tout le reste, y compris les vérifications qui lisent d'autres états :

```rust
use active::forms::{rule, Form};
use active::prelude::*;

fn change_password() -> Element {
    let (password, confirm) = (use_state(String::new()), use_state(String::new()));
    let form = Form::new();
    let confirm_field = form.field(
        confirm,
        [rule(move |v| v == password.get(), "Les mots de passe diffèrent")],
    );
    div()
        .child(input().attr("type", "password").bind_value(password))
        .child(input().attr("type", "password").bind_field(confirm_field))
        .child(p().text_dyn(move || confirm_field.message().unwrap_or_default()))
}
```

## Le formulaire

| Méthode | Ce qu'elle fait |
| --- | --- |
| `Form::new()` | Un formulaire sans champ. |
| `field(state, rules) -> Field` | Ajoute un champ. |
| `is_valid()` | Si tous les champs sont valides (réactif : désactivez un bouton avec). |
| `submit() -> bool` | Affiche désormais toutes les erreurs et renvoie si le formulaire est valide. |
| `submitted()` | Si `submit` a été appelé depuis la dernière réinitialisation. |
| `reset()` | Cache à nouveau les erreurs ; videz vous-même les états pour vider les champs. |

| Méthode de `Field` | Ce qu'elle fait |
| --- | --- |
| `value()` | Le `State<String>` du champ. |
| `error()` / `is_valid()` | L'erreur actuelle, et s'il n'y en a aucune. |
| `message()` | L'erreur à afficher (après `blur` ou une soumission). |
| `touched()` / `touch()` | Si le visiteur a quitté le champ ; le marquer comme quitté. |

Un bouton qui ne s'active que lorsque le formulaire est valide :

```rust
use active::forms::Form;
use active::prelude::*;

fn submit_button(form: Form) -> Element {
    button()
        .attr("type", "submit")
        .bool_attr("disabled", move || !form.is_valid())
        .text("Envoyer")
}
```

## Des champs de tout type

`form.field_with(état, vérification)` valide un état de n'importe quel type — le `bool` d'une case à cocher, un nombre, une date, une liste de choix — avec une fonction qui renvoie le message d'erreur ou `None`. Liez-le avec `bind_checked_field` (cases à cocher) ou `bind_parsed_field` (nombres et tout ce qui se lit depuis du texte), qui ajoutent `aria-invalid` et le champ touché au `blur`, comme `bind_field` :

```rust
let terms = use_state(false);
let guests = use_state(2u32);
let terms_field = form.field_with(terms, |ok| (!ok).then(|| "Acceptez les conditions".to_string()));
let guests_field = form.field_with(guests, |n| (*n > 8).then(|| "8 invités au plus".to_string()));

input().attr("type", "checkbox").bind_checked_field(terms_field);
input().attr("type", "number").bind_parsed_field(guests_field);
```

## Vérifications asynchrones

Certaines vérifications demandent le serveur : « ce nom d'utilisateur est pris », « ce code promo a expiré ». `validate_async` en ajoute une à un champ, lancée une fois ses autres règles respectées, à chaque changement de valeur (les validateurs asynchrones d'Angular) :

```rust
let field = form
    .field(username, [required("Choisissez un nom d'utilisateur")])
    .validate_async(|name: String| async move {
        let taken = api::username_taken(&name).await;
        taken.then(|| "Ce nom d'utilisateur est pris".to_string())
    });

span().class_if("spinner", move || field.is_pending());
```

Pendant la vérification, `field.is_pending()` et `form.is_pending()` valent `true` et le formulaire n'est pas valide : `form.submit()` répond `false`. Une réponse arrivée après un nouveau changement de valeur est ignorée. Attendez un peu dans le future avant la requête (un debounce) pour en envoyer moins.

## Côté serveur

La validation dans le navigateur est un confort ; le serveur doit vérifier à nouveau ce qu'il reçoit. `Rule::check(value)` applique une règle à un simple `&str`, le serveur peut donc réutiliser les mêmes règles :

```rust
use active::forms::{email, required, Rule};

fn check_email(value: &str) -> Option<String> {
    let rules: [Rule; 2] = [required("Saisissez votre email"), email("Adresse email invalide")];
    rules.iter().find_map(|rule| rule.check(value))
}
```

Un formulaire envoyé avant le chargement du WebAssembly (ou sans lui) part vers son `action` comme d'habitude : donnez au `<form>` un `action` et une `method`, et le serveur répond avec la page et ses erreurs. C'est l'amélioration progressive des form actions de SvelteKit, sans rien de spécial à écrire.
