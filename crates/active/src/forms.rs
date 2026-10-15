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
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::reactive::{State, batch, memo, untrack, use_state};
use crate::resource::Resource;

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

/// A form: the fields created with [`field`](Form::field) and
/// [`field_with`](Form::field_with), whether they are all valid, and whether the visitor
/// tried to submit it. `Copy`.
#[derive(Clone, Copy, Debug)]
pub struct Form {
    fields: State<Vec<Status>>,
    submitted: State<bool>,
}

/// The validation state of a field, whatever the type of its value.
#[derive(Clone, Copy, Debug)]
struct Status {
    /// The error of the rules checked at once.
    error: State<Option<String>>,
    /// Set by [`Field::validate_async`]: the server's answer (`None` inside while valid).
    remote: State<Option<Resource<Option<String>>>>,
    touched: State<bool>,
}

impl Status {
    fn error(self) -> Option<String> {
        self.error.get().or_else(|| {
            self.remote
                .get()
                .and_then(|r| r.with(|e| e.cloned().flatten()))
        })
    }

    fn pending(self) -> bool {
        self.remote.get().is_some_and(|r| r.loading())
    }

    fn is_valid(self) -> bool {
        self.error().is_none() && !self.pending()
    }
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
        self.field_with(value, move |v: &String| {
            rules.iter().find_map(|r| r.check(v))
        })
    }

    /// A field holding any type of value — a checkbox's `bool`, a number bound with
    /// [`bind_parsed`](crate::Element::bind_parsed), a date, a list of choices — checked by
    /// `check`, which returns the error message or `None` when the value is valid.
    ///
    /// ```
    /// use active::forms::Form;
    /// use active::{html::*, use_state, Node};
    ///
    /// let (terms, guests) = (use_state(false), use_state(2u32));
    /// let form = Form::new();
    /// let terms_field = form.field_with(terms, |ok| (!ok).then(|| "Accept the terms".to_string()));
    /// let guests_field = form.field_with(guests, |n| (*n > 8).then(|| "8 guests at most".to_string()));
    /// let view = div()
    ///     .child(input().attr("type", "checkbox").bind_checked_field(terms_field))
    ///     .child(input().attr("type", "number").bind_parsed_field(guests_field));
    /// assert!(!form.is_valid() && guests_field.is_valid());
    /// assert_eq!(Node::from(view).render(), r#"<div><input type="checkbox"/><input type="number" value="2"/></div>"#);
    /// ```
    pub fn field_with<T: 'static>(
        self,
        value: State<T>,
        check: impl Fn(&T) -> Option<String> + 'static,
    ) -> Field<T> {
        let status = Status {
            error: memo(move || value.with(&check)),
            remote: use_state(None),
            touched: use_state(false),
        };
        untrack(|| self.fields.update(|f| f.push(status)));
        Field {
            value,
            status,
            submitted: self.submitted,
        }
    }

    /// Whether every field is valid, no [asynchronous check](Field::validate_async) being
    /// in progress (reactive: disable a button with it).
    pub fn is_valid(self) -> bool {
        self.fields
            .with(|fields| fields.iter().all(|f| f.is_valid()))
    }

    /// Whether an [asynchronous check](Field::validate_async) is in progress (show a
    /// spinner, wait before sending).
    pub fn is_pending(self) -> bool {
        self.fields
            .with(|fields| fields.iter().any(|f| f.pending()))
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
/// `Copy`. Bind a text field to an `<input>` with [`bind_field`](crate::Element::bind_field),
/// a `bool` with [`bind_checked_field`](crate::Element::bind_checked_field), a number with
/// [`bind_parsed_field`](crate::Element::bind_parsed_field).
pub struct Field<T = String> {
    value: State<T>,
    status: Status,
    submitted: State<bool>,
}

impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Field<T> {}

impl<T> fmt::Debug for Field<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl<T: 'static> Field<T> {
    /// The state holding the value.
    pub fn value(self) -> State<T> {
        self.value
    }

    /// The current error, even before the visitor has left the field (reactive): the first
    /// rule that fails, or else the answer of the [asynchronous check](Field::validate_async).
    pub fn error(self) -> Option<String> {
        self.status.error()
    }

    /// Whether the value passes every rule (and the asynchronous check is over and passed).
    pub fn is_valid(self) -> bool {
        self.status.is_valid()
    }

    /// Whether the [asynchronous check](Field::validate_async) is in progress.
    pub fn is_pending(self) -> bool {
        self.status.pending()
    }

    /// The error to show: the current error once the visitor has left the field or tried to
    /// submit the form, `None` before (nobody likes "Required" before typing anything).
    pub fn message(self) -> Option<String> {
        if self.status.touched.get() || self.submitted.get() {
            self.error()
        } else {
            None
        }
    }

    /// Whether the visitor has left the field (`blur`) since the form was reset.
    pub fn touched(self) -> bool {
        self.status.touched.get()
    }

    /// Marks the field as left: its error shows from now on.
    pub fn touch(self) {
        if !untrack(|| self.status.touched.get()) {
            self.status.touched.set(true);
        }
    }

    /// Adds a check that needs a server — "this username is taken", "this coupon has
    /// expired" — run once the other rules pass, each time the value changes (Angular's
    /// async validators, VeeValidate's async rules). `check` returns a future giving the
    /// error message, or `None` when the value is valid.
    ///
    /// While it runs, [`is_pending`](Field::is_pending) is `true` and the field (and the
    /// form) is not valid. An answer that comes after the value changed again is ignored.
    /// Wait a little in `check` before the request (a debounce) to send fewer of them.
    ///
    /// ```
    /// use active::forms::{Form, required};
    /// use active::use_state;
    ///
    /// async fn username_taken(name: String) -> bool {
    ///     name == "ada" // a request to the server
    /// }
    ///
    /// let name = use_state("ada".to_string());
    /// let form = Form::new();
    /// let field = form.field(name, [required("Choose a username")]).validate_async(|name: String| async move {
    ///     username_taken(name).await.then(|| "This username is taken".to_string())
    /// });
    /// assert_eq!(field.error().as_deref(), Some("This username is taken"));
    /// ```
    #[must_use]
    pub fn validate_async<F>(self, check: impl Fn(T) -> F + 'static) -> Self
    where
        T: Clone + PartialEq,
        F: Future<Output = Option<String>> + 'static,
    {
        let (value, sync) = (self.value, self.status.error);
        let checked = crate::resource::resource(
            move || (value.get(), sync.with(Option::is_none)),
            move |(v, valid)| {
                let task: Pin<Box<dyn Future<Output = Option<String>>>> = if valid {
                    Box::pin(check(v))
                } else {
                    Box::pin(std::future::ready(None))
                };
                task
            },
        );
        untrack(|| self.status.remote.set(Some(checked)));
        self
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

    #[test]
    fn async_checks_run_after_the_rules_and_ignore_stale_answers() {
        use std::cell::RefCell;
        type Answers = Rc<
            RefCell<
                Vec<(
                    String,
                    crate::resource::tests_support::Later<Option<String>>,
                )>,
            >,
        >;
        let answers: Answers = Rc::default();
        let log = answers.clone();
        let name = use_state(String::new());
        let form = Form::new();
        let field = form
            .field(name, [required("Name?")])
            .validate_async(move |v: String| {
                let later = crate::resource::tests_support::Later::default();
                log.borrow_mut().push((v, later.clone()));
                later
            });
        // Required fails: no request, the rule's error.
        assert_eq!(answers.borrow().len(), 0);
        assert_eq!(field.error().as_deref(), Some("Name?"));
        name.set("ada".into());
        assert!(field.is_pending() && form.is_pending() && !form.is_valid());
        name.set("bob".into());
        let (ada, bob) = {
            let a = answers.borrow();
            assert_eq!(
                a.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>(),
                ["ada", "bob"]
            );
            (a[0].1.clone(), a[1].1.clone())
        };
        bob.resolve(None);
        assert!(!field.is_pending() && field.is_valid() && form.is_valid());
        ada.resolve(Some("Taken".into()));
        assert!(field.is_valid(), "a stale answer is ignored");
    }

    #[test]
    fn typed_fields_check_any_value_and_bind_to_checkboxes_and_numbers() {
        let (terms, guests) = (use_state(false), use_state(2u32));
        let form = Form::new();
        let terms_field = form.field_with(terms, |ok| (!ok).then(|| "Accept".to_string()));
        let guests_field = form.field_with(guests, |n| (*n > 8).then(|| "Too many".to_string()));
        assert!(!form.is_valid());
        terms.set(true);
        assert!(form.is_valid());
        guests.set(9);
        assert_eq!(guests_field.error().as_deref(), Some("Too many"));
        guests_field.touch();
        let html = Node::from(input().bind_parsed_field(guests_field)).render();
        assert_eq!(html, r#"<input value="9" aria-invalid="true"/>"#);
        let html = Node::from(input().bind_checked_field(terms_field)).render();
        assert_eq!(html, r#"<input checked=""/>"#);
    }
}
