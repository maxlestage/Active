//! Context: values given to a whole subtree without passing them through every component
//! ([`provide`] and [`use_context`], Vue's `provide` / `inject`).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// One provided value, linked to the values provided around it.
struct Frame {
    type_id: TypeId,
    value: Rc<dyn Any>,
    parent: Option<Rc<Frame>>,
}

thread_local! {
    /// The values provided around the code being built.
    static CURRENT: RefCell<Option<Rc<Frame>>> = const { RefCell::new(None) };
}

/// The provided values captured when a dynamic region is created, given back each time it
/// rebuilds its nodes.
#[derive(Clone)]
pub(crate) struct Captured(Rc<Frame>);

/// Puts `frame` back as the current context when dropped (even after a panic).
struct Restore(Option<Rc<Frame>>);

impl Drop for Restore {
    fn drop(&mut self) {
        let previous = self.0.take();
        CURRENT.with(|c| *c.borrow_mut() = previous);
    }
}

fn run_with<R>(frame: Option<Rc<Frame>>, f: impl FnOnce() -> R) -> R {
    let previous = CURRENT.with(|c| std::mem::replace(&mut *c.borrow_mut(), frame));
    let _restore = Restore(previous);
    f()
}

/// Gives `value` to everything built by `f`, however deep: any component built inside can
/// read it with [`use_context`], without it being passed as an argument through every
/// layer. The equivalent of Vue's `provide` (and React's context providers).
///
/// Provide a theme, the signed-in user, a translation table, or a store: a struct of
/// [`State`](crate::State)s is `Copy`, so every component that reads it shares the same
/// states. An inner `provide` of the same type hides the outer one inside `f`.
///
/// Dynamic regions ([`fragment_dyn`](crate::fragment_dyn), [`keyed`](crate::keyed),
/// [`switch`](crate::switch)…) created inside `f` keep the values: their content can read
/// them each time it is rebuilt in the browser.
///
/// ```
/// use active::{html::*, provide, use_context, Node};
///
/// #[derive(Clone)]
/// struct Theme(&'static str);
///
/// fn button_themed(label: &'static str) -> Node {
///     let theme = use_context::<Theme>().map_or("light", |t| t.0);
///     button().class(theme).text(label).into()
/// }
///
/// let view = provide(Theme("dark"), || div().child(button_themed("Save")));
/// assert_eq!(Node::from(view).render(), r#"<div><button class="dark">Save</button></div>"#);
/// assert_eq!(button_themed("Save").render(), r#"<button class="light">Save</button>"#);
/// ```
pub fn provide<T: 'static, R>(value: T, f: impl FnOnce() -> R) -> R {
    let parent = CURRENT.with(|c| c.borrow().clone());
    let frame = Rc::new(Frame {
        type_id: TypeId::of::<T>(),
        value: Rc::new(value),
        parent,
    });
    run_with(Some(frame), f)
}

/// The closest value of type `T` given by an enclosing [`provide`], cloned, or `None`. The
/// equivalent of Vue's `inject`.
///
/// Call it while building the view (in the component's body or in a dynamic region's
/// closure), not in an event handler or an effect, which run later: read the value once and
/// move it into the closures that need it.
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    CURRENT.with(|c| {
        let mut frame = c.borrow().clone();
        while let Some(f) = frame {
            if f.type_id == TypeId::of::<T>() {
                return f.value.downcast_ref::<T>().cloned();
            }
            frame = f.parent.clone();
        }
        None
    })
}

/// Like [`use_context`], for a value that must have been provided.
///
/// # Panics
///
/// When no enclosing [`provide`] gave a value of type `T`; the message names the type.
///
/// ```
/// use active::{expect_context, provide};
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct User { name: String }
///
/// let name = provide(User { name: "Ada".into() }, || expect_context::<User>().name);
/// assert_eq!(name, "Ada");
/// ```
pub fn expect_context<T: Clone + 'static>() -> T {
    use_context::<T>().unwrap_or_else(|| {
        panic!(
            "active: no value of type `{}` was provided: wrap the view in `active::provide(value, || …)`",
            std::any::type_name::<T>()
        )
    })
}

/// The values provided around the code being built, if any.
pub(crate) fn capture() -> Option<Captured> {
    CURRENT.with(|c| c.borrow().clone().map(Captured))
}

/// Runs `f` with the captured values as the current context.
pub(crate) fn with_captured<R>(captured: &Captured, f: impl FnOnce() -> R) -> R {
    run_with(Some(captured.0.clone()), f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::*;
    use crate::{Node, fragment_dyn, keyed, use_state};

    #[derive(Clone, Debug, PartialEq)]
    struct Lang(&'static str);

    #[test]
    fn the_closest_value_wins_and_disappears_after_provide() {
        assert_eq!(use_context::<Lang>(), None);
        provide(Lang("en"), || {
            assert_eq!(use_context::<Lang>(), Some(Lang("en")));
            provide(7u32, || {
                // Another type does not hide it.
                assert_eq!(use_context::<Lang>(), Some(Lang("en")));
                provide(Lang("fr"), || {
                    assert_eq!(use_context::<Lang>(), Some(Lang("fr")));
                    assert_eq!(use_context::<u32>(), Some(7));
                });
            });
            assert_eq!(use_context::<Lang>(), Some(Lang("en")));
        });
        assert_eq!(use_context::<Lang>(), None);
    }

    #[test]
    fn a_panic_inside_provide_does_not_leak_the_value() {
        let result = std::panic::catch_unwind(|| provide(Lang("x"), || panic!("boom")));
        assert!(result.is_err());
        assert_eq!(use_context::<Lang>(), None);
    }

    #[test]
    #[should_panic(expected = "no value of type")]
    fn expect_context_names_the_missing_type() {
        expect_context::<Lang>();
    }

    #[test]
    fn dynamic_regions_and_keyed_rows_see_the_values_when_they_rebuild() {
        let items = use_state(vec![1, 2]);
        let view = provide(Lang("es"), || {
            div()
                .child(fragment_dyn(move || {
                    let lang = expect_context::<Lang>().0;
                    vec![p().text(format!("{lang} {}", items.with(Vec::len))).into()]
                }))
                .child(keyed(
                    move || items.get(),
                    |n| *n,
                    |n| {
                        li().text(format!("{} {n}", expect_context::<Lang>().0))
                            .into()
                    },
                ))
        });
        // Rendered (and so run) outside `provide`.
        let view = Node::from(view);
        assert_eq!(
            view.render(),
            "<div><!--[--><p>es 2</p><!--]--><!--[--><li>es 1</li><li>es 2</li><!--]--></div>"
        );
        items.set(vec![3]);
        assert!(view.render().contains("<p>es 1</p>"));
    }
}
