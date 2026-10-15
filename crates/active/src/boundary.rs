//! Error boundaries: [`error_boundary`] shows a fallback instead of a part of the page that
//! failed, like React's error boundaries and Solid's `<ErrorBoundary>`.

use std::fmt::Display;

use crate::context::{provide, use_context};
use crate::reactive::{State, batch, untrack, use_state};
use crate::view::{Node, fragment_dyn};

/// The [`error_boundary`] around a view: report errors to it, read them, reset it. `Copy`.
///
/// Inside the view, get it with `use_context::<ErrorBoundary>()` while building, then call
/// [`report`](ErrorBoundary::report) from an event handler or the end of a request.
#[derive(Clone, Copy, Debug)]
pub struct ErrorBoundary {
    errors: State<Vec<String>>,
    generation: State<u64>,
}

impl ErrorBoundary {
    /// Reports an error: the boundary replaces its view with its fallback.
    pub fn report(self, error: impl Display) {
        if self.errors.is_alive() {
            let message = error.to_string();
            self.errors.update(|e| e.push(message));
        }
    }

    /// The errors reported since the last [`reset`](ErrorBoundary::reset), oldest first.
    pub fn errors(self) -> Vec<String> {
        self.errors.get()
    }

    /// Whether an error was reported (the fallback is shown).
    pub fn has_errors(self) -> bool {
        self.errors.with(|e| !e.is_empty())
    }

    /// Forgets the errors and builds the view again (a "Try again" button).
    pub fn reset(self) {
        if self.errors.is_alive() {
            batch(|| {
                self.errors.set(Vec::new());
                self.generation.update(|g| *g += 1);
            });
        }
    }
}

/// Reports `error` to the closest [`error_boundary`] around the view being built, and
/// returns whether there was one. Without a boundary, the error is written to the browser's
/// console, or to the standard error on the server.
///
/// Call it while building the view; from an event handler, which runs later, keep the
/// boundary (`use_context::<ErrorBoundary>()`) and call [`ErrorBoundary::report`].
pub fn report_error(error: impl Display) -> bool {
    match use_context::<ErrorBoundary>() {
        Some(boundary) => {
            boundary.report(error);
            true
        }
        None => {
            #[cfg(feature = "client")]
            web_sys::console::error_1(&format!("active: {error}").into());
            #[cfg(not(feature = "client"))]
            eprintln!("active: {error}");
            false
        }
    }
}

/// Builds `view`, and shows `fallback` instead as soon as an error is reported inside it:
/// React's error boundaries, Solid's `<ErrorBoundary>`.
///
/// Errors reach the boundary in two ways:
/// - a component returns a `Result`: an `Err` converted into a [`Node`] (`.child(result)`,
///   `result.into()`) is reported to the closest boundary and shows nothing itself;
/// - code calls [`report_error`] while building, or [`ErrorBoundary::report`] later (an
///   event handler, a failed request).
///
/// `fallback` receives the boundary: show [`errors`](ErrorBoundary::errors) and offer a
/// [`reset`](ErrorBoundary::reset), which builds `view` again. Everything the failed view
/// created (states, effects, listeners) is released when the fallback replaces it.
///
/// Rust components do not throw: a panic is not caught (in the browser, it stops the
/// WebAssembly module). Return `Result`s instead.
///
/// ```
/// use active::{error_boundary, html::*, Node};
///
/// fn price(raw: &str) -> Result<Node, std::num::ParseFloatError> {
///     let value: f64 = raw.parse()?;
///     Ok(span().text(format!("{value:.2} €")).into())
/// }
///
/// let ok = error_boundary(|| price("9.5").into(), |_| p().text("Oops").into());
/// assert_eq!(ok.render(), "<!--[--><span>9.50 €</span><!--]-->");
///
/// let failed = error_boundary(
///     || div().child(price("nine")).into(),
///     |boundary| p().text(format!("Error: {}", boundary.errors().join(", "))).into(),
/// );
/// assert_eq!(failed.render(), "<!--[--><p>Error: invalid float literal</p><!--]-->");
/// ```
pub fn error_boundary(
    view: impl Fn() -> Node + 'static,
    fallback: impl Fn(ErrorBoundary) -> Node + 'static,
) -> Node {
    let boundary = ErrorBoundary {
        errors: use_state(Vec::new()),
        generation: use_state(0),
    };
    fragment_dyn(move || {
        boundary.generation.with(|_| ());
        if boundary.has_errors() {
            return vec![untrack(|| fallback(boundary))];
        }
        let content = untrack(|| provide(boundary, &view));
        // Errors reported while building.
        if untrack(|| boundary.has_errors()) {
            vec![untrack(|| fallback(boundary))]
        } else {
            vec![content]
        }
    })
}

/// An `Ok` is its node; an `Err` is reported to the closest [`error_boundary`] (see
/// [`report_error`]) and shows nothing.
impl<T: Into<Node>, E: Display> From<Result<T, E>> for Node {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(node) => node.into(),
            Err(error) => {
                report_error(error);
                Node::Empty
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::*;
    use crate::reactive::{effect, scope};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[test]
    fn errors_reported_later_show_the_fallback_and_reset_rebuilds_the_view() {
        let builds = Rc::new(Cell::new(0));
        let b = builds.clone();
        let saved: Rc<Cell<Option<ErrorBoundary>>> = Rc::default();
        let s = saved.clone();
        let html = Rc::new(RefCell::new(String::new()));
        let out = html.clone();
        let ((), owner) = scope(|| {
            let view = error_boundary(
                move || {
                    b.set(b.get() + 1);
                    s.set(use_context::<ErrorBoundary>());
                    p().text("Fine").into()
                },
                |boundary| {
                    p().text(format!("Failed: {}", boundary.errors().join("; ")))
                        .into()
                },
            );
            effect(move || *out.borrow_mut() = view.render());
        });
        assert!(html.borrow().contains("Fine"));
        let boundary = saved.get().expect("the view sees its boundary");
        boundary.report("network down");
        assert!(html.borrow().contains("Failed: network down"));
        assert!(boundary.has_errors());
        boundary.reset();
        assert!(html.borrow().contains("Fine"));
        assert!(!boundary.has_errors());
        // Built once per render, plus once after the reset.
        assert!(builds.get() >= 2);
        owner.dispose();
        boundary.report("ignored once released");
    }

    #[test]
    fn the_closest_boundary_catches_and_ok_results_render() {
        let inner_err: Result<Node, String> = Err("inner".into());
        let html = error_boundary(
            move || {
                let caught = error_boundary(
                    {
                        let e = inner_err.clone();
                        move || div().child(e.clone()).into()
                    },
                    |b| em().text(b.errors().join("")).into(),
                );
                let ok: Result<&'static str, String> = Ok("fine");
                div().child(caught).child(ok).into()
            },
            |_| p().text("outer").into(),
        )
        .render();
        assert_eq!(
            html,
            "<!--[--><div><!--[--><em>inner</em><!--]-->fine</div><!--]-->"
        );
        assert!(!report_error("no boundary here"));
    }
}
