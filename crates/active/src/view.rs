//! Views: [`Node`] trees built with element functions, rendered to HTML on the server and
//! hydrated in the browser.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use crate::escape::escape_into;
use crate::event::Event;
use crate::island::IslandNode;
use crate::seo::Seo;

/// A string that is either borrowed for the whole program or owned.
pub type Str = Cow<'static, str>;

pub(crate) type DynText = Rc<dyn Fn() -> String>;
pub(crate) type DynNodes = Rc<dyn Fn() -> Vec<Node>>;

/// One piece of a view: an element, text, a list of nodes, SEO data or an island.
///
/// Anything that converts into a `Node` can be a child: `&'static str`, `String`,
/// [`Element`], `Vec<Node>`, `Option<Node>`, [`Seo`]…
#[derive(Clone, Default)]
pub enum Node {
    /// Nothing.
    #[default]
    Empty,
    /// An HTML element.
    Element(Box<Element>),
    /// Escaped text.
    Text(Str),
    /// Text computed from [`State`](crate::State)s, kept up to date in the browser.
    DynText(DynText),
    /// Several nodes side by side.
    Fragment(Vec<Node>),
    /// SEO tags for the page `<head>`, merged by [`Document`](crate::Document).
    Seo(Box<Seo>),
    /// An interactive island, hydrated in the browser.
    Island(Box<IslandNode>),
    /// Nodes recomputed from [`State`](crate::State)s, replaced in the browser when they
    /// change: see [`fragment_dyn`].
    Dynamic(DynNodes),
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

pub(crate) enum Attr {
    Static(Str),
    Dyn(Rc<dyn Fn() -> Option<String>>),
}

impl Clone for Attr {
    fn clone(&self) -> Self {
        match self {
            Attr::Static(s) => Attr::Static(s.clone()),
            Attr::Dyn(f) => Attr::Dyn(f.clone()),
        }
    }
}

#[derive(Clone)]
pub(crate) enum Content {
    /// One text, or nothing when empty: the most common content (`li().text(…)`) needs no
    /// list.
    Text(Str),
    Children(Vec<Node>),
    Html(Str),
    DynHtml(DynText),
}

pub(crate) type Handler = Rc<dyn Fn(Event)>;
pub(crate) type ClassToggle = (Str, Rc<dyn Fn() -> bool>);
pub(crate) type Mount = Rc<dyn Fn(&crate::event::Dom)>;

/// An HTML element under construction. Create one with a tag function such as
/// [`div`](crate::html::div) or with [`el`], then chain methods.
#[derive(Clone)]
pub struct Element {
    pub(crate) tag: &'static str,
    pub(crate) attrs: Vec<(Str, Attr)>,
    pub(crate) classes: Vec<Str>,
    pub(crate) content: Content,
    /// Created by the first `class_if`, `on` or `on_mount`: static elements stay small.
    pub(crate) live: Option<Box<Live>>,
}

/// The reactive classes, event handlers and mount callbacks of an element.
#[derive(Clone, Default)]
pub(crate) struct Live {
    pub(crate) class_toggles: Vec<ClassToggle>,
    pub(crate) events: Vec<(&'static str, Handler)>,
    pub(crate) mounts: Vec<Mount>,
}

/// An element with any tag name, e.g. `el("my-widget")`.
pub fn el(tag: &'static str) -> Element {
    Element {
        tag,
        attrs: Vec::new(),
        classes: Vec::new(),
        content: Content::Text(Cow::Borrowed("")),
        live: None,
    }
}

/// Text node: `text(format!("{n} items"))`. `&str` and `String` also convert directly.
pub fn text(value: impl Into<Str>) -> Node {
    Node::Text(value.into())
}

/// Text recomputed whenever the [`State`](crate::State)s it reads change.
pub fn text_dyn(f: impl Fn() -> String + 'static) -> Node {
    Node::DynText(Rc::new(f))
}

/// Several nodes side by side, without a wrapper element.
pub fn fragment(nodes: impl IntoIterator<Item = impl Into<Node>>) -> Node {
    Node::Fragment(nodes.into_iter().map(Into::into).collect())
}

/// Nodes recomputed whenever the [`State`](crate::State)s they read change: a list, a
/// condition, the current page of a router. Only this part of the page is rebuilt, and the
/// states, effects and listeners created for the previous nodes are released.
///
/// ```
/// use active::{fragment_dyn, html::*, use_state, Node};
///
/// let todos = use_state(vec!["Write docs", "Ship"]);
/// let view = ul().child(li().text("Always first")).child(fragment_dyn(move || {
///     todos.get().into_iter().map(|t| li().text(t).into()).collect()
/// }));
/// assert_eq!(
///     Node::from(view).render(),
///     "<ul><li>Always first</li><!--[--><li>Write docs</li><li>Ship</li><!--]--></ul>"
/// );
/// ```
pub fn fragment_dyn(f: impl Fn() -> Vec<Node> + 'static) -> Node {
    Node::Dynamic(Rc::new(f))
}

impl Element {
    /// Sets an attribute; setting the same name again replaces the value.
    /// `attr("class", …)` adds classes, like [`class`](Element::class).
    #[must_use]
    pub fn attr(self, name: impl Into<Str>, value: impl Into<Str>) -> Self {
        let name = name.into();
        if name == "class" {
            return self.class(value);
        }
        self.set_attr(name, Attr::Static(value.into()))
    }

    /// An attribute recomputed from [`State`](crate::State)s. For `value` and `checked`, the
    /// browser also updates the form field itself.
    #[must_use]
    pub fn attr_dyn(self, name: impl Into<Str>, f: impl Fn() -> String + 'static) -> Self {
        self.set_attr(name.into(), Attr::Dyn(Rc::new(move || Some(f()))))
    }

    /// A boolean attribute (`hidden`, `disabled`, `checked`…), present while `f` returns true.
    #[must_use]
    pub fn bool_attr(self, name: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self {
        self.set_attr(
            name.into(),
            Attr::Dyn(Rc::new(move || f().then(String::new))),
        )
    }

    fn set_attr(mut self, name: Str, value: Attr) -> Self {
        match self.attrs.iter_mut().find(|(n, _)| *n == name) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((name, value)),
        }
        self
    }

    /// `id="…"`.
    #[must_use]
    pub fn id(self, id: impl Into<Str>) -> Self {
        self.attr("id", id)
    }

    /// Adds classes (space separated). Can be called several times.
    #[must_use]
    pub fn class(mut self, class: impl Into<Str>) -> Self {
        self.classes.push(class.into());
        self
    }

    /// Adds `class` while `f` returns true.
    #[must_use]
    pub fn class_if(mut self, class: impl Into<Str>, f: impl Fn() -> bool + 'static) -> Self {
        self.live_mut()
            .class_toggles
            .push((class.into(), Rc::new(f)));
        self
    }

    fn live_mut(&mut self) -> &mut Live {
        self.live.get_or_insert_with(Box::default)
    }

    /// The reactive classes (empty for static elements).
    pub(crate) fn class_toggles(&self) -> &[ClassToggle] {
        self.live.as_ref().map_or(&[], |l| &l.class_toggles)
    }

    /// `style="…"`.
    #[must_use]
    pub fn style(self, style: impl Into<Str>) -> Self {
        self.attr("style", style)
    }

    /// `href="…"`.
    #[must_use]
    pub fn href(self, href: impl Into<Str>) -> Self {
        self.attr("href", href)
    }

    /// Appends a child.
    #[must_use]
    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.push_child(child.into());
        self
    }

    /// Appends several children.
    #[must_use]
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Node>>) -> Self {
        let children = children.into_iter();
        let more = children.size_hint().0;
        match &mut self.content {
            Content::Children(list) => list.reserve(more),
            Content::Text(text) if text.is_empty() && more > 1 => {
                self.content = Content::Children(Vec::with_capacity(more));
            }
            _ => {}
        }
        for child in children {
            self.push_child(child.into());
        }
        self
    }

    fn push_child(&mut self, child: Node) {
        match &mut self.content {
            Content::Children(children) => children.push(child),
            Content::Text(text) if text.is_empty() => {
                self.content = match child {
                    Node::Text(text) => Content::Text(text),
                    child => Content::Children(vec![child]),
                };
            }
            Content::Text(text) => {
                let first = Node::Text(std::mem::take(text));
                self.content = Content::Children(vec![first, child]);
            }
            // Children replace raw HTML.
            Content::Html(_) | Content::DynHtml(_) => {
                self.content = Content::Children(vec![child]);
            }
        }
    }

    /// Appends text.
    #[must_use]
    pub fn text(self, value: impl Into<Str>) -> Self {
        self.child(Node::Text(value.into()))
    }

    /// Appends text recomputed from [`State`](crate::State)s.
    #[must_use]
    pub fn text_dyn(self, f: impl Fn() -> String + 'static) -> Self {
        self.child(text_dyn(f))
    }

    /// Replaces the content with trusted, already-escaped HTML (e.g. rendered Markdown).
    #[must_use]
    pub fn html(mut self, html: impl Into<Str>) -> Self {
        self.content = Content::Html(html.into());
        self
    }

    /// Like [`html`](Element::html), recomputed from [`State`](crate::State)s.
    #[must_use]
    pub fn html_dyn(mut self, f: impl Fn() -> String + 'static) -> Self {
        self.content = Content::DynHtml(Rc::new(f));
        self
    }

    /// Appends children recomputed from [`State`](crate::State)s (a list, a condition, the
    /// current page…): shorthand for `.child(fragment_dyn(f))`. Children added before or
    /// after stay where they are.
    #[must_use]
    pub fn children_dyn(self, f: impl Fn() -> Vec<Node> + 'static) -> Self {
        self.child(fragment_dyn(f))
    }

    /// Listens to a DOM event in the browser (ignored on the server).
    ///
    /// ```
    /// use active::{html::button, use_state};
    ///
    /// let count = use_state(0);
    /// let view = button().on("click", move |_| count.update(|n| *n += 1)).text_dyn(move || count.get().to_string());
    /// ```
    #[must_use]
    pub fn on(mut self, event: &'static str, handler: impl Fn(Event) + 'static) -> Self {
        if cfg!(feature = "client") {
            self.live_mut().events.push((event, Rc::new(handler)));
        }
        self
    }

    /// Shorthand for `on("click", …)`.
    #[must_use]
    pub fn on_click(self, handler: impl Fn(Event) + 'static) -> Self {
        self.on("click", handler)
    }

    /// Shorthand for `on("input", …)`; read the new value with [`Event::value`].
    #[must_use]
    pub fn on_input(self, handler: impl Fn(Event) + 'static) -> Self {
        self.on("input", handler)
    }

    /// Runs `f` with the DOM element once it is in the page (browser only), e.g. to draw on
    /// a canvas or start an animation.
    #[must_use]
    pub fn on_mount(mut self, f: impl Fn(&crate::event::Dom) + 'static) -> Self {
        if cfg!(feature = "client") {
            self.live_mut().mounts.push(Rc::new(f));
        }
        self
    }

    /// Writes ` class="…"` (static classes, then the reactive ones that are on), or nothing.
    fn write_class(&self, out: &mut String) {
        let toggles = self.class_toggles().iter().filter(|(_, on)| on());
        let mut open = false;
        for class in self.classes.iter().chain(toggles.map(|(c, _)| c)) {
            if class.is_empty() {
                continue;
            }
            out.push_str(if open { " " } else { " class=\"" });
            open = true;
            escape_into(out, class);
        }
        if open {
            out.push('"');
        }
    }
}

/// Elements whose start tag is also their end.
pub(crate) const VOID: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

impl Node {
    /// Renders to an HTML string.
    ///
    /// ```
    /// use active::html::{li, ul};
    ///
    /// let list = ul().class("menu").children(["Home", "Docs"].map(|t| li().text(t)));
    /// assert_eq!(active::Node::from(list).render(), r#"<ul class="menu"><li>Home</li><li>Docs</li></ul>"#);
    /// ```
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(256);
        self.render_into(&mut out);
        out
    }

    /// Appends the HTML to `out`.
    pub fn render_into(&self, out: &mut String) {
        crate::reactive::render_guard(|| {
            let mut text_before = false;
            render_node(self, out, &mut text_before);
        });
    }
}

/// The `Seo` nodes of a tree without dynamic regions, borrowed, in order. Returns `false`
/// (and stops) at the first dynamic region: its nodes only exist while rendering.
pub(crate) fn static_seo<'a>(node: &'a Node, out: &mut Vec<&'a Seo>) -> bool {
    match node {
        Node::Seo(seo) => {
            out.push(seo);
            true
        }
        Node::Fragment(nodes) => nodes.iter().all(|n| static_seo(n, out)),
        Node::Element(e) => match &e.content {
            Content::Children(children) => children.iter().all(|n| static_seo(n, out)),
            _ => true,
        },
        Node::Island(island) => static_seo(&island.view, out),
        Node::Dynamic(_) => false,
        Node::Empty | Node::Text(_) | Node::DynText(_) => true,
    }
}

thread_local! {
    /// While [`Document`](crate::Document) renders its body: the `Seo` nodes met, in order.
    static SEO_FOUND: std::cell::RefCell<Option<Vec<Seo>>> = const { std::cell::RefCell::new(None) };
}

/// Renders `body` into `out` and returns the `Seo` nodes it contained, dynamic regions
/// included, so that each region is evaluated once.
pub(crate) fn render_collecting_seo(body: &Node, out: &mut String) -> Vec<Seo> {
    let previous = SEO_FOUND.with(|f| f.replace(Some(Vec::new())));
    body.render_into(out);
    SEO_FOUND.with(|f| f.replace(previous)).unwrap_or_default()
}

/// Writes a node. `text_before` tracks whether the previous sibling ended with text, so that
/// two adjacent texts get a `<!---->` separator and stay two DOM text nodes when hydrated.
fn render_node(node: &Node, out: &mut String, text_before: &mut bool) {
    match node {
        Node::Empty => {}
        Node::Seo(seo) => SEO_FOUND.with(|f| {
            if let Some(found) = f.borrow_mut().as_mut() {
                found.push((**seo).clone());
            }
        }),
        Node::Text(s) => {
            if s.is_empty() {
                return;
            }
            if *text_before {
                out.push_str("<!---->");
            }
            escape_into(out, s);
            *text_before = true;
        }
        Node::DynText(f) => {
            // The marker lets the browser find (or create) the text node, even when empty.
            out.push_str("<!--#-->");
            escape_into(out, &f());
            *text_before = true;
        }
        Node::Fragment(nodes) => {
            for node in nodes {
                render_node(node, out, text_before);
            }
        }
        Node::Element(e) => {
            render_element(e, out);
            *text_before = false;
        }
        Node::Island(island) => {
            island.render_into(out);
            *text_before = false;
        }
        Node::Dynamic(f) => {
            // The markers delimit the part the browser replaces.
            out.push_str("<!--[-->");
            let mut inner = false;
            for node in f() {
                render_node(&node, out, &mut inner);
            }
            out.push_str("<!--]-->");
            *text_before = false;
        }
    }
}

pub(crate) fn render_element(e: &Element, out: &mut String) {
    out.push('<');
    out.push_str(e.tag);
    e.write_class(out);
    for (name, value) in &e.attrs {
        let value = match value {
            Attr::Static(v) => Some(Cow::Borrowed(&**v)),
            Attr::Dyn(f) => f().map(Cow::Owned),
        };
        if let Some(value) = value {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            escape_into(out, &value);
            out.push('"');
        }
    }
    if VOID.contains(&e.tag) {
        out.push_str("/>");
        return;
    }
    out.push('>');
    render_content(&e.content, out);
    out.push_str("</");
    out.push_str(e.tag);
    out.push('>');
}

pub(crate) fn render_content(content: &Content, out: &mut String) {
    let mut text_before = false;
    match content {
        Content::Text(text) => escape_into(out, text),
        Content::Children(children) => {
            for child in children {
                render_node(child, out, &mut text_before);
            }
        }
        Content::Html(html) => out.push_str(html),
        Content::DynHtml(f) => out.push_str(&f()),
    }
}

/// Visits every [`Seo`] value in tree order (used by [`Document`](crate::Document)).
/// Every `Seo` of the tree, in document order. With `dynamic`, the content of
/// [`children_dyn`](Element::children_dyn) is evaluated too (the server renders it anyway).
pub(crate) fn collect_seo(node: &Node, dynamic: bool, out: &mut Vec<Seo>) {
    match node {
        Node::Seo(seo) => out.push((**seo).clone()),
        Node::Fragment(nodes) => nodes.iter().for_each(|n| collect_seo(n, dynamic, out)),
        Node::Element(e) => {
            if let Content::Children(children) = &e.content {
                children.iter().for_each(|n| collect_seo(n, dynamic, out));
            }
        }
        Node::Island(island) => collect_seo(&island.view, dynamic, out),
        Node::Dynamic(f) if dynamic => f().iter().for_each(|n| collect_seo(n, dynamic, out)),
        Node::Empty | Node::Text(_) | Node::DynText(_) | Node::Dynamic(_) => {}
    }
}

impl From<Element> for Node {
    fn from(e: Element) -> Self {
        Node::Element(Box::new(e))
    }
}
impl From<&'static str> for Node {
    fn from(s: &'static str) -> Self {
        Node::Text(Cow::Borrowed(s))
    }
}
impl From<String> for Node {
    fn from(s: String) -> Self {
        Node::Text(Cow::Owned(s))
    }
}
impl From<Str> for Node {
    fn from(s: Str) -> Self {
        Node::Text(s)
    }
}
impl From<Seo> for Node {
    fn from(seo: Seo) -> Self {
        Node::Seo(Box::new(seo))
    }
}
impl<T: Into<Node>> From<Vec<T>> for Node {
    fn from(nodes: Vec<T>) -> Self {
        Node::Fragment(nodes.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Node>> From<Option<T>> for Node {
    fn from(node: Option<T>) -> Self {
        node.map_or(Node::Empty, Into::into)
    }
}
impl<T: Into<Node>, const N: usize> From<[T; N]> for Node {
    fn from(nodes: [T; N]) -> Self {
        Node::Fragment(nodes.into_iter().map(Into::into).collect())
    }
}

/// Element functions: `div()`, `a()`, `button()`… One per common HTML and SVG tag.
pub mod html {
    use super::{Element, el};

    macro_rules! tags {
        ($($name:ident $tag:literal),* $(,)?) => {$(
            #[doc = concat!("`<", $tag, ">`")]
            pub fn $name() -> Element {
                el($tag)
            }
        )*};
    }

    tags! {
        a "a", abbr "abbr", address "address", area "area", article "article", aside "aside",
        audio "audio", b "b", bdi "bdi", blockquote "blockquote", br "br", button "button",
        canvas "canvas", caption "caption", cite "cite", code "code", col "col",
        colgroup "colgroup", data "data", datalist "datalist", dd "dd", del "del",
        details "details", dfn "dfn", dialog "dialog", div "div", dl "dl", dt "dt", em "em",
        embed "embed", fieldset "fieldset", figcaption "figcaption", figure "figure",
        footer "footer", form "form", h1 "h1", h2 "h2", h3 "h3", h4 "h4", h5 "h5", h6 "h6",
        header "header", hgroup "hgroup", hr "hr", i "i", iframe "iframe", img "img",
        input "input", ins "ins", kbd "kbd", label "label", legend "legend", li "li",
        main "main", map "map", mark "mark", menu "menu", meter "meter", nav "nav",
        noscript "noscript", object "object", ol "ol", optgroup "optgroup", option "option",
        output "output", p "p", picture "picture", pre "pre", progress "progress", q "q",
        s "s", samp "samp", search "search", section "section", select "select",
        small "small", source "source", span "span", strong "strong", sub "sub",
        summary "summary", sup "sup", table "table", tbody "tbody", td "td",
        template "template", textarea "textarea", tfoot "tfoot", th "th", thead "thead",
        time "time", tr "tr", track "track", u "u", ul "ul", var "var", video "video",
        wbr "wbr",
        // SVG
        svg "svg", circle "circle", ellipse "ellipse", g "g", line "line", path "path",
        polygon "polygon", polyline "polyline", rect "rect", text_svg "text", tspan "tspan",
        defs "defs", linear_gradient "linearGradient", stop "stop", use_svg "use",
    }
}

#[cfg(test)]
mod tests {
    use super::html::*;
    use super::*;
    use crate::use_state;

    #[test]
    fn renders_elements_attributes_and_escaped_text() {
        let view = div()
            .id("x")
            .class("a")
            .class("b")
            .attr("data-v", "1 < 2 & \"q\"")
            .child(p().text("Tom & <Jerry>"))
            .child(img().attr("src", "/a.png").attr("alt", ""));
        assert_eq!(
            Node::from(view).render(),
            r#"<div class="a b" id="x" data-v="1 &lt; 2 &amp; &quot;q&quot;"><p>Tom &amp; &lt;Jerry&gt;</p><img src="/a.png" alt=""/></div>"#
        );
    }

    #[test]
    fn adjacent_texts_are_separated_and_dynamic_text_is_marked() {
        let n = use_state(3);
        let view = p()
            .text("a")
            .text("b")
            .text_dyn(move || format!("{n}!", n = n.get()))
            .text("c")
            .text("");
        assert_eq!(
            Node::from(view).render(),
            "<p>a<!---->b<!--#-->3!<!---->c</p>"
        );
    }

    #[test]
    fn dynamic_attributes_classes_and_children_render_their_current_value() {
        let on = use_state(true);
        let items = use_state(vec!["x", "y"]);
        let view = ul()
            .class("list")
            .class_if("on", move || on.get())
            .bool_attr("hidden", move || !on.get())
            .attr_dyn("data-n", move || items.with(Vec::len).to_string())
            .children_dyn(move || {
                items
                    .get()
                    .into_iter()
                    .map(|i| li().text(i).into())
                    .collect()
            });
        assert_eq!(
            Node::from(view).render(),
            r#"<ul class="list on" data-n="2"><!--[--><li>x</li><li>y</li><!--]--></ul>"#
        );
    }

    #[test]
    fn a_view_renders_as_often_as_needed() {
        let n = use_state(7);
        let view = Node::from(p().text_dyn(move || n.get().to_string()));
        let debug = format!("{view:?}");
        assert_eq!(debug, "<p><!--#-->7</p>");
        assert_eq!(view.render(), debug);
        assert_eq!(view.render(), debug);
    }

    #[test]
    fn the_server_releases_states_no_render_uses_anymore() {
        let old = use_state(1);
        Node::from(p().text_dyn(move || old.get().to_string())).render();
        for i in 0..1000 {
            let n = use_state(i);
            Node::from(p().text_dyn(move || n.get().to_string())).render();
        }
        let live = crate::reactive::live_states();
        assert!(live < 10, "{live} states still alive");
        let read = std::panic::catch_unwind(|| old.get());
        assert!(read.is_err(), "a released state must not be readable");
    }

    #[test]
    fn setting_an_attribute_again_replaces_it() {
        let view = a()
            .href("/a")
            .href("/b")
            .attr("class", "x")
            .class("y")
            .bool_attr("hidden", || false)
            .bool_attr("hidden", || true);
        assert_eq!(
            Node::from(view).render(),
            r#"<a class="x y" href="/b" hidden="">"#.to_string() + "</a>"
        );
    }

    #[test]
    fn fragments_options_and_raw_html() {
        let view = div()
            .child(fragment(["a", "b"]))
            .child(None::<Node>)
            .child(Some(span().html("<b>raw</b>")));
        assert_eq!(
            Node::from(view).render(),
            "<div>a<!---->b<span><b>raw</b></span></div>"
        );
    }

    #[test]
    fn events_and_mount_callbacks_are_ignored_on_the_server() {
        let view = button()
            .on_click(|_| panic!("never on the server"))
            .on_mount(|_| panic!())
            .text("ok");
        assert_eq!(Node::from(view).render(), "<button>ok</button>");
    }
}
