//! Islands: the interactive parts of a page. The server renders them like any view and
//! marks them; the browser hydrates only them, so static content ships no code.

use crate::escape::escape_into;
use crate::view::{Node, Str};

/// A component that can become an island: it builds its view from a props string, the same
/// way on the server and in the browser.
pub type IslandFn = fn(&str) -> Node;

#[doc(hidden)]
#[derive(Clone)]
pub struct IslandNode {
    pub(crate) name: &'static str,
    pub(crate) props: Str,
    pub(crate) view: Node,
    pub(crate) hydrate: Hydrate,
}

impl IslandNode {
    pub(crate) fn render_into(&self, out: &mut String) {
        out.push_str("<active-island data-island=\"");
        escape_into(out, self.name);
        out.push_str("\" data-props=\"");
        escape_into(out, &self.props);
        if let Some(when) = self.hydrate.attribute() {
            out.push_str("\" data-hydrate=\"");
            escape_into(out, &when);
        }
        out.push_str("\">");
        // Inside the island, text separators restart like inside an element.
        out.push_str(&self.view.render());
        out.push_str("</active-island>");
    }
}

/// Marks `component` as an island named `name`, rendered with `props`.
///
/// The browser bundle registers the same component under the same name with
/// [`hydrate`](crate::hydrate); the props string travels in the HTML (use JSON, a number,
/// anything you can parse back).
///
/// ```
/// use active::{html::button, island, use_state, Node};
///
/// fn counter(props: &str) -> Node {
///     let count = use_state(props.parse::<i32>().unwrap_or(0));
///     button().on_click(move |_| count.update(|n| *n += 1)).text_dyn(move || count.get().to_string()).into()
/// }
///
/// let html = island("counter", "5", counter).render();
/// assert_eq!(html, r#"<active-island data-island="counter" data-props="5"><button><!--#-->5</button></active-island>"#);
/// ```
pub fn island(name: &'static str, props: impl Into<Str>, component: IslandFn) -> Node {
    island_with(name, props, component, Hydrate::Load)
}

/// When the browser hydrates an island: Astro's `client:*` directives, Qwik's hydration on
/// interaction. Give it to [`island_with`].
///
/// Until it is hydrated, an island is the HTML rendered by the server: readable, styled, its
/// links working, but its buttons doing nothing yet. Delaying the hydration of what is
/// below the fold, rarely used or only for some screens leaves the main thread free for what
/// the visitor sees first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hydrate {
    /// As soon as the bundle starts (`client:load`): what [`island`] does.
    #[default]
    Load,
    /// Once the browser is idle (`client:idle`), or after 200 ms where `requestIdleCallback`
    /// does not exist: a chat button, a newsletter form.
    Idle,
    /// When the island scrolls into view (`client:visible`): comments, a map, a chart at the
    /// bottom of the page.
    Visible,
    /// While a CSS media query matches, from the first time it does (`client:media`): a
    /// mobile menu with `"(max-width: 50em)"`.
    Media(Str),
    /// At the first interaction with the island: the pointer over it, a touch, the focus
    /// (Qwik's resumability, simplified). The hydration happens before the click that
    /// follows the pointer, so the click works.
    Interaction,
}

impl Hydrate {
    /// The `data-hydrate` attribute, `None` for [`Hydrate::Load`].
    fn attribute(&self) -> Option<String> {
        match self {
            Hydrate::Load => None,
            Hydrate::Idle => Some("idle".into()),
            Hydrate::Visible => Some("visible".into()),
            Hydrate::Media(query) => Some(format!("media:{query}")),
            Hydrate::Interaction => Some("interaction".into()),
        }
    }
}

/// Like [`island`], hydrated in the browser when `when` says ([`Hydrate::Visible`],
/// [`Hydrate::Idle`]…) rather than as soon as the bundle starts. The server renders the same
/// HTML, plus a `data-hydrate` attribute read by [`hydrate`](crate::hydrate).
///
/// ```
/// use active::{html::*, island_with, Hydrate, Node};
///
/// fn comments(post: &str) -> Node {
///     section().text(format!("Comments of {post}")).into()
/// }
///
/// let html = island_with("comments", "42", comments, Hydrate::Visible).render();
/// assert_eq!(
///     html,
///     r#"<active-island data-island="comments" data-props="42" data-hydrate="visible"><section>Comments of 42</section></active-island>"#
/// );
/// ```
pub fn island_with(
    name: &'static str,
    props: impl Into<Str>,
    component: IslandFn,
    when: Hydrate,
) -> Node {
    let props = props.into();
    let view = component(&props);
    Node::Island(Box::new(IslandNode {
        name,
        props,
        view,
        hydrate: when,
    }))
}
