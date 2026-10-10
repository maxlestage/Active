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
}

impl IslandNode {
    pub(crate) fn render_into(&self, out: &mut String) {
        out.push_str("<active-island data-island=\"");
        escape_into(out, self.name);
        out.push_str("\" data-props=\"");
        escape_into(out, &self.props);
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
    let props = props.into();
    let view = component(&props);
    Node::Island(Box::new(IslandNode { name, props, view }))
}
