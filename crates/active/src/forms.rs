//! Form validation: rules on fields, an error per field shown once the visitor has left it
//! or tried to submit, and a [`Form`] that knows whether everything is valid. Angular's
//! reactive forms and validators, VeeValidate's `useField`, in a few `Copy` handles.
//!
//! ```
//! use active::forms::{Form, email, min_length, required};
//! use active::{html::*, use_state, Node};
//! use active::html::form as form_; // `form` is the name of the variable below
//!
//! fn signup() -> Node {
//!     let (address, password) = (use_state(String::new()), use_state(String::new()));
//!     let form = Form::new();
//!     let address_field = form.field(address, [required("Enter your email"), email("This is not an email")]);
//!     let password_field = form.field(password, [min_length(8, "At least 8 characters")]);
//!     form_()
//!         .on_submit(move |_| {
//!             if form.submit() {
//!                 // send address.get() and password.get()
//!             }
//!         })
//!         .child(input().attr("type", "email").bind_field(address_field))
//!         .child(p().class("error").text_dyn(move || address_field.message().unwrap_or_default()))
//!         .child(input().attr("type", "password").bind_field(password_field))
//!         .child(p().class("error").text_dyn(move || password_field.message().unwrap_or_default()))
//!         .child(button().bool_attr("disabled", move || form.submitted() && !form.is_valid()).text("Sign up"))
//!         .into()
//! }
//! assert!(signup().render().contains(r#"<input type="email" value=""/>"#));
//! ```
//!
//! Validation runs in the browser as the visitor types. Validate again on the server, which
//! receives the form: [`Rule::check`] runs the same rules on a submitted value.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use crate::reactive::{State, batch, memo, untrack, use_state};

/// A validation rule: checks a value and gives the error message when it is not valid.
/// Build them with [`required`], [`min_length`], [`email`], [`rule`]…
#[derive(Clone)]
pub struct Rule(Rc<Check>);

type Check = dyn Fn(&str) -> Option<String>;

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Rule")
    }
}

impl Rule {
    /// The error message for `value`, or `None` when it is valid (for the server, which
    /// checks the submitted values with the same rules).
    ///
    /// ```
    /// use active::forms::max_length;
    ///
    /// let rule = max_length(3, "Too long");
    /// assert_eq!(rule.check("abcd").as_deref(), Some("Too long"));
    /// assert_eq!(rule.check("abc"), None);
    /// ```
    pub fn check(&self, value: &str) -> Option<String> {
        (self.0)(value)
    }
}

/// A rule from a test: `value` is valid when `valid(value)` is `true`.
///
/// ```
/// use active::forms::rule;
///
/// let even = rule(|v| v.parse::<u32>().is_ok_and(|n| n % 2 == 0), "Enter an even number");
/// assert_eq!(even.check("4"), None);
/// assert!(even.check("5").is_some());
/// ```
pub fn rule(valid: impl Fn(&str) -> bool + 'static, message: impl Into<Cow<'static, str>>) -> Rule {
    let message = message.into();
    Rule(Rc::new(move |v| (!valid(v)).then(|| message.to_string())))
}

/// The value must not be empty (spaces alone count as empty).
pub fn required(message: impl Into<Cow<'static, str>>) -> Rule {
    rule(|v| !v.trim().is_empty(), message)
}

/// At least `n` characters, unless the value is empty (combine with [`required`]).
pub fn min_length(n: usize, message: impl Into<Cow<'static, str>>) -> Rule {
    rule(move |v| v.is_empty() || v.chars().count() >= n, message)
}

/// At most `n` characters.
pub fn max_length(n: usize, message: impl Into<Cow<'static, str>>) -> Rule {
    rule(move |v| v.chars().count() <= n, message)
}

/// Looks like an email address (`name@domain.tld`, no spaces), unless the value is empty.
/// The server sending the confirmation email is the real check.
pub fn email(message: impl Into<Cow<'static, str>>) -> Rule {
    rule(
        |v| {
            let v = v.trim();
            if v.is_empty() {
                return true;
            }
            let Some((name, domain)) = v.split_once('@') else {
                return false;
            };
            !name.is_empty()
                && !v.contains(char::is_whitespace)
                && !domain.contains('@')
                && domain
                    .split_once('.')
                    .is_some_and(|(host, tld)| !host.is_empty() && !tld.is_empty())
                && !domain.ends_with('.')
        },
        message,
    )
}

/// A number (parsed as `f64`) between `min` and `max` included, unless the value is empty.
pub fn range(min: f64, max: f64, message: impl Into<Cow<'static, str>>) -> Rule {
    rule(
        move |v| {
            let v = v.trim();
            v.is_empty() || v.parse::<f64>().is_ok_and(|n| n >= min && n <= max)
        },
        message,
    )
}

/// A form: the fields created with [`field`](Form::field), whether they are all valid, and
/// whether the visitor tried to submit it. `Copy`.
#[derive(Clone, Copy, Debug)]
pub struct Form {
    fields: State<Vec<Field>>,
    submitted: State<bool>,
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

impl Form {
    /// A form without fields, owned by the component being built.
    pub fn new() -> Self {
        Form {
            fields: use_state(Vec::new()),
            submitted: use_state(false),
        }
    }

    /// A field of this form bound to `value` and checked by `rules`, in order: the error is
    /// the message of the first rule that fails.
    pub fn field(self, value: State<String>, rules: impl IntoIterator<Item = Rule>) -> Field {
        let rules: Vec<Rule> = rules.into_iter().collect();
        let field = Field {
            value,
            error: memo(move || value.with(|v| rules.iter().find_map(|r| r.check(v)))),
            touched: use_state(false),
            submitted: self.submitted,
        };
        untrack(|| self.fields.update(|f| f.push(field)));
        field
    }

    /// Whether every field is valid (reactive: disable a button with it).
    pub fn is_valid(self) -> bool {
        self.fields
            .with(|fields| fields.iter().all(|f| f.is_valid()))
    }

    /// Call it when the form is submitted: from then on every field shows its error, and it
    /// returns whether the form is valid (send it only then).
    pub fn submit(self) -> bool {
        if !untrack(|| self.submitted.get()) {
            self.submitted.set(true);
        }
        untrack(|| self.is_valid())
    }

    /// Whether [`submit`](Form::submit) was called since the last [`reset`](Form::reset).
    pub fn submitted(self) -> bool {
        self.submitted.get()
    }

    /// Hides the errors again (every field untouched, not submitted). Values are left alone:
    /// clear the states yourself to empty the form.
    pub fn reset(self) {
        let fields = untrack(|| self.fields.get());
        batch(|| {
            self.submitted.set(false);
            for f in fields {
                f.touched.set(false);
            }
        });
    }
}

/// One field of a [`Form`]: its value, its error and whether the visitor has left it.
/// `Copy`. Bind it to an `<input>` with [`bind_field`](crate::Element::bind_field).
#[derive(Clone, Copy, Debug)]
pub struct Field {
    value: State<String>,
    error: State<Option<String>>,
    touched: State<bool>,
    submitted: State<bool>,
}

impl Field {
    /// The state holding the value.
    pub fn value(self) -> State<String> {
        self.value
    }

    /// The current error, even before the visitor has left the field (reactive).
    pub fn error(self) -> Option<String> {
        self.error.get()
    }

    /// Whether the value passes every rule.
    pub fn is_valid(self) -> bool {
        self.error.with(Option::is_none)
    }

    /// The error to show: the current error once the visitor has left the field or tried to
    /// submit the form, `None` before (nobody likes "Required" before typing anything).
    pub fn message(self) -> Option<String> {
        if self.touched.get() || self.submitted.get() {
            self.error()
        } else {
            None
        }
    }

    /// Whether the visitor has left the field (`blur`) since the form was reset.
    pub fn touched(self) -> bool {
        self.touched.get()
    }

    /// Marks the field as left: its error shows from now on.
    pub fn touch(self) {
        if !untrack(|| self.touched.get()) {
            self.touched.set(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;
    use crate::html::*;

    #[test]
    fn rules_check_values() {
        let mail = email("bad");
        for ok in ["", "a@b.co", " ada@lovelace.dev "] {
            assert_eq!(mail.check(ok), None, "{ok}");
        }
        for bad in ["a", "a@", "@b.c", "a@b", "a@b.", "a b@c.d", "a@b@c.d"] {
            assert!(mail.check(bad).is_some(), "{bad}");
        }
        assert!(required("x").check("  ").is_some());
        assert_eq!(min_length(3, "x").check(""), None);
        assert!(min_length(3, "x").check("éé").is_some());
        assert_eq!(min_length(2, "x").check("éé"), None);
        assert!(range(1.0, 5.0, "x").check("6").is_some());
        assert!(range(1.0, 5.0, "x").check("a").is_some());
        assert_eq!(range(1.0, 5.0, "x").check("2.5"), None);
    }

    #[test]
    fn errors_show_once_touched_or_submitted_and_the_form_follows_its_fields() {
        let (name, mail) = (use_state(String::new()), use_state(String::new()));
        let form = Form::new();
        let name_field = form.field(name, [required("Name?"), max_length(5, "Too long")]);
        let mail_field = form.field(mail, [email("Email?")]);
        assert_eq!(name_field.error().as_deref(), Some("Name?"));
        assert_eq!(name_field.message(), None);
        assert!(mail_field.is_valid());
        assert!(!form.is_valid());
        name_field.touch();
        assert_eq!(name_field.message().as_deref(), Some("Name?"));
        name.set("Ada Lovelace".into());
        assert_eq!(name_field.message().as_deref(), Some("Too long"));
        name.set("Ada".into());
        mail.set("ada@".into());
        assert_eq!(mail_field.message(), None);
        assert!(!form.submit());
        assert_eq!(mail_field.message().as_deref(), Some("Email?"));
        mail.set("ada@x.io".into());
        assert!(form.submit());
        form.reset();
        assert!(!form.submitted() && !name_field.touched());
    }

    #[test]
    fn bind_field_renders_the_value_and_aria_invalid_once_touched() {
        let name = use_state(String::new());
        let form = Form::new();
        let field = form.field(name, [required("Name?")]);
        let html = || Node::from(input().bind_field(field)).render();
        assert_eq!(html(), r#"<input value=""/>"#);
        field.touch();
        assert_eq!(html(), r#"<input value="" aria-invalid="true"/>"#);
    }
}
