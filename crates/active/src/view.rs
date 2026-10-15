//! Views: [`Node`] trees built with element functions, rendered to HTML on the server and
//! hydrated in the browser.

use std::any::Any;
use std::borrow::Cow;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use crate::escape::escape_into;
use crate::event::Event;
use crate::island::IslandNode;
use crate::reactive::{State, effect, memo, untrack, use_state};
use crate::seo::Seo;

/// A string that is either borrowed for the whole program or owned.
pub type Str = Cow<'static, str>;

pub(crate) type DynText = Rc<dyn Fn() -> String>;
pub(crate) type DynNodes = Rc<dyn Fn() -> Vec<Node>>;

/// The items of a keyed list, each with the hash of its key (item type erased).
pub(crate) type KeyedItems = Rc<dyn Fn() -> Vec<(u64, Rc<dyn Any>)>>;
/// Builds the row of an item of a keyed list.
pub(crate) type KeyedRow = Rc<dyn Fn(&dyn Any) -> Node>;

/// A list kept by key (see [`keyed`]), with its item type erased.
#[doc(hidden)]
pub struct Keyed {
    /// The current items, each with the hash of its key.
    pub(crate) items: KeyedItems,
    /// Whether two items are equal (a kept row whose item changed is rebuilt).
    #[cfg_attr(not(feature = "client"), allow(dead_code))]
    pub(crate) same: fn(&dyn Any, &dyn Any) -> bool,
    pub(crate) row: KeyedRow,
    /// Browser: keeps the rows in sync. Set here, so that only apps using `keyed` contain
    /// this code.
    #[cfg(feature = "client")]
    pub(crate) bind: crate::client::BindKeyed,
    /// Set by [`transition_group`]: the name of the transitions of the rows.
    #[cfg(feature = "client")]
    pub(crate) group: Option<&'static str>,
}

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
    /// A list whose rows are kept by key: see [`keyed`].
    Keyed(Rc<Keyed>),
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
/// In the browser, a region rebuilt while it holds the focus gives it back to the element
/// that takes the focused one's place: the one with the same `id`, or else the one at the
/// same position with the same tags and classes (a text field also keeps its selection if
/// its text is unchanged). A menu rebuilt in another language keeps the keyboard where it
/// was; another page, built from other elements, takes nothing.
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
    // Browser: a rebuilt region gives the focus back to its new content. Set here, so that
    // only apps with dynamic regions contain this code.
    #[cfg(feature = "client")]
    crate::client::keep_focus_in_regions();
    // The values given by `provide` around the region stay readable when it rebuilds.
    match crate::context::capture() {
        None => Node::Dynamic(Rc::new(f)),
        Some(context) => {
            Node::Dynamic(Rc::new(move || crate::context::with_captured(&context, &f)))
        }
    }
}

/// A list whose rows are kept by key. When the [`State`](crate::State)s read by `items`
/// change, the browser keeps the DOM of every row whose key is still there and whose item
/// is unchanged (with its states, listeners, focus and scroll position), moves it if the
/// order changed, builds the new or changed rows and releases the removed ones.
///
/// [`fragment_dyn`] rebuilds everything, which is simpler and fine for short lists; use
/// `keyed` for long or interactive ones. Make each row one element (`li`, `tr`…).
///
/// ```
/// use active::{html::*, keyed, use_state, Node};
///
/// #[derive(Clone, PartialEq)]
/// struct Todo { id: u32, text: String }
///
/// let todos = use_state(vec![Todo { id: 1, text: "Write".into() }, Todo { id: 2, text: "Ship".into() }]);
/// let list = ul().child(keyed(
///     move || todos.get(),
///     |todo| todo.id,
///     |todo| li().text(todo.text.clone()).into(),
/// ));
/// assert_eq!(Node::from(list).render(), "<ul><!--[--><li>Write</li><li>Ship</li><!--]--></ul>");
/// ```
pub fn keyed<T, K>(
    items: impl Fn() -> Vec<T> + 'static,
    key: impl Fn(&T) -> K + 'static,
    row: impl Fn(&T) -> Node + 'static,
) -> Node
where
    T: PartialEq + 'static,
    K: Hash,
{
    fn same<T: PartialEq + 'static>(a: &dyn Any, b: &dyn Any) -> bool {
        a.downcast_ref::<T>() == b.downcast_ref::<T>()
    }
    // The values given by `provide` around the list stay readable when rows are built.
    let context = crate::context::capture();
    let row = move |item: &T| match &context {
        Some(context) => crate::context::with_captured(context, || row(item)),
        None => row(item),
    };
    Node::Keyed(Rc::new(Keyed {
        items: Rc::new(move || {
            items()
                .into_iter()
                .map(|item| {
                    let mut hasher = DefaultHasher::new();
                    key(&item).hash(&mut hasher);
                    (hasher.finish(), Rc::new(item) as Rc<dyn Any>)
                })
                .collect()
        }),
        same: same::<T>,
        row: Rc::new(move |item| row(item.downcast_ref::<T>().expect("keyed: item type"))),
        #[cfg(feature = "client")]
        bind: crate::client::bind_keyed,
        #[cfg(feature = "client")]
        group: None,
    }))
}

/// Animates the rows of a [`keyed`] list (or [`indexed`], or `children_keyed`'s content):
/// Vue's `<TransitionGroup>`, Svelte's `animate:flip`.
///
/// - an added row plays `{name}-enter-from` / `-enter-active` / `-enter-to`;
/// - a removed row stays in the page while it plays `{name}-leave-from` / `-leave-active` /
///   `-leave-to`, then goes;
/// - a row that moves (sorting, filtering, an insertion above it) slides from its old place
///   to its new one (the FLIP technique) with the class `{name}-move`, which holds the
///   transition.
///
/// Nothing plays when the list first appears or is hydrated. Any other node is returned as
/// it is; on the server, the list renders as usual.
///
/// ```css
/// .list-enter-active, .list-leave-active, .list-move { transition: all .3s ease; }
/// .list-enter-from, .list-leave-to { opacity: 0; transform: translateX(2rem); }
/// /* Leaving rows leave the flow, so that the others can slide into their place. */
/// .list-leave-active { position: absolute; }
/// ```
///
/// ```
/// use active::{html::*, keyed, transition_group, use_state, Node};
///
/// let items = use_state(vec![1, 2, 3]);
/// let list = ul().child(transition_group(
///     "list",
///     keyed(move || items.get(), |n| *n, |n| li().text(n.to_string()).into()),
/// ));
/// assert_eq!(Node::from(list).render(), "<ul><!--[--><li>1</li><li>2</li><li>3</li><!--]--></ul>");
/// ```
pub fn transition_group(name: &'static str, list: Node) -> Node {
    #[cfg(feature = "client")]
    if let Node::Keyed(keyed) = &list {
        return Node::Keyed(Rc::new(Keyed {
            items: keyed.items.clone(),
            same: keyed.same,
            row: keyed.row.clone(),
            bind: keyed.bind,
            group: Some(name),
        }));
    }
    let _ = name;
    list
}

/// A list whose rows are kept by **position** and updated in place: row `i` is built once,
/// with a [`State`] holding the `i`-th item, and when the items change only the texts,
/// attributes and classes bound to that state change. Rows are added or removed at the end
/// when the list grows or shrinks.
///
/// Use it for lists whose items change in place, such as a ranking refreshed every second or
/// the results of another season: rows are never rebuilt, so they keep their DOM, their
/// animations and their focus. [`keyed`] follows each item instead (rows move with their
/// item); [`fragment_dyn`] rebuilds everything.
///
/// ```
/// use active::{html::*, indexed, use_state, Node};
///
/// let ranking = use_state(vec!["Norris", "Piastri"]);
/// let list = ol().child(indexed(
///     move || ranking.get(),
///     |name| li().text_dyn(move || name.get().to_string()).into(),
/// ));
/// assert_eq!(
///     Node::from(list).render(),
///     "<ol><!--[--><li><!--#-->Norris</li><li><!--#-->Piastri</li><!--]--></ol>"
/// );
/// ```
pub fn indexed<T>(
    items: impl Fn() -> Vec<T> + 'static,
    row: impl Fn(State<T>) -> Node + 'static,
) -> Node
where
    T: Clone + PartialEq + 'static,
{
    let list = memo(items);
    keyed(
        move || list.with(|items| (0..items.len()).collect()),
        |i: &usize| *i,
        move |&i| {
            let Some(first) = untrack(|| list.with(|items| items.get(i).cloned())) else {
                return Node::Empty;
            };
            let item = use_state(first);
            effect(move || {
                // `None`: the list just shrank, this row is about to be removed.
                if let Some(next) = list.with(|items| items.get(i).cloned()) {
                    untrack(|| {
                        if item.with(|current| *current != next) {
                            item.set(next);
                        }
                    });
                }
            });
            row(item)
        },
    )
}

/// Shows a view for a value that changes, and rebuilds it only when the **kind** of the
/// value changes. While `kind` returns the same thing, the view is kept (its states, requests,
/// DOM and focus) and the [`State`] it was built with receives the new value.
///
/// The typical use is a router: `value` reads the current route, `kind` says which page it is,
/// and going from `/posts/1` to `/posts/2` keeps the post page and only updates what depends
/// on the post id. The state given to a view only ever holds values of that view's kind.
///
/// ```
/// use active::{html::*, switch, untrack, use_state, Node, State};
///
/// #[derive(Clone, PartialEq)]
/// enum Page { Home, Post(u32) }
///
/// fn post(page: State<Page>) -> Node {
///     let id = move || page.with(|p| match p { Page::Post(id) => *id, _ => 0 });
///     h1().text_dyn(move || format!("Post {}", id())).into()
/// }
///
/// let current = use_state(Page::Post(7));
/// let view = switch(
///     move || current.get(),
///     |page| std::mem::discriminant(page),
///     |page| match untrack(|| page.get()) {
///         Page::Home => h1().text("Home").into(),
///         Page::Post(_) => post(page),
///     },
/// );
/// assert_eq!(view.render(), "<!--[--><h1><!--#-->Post 7</h1><!--]-->");
/// ```
pub fn switch<V, K>(
    value: impl Fn() -> V + 'static,
    kind: impl Fn(&V) -> K + 'static,
    view: impl Fn(State<V>) -> Node + 'static,
) -> Node
where
    V: Clone + PartialEq + 'static,
    K: PartialEq + 'static,
{
    let latest = Rc::new(std::cell::RefCell::new(untrack(&value)));
    let shown = use_state(kind(&latest.borrow()));
    // The value state of the view on screen (created with it, released with it).
    let current: Rc<std::cell::Cell<Option<State<V>>>> = Rc::default();
    {
        let (latest, current) = (latest.clone(), current.clone());
        effect(move || {
            let next = value();
            untrack(|| {
                let next_kind = kind(&next);
                *latest.borrow_mut() = next.clone();
                if shown.with(|k| *k != next_kind) {
                    // Another kind: the view is rebuilt below, with the new value.
                    shown.set(next_kind);
                } else if let Some(state) = current.get().filter(|s| s.is_alive()) {
                    if state.with(|v| *v != next) {
                        state.set(next);
                    }
                }
            });
        });
    }
    fragment_dyn(move || {
        shown.with(|_| ());
        untrack(|| {
            let state = use_state(latest.borrow().clone());
            current.set(Some(state));
            vec![view(state)]
        })
    })
}

/// Renders `view` at the end of the element matching `target` (`"body"`, `"#modals"`)
/// rather than where the portal is: Vue's `<Teleport>`, React's `createPortal`. Use it for
/// dialogs, toasts and tooltips, which must escape the `overflow: hidden` or the stacking
/// context of their parents. The view keeps the context ([`provide`](crate::provide)) of
/// where the portal is, and is removed with it.
///
/// In the browser only: the server renders nothing in its place (as React does), so put
/// in a portal what appears after an interaction.
///
/// ```
/// use active::{html::*, portal, use_state, Node};
///
/// let open = use_state(true);
/// let page = div().children_dyn(move || {
///     if open.get() {
///         vec![portal("body", div().class("dialog").text("Saved"))]
///     } else {
///         vec![]
///     }
/// });
/// assert_eq!(Node::from(page).render(), "<div><!--[--><!--[--><!--]--><!--]--></div>");
/// ```
pub fn portal(target: &'static str, view: impl Into<Node>) -> Node {
    let view = view.into();
    #[cfg(feature = "client")]
    {
        fragment_dyn(move || {
            untrack(|| crate::client::portal(target, &view));
            Vec::new()
        })
    }
    #[cfg(not(feature = "client"))]
    {
        let _ = (target, view);
        fragment_dyn(Vec::new)
    }
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

    /// Appends a list whose rows are kept by key: shorthand for
    /// `.child(keyed(items, key, row))`, see [`keyed`].
    #[must_use]
    pub fn children_keyed<T, K>(
        self,
        items: impl Fn() -> Vec<T> + 'static,
        key: impl Fn(&T) -> K + 'static,
        row: impl Fn(&T) -> Node + 'static,
    ) -> Self
    where
        T: PartialEq + 'static,
        K: Hash,
    {
        self.child(keyed(items, key, row))
    }

    /// Appends a list whose rows are kept by position and updated in place: shorthand for
    /// `.child(indexed(items, row))`, see [`indexed`].
    #[must_use]
    pub fn children_indexed<T>(
        self,
        items: impl Fn() -> Vec<T> + 'static,
        row: impl Fn(State<T>) -> Node + 'static,
    ) -> Self
    where
        T: Clone + PartialEq + 'static,
    {
        self.child(indexed(items, row))
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

    /// Shorthand for `on("submit", …)` that first cancels the browser's default action
    /// (Vue's `@submit.prevent`): the page is not reloaded, `handler` does the work.
    #[must_use]
    pub fn on_submit(self, handler: impl Fn(Event) + 'static) -> Self {
        self.on("submit", move |e| {
            e.prevent_default();
            handler(e);
        })
    }

    /// Calls `handler` on `keydown` when the key pressed is `key` (`"Enter"`, `"Escape"`,
    /// `"ArrowDown"`, `"k"`…), like Vue's `@keydown.enter`. Several `on_key` on the same
    /// element each listen to their own key.
    #[must_use]
    pub fn on_key(self, key: &'static str, handler: impl Fn(Event) + 'static) -> Self {
        self.on("keydown", move |e| {
            if e.key() == key {
                handler(e);
            }
        })
    }

    /// Binds a text field to `state` in both directions (Vue's `v-model`): the field shows
    /// the state, and what the user types is written back into it on each `input` event.
    ///
    /// Works with `<input>`, `<textarea>` and `<select>`. The server renders the state as the
    /// field's `value` (as the text of a `<textarea>`); for a `<select>`, also mark the
    /// matching `<option>` with `bool_attr("selected", …)` so the page shows it before the
    /// WebAssembly has loaded.
    ///
    /// ```
    /// use active::{html::*, use_state, Node};
    ///
    /// let name = use_state(String::from("Ada"));
    /// let field = input().attr("type", "text").bind_value(name);
    /// assert_eq!(Node::from(field).render(), r#"<input type="text" value="Ada"/>"#);
    ///
    /// let notes = use_state(String::from("1 < 2"));
    /// assert_eq!(Node::from(textarea().bind_value(notes)).render(), "<textarea>1 &lt; 2</textarea>");
    /// ```
    #[must_use]
    pub fn bind_value(self, state: State<String>) -> Self {
        let event = if self.tag == "select" {
            "change"
        } else {
            "input"
        };
        let field = if self.tag == "textarea" {
            // A `<textarea>` shows its content; in the browser, the `value` binding follows.
            let first = untrack(|| state.get());
            let field = self.text(first);
            if cfg!(feature = "client") {
                field.attr_dyn("value", move || state.get())
            } else {
                field
            }
        } else {
            self.attr_dyn("value", move || state.get())
        };
        field.on(event, move |e| {
            let value = e.value();
            if state.with(|current| *current != value) {
                state.set(value);
            }
        })
    }

    /// Binds a checkbox to `state` in both directions (Vue's `v-model` on a checkbox): it is
    /// checked while the state is `true`, and clicking it writes the new value.
    ///
    /// ```
    /// use active::{html::*, use_state, Node};
    ///
    /// let newsletter = use_state(true);
    /// let checkbox = input().attr("type", "checkbox").bind_checked(newsletter);
    /// assert_eq!(Node::from(checkbox).render(), r#"<input type="checkbox" checked=""/>"#);
    /// ```
    #[must_use]
    pub fn bind_checked(self, state: State<bool>) -> Self {
        self.bool_attr("checked", move || state.get())
            .on("change", move |e| state.set(e.checked()))
    }

    /// Binds a field to a state of any type that parses from and displays as text, such as a
    /// number (Vue's `v-model.number`). What the user types is written into the state when it
    /// parses; while it does not (an empty field, `"1."`, `"-"`), the state keeps its last
    /// value and the text typed is left alone.
    ///
    /// ```
    /// use active::{html::*, use_state, Node};
    ///
    /// let quantity = use_state(3u32);
    /// let field = input().attr("type", "number").bind_parsed(quantity);
    /// assert_eq!(Node::from(field).render(), r#"<input type="number" value="3"/>"#);
    /// ```
    #[must_use]
    pub fn bind_parsed<T>(self, state: State<T>) -> Self
    where
        T: std::str::FromStr + fmt::Display + PartialEq + 'static,
    {
        // What the field shows: the user's text, replaced only when the state changes to a
        // value that text does not already stand for.
        let shown = use_state(untrack(|| state.with(ToString::to_string)));
        effect(move || {
            let text = state.with(ToString::to_string);
            untrack(|| {
                let same = shown.with(|s| {
                    *s == text || state.with(|v| s.trim().parse::<T>().is_ok_and(|p| p == *v))
                });
                if !same {
                    shown.set(text);
                }
            });
        });
        let event = if self.tag == "select" {
            "change"
        } else {
            "input"
        };
        self.attr_dyn("value", move || shown.get())
            .on(event, move |e| {
                let text = e.value();
                let parsed = text.trim().parse::<T>().ok();
                shown.set(text);
                if let Some(value) = parsed {
                    if state.with(|current| *current != value) {
                        state.set(value);
                    }
                }
            })
    }

    /// One CSS property of the `style` attribute, recomputed from [`State`]s (Vue's
    /// `:style="{ width: … }"`). `f` returns the value (`"40%"`, `"red"`); an empty string
    /// leaves the property out. Combines with [`style`](Element::style) set before it and
    /// with other `style_dyn`; a `style` set after replaces them all.
    ///
    /// ```
    /// use active::{html::*, use_state, Node};
    ///
    /// let progress = use_state(40);
    /// let color = use_state(String::new());
    /// let bar = div()
    ///     .style("height: 4px")
    ///     .style_dyn("width", move || format!("{}%", progress.get()))
    ///     .style_dyn("background", move || color.get());
    /// assert_eq!(Node::from(bar).render(), r#"<div style="height: 4px; width: 40%"></div>"#);
    /// ```
    #[must_use]
    pub fn style_dyn(self, property: &'static str, f: impl Fn() -> String + 'static) -> Self {
        let base: Option<Rc<dyn Fn() -> Option<String>>> = self
            .attrs
            .iter()
            .find(|(n, _)| n == "style")
            .map(|(_, value)| match value {
                Attr::Static(s) => {
                    let s = s.clone();
                    Rc::new(move || Some(s.to_string())) as Rc<dyn Fn() -> Option<String>>
                }
                Attr::Dyn(f) => f.clone(),
            });
        self.set_attr(
            Cow::Borrowed("style"),
            Attr::Dyn(Rc::new(move || {
                let mut style = base.as_ref().and_then(|b| b()).unwrap_or_default();
                let value = f();
                if !value.is_empty() {
                    let style_end = style.trim_end().trim_end_matches(';').len();
                    style.truncate(style_end);
                    if !style.is_empty() {
                        style.push_str("; ");
                    }
                    style.push_str(property);
                    style.push_str(": ");
                    style.push_str(&value);
                }
                (!style.is_empty()).then_some(style)
            })),
        )
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

    /// Attaches an **action** to the element: Svelte's `use:`, a Vue directive. `action` is
    /// called with the DOM element once it is in the page (browser only) and returns what
    /// undoes it, called when the element is removed (its region rebuilt, its row removed,
    /// its page left).
    ///
    /// Actions package behaviour that needs the element and must be cleaned up: a tooltip, a
    /// click-outside handler, an `IntersectionObserver`, a chart library, focus trapping.
    /// Write one as a function and reuse it on any element; effects created inside follow
    /// states and stop with the element.
    ///
    /// ```
    /// use active::{html::*, Dom, Node};
    ///
    /// fn tooltip(text: &'static str) -> impl Fn(&Dom) -> Box<dyn FnOnce()> {
    ///     move |dom: &Dom| {
    ///         // Browser: create the tooltip, listen to `pointerenter` on `dom`…
    ///         let _ = (dom, text);
    ///         Box::new(|| { /* remove the tooltip */ })
    ///     }
    /// }
    ///
    /// // `use<>`: the returned cleanup does not borrow the element.
    /// fn autofocus(dom: &Dom) -> impl FnOnce() + use<> {
    ///     let _ = dom; // `dom.dyn_ref::<web_sys::HtmlElement>()` → `.focus()`
    ///     || {}
    /// }
    ///
    /// let saved = button().use_action(tooltip("Saved to your account")).text("Save");
    /// assert_eq!(Node::from(saved).render(), "<button>Save</button>");
    /// let field = input().use_action(autofocus);
    /// assert_eq!(Node::from(field).render(), "<input/>");
    /// ```
    #[must_use]
    pub fn use_action<C: FnOnce() + 'static>(
        self,
        action: impl Fn(&crate::event::Dom) -> C + 'static,
    ) -> Self {
        self.on_mount(move |dom| {
            let undo = action(dom);
            crate::reactive::on_cleanup(undo);
        })
    }

    /// Binds a text field to a validated [`Field`](crate::forms::Field) of a
    /// [`Form`](crate::forms::Form): [`bind_value`](Element::bind_value) on its value, the
    /// field marked as touched when the visitor leaves it (`blur`, from then on its error
    /// shows), and `aria-invalid="true"` while its error is shown, for screen readers and for
    /// CSS (`input[aria-invalid] { border-color: red }`).
    ///
    /// ```
    /// use active::forms::{Form, required};
    /// use active::{html::*, use_state, Node};
    ///
    /// let name = use_state(String::new());
    /// let field = Form::new().field(name, [required("Enter your name")]);
    /// let html = Node::from(input().bind_field(field)).render();
    /// assert_eq!(html, r#"<input value=""/>"#); // no error shown before the visitor leaves it
    /// ```
    #[must_use]
    pub fn bind_field(self, field: crate::forms::Field) -> Self {
        self.bind_value(field.value())
            .on("blur", move |_| field.touch())
            .set_attr(
                Cow::Borrowed("aria-invalid"),
                Attr::Dyn(Rc::new(move || {
                    field.message().is_some().then(|| "true".to_string())
                })),
            )
    }

    /// Binds a checkbox to a `bool` [`Field`](crate::forms::Field) (see
    /// [`Form::field_with`](crate::forms::Form::field_with)), like
    /// [`bind_field`](Element::bind_field): [`bind_checked`](Element::bind_checked), touched
    /// on `blur`, `aria-invalid` while its error shows.
    #[must_use]
    pub fn bind_checked_field(self, field: crate::forms::Field<bool>) -> Self {
        self.bind_checked(field.value()).field_status(field)
    }

    /// Binds a number (or any value that parses from text) to a
    /// [`Field`](crate::forms::Field), like [`bind_field`](Element::bind_field), with
    /// [`bind_parsed`](Element::bind_parsed).
    #[must_use]
    pub fn bind_parsed_field<T>(self, field: crate::forms::Field<T>) -> Self
    where
        T: std::str::FromStr + fmt::Display + PartialEq + 'static,
    {
        self.bind_parsed(field.value()).field_status(field)
    }

    /// Touched on `blur`, `aria-invalid` while the field's error shows.
    fn field_status<T: 'static>(self, field: crate::forms::Field<T>) -> Self {
        self.on("blur", move |_| field.touch()).set_attr(
            Cow::Borrowed("aria-invalid"),
            Attr::Dyn(Rc::new(move || {
                field.message().is_some().then(|| "true".to_string())
            })),
        )
    }

    /// Shows the element while `when` is `true`, and hides it (`hidden`) otherwise, playing
    /// CSS transitions each time it appears or disappears: Vue's `<Transition>` with `v-show`,
    /// Svelte's `transition:`.
    ///
    /// As in Vue, the element gets classes named after `name` during each phase:
    /// `{name}-enter-from`, `{name}-enter-active` and `{name}-enter-to` while it appears,
    /// `{name}-leave-from`, `{name}-leave-active` and `{name}-leave-to` while it disappears;
    /// `hidden` is set once the leave transition (or animation) is over. The first state is
    /// shown without a transition, and the server renders `hidden` when `when` is `false`.
    ///
    /// ```css
    /// .fade-enter-active, .fade-leave-active { transition: opacity .2s ease; }
    /// .fade-enter-from, .fade-leave-to { opacity: 0; }
    /// ```
    ///
    /// ```
    /// use active::{html::*, use_state, Node};
    ///
    /// let open = use_state(false);
    /// let menu = nav().transition("fade", move || open.get()).text("Menu");
    /// assert_eq!(Node::from(menu).render(), r#"<nav hidden="">Menu</nav>"#);
    /// ```
    #[must_use]
    pub fn transition(self, name: &'static str, when: impl Fn() -> bool + 'static) -> Self {
        #[cfg(feature = "client")]
        {
            let when: Rc<dyn Fn() -> bool> = Rc::new(when);
            self.on_mount(move |dom| crate::client::transition(dom, name, when.clone()))
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = name;
            self.bool_attr("hidden", move || !when())
        }
    }

    /// Plays the `{name}-enter-*` transition when the element is added to the page (not when
    /// the page is hydrated): rows added to a list, a page shown by the router, a toast. See
    /// [`transition`](Element::transition) for the classes; Svelte's `in:`.
    ///
    /// ```css
    /// .slide-enter-active { transition: transform .25s ease-out; }
    /// .slide-enter-from { transform: translateY(1rem); }
    /// ```
    #[must_use]
    pub fn transition_in(self, name: &'static str) -> Self {
        #[cfg(feature = "client")]
        {
            self.on_mount(move |dom| crate::client::transition_in(dom, name))
        }
        #[cfg(not(feature = "client"))]
        {
            let _ = name;
            self
        }
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
        Node::Dynamic(_) | Node::Keyed(_) => false,
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
        Node::Keyed(list) => {
            out.push_str("<!--[-->");
            let mut inner = false;
            for (_, item) in (list.items)() {
                render_node(&(list.row)(&*item), out, &mut inner);
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
        Node::Keyed(list) if dynamic => (list.items)()
            .iter()
            .for_each(|(_, item)| collect_seo(&(list.row)(&**item), dynamic, out)),
        Node::Empty | Node::Text(_) | Node::DynText(_) | Node::Dynamic(_) | Node::Keyed(_) => {}
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
    fn transitions_render_hidden_and_portals_render_an_empty_region() {
        let open = use_state(true);
        let panel = || {
            Node::from(
                div()
                    .transition("fade", move || open.get())
                    .transition_in("slide"),
            )
            .render()
        };
        assert_eq!(panel(), "<div></div>");
        open.set(false);
        assert_eq!(panel(), r#"<div hidden=""></div>"#);
        assert_eq!(
            portal("#modals", p().text("Hi")).render(),
            "<!--[--><!--]-->"
        );
    }

    #[test]
    fn islands_carry_their_hydration_strategy() {
        use crate::{Hydrate, island, island_with};
        fn widget(props: &str) -> Node {
            span().text(props.to_string()).into()
        }
        let html = |when| island_with("w", "1", widget, when).render();
        assert_eq!(html(Hydrate::Load), island("w", "1", widget).render());
        assert!(!html(Hydrate::Load).contains("data-hydrate"));
        assert!(html(Hydrate::Idle).contains(r#" data-hydrate="idle">"#));
        assert!(html(Hydrate::Interaction).contains(r#" data-hydrate="interaction">"#));
        assert!(
            html(Hydrate::Media("(max-width: 50em) and (hover: none)".into()))
                .contains(r#" data-hydrate="media:(max-width: 50em) and (hover: none)">"#)
        );
        assert!(html(Hydrate::Media("\"><x".into())).contains("media:&quot;&gt;&lt;x"));
    }

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
    fn keyed_lists_render_every_item_even_with_repeated_keys() {
        let items = use_state(vec![(1, "a"), (2, "b"), (1, "c")]);
        let view = ol().children_keyed(
            move || items.get(),
            |item| item.0,
            |item| li().text(item.1).into(),
        );
        assert_eq!(
            Node::from(view).render(),
            "<ol><!--[--><li>a</li><li>b</li><li>c</li><!--]--></ol>"
        );
    }

    #[test]
    fn indexed_rows_follow_their_position_in_place() {
        use std::cell::RefCell;
        let items = use_state(vec![1, 2, 3]);
        let rows: Rc<RefCell<Vec<State<i32>>>> = Rc::default();
        let seen = rows.clone();
        let list = ul().children_indexed(
            move || items.get(),
            move |n| {
                seen.borrow_mut().push(n);
                li().text_dyn(move || n.get().to_string()).into()
            },
        );
        assert_eq!(
            Node::from(list).render(),
            "<ul><!--[--><li><!--#-->1</li><li><!--#-->2</li><li><!--#-->3</li><!--]--></ul>"
        );
        let rows = rows.borrow().clone();
        // The second item changes: the second row's state follows, the others stay.
        items.set(vec![1, 20, 3]);
        assert_eq!(rows.iter().map(|r| r.get()).collect::<Vec<_>>(), [1, 20, 3]);
        // Shrinking leaves the remaining rows untouched.
        items.set(vec![5]);
        assert_eq!(rows[0].get(), 5);
    }

    #[test]
    fn a_switch_keeps_its_view_while_the_kind_stays_the_same() {
        use std::cell::{Cell, RefCell};
        let route = use_state((1, "a"));
        let built = Rc::new(Cell::new(0));
        type Route = (i32, &'static str);
        let states: Rc<RefCell<Vec<State<Route>>>> = Rc::default();
        let (count, seen) = (built.clone(), states.clone());
        let view = switch(
            move || route.get(),
            |r| r.0,
            move |r| {
                count.set(count.get() + 1);
                seen.borrow_mut().push(r);
                p().text_dyn(move || r.get().1.to_string()).into()
            },
        );
        assert_eq!(view.render(), "<!--[--><p><!--#-->a</p><!--]-->");
        let first = states.borrow()[0];
        route.set((1, "b")); // same kind: the view's state is updated
        assert_eq!(first.get(), (1, "b"));
        route.set((2, "c")); // another kind: the old view never sees it
        assert_eq!(first.get(), (1, "b"));
        assert_eq!(built.get(), 1);
    }

    #[test]
    fn two_way_bindings_render_the_state_and_follow_it() {
        let name = use_state("Ada & Bob".to_string());
        let on = use_state(false);
        let size = use_state(1.5f64);
        let view = form()
            .on_submit(|_| panic!("never on the server"))
            .child(input().bind_value(name).on_key("Enter", |_| panic!()))
            .child(textarea().bind_value(name))
            .child(input().attr("type", "checkbox").bind_checked(on))
            .child(input().bind_parsed(size));
        let view = Node::from(view);
        assert_eq!(
            view.render(),
            concat!(
                r#"<form><input value="Ada &amp; Bob"/><textarea>Ada &amp; Bob</textarea>"#,
                r#"<input type="checkbox"/><input value="1.5"/></form>"#
            )
        );
        on.set(true);
        size.set(2.0);
        name.set("Eve".into());
        assert_eq!(
            view.render(),
            concat!(
                r#"<form><input value="Eve"/><textarea>Ada &amp; Bob</textarea>"#,
                r#"<input type="checkbox" checked=""/><input value="2"/></form>"#
            )
        );
    }

    #[test]
    fn style_properties_combine_with_the_static_style() {
        let width = use_state(10);
        let color = use_state(String::new());
        let bar = div()
            .style("height: 2px;")
            .style_dyn("width", move || format!("{}px", width.get()))
            .style_dyn("color", move || color.get());
        let bar = Node::from(bar);
        assert_eq!(
            bar.render(),
            r#"<div style="height: 2px; width: 10px"></div>"#
        );
        color.set("red".into());
        width.set(20);
        assert_eq!(
            bar.render(),
            r#"<div style="height: 2px; width: 20px; color: red"></div>"#
        );
        let empty = div().style_dyn("color", String::new);
        assert_eq!(Node::from(empty).render(), "<div></div>");
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
