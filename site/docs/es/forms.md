---
title: Formularios y validación
description: Validar los campos de un formulario con reglas como required, min_length y email, mostrar un error por campo cuando el visitante lo abandona, y enviar solo un formulario válido.
---

`bind_value` sincroniza un campo y un estado (ver [Eventos y formularios](../events/)). El módulo `active::forms` añade lo que todo formulario real necesita después: **reglas** que comprueban cada campo, **un error por campo** mostrado en el momento adecuado, y un **formulario** que sabe si todo es válido. Es la idea de los formularios reactivos y validadores de Angular, o de `useField` de VeeValidate, con unos pocos handles `Copy`.

## Un formulario validado

```rust
use active::forms::{email, min_length, required, Form};
use active::html::form as form_; // `form` es el nombre de una variable más abajo
use active::prelude::*;

fn signup() -> Element {
    let address = use_state(String::new());
    let password = use_state(String::new());
    let form = Form::new();
    let address_field = form.field(
        address,
        [required("Escribe tu email"), email("Esto no es una dirección de email")],
    );
    let password_field = form.field(password, [min_length(8, "Al menos 8 caracteres")]);

    form_()
        .on_submit(move |_| {
            if form.submit() {
                // Válido: enviar address.get() y password.get().
            }
        })
        .child(label().text("Email").child(input().attr("type", "email").bind_field(address_field)))
        .child(p().class("error").text_dyn(move || address_field.message().unwrap_or_default()))
        .child(label().text("Contraseña").child(input().attr("type", "password").bind_field(password_field)))
        .child(p().class("error").text_dyn(move || password_field.message().unwrap_or_default()))
        .child(button().text("Registrarse"))
}
```

1. `Form::new()` crea el formulario, propiedad del componente.
2. `form.field(state, rules)` añade un campo vinculado a un `State<String>` y comprobado por las reglas, en orden: su error es el mensaje de la primera regla que falla.
3. `.bind_field(field)` en un `<input>`, `<textarea>` o `<select>` vincula el valor en ambos sentidos, marca el campo como tocado cuando el visitante lo abandona, y pone `aria-invalid="true"` mientras se muestra su error.
4. `form.submit()` en el manejador de envío muestra todos los errores y devuelve si el formulario es válido.

## Cuándo se muestran los errores

A nadie le gusta ver «Obligatorio» en rojo antes de haber escrito nada. Un campo tiene dos tipos de error:

- `field.error()`: el error actual, siempre calculado (reactivo).
- `field.message()`: el error **que hay que mostrar**: `None` mientras el visitante no haya abandonado el campo (`blur`) ni intentado enviar el formulario; después, el error actual, actualizado mientras escribe.

Da estilo a los campos no válidos con CSS, sin clases que gestionar:

```css
input[aria-invalid] { border-color: #d33; }
.error:empty { display: none; }
```

## Reglas

| Regla | Válido cuando |
| --- | --- |
| `required(message)` | El valor no está vacío (solo espacios cuenta como vacío). |
| `min_length(n, message)` | Al menos `n` caracteres, o vacío. |
| `max_length(n, message)` | Como mucho `n` caracteres. |
| `email(message)` | Parece `nombre@dominio.tld`, o está vacío. |
| `range(min, max, message)` | Un número entre `min` y `max` incluidos, o vacío. |
| `rule(test, message)` | `test(value)` devuelve `true`. |

Las reglas distintas de `required` aceptan un valor vacío: un campo opcional se comprueba igualmente si se rellena. Los mensajes son `&'static str` o `String`: tradúcelos como cualquier otro texto.

`rule` cubre todo lo demás, incluidas las comprobaciones que leen otros estados:

```rust
use active::forms::{rule, Form};
use active::prelude::*;

fn change_password() -> Element {
    let (password, confirm) = (use_state(String::new()), use_state(String::new()));
    let form = Form::new();
    let confirm_field = form.field(
        confirm,
        [rule(move |v| v == password.get(), "Las contraseñas no coinciden")],
    );
    div()
        .child(input().attr("type", "password").bind_value(password))
        .child(input().attr("type", "password").bind_field(confirm_field))
        .child(p().text_dyn(move || confirm_field.message().unwrap_or_default()))
}
```

## El formulario

| Método | Qué hace |
| --- | --- |
| `Form::new()` | Un formulario sin campos. |
| `field(state, rules) -> Field` | Añade un campo. |
| `is_valid()` | Si todos los campos son válidos (reactivo: desactiva un botón con él). |
| `submit() -> bool` | Muestra a partir de ahora todos los errores y devuelve si el formulario es válido. |
| `submitted()` | Si se llamó a `submit` desde el último reinicio. |
| `reset()` | Vuelve a ocultar los errores; vacía tú los estados para vaciar los campos. |

| Método de `Field` | Qué hace |
| --- | --- |
| `value()` | El `State<String>` del campo. |
| `error()` / `is_valid()` | El error actual, y si no hay ninguno. |
| `message()` | El error que hay que mostrar (tras `blur` o un envío). |
| `touched()` / `touch()` | Si el visitante abandonó el campo; marcarlo como abandonado. |

Un botón que solo se activa cuando el formulario es válido:

```rust
use active::forms::Form;
use active::prelude::*;

fn submit_button(form: Form) -> Element {
    button()
        .attr("type", "submit")
        .bool_attr("disabled", move || !form.is_valid())
        .text("Enviar")
}
```

## Campos de cualquier tipo

`form.field_with(estado, comprobación)` valida un estado de cualquier tipo — el `bool` de una casilla, un número, una fecha, una lista de opciones — con una función que devuelve el mensaje de error o `None`. Vincúlalo con `bind_checked_field` (casillas) o `bind_parsed_field` (números y todo lo que se lee desde texto), que añaden `aria-invalid` y el campo tocado en `blur`, como `bind_field`:

```rust
let terms = use_state(false);
let guests = use_state(2u32);
let terms_field = form.field_with(terms, |ok| (!ok).then(|| "Acepta las condiciones".to_string()));
let guests_field = form.field_with(guests, |n| (*n > 8).then(|| "8 invitados como máximo".to_string()));

input().attr("type", "checkbox").bind_checked_field(terms_field);
input().attr("type", "number").bind_parsed_field(guests_field);
```

## Comprobaciones asíncronas

Algunas comprobaciones necesitan el servidor: «este nombre de usuario está cogido», «este cupón ha caducado». `validate_async` añade una a un campo, lanzada cuando sus otras reglas se cumplen, cada vez que cambia el valor (los validadores asíncronos de Angular):

```rust
let field = form
    .field(username, [required("Elige un nombre de usuario")])
    .validate_async(|name: String| async move {
        let taken = api::username_taken(&name).await;
        taken.then(|| "Este nombre de usuario ya existe".to_string())
    });

span().class_if("spinner", move || field.is_pending());
```

Mientras se comprueba, `field.is_pending()` y `form.is_pending()` valen `true` y el formulario no es válido: `form.submit()` responde `false`. Una respuesta que llega después de otro cambio de valor se ignora. Espera un poco dentro del future antes de la petición (un debounce) para enviar menos.

## En el servidor

La validación en el navegador es una comodidad; el servidor debe volver a comprobar lo que recibe. `Rule::check(value)` aplica una regla a un simple `&str`, así que el servidor puede usar las mismas reglas:

```rust
use active::forms::{email, required, Rule};

fn check_email(value: &str) -> Option<String> {
    let rules: [Rule; 2] = [required("Escribe tu email"), email("Dirección de email no válida")];
    rules.iter().find_map(|rule| rule.check(value))
}
```

Un formulario enviado antes de que cargue el WebAssembly (o sin él) va a su `action` como siempre: da al `<form>` un `action` y un `method`, y el servidor responde con la página y sus errores. Es la mejora progresiva de las form actions de SvelteKit, sin nada especial que escribir.
