---
title: Forms and validation
description: Validate form fields with rules such as required, min_length and email, show one error per field once the visitor has left it, and submit only a valid form.
---

`bind_value` keeps a field and a state in sync (see [Events and forms](../events/)). The `active::forms` module adds what every real form needs next: **rules** that check each field, **one error per field** shown at the right moment, and a **form** that knows whether everything is valid. It is the idea of Angular's reactive forms and validators, or VeeValidate's `useField`, with a few `Copy` handles.

## A validated form

```rust
use active::forms::{email, min_length, required, Form};
use active::html::form as form_; // `form` is the name of a variable below
use active::prelude::*;

fn signup() -> Element {
    let address = use_state(String::new());
    let password = use_state(String::new());
    let form = Form::new();
    let address_field = form.field(
        address,
        [required("Enter your email"), email("This is not an email address")],
    );
    let password_field = form.field(password, [min_length(8, "At least 8 characters")]);

    form_()
        .on_submit(move |_| {
            if form.submit() {
                // Valid: send address.get() and password.get().
            }
        })
        .child(label().text("Email").child(input().attr("type", "email").bind_field(address_field)))
        .child(p().class("error").text_dyn(move || address_field.message().unwrap_or_default()))
        .child(label().text("Password").child(input().attr("type", "password").bind_field(password_field)))
        .child(p().class("error").text_dyn(move || password_field.message().unwrap_or_default()))
        .child(button().text("Sign up"))
}
```

1. `Form::new()` creates the form, owned by the component.
2. `form.field(state, rules)` adds a field bound to a `State<String>` and checked by the rules, in order: its error is the message of the first rule that fails.
3. `.bind_field(field)` on an `<input>`, `<textarea>` or `<select>` binds the value both ways, marks the field as touched when the visitor leaves it, and sets `aria-invalid="true"` while its error is shown.
4. `form.submit()` in the submit handler shows every error and returns whether the form is valid.

## When errors show

Nobody likes "Required" in red before having typed anything. A field has two kinds of error:

- `field.error()`: the current error, always computed (reactive).
- `field.message()`: the error **to show**: `None` until the visitor has left the field (`blur`) or tried to submit the form, then the current error, updated as they type.

Style the invalid fields with CSS, without any class to manage:

```css
input[aria-invalid] { border-color: #d33; }
.error:empty { display: none; }
```

## Rules

| Rule | Valid when |
| --- | --- |
| `required(message)` | The value is not empty (spaces alone count as empty). |
| `min_length(n, message)` | At least `n` characters, or empty. |
| `max_length(n, message)` | At most `n` characters. |
| `email(message)` | It looks like `name@domain.tld`, or is empty. |
| `range(min, max, message)` | A number between `min` and `max` included, or empty. |
| `rule(test, message)` | `test(value)` returns `true`. |

Rules other than `required` accept an empty value, so an optional field can still be checked when filled. Messages are `&'static str` or `String`: translate them like any other text.

`rule` covers everything else, including checks that read other states:

```rust
use active::forms::{rule, Form};
use active::prelude::*;

fn change_password() -> Element {
    let (password, confirm) = (use_state(String::new()), use_state(String::new()));
    let form = Form::new();
    let confirm_field = form.field(
        confirm,
        [rule(move |v| v == password.get(), "The passwords differ")],
    );
    div()
        .child(input().attr("type", "password").bind_value(password))
        .child(input().attr("type", "password").bind_field(confirm_field))
        .child(p().text_dyn(move || confirm_field.message().unwrap_or_default()))
}
```

## The form

| Method | What it does |
| --- | --- |
| `Form::new()` | A form without fields. |
| `field(state, rules) -> Field` | Adds a field. |
| `is_valid()` | Whether every field is valid (reactive: disable a button with it). |
| `submit() -> bool` | Shows every error from now on and returns whether the form is valid. |
| `submitted()` | Whether `submit` was called since the last reset. |
| `reset()` | Hides the errors again; clear the states yourself to empty the fields. |

| `Field` method | What it does |
| --- | --- |
| `value()` | The `State<String>` of the field. |
| `error()` / `is_valid()` | The current error, and whether there is none. |
| `message()` | The error to show (after `blur` or a submit). |
| `touched()` / `touch()` | Whether the visitor has left the field; mark it as left. |

A disabled button that only enables itself when the form is valid:

```rust
use active::forms::Form;
use active::prelude::*;

fn submit_button(form: Form) -> Element {
    button()
        .attr("type", "submit")
        .bool_attr("disabled", move || !form.is_valid())
        .text("Send")
}
```

## Fields of any type

`form.field_with(state, check)` validates a state of any type — a checkbox's `bool`, a number, a date, a list of choices — with a function that returns the error message or `None`. Bind it with `bind_checked_field` (checkboxes) or `bind_parsed_field` (numbers and anything that parses from text), which add `aria-invalid` and the touch on `blur` like `bind_field`:

```rust
let terms = use_state(false);
let guests = use_state(2u32);
let terms_field = form.field_with(terms, |ok| (!ok).then(|| "Accept the terms".to_string()));
let guests_field = form.field_with(guests, |n| (*n > 8).then(|| "8 guests at most".to_string()));

input().attr("type", "checkbox").bind_checked_field(terms_field);
input().attr("type", "number").bind_parsed_field(guests_field);
```

## Asynchronous checks

Some checks need the server: "this username is taken", "this coupon has expired". `validate_async` adds one to a field, run once its other rules pass, each time the value changes (Angular's async validators):

```rust
let field = form
    .field(username, [required("Choose a username")])
    .validate_async(|name: String| async move {
        let taken = api::username_taken(&name).await;
        taken.then(|| "This username is taken".to_string())
    });

span().class_if("spinner", move || field.is_pending());
```

While the check runs, `field.is_pending()` and `form.is_pending()` are `true` and the form is not valid, so `form.submit()` answers `false`. An answer that arrives after the value changed again is ignored. Wait a little inside the future before the request (a debounce) to send fewer of them.

## On the server

Validation in the browser is for comfort; the server must check again what it receives. `Rule::check(value)` runs a rule on a plain `&str`, so the server can use the same rules:

```rust
use active::forms::{email, required, Rule};

fn check_email(value: &str) -> Option<String> {
    let rules: [Rule; 2] = [required("Enter your email"), email("Not an email address")];
    rules.iter().find_map(|rule| rule.check(value))
}
```

A form submitted before the WebAssembly has loaded (or with it disabled) is sent to its `action` as usual: give the `<form>` an `action` and a `method`, and the server answers with the page and its errors. That is the progressive enhancement of SvelteKit's form actions, without anything special to write.
