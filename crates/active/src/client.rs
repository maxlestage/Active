//! Browser side (crate feature `client`): hydrating islands, mounting client-side apps,
//! routing and keeping the `<head>` in sync.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    Document as DomDocument, Element as DomElement, HtmlAnchorElement, HtmlInputElement,
    HtmlSelectElement, HtmlTextAreaElement, Node as DomNode, Text,
};

use crate::event::Event;
use crate::island::IslandFn;
use crate::reactive::{Scope, State, effect, keep_alive, on_cleanup, root_state, scope, untrack};
use crate::seo::Seo;
use crate::view::{Attr, Content, DynNodes, DynText, Element, Keyed, Node, collect_seo};

const SVG: &str = "http://www.w3.org/2000/svg";

/// The `<head>` tags written by [`update_head`]; everything else in the head is left alone.
const MANAGED: &str = concat!(
    r#"meta[name="description"],meta[name="robots"],meta[name^="twitter:"],"#,
    r#"meta[property^="og:"],link[rel="canonical"],link[rel="alternate"][hreflang],"#,
    r#"script[type="application/ld+json"]"#
);

thread_local! {
    /// The SEO of the view being mounted, merged from the root down, when the app manages the
    /// head (`mount`). `None` while hydrating islands: the server wrote the head.
    static HEAD: RefCell<Option<Rc<Seo>>> = const { RefCell::new(None) };
    /// Set by [`mount`]. Called through a pointer so that bundles that only hydrate islands
    /// do not contain the head code.
    static HEAD_UPDATE: Cell<Option<HeadUpdate>> = const { Cell::new(None) };
    /// The router's path and query string, created by its first use.
    static ROUTER: Cell<Option<(State<String>, State<String>)>> = const { Cell::new(None) };
    /// Set by [`fragment_dyn`](crate::fragment_dyn). Called through a pointer so that bundles
    /// without dynamic regions do not contain the focus code.
    static KEEP_FOCUS: Cell<Option<KeepFocus>> = const { Cell::new(None) };
}

type HeadUpdate = fn(&Seo, &[Node], &Cell<bool>) -> Seo;

/// Notes where the focus is in the region between two markers, before it is rebuilt, and
/// returns what gives it back once the new content is in place.
type KeepFocus = fn(&DomNode, &DomNode) -> Option<Box<dyn FnOnce()>>;

/// Lets dynamic regions keep the focus when they are rebuilt (see [`Focus`]).
pub(crate) fn keep_focus_in_regions() {
    KEEP_FOCUS.with(|k| k.set(Some(keep_focus)));
}

fn keep_focus(start: &DomNode, end: &DomNode) -> Option<Box<dyn FnOnce()>> {
    let focus = Focus::find(start, end)?;
    let (start, end) = (start.clone(), end.clone());
    Some(Box::new(move || focus.restore(&start, &end)))
}

#[derive(Debug)]
struct Mismatch(&'static str);

fn window() -> web_sys::Window {
    web_sys::window().expect("active: no window")
}

fn document() -> DomDocument {
    window().document().expect("active: no document")
}

/// Listens to `event` on `target` for the page's lifetime.
fn listen(target: &web_sys::EventTarget, event: &str, f: impl Fn(web_sys::Event) + 'static) {
    let closure = Closure::<dyn Fn(web_sys::Event)>::new(f);
    let _ = target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// Hydrates every island of the page: finds each `<active-island>` rendered by
/// [`island`](crate::island), rebuilds its view with the registered component and attaches
/// events and reactive bindings to the existing HTML (nothing is re-created).
///
/// Call it once from your `#[wasm_bindgen(start)]` function:
///
/// ```ignore
/// #[wasm_bindgen(start)]
/// pub fn start() {
///     active::hydrate(&[("counter", counter), ("search", search)]);
/// }
/// ```
pub fn hydrate(islands: &[(&str, IslandFn)]) {
    let Ok(found) = document().query_selector_all("active-island[data-island]") else {
        return;
    };
    for i in 0..found.length() {
        let Some(root) = found.item(i).and_then(|n| n.dyn_into::<DomElement>().ok()) else {
            continue;
        };
        if root.has_attribute("data-hydrated") {
            continue;
        }
        let name = root.get_attribute("data-island").unwrap_or_default();
        let Some((_, component)) = islands.iter().find(|(n, _)| *n == name) else {
            continue;
        };
        let props = root.get_attribute("data-props").unwrap_or_default();
        hydrate_island(&root, component(&props));
    }
}

fn hydrate_island(root: &DomElement, view: Node) {
    let (result, attempt) = scope(|| hydrate_children(std::slice::from_ref(&view), root));
    if let Err(Mismatch(what)) = result {
        // The HTML does not match the view (props changed, HTML edited…): start over.
        attempt.dispose();
        web_sys::console::warn_1(&format!("active: re-rendering island ({what} mismatch)").into());
        root.set_inner_html("");
        std::mem::forget(scope(|| mount_node(&view, root, None, &document())).1);
    } else {
        std::mem::forget(attempt); // islands live as long as the page
    }
    let _ = root.set_attribute("data-hydrated", "");
}

/// Renders `view` into `parent` from scratch, after the existing children.
///
/// The `Seo` nodes of the view (and of every [`fragment_dyn`](crate::fragment_dyn)
/// re-render below it) are applied to the page's `<head>` with [`update_head`], merged from
/// the root down: put the site defaults in the root view and a `Seo` in each page.
pub fn mount(view: &Node, parent: &DomElement) {
    HEAD_UPDATE.with(|h| h.set(Some(region_head)));
    let mut found = Vec::new();
    collect_seo(view, false, &mut found);
    let head = found.iter().fold(Seo::new(), |acc, seo| acc.merged(seo));
    if !found.is_empty() {
        update_head(&head);
    }
    let previous = HEAD.with(|h| h.replace(Some(Rc::new(head))));
    mount_node(view, parent, None, &document());
    HEAD.with(|h| *h.borrow_mut() = previous);
}

/// Mounts `view` into the first element matching `selector`, replacing its content: the
/// entry point of a client-side app.
///
/// ```ignore
/// #[wasm_bindgen(start)]
/// pub fn start() {
///     active::mount_to("#app", app());
/// }
/// ```
pub fn mount_to(selector: &str, view: impl Into<Node>) {
    match document().query_selector(selector) {
        Ok(Some(parent)) => {
            parent.set_inner_html("");
            mount(&view.into(), &parent);
        }
        _ => web_sys::console::warn_1(&format!("active: no element matches {selector:?}").into()),
    }
}

/// Writes `seo` into the current page's `<head>`: `document.title`, description, robots,
/// canonical, alternates, Open Graph, Twitter, JSON-LD and the extra meta tags it sets. The
/// previous tags of these kinds are removed first, the rest of the head is left alone.
///
/// Called for you by [`mount`]; call it yourself to change the head from an event handler.
pub fn update_head(seo: &Seo) {
    let doc = document();
    let Some(head) = doc.head() else { return };
    if let Ok(old) = head.query_selector_all(MANAGED) {
        for i in 0..old.length() {
            if let Some(tag) = old.item(i).and_then(|n| n.dyn_into::<DomElement>().ok()) {
                tag.remove();
            }
        }
    }
    let Ok(holder) = doc.create_element("div") else {
        return;
    };
    holder.set_inner_html(&seo.to_html());
    let children = holder.children();
    let tags: Vec<DomElement> = (0..children.length())
        .filter_map(|i| children.item(i))
        .collect();
    // No title nor site name: no title, rather than the previous page's.
    let mut title = String::new();
    for tag in tags {
        let name = tag.tag_name().to_ascii_lowercase();
        if name == "title" {
            title = tag.text_content().unwrap_or_default();
            continue;
        }
        if name == "meta" {
            remove_same_meta(&head, &tag);
        }
        let _ = head.append_child(&tag);
    }
    doc.set_title(&title);
}

/// The head of a dynamic region: its `Seo` nodes merged over its ancestors' (`parent`),
/// written to the page when it has some, or when the previous content had some (`applied`).
fn region_head(parent: &Seo, nodes: &[Node], applied: &Cell<bool>) -> Seo {
    let mut found = Vec::new();
    nodes.iter().for_each(|n| collect_seo(n, false, &mut found));
    let head = found.iter().fold(parent.clone(), |acc, s| acc.merged(s));
    if !found.is_empty() || applied.get() {
        update_head(&head);
    }
    applied.set(!found.is_empty());
    head
}

/// Removes the `<meta>` of `head` that `tag` replaces (same `name` or `property`).
fn remove_same_meta(head: &DomElement, tag: &DomElement) {
    let Ok(metas) = head.query_selector_all("meta") else {
        return;
    };
    for attr in ["name", "property"] {
        let Some(key) = tag.get_attribute(attr) else {
            continue;
        };
        for i in 0..metas.length() {
            if let Some(meta) = metas.item(i).and_then(|n| n.dyn_into::<DomElement>().ok()) {
                if meta.get_attribute(attr).as_deref() == Some(key.as_str()) {
                    meta.remove();
                }
            }
        }
    }
}

/// The path of the current page (`"/"`, `"/docs/state"`), as a [`State`]: read it in a
/// [`children_dyn`](Element::children_dyn) to show the matching page.
///
/// The first call starts the router: the back and forward buttons update the path, and
/// clicks on links to the same site call [`navigate`] instead of loading a new page. Links
/// with a `target`, a `download` attribute or `rel="external"` (a page the server renders
/// itself, a file), or clicked with a modifier key, are left to the browser.
///
/// ```ignore
/// fn app() -> Node {
///     let path = active::location();
///     div()
///         .child(nav().child(a().href("/").text("Home")).child(a().href("/about").text("About")))
///         .child(main().children_dyn(move || match path.get().as_str() {
///             "/" => vec![home()],
///             "/about" => vec![about()],
///             _ => vec![not_found()],
///         }))
///         .into()
/// }
/// ```
pub fn location() -> State<String> {
    router().0
}

/// The query string of the current page, without the `?` (`"q=rust&page=2"`), as a
/// [`State`] updated by the router like [`location`].
pub fn query() -> State<String> {
    router().1
}

/// One parameter of the query string, decoded (`query_param("q")` on `/search?q=caf%C3%A9`
/// gives `Some("café")`). Reading it in a view or an effect subscribes to [`query`].
pub fn query_param(name: &str) -> Option<String> {
    let query = query().get();
    web_sys::UrlSearchParams::new_with_str(&query)
        .ok()?
        .get(name)
}

fn router() -> (State<String>, State<String>) {
    if let Some(router) = ROUTER.with(Cell::get) {
        return router;
    }
    let (path, search) = current_url();
    let router = (root_state(path), root_state(search));
    ROUTER.with(|r| r.set(Some(router)));
    listen(&window(), "popstate", |_| sync_router());
    listen(&document(), "click", follow_link);
    router
}

/// Copies the browser's URL into the router's states (only what changed, so only the views
/// that depend on it re-render).
fn sync_router() {
    let Some((path_state, search_state)) = ROUTER.with(Cell::get) else {
        return;
    };
    let (path, search) = current_url();
    if path_state.with(|p| *p != path) {
        path_state.set(path);
    }
    if search_state.with(|s| *s != search) {
        search_state.set(search);
    }
}

/// Goes to `to` (`"/about"`, `"/search?q=rust"`, `"/docs#install"`) without reloading the
/// page: adds a browser history entry, updates [`location`] and [`query`], then scrolls to
/// the top, or to the element whose id is the `#fragment`.
pub fn navigate(to: &str) {
    let window = window();
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(to));
    }
    sync_router();
    match to.split_once('#') {
        Some((_, id)) if !id.is_empty() => {
            if let Some(target) = document().get_element_by_id(id) {
                target.scroll_into_view();
            }
        }
        _ => window.scroll_to_with_x_and_y(0.0, 0.0),
    }
}

/// The path and the query string (without `?`) of the browser's URL.
fn current_url() -> (String, String) {
    let location = window().location();
    let path = location.pathname().unwrap_or_else(|_| "/".into());
    let search = location.search().unwrap_or_default();
    let search = search.strip_prefix('?').unwrap_or(&search).to_string();
    (path, search)
}

/// Document click listener installed by [`location`].
fn follow_link(event: web_sys::Event) {
    let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() else {
        return;
    };
    if event.default_prevented()
        || mouse.button() != 0
        || mouse.meta_key()
        || mouse.ctrl_key()
        || mouse.shift_key()
        || mouse.alt_key()
    {
        return;
    }
    let Some(link) = event
        .target()
        .and_then(|t| t.dyn_into::<DomElement>().ok())
        .and_then(|t| t.closest("a[href]").ok().flatten())
        .and_then(|a| a.dyn_into::<HtmlAnchorElement>().ok())
    else {
        return;
    };
    let target = link.target();
    let external = link.rel().split_ascii_whitespace().any(|r| r == "external");
    if (!target.is_empty() && target != "_self") || link.has_attribute("download") || external {
        return;
    }
    let here = window().location();
    if link.origin() != here.origin().unwrap_or_default() {
        return;
    }
    let (path, search, hash) = (link.pathname(), link.search(), link.hash());
    let same_page =
        path == here.pathname().unwrap_or_default() && search == here.search().unwrap_or_default();
    if same_page && !hash.is_empty() {
        return; // an anchor on this page: the browser scrolls
    }
    event.prevent_default();
    navigate(&format!("{path}{search}{hash}"));
}

fn skip_separators(cursor: &mut Option<DomNode>) {
    while let Some(node) = cursor {
        if node.node_type() == DomNode::COMMENT_NODE
            && node.text_content().unwrap_or_default().is_empty()
        {
            *cursor = node.next_sibling();
        } else {
            break;
        }
    }
}

fn hydrate_children(nodes: &[Node], parent: &DomElement) -> Result<(), Mismatch> {
    let mut cursor = parent.first_child();
    for node in nodes {
        hydrate_node(node, parent, &mut cursor)?;
    }
    Ok(())
}

fn hydrate_node(
    node: &Node,
    parent: &DomElement,
    cursor: &mut Option<DomNode>,
) -> Result<(), Mismatch> {
    match node {
        Node::Empty | Node::Seo(_) => Ok(()),
        Node::Fragment(nodes) => nodes
            .iter()
            .try_for_each(|n| hydrate_node(n, parent, cursor)),
        Node::Text(text) => {
            if text.is_empty() {
                return Ok(());
            }
            skip_separators(cursor);
            match cursor.take() {
                Some(n) if n.node_type() == DomNode::TEXT_NODE => {
                    *cursor = n.next_sibling();
                    Ok(())
                }
                _ => Err(Mismatch("text")),
            }
        }
        Node::DynText(f) => {
            skip_separators(cursor);
            let marker = cursor.take().filter(|n| {
                n.node_type() == DomNode::COMMENT_NODE && n.text_content().as_deref() == Some("#")
            });
            let marker = marker.ok_or(Mismatch("dynamic text"))?;
            let next = marker.next_sibling();
            let text = match next {
                Some(n) if n.node_type() == DomNode::TEXT_NODE => {
                    *cursor = n.next_sibling();
                    n.unchecked_into::<Text>()
                }
                other => {
                    // Empty text was rendered as nothing: create the node.
                    let t = document().create_text_node("");
                    let _ = parent.insert_before(&t, other.as_ref());
                    *cursor = other;
                    t
                }
            };
            bind_text(text, f.clone());
            Ok(())
        }
        Node::Element(e) => {
            skip_separators(cursor);
            let dom = take_element(cursor, e.tag)?;
            bind_element(e, &dom, true)
        }
        Node::Dynamic(f) => {
            let (start, end) = take_region(cursor)?;
            // The server's nodes are replaced by freshly mounted ones on the first run.
            bind_dynamic(parent.clone(), start, end, f.clone());
            Ok(())
        }
        Node::Keyed(list) => {
            let (start, end) = take_region(cursor)?;
            bind_list(parent.clone(), start, end, list.clone());
            Ok(())
        }
        Node::Island(island) => {
            skip_separators(cursor);
            let dom = take_element(cursor, "active-island")?;
            hydrate_children(std::slice::from_ref(&island.view), &dom)?;
            let _ = dom.set_attribute("data-hydrated", "");
            Ok(())
        }
    }
}

/// The `<!--[-->` … `<!--]-->` markers of a dynamic region rendered by the server.
fn take_region(cursor: &mut Option<DomNode>) -> Result<(DomNode, DomNode), Mismatch> {
    skip_separators(cursor);
    let start = cursor
        .take()
        .filter(|n| is_marker(n, "["))
        .ok_or(Mismatch("dynamic nodes"))?;
    // Up to the matching end marker (dynamic regions can be nested).
    let mut depth = 0;
    let mut next = start.next_sibling();
    let end = loop {
        let n = next.ok_or(Mismatch("dynamic nodes end"))?;
        if is_marker(&n, "[") {
            depth += 1;
        } else if is_marker(&n, "]") {
            if depth == 0 {
                break n;
            }
            depth -= 1;
        }
        next = n.next_sibling();
    };
    *cursor = end.next_sibling();
    Ok((start, end))
}

fn is_marker(node: &DomNode, text: &str) -> bool {
    node.node_type() == DomNode::COMMENT_NODE && node.text_content().as_deref() == Some(text)
}

fn take_element(cursor: &mut Option<DomNode>, tag: &str) -> Result<DomElement, Mismatch> {
    let node = cursor.take().ok_or(Mismatch("missing element"))?;
    *cursor = node.next_sibling();
    let dom = node
        .dyn_into::<DomElement>()
        .map_err(|_| Mismatch("element"))?;
    if dom.tag_name().eq_ignore_ascii_case(tag) {
        Ok(dom)
    } else {
        Err(Mismatch("tag"))
    }
}

fn bind_text(text: Text, f: DynText) {
    effect(move || text.set_data(&f()));
}

/// Attaches reactive attributes, classes, events and content to `dom`. With `hydrating`,
/// children already exist and are claimed; otherwise they are created.
fn bind_element(e: &Element, dom: &DomElement, hydrating: bool) -> Result<(), Mismatch> {
    if let Some(live) = &e.live {
        for (class, on) in &live.class_toggles {
            let (list, class, on) = (dom.class_list(), class.to_string(), on.clone());
            effect(move || {
                let _ = list.toggle_with_force(&class, on());
            });
        }
        for (name, handler) in &live.events {
            let handler = handler.clone();
            let closure = Closure::<dyn Fn(web_sys::Event)>::new(move |raw| handler(Event { raw }));
            let _ = dom.add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
            keep_alive(Box::new(Listener {
                target: dom.clone().into(),
                event: name,
                closure,
            }));
        }
    }
    match &e.content {
        Content::Text(text) => {
            if !hydrating && !text.is_empty() {
                dom.set_text_content(Some(text));
            }
        }
        Content::Children(children) => {
            if hydrating {
                hydrate_children(children, dom)?;
            } else {
                let doc = document();
                for child in children {
                    mount_node(child, dom, None, &doc);
                }
            }
        }
        Content::Html(html) => {
            if !hydrating {
                dom.set_inner_html(html);
            }
        }
        Content::DynHtml(f) => {
            let (dom, f) = (dom.clone(), f.clone());
            effect(move || dom.set_inner_html(&f()));
        }
    }
    // After the content: a `<select>` can only take a value once its options exist.
    for (name, value) in &e.attrs {
        if let Attr::Dyn(f) = value {
            let (dom, name, f) = (dom.clone(), name.to_string(), f.clone());
            effect(move || {
                let value = f();
                let _ = match &value {
                    Some(v) => dom.set_attribute(&name, v),
                    None => dom.remove_attribute(&name),
                };
                sync_property(&dom, &name, value.as_deref());
            });
        }
    }
    if let Some(live) = &e.live {
        for mounted in &live.mounts {
            mounted(dom);
        }
    }
    Ok(())
}

/// An event listener of an element, kept as long as the scope that created it.
///
/// Released, it is removed from the element before its closure is freed. A region can be
/// rebuilt while an event is still being dispatched (a click that closes a dialog, bubbling up
/// through the elements that were just replaced): the DOM skips listeners removed during a
/// dispatch, so the event never reaches a closure that no longer exists.
struct Listener {
    target: web_sys::EventTarget,
    event: &'static str,
    closure: Closure<dyn Fn(web_sys::Event)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.closure.as_ref().unchecked_ref());
    }
}

/// A form field shows its `value` / `checked` *property*, which stops following the
/// attribute as soon as the user types or clicks: keep the property in sync too.
fn sync_property(dom: &DomElement, name: &str, value: Option<&str>) {
    match name {
        "value" => {
            let value = value.unwrap_or("");
            if let Some(input) = dom.dyn_ref::<HtmlInputElement>() {
                if input.value() != value {
                    input.set_value(value);
                }
            } else if let Some(area) = dom.dyn_ref::<HtmlTextAreaElement>() {
                if area.value() != value {
                    area.set_value(value);
                }
            } else if let Some(select) = dom.dyn_ref::<HtmlSelectElement>() {
                if select.value() != value {
                    select.set_value(value);
                }
            }
        }
        "checked" => {
            if let Some(input) = dom.dyn_ref::<HtmlInputElement>() {
                input.set_checked(value.is_some());
            }
        }
        _ => {}
    }
}

/// Rebuilds the nodes between `start` and `end` whenever a state read by `f` changes;
/// everything created for the previous nodes (states, effects, listeners) is released first.
///
/// In a mounted app, the `Seo` nodes of the new nodes are merged over those of the
/// ancestors and written to the `<head>` (a router showing another page changes the title).
fn bind_dynamic(parent: DomElement, start: DomNode, end: DomNode, f: DynNodes) {
    let current: Rc<RefCell<Option<Scope>>> = Rc::default();
    // When the component that contains this region is released, what the region built is
    // released too: its cleanups run, its effects stop and its states are freed.
    {
        let current = current.clone();
        on_cleanup(move || {
            let content = current.try_borrow_mut().ok().and_then(|mut c| c.take());
            if let Some(content) = content {
                content.dispose();
            }
        });
    }
    let parent_head = HEAD.with(|h| h.borrow().clone());
    let applied = Cell::new(false);
    effect(move || {
        if let Some(previous) = current.borrow_mut().take() {
            previous.dispose();
        }
        let ((), next) = scope(|| {
            let nodes = f();
            // Only `f` decides when the region re-renders: what mounting reads (an
            // `on_mount` callback, a nested component) must not subscribe it.
            untrack(|| {
                let head = parent_head.as_ref().and_then(|parent| {
                    // Opaque to the optimizer, which would otherwise call `region_head`
                    // directly and keep the head code in every bundle.
                    let update = std::hint::black_box(HEAD_UPDATE.with(Cell::get))?;
                    Some(Rc::new(update(parent, &nodes, &applied)))
                });
                // Removing the focused element sends the focus to the page: noted first, it
                // goes back to the same element of the new content.
                let keep = std::hint::black_box(KEEP_FOCUS.with(Cell::get));
                let restore_focus = keep.and_then(|keep| keep(&start, &end));
                while let Some(old) = start.next_sibling() {
                    if old.is_same_node(Some(&end)) {
                        break;
                    }
                    let _ = parent.remove_child(&old);
                }
                let previous = HEAD.with(|h| h.replace(head));
                let doc = document();
                for node in &nodes {
                    mount_node(node, &parent, Some(&end), &doc);
                }
                HEAD.with(|h| *h.borrow_mut() = previous);
                if let Some(restore) = restore_focus {
                    restore();
                }
            });
        });
        *current.borrow_mut() = Some(next);
    });
}

/// Where the focus was in a dynamic region about to be rebuilt.
///
/// The new content gets it back on the element that takes the old one's place: the element
/// with the same `id`, or else the one at the same position whose tag and classes are those of
/// the old element and of each of its ancestors in the region. Content that shows something
/// else (another page, another view) does not match, and the focus stays on the page.
struct Focus {
    id: String,
    /// From the region down to the element: index among the sibling elements, tag, classes.
    path: Vec<(u32, String, String)>,
    /// A text field's selection, kept if the new field holds the same text.
    selection: Option<(u32, u32, String)>,
}

impl Focus {
    /// The focus, if it is inside the region between `start` and `end`.
    fn find(start: &DomNode, end: &DomNode) -> Option<Focus> {
        let focused = document().active_element()?;
        let top = region_elements(start, end);
        if !top.iter().any(|t| t.contains(Some(&focused))) {
            return None;
        }
        // From the element up to the region's top level.
        let mut path = Vec::new();
        let mut el = focused.clone();
        loop {
            let (tag, class) = (el.tag_name(), el.get_attribute("class").unwrap_or_default());
            if let Some(i) = top.iter().position(|t| t.is_same_node(Some(&el))) {
                path.push((i as u32, tag, class));
                break;
            }
            let parent = el.parent_element()?;
            let siblings = parent.children();
            let i = (0..siblings.length())
                .find(|&i| siblings.item(i).is_some_and(|s| s.is_same_node(Some(&el))))?;
            path.push((i, tag, class));
            el = parent;
        }
        path.reverse();
        Some(Focus {
            id: focused.id(),
            path,
            selection: text_selection(&focused),
        })
    }

    /// Focuses the matching element of the new content, without scrolling.
    fn restore(&self, start: &DomNode, end: &DomNode) {
        let top = region_elements(start, end);
        let by_id = (!self.id.is_empty())
            .then(|| document().get_element_by_id(&self.id))
            .flatten()
            .filter(|el| top.iter().any(|t| t.contains(Some(el))));
        let Some(target) = by_id.or_else(|| self.follow(&top)) else {
            return;
        };
        if let Some(html) = target.dyn_ref::<web_sys::HtmlElement>() {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = html.focus_with_options(&options);
        }
        if let Some((from, to, text)) = &self.selection {
            if text_selection(&target).is_some_and(|(_, _, now)| now == *text) {
                if let Some(input) = target.dyn_ref::<HtmlInputElement>() {
                    let _ = input.set_selection_range(*from, *to);
                } else if let Some(area) = target.dyn_ref::<HtmlTextAreaElement>() {
                    let _ = area.set_selection_range(*from, *to);
                }
            }
        }
    }

    /// The element at the same position, if every step has the same tag and classes.
    fn follow(&self, top: &[DomElement]) -> Option<DomElement> {
        let same = |el: &DomElement, tag: &str, class: &str| {
            el.tag_name() == tag && el.get_attribute("class").unwrap_or_default() == class
        };
        let mut steps = self.path.iter();
        let (i, tag, class) = steps.next()?;
        let mut el = top
            .get(*i as usize)
            .filter(|el| same(el, tag, class))?
            .clone();
        for (i, tag, class) in steps {
            el = el.children().item(*i).filter(|el| same(el, tag, class))?;
        }
        Some(el)
    }
}

/// The top-level elements between the markers of a region.
fn region_elements(start: &DomNode, end: &DomNode) -> Vec<DomElement> {
    let mut elements = Vec::new();
    let mut node = start.next_sibling();
    while let Some(n) = node {
        if n.is_same_node(Some(end)) {
            break;
        }
        node = n.next_sibling();
        if let Ok(el) = n.dyn_into::<DomElement>() {
            elements.push(el);
        }
    }
    elements
}

/// The selection and text of a text field (`None` for other elements, and for fields without
/// a selection, such as checkboxes).
fn text_selection(el: &DomElement) -> Option<(u32, u32, String)> {
    if let Some(input) = el.dyn_ref::<HtmlInputElement>() {
        let (Ok(Some(from)), Ok(Some(to))) = (input.selection_start(), input.selection_end())
        else {
            return None;
        };
        return Some((from, to, input.value()));
    }
    let area = el.dyn_ref::<HtmlTextAreaElement>()?;
    let (Ok(Some(from)), Ok(Some(to))) = (area.selection_start(), area.selection_end()) else {
        return None;
    };
    Some((from, to, area.value()))
}

/// One row of a keyed list: its item, the first and last of its top-level nodes (nodes in
/// between may change: they are read again when the row moves) and what it created.
struct Row {
    key: u64,
    item: Rc<dyn std::any::Any>,
    range: Option<(DomNode, DomNode)>,
    scope: Scope,
}

impl Row {
    fn nodes(&self) -> Vec<DomNode> {
        let Some((first, last)) = &self.range else {
            return Vec::new();
        };
        let mut nodes = vec![first.clone()];
        let mut node = first.clone();
        while !node.is_same_node(Some(last)) {
            match node.next_sibling() {
                Some(next) => {
                    nodes.push(next.clone());
                    node = next;
                }
                None => break,
            }
        }
        nodes
    }

    fn release(self, parent: &DomElement) {
        for node in self.nodes() {
            let _ = parent.remove_child(&node);
        }
        self.scope.dispose();
    }
}

pub(crate) type BindKeyed = fn(DomElement, DomNode, DomNode, Rc<Keyed>);

/// Calls the list's own binder: bundles that never call `keyed` do not contain it.
fn bind_list(parent: DomElement, start: DomNode, end: DomNode, list: Rc<Keyed>) {
    let bind = std::hint::black_box(list.bind);
    bind(parent, start, end, list);
}

/// Keeps the rows of `list` between `start` and `end` in sync with its items: unchanged
/// rows keep their DOM and move if needed, new or changed rows are built, removed ones are
/// released. On the first run (hydration), the server's nodes are replaced.
pub(crate) fn bind_keyed(parent: DomElement, start: DomNode, end: DomNode, list: Rc<Keyed>) {
    let rows: Rc<RefCell<Vec<Row>>> = Rc::default();
    // Released with the component that contains the list, like a dynamic region.
    {
        let rows = rows.clone();
        on_cleanup(move || {
            let released = rows
                .try_borrow_mut()
                .map(|mut r| std::mem::take(&mut *r))
                .unwrap_or_default();
            for row in released {
                row.scope.dispose();
            }
        });
    }
    let first_run = Cell::new(true);
    effect(move || {
        let items = (list.items)();
        if first_run.replace(false) {
            while let Some(old) = start.next_sibling() {
                if old.is_same_node(Some(&end)) {
                    break;
                }
                let _ = parent.remove_child(&old);
            }
        }
        let mut old: Vec<Option<Row>> = rows.borrow_mut().drain(..).map(Some).collect();
        // Lists mostly keep their order: look first where the previous row was found. Rows
        // with the same key are matched in order.
        let mut hint = 0;
        let mut take = |key: u64| {
            let n = old.len();
            let start = hint;
            let i = (0..n)
                .map(|offset| (start + offset) % n)
                .find(|&i| old[i].as_ref().is_some_and(|r| r.key == key))?;
            hint = i + 1;
            old[i].take()
        };
        let doc = document();
        let mut next = Vec::with_capacity(items.len());
        for (key, item) in items {
            let row = match take(key) {
                Some(row) if (list.same)(&*row.item, &*item) => row,
                changed => {
                    if let Some(row) = changed {
                        row.release(&parent);
                    }
                    // Built at the end, then put in place below.
                    let before = end.previous_sibling();
                    let ((), scope) = untrack(|| {
                        scope(|| mount_node(&(list.row)(&*item), &parent, Some(&end), &doc))
                    });
                    let first = before
                        .and_then(|b| b.next_sibling())
                        .or_else(|| start.next_sibling())
                        .filter(|n| !n.is_same_node(Some(&end)));
                    let range = first.zip(end.previous_sibling());
                    Row {
                        key,
                        item,
                        range,
                        scope,
                    }
                }
            };
            next.push(row);
        }
        for row in old.into_iter().flatten() {
            row.release(&parent);
        }
        // Rows already in place are skipped, the others are moved before the cursor.
        let mut cursor = start.next_sibling();
        for row in &next {
            let Some((first, last)) = &row.range else {
                continue;
            };
            if cursor.as_ref().is_some_and(|c| c.is_same_node(Some(first))) {
                cursor = last.next_sibling();
            } else {
                for node in row.nodes() {
                    let _ = parent.insert_before(&node, cursor.as_ref());
                }
            }
        }
        *rows.borrow_mut() = next;
    });
}

/// Creates the DOM for `node` and inserts it in `parent`, before `before` (at the end when
/// `None`).
fn mount_node(node: &Node, parent: &DomElement, before: Option<&DomNode>, doc: &DomDocument) {
    let insert = |child: &DomNode| {
        let _ = parent.insert_before(child, before);
    };
    match node {
        Node::Empty | Node::Seo(_) => {}
        Node::Text(text) => {
            if !text.is_empty() {
                insert(&doc.create_text_node(text));
            }
        }
        Node::DynText(f) => {
            let text = doc.create_text_node("");
            insert(&text);
            bind_text(text, f.clone());
        }
        Node::Fragment(nodes) => nodes
            .iter()
            .for_each(|n| mount_node(n, parent, before, doc)),
        Node::Dynamic(f) => {
            let (start, end) = (doc.create_comment("["), doc.create_comment("]"));
            insert(&start);
            insert(&end);
            bind_dynamic(parent.clone(), start.into(), end.into(), f.clone());
        }
        Node::Keyed(list) => {
            let (start, end) = (doc.create_comment("["), doc.create_comment("]"));
            insert(&start);
            insert(&end);
            bind_list(parent.clone(), start.into(), end.into(), list.clone());
        }
        Node::Element(e) => {
            let svg = e.tag == "svg" || parent.namespace_uri().as_deref() == Some(SVG);
            let created = if svg {
                doc.create_element_ns(Some(SVG), e.tag)
            } else {
                doc.create_element(e.tag)
            };
            let Ok(dom) = created else { return };
            for class in &e.classes {
                for c in class.split_whitespace() {
                    let _ = dom.class_list().add_1(c);
                }
            }
            for (name, value) in &e.attrs {
                if let Attr::Static(v) = value {
                    let _ = dom.set_attribute(name, v);
                }
            }
            insert(&dom);
            let _ = bind_element(e, &dom, false);
        }
        Node::Island(island) => {
            if let Ok(dom) = doc.create_element("active-island") {
                let _ = dom.set_attribute("data-island", island.name);
                let _ = dom.set_attribute("data-props", &island.props);
                let _ = dom.set_attribute("data-hydrated", "");
                insert(&dom);
                mount_node(&island.view, &dom, None, doc);
            }
        }
    }
}
