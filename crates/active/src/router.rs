//! Route patterns: [`match_path`] (`"/posts/:id"`) and [`Routes`], a route table that keeps
//! a page while only its parameters change, like Vue Router.

use std::rc::Rc;
use std::str::FromStr;

use std::cell::RefCell;
use std::future::Future;

use crate::data::Data;
use crate::reactive::{State, untrack, use_state, watch};
use crate::view::{Node, fragment_dyn, switch};

/// The parameters extracted from a path by [`match_path`] or [`Routes`]: `id` in
/// `"/posts/:id"`, `rest` in `"/files/*rest"`. Values are percent-decoded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Params {
    /// The index of the route that matched in its [`Routes`] (`NO_ROUTE` when none did).
    route: usize,
    pairs: Vec<(String, String)>,
}

const NO_ROUTE: usize = usize::MAX;

impl Params {
    /// The value of parameter `name`, or `None` when the pattern has no such parameter (or an
    /// optional one, `:name?`, was absent).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// The value of parameter `name` parsed (`params.parse::<u32>("id")`), or `None` when it
    /// is missing or does not parse.
    pub fn parse<T: FromStr>(&self, name: &str) -> Option<T> {
        self.get(name)?.parse().ok()
    }

    /// The parameters, `(name, value)`, in the order of the pattern.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs.iter().map(|(n, v)| (n.as_str(), v.as_str()))
    }

    /// Whether there is no parameter.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

enum Segment<'a> {
    Static(&'a str),
    Param(&'a str),
    Optional(&'a str),
    Rest(&'a str),
}

fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|s| !s.is_empty())
}

/// Decodes `%XX` escapes (invalid ones are kept as they are).
fn decode(segment: &str) -> String {
    if !segment.contains('%') {
        return segment.to_string();
    }
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn match_segments(pattern: &[Segment<'_>], path: &[&str], out: &mut Vec<(String, String)>) -> bool {
    let Some((first, more)) = pattern.split_first() else {
        return path.is_empty();
    };
    match first {
        Segment::Static(s) => {
            path.first().is_some_and(|p| decode(p) == *s) && match_segments(more, &path[1..], out)
        }
        Segment::Param(name) => {
            let Some(value) = path.first() else {
                return false;
            };
            out.push((name.to_string(), decode(value)));
            if match_segments(more, &path[1..], out) {
                return true;
            }
            out.pop();
            false
        }
        Segment::Optional(name) => {
            if let Some(value) = path.first() {
                let len = out.len();
                out.push((name.to_string(), decode(value)));
                if match_segments(more, &path[1..], out) {
                    return true;
                }
                out.truncate(len);
            }
            match_segments(more, path, out)
        }
        Segment::Rest(name) => {
            if !name.is_empty() {
                let rest: Vec<String> = path.iter().map(|s| decode(s)).collect();
                out.push((name.to_string(), rest.join("/")));
            }
            true
        }
    }
}

/// Matches `path` against `pattern` and returns its parameters, or `None` when it does not
/// match. The query string and the `#fragment` of `path` are ignored, and so are leading,
/// trailing and repeated slashes.
///
/// | Pattern segment | Matches |
/// | --- | --- |
/// | `posts` | exactly `posts` |
/// | `:id` | any one segment, captured as `id` |
/// | `:id?` | one segment captured as `id`, or nothing |
/// | `*rest` (last) | everything left, possibly nothing, captured as `rest` (`*` alone captures nothing) |
///
/// ```
/// use active::match_path;
///
/// let params = match_path("/posts/:id", "/posts/42?tab=comments").unwrap();
/// assert_eq!(params.get("id"), Some("42"));
/// assert_eq!(params.parse::<u32>("id"), Some(42));
///
/// assert_eq!(match_path("/posts/:id", "/posts"), None);
/// assert_eq!(match_path("/docs/:lang?", "/docs").unwrap().get("lang"), None);
/// assert_eq!(match_path("/files/*path", "/files/a/b%20c.txt").unwrap().get("path"), Some("a/b c.txt"));
/// ```
pub fn match_path(pattern: &str, path: &str) -> Option<Params> {
    let path = path.split(['?', '#']).next().unwrap_or("");
    let pattern: Vec<Segment<'_>> = segments(pattern)
        .map(|s| {
            if let Some(name) = s.strip_prefix(':') {
                match name.strip_suffix('?') {
                    Some(name) => Segment::Optional(name),
                    None => Segment::Param(name),
                }
            } else if let Some(name) = s.strip_prefix('*') {
                Segment::Rest(name)
            } else {
                Segment::Static(s)
            }
        })
        .collect();
    let path: Vec<&str> = segments(path).collect();
    let mut pairs = Vec::new();
    match_segments(&pattern, &path, &mut pairs).then_some(Params { route: 0, pairs })
}

type RouteView = Rc<dyn Fn(State<Params>) -> Node>;
type Layout = Rc<dyn Fn(Node) -> Node>;
type Loading = Rc<RefCell<Option<Rc<dyn Fn() -> Node>>>>;

/// The name of the parameter holding the rest of the path of a nested table.
const REST: &str = "__rest";

#[derive(Clone)]
struct Route {
    pattern: String,
    view: RouteView,
    /// Set by [`Routes::nest`]: the table that handles the rest of the path.
    nested: Option<Rc<Routes>>,
}

/// A route table: the first pattern (see [`match_path`]) that matches the path picks the
/// page, built with a [`State<Params>`](Params) holding its parameters.
///
/// [`view`](Routes::view) follows a path that changes, like Vue Router's `<router-view>`:
/// going from `/posts/1` to `/posts/2` **keeps** the post page (its states, requests and
/// DOM) and only updates its parameters state, while going to another route builds the
/// other page. [`render`](Routes::render) builds the page of one path, for a server, and
/// [`respond`](Routes::respond) adds the HTTP status and redirect the page asked for.
///
/// Pages can share a [`layout`](Routes::layout) (kept on screen while the page changes),
/// tables can be [nested](Routes::nest) under a prefix with their own layout, and
/// [`route_with_data`](Routes::route_with_data) loads a page's data before showing it
/// (SvelteKit's `load`, Remix's and Nuxt's loaders).
///
/// ```
/// use active::{html::*, Node, Params, Routes, State};
///
/// fn post(params: State<Params>) -> Node {
///     h1().text_dyn(move || format!("Post {}", params.with(|p| p.get("id").unwrap_or("").to_string()))).into()
/// }
///
/// let routes = Routes::new()
///     .route("/", |_| h1().text("Home").into())
///     .route("/posts/:id", post)
///     .fallback(|params| p().text(format!("No page at /{}", params.with(|p| p.get("path").unwrap_or("").to_string()))).into());
///
/// assert_eq!(routes.render("/posts/7").render(), "<h1><!--#-->Post 7</h1>");
/// assert_eq!(routes.render("/nope").render(), "<p>No page at /nope</p>");
/// ```
///
/// In a client-side app: `main().child(routes.view(move || active::location().get()))`.
#[derive(Clone, Default)]
pub struct Routes {
    table: Vec<Route>,
    fallback: Option<RouteView>,
    layout: Option<Layout>,
    loading: Loading,
}

impl std::fmt::Debug for Routes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Routes")
            .field(
                "patterns",
                &self.table.iter().map(|r| &r.pattern).collect::<Vec<_>>(),
            )
            .field("fallback", &self.fallback.is_some())
            .field("layout", &self.layout.is_some())
            .finish()
    }
}

impl Routes {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a route: `view` builds the page when `pattern` is the first one to match. It is
    /// given the parameters as a state, updated in place while the same route stays matched.
    #[must_use]
    pub fn route(
        mut self,
        pattern: impl Into<String>,
        view: impl Fn(State<Params>) -> Node + 'static,
    ) -> Self {
        self.table.push(Route {
            pattern: pattern.into(),
            view: Rc::new(view),
            nested: None,
        });
        self
    }

    /// The page shown when no route matches (a 404 page). Its parameters hold the whole
    /// path, without the leading slash, as `path`. Without a fallback, nothing is shown.
    #[must_use]
    pub fn fallback(mut self, view: impl Fn(State<Params>) -> Node + 'static) -> Self {
        self.fallback = Some(Rc::new(view));
        self
    }

    /// Wraps every page of the table (the fallback included) in `layout`, given the page
    /// as its `outlet`: Next's and Nuxt's layouts, SvelteKit's `+layout`, Vue Router's
    /// nested `<router-view>`.
    ///
    /// With [`view`](Routes::view), the layout is built **once** and stays on screen while
    /// the visitor goes from page to page: its states (an open menu, a search field, a
    /// player) are kept, only the outlet changes.
    ///
    /// ```
    /// use active::{html::*, Node, Routes};
    ///
    /// let routes = Routes::new()
    ///     .layout(|outlet| div().class("shell").child(nav().text("Menu")).child(main().child(outlet)).into())
    ///     .route("/", |_| h1().text("Home").into());
    /// assert_eq!(
    ///     routes.render("/").render(),
    ///     r#"<div class="shell"><nav>Menu</nav><main><h1>Home</h1></main></div>"#
    /// );
    /// ```
    #[must_use]
    pub fn layout(mut self, layout: impl Fn(Node) -> Node + 'static) -> Self {
        self.layout = Some(Rc::new(layout));
        self
    }

    /// Hands every path under `prefix` (`"/settings"`, `"/users/:id"`) to another table,
    /// whose patterns are relative to the prefix (`"/"`, `"/profile"`) and whose
    /// [`layout`](Routes::layout) goes inside this table's. The parameters of the prefix
    /// (`id`) are given to the nested pages too.
    ///
    /// With [`view`](Routes::view), the nested layout stays on screen while the visitor moves
    /// between the nested pages, and is rebuilt only when they leave the prefix.
    ///
    /// ```
    /// use active::{html::*, Node, Routes};
    ///
    /// let settings = Routes::new()
    ///     .layout(|outlet| section().child(h2().text("Settings")).child(outlet).into())
    ///     .route("/", |_| p().text("Overview").into())
    ///     .route("/profile", |_| p().text("Profile").into());
    /// let routes = Routes::new()
    ///     .route("/", |_| h1().text("Home").into())
    ///     .nest("/settings", settings);
    /// assert_eq!(
    ///     routes.render("/settings/profile").render(),
    ///     "<section><h2>Settings</h2><!--[--><p>Profile</p><!--]--></section>"
    /// );
    /// assert!(routes.matches("/settings/profile"));
    /// assert!(!routes.matches("/settings/nope"));
    /// ```
    #[must_use]
    pub fn nest(mut self, prefix: &str, routes: Routes) -> Self {
        let pattern = format!("{}/*{REST}", prefix.trim_end_matches('/'));
        let nested = Rc::new(routes);
        let inner = nested.clone();
        let view: RouteView = Rc::new(move |params: State<Params>| {
            view_of(inner.clone(), move || {
                params.with(|p| {
                    let rest = format!("/{}", p.get(REST).unwrap_or(""));
                    let inherited = p.pairs.iter().filter(|(k, _)| k != REST).cloned().collect();
                    (rest, inherited)
                })
            })
        });
        self.table.push(Route {
            pattern,
            view,
            nested: Some(nested),
        });
        self
    }

    /// What [`route_with_data`](Routes::route_with_data) shows while a page's data loads
    /// for the first time (a skeleton, a spinner). Nothing by default.
    #[must_use]
    pub fn loading(self, view: impl Fn() -> Node + 'static) -> Self {
        *self.loading.borrow_mut() = Some(Rc::new(view));
        self
    }

    /// Adds a route whose data is loaded before its page is shown: SvelteKit's `load`,
    /// Remix's and Nuxt's loaders.
    ///
    /// `load` is called with the route's parameters and returns a future (an HTTP request, a
    /// database query on the server); the page is built with a state holding its result once
    /// it is there, and shows the [`loading`](Routes::loading) view until then. Going to
    /// the same route with other parameters (`/posts/1` to `/posts/2`) loads again and
    /// updates the state in place, keeping the page.
    ///
    /// The data is loaded with [`ssr::resource`](crate::ssr::resource) under the key
    /// `"route:{pattern}"`: a page rendered by [`ssr::render`](crate::ssr::render) or
    /// streamed by [`ssr::stream`](crate::ssr::stream) sends it along, and the browser
    /// hydrates without loading it again.
    ///
    /// ```
    /// use active::{html::*, Routes};
    ///
    /// async fn load_post(id: u32) -> String {
    ///     format!("Post number {id}") // a database, an API…
    /// }
    ///
    /// let routes = Routes::new()
    ///     .loading(|| p().class("skeleton").into())
    ///     .route_with_data(
    ///         "/posts/:id",
    ///         |params| load_post(params.parse("id").unwrap_or(0)),
    ///         |post, _params| h1().text_dyn(move || post.get()).into(),
    ///     );
    /// assert!(routes.render("/posts/3").render().contains("<h1><!--#-->Post number 3</h1>"));
    /// ```
    #[must_use]
    pub fn route_with_data<T, F>(
        self,
        pattern: impl Into<String>,
        load: impl Fn(Params) -> F + 'static,
        page: impl Fn(State<T>, State<Params>) -> Node + 'static,
    ) -> Self
    where
        T: Data + Clone + PartialEq + 'static,
        F: Future<Output = T> + 'static,
    {
        let pattern = pattern.into();
        let key = format!("route:{pattern}");
        let (load, page, loading) = (Rc::new(load), Rc::new(page), self.loading.clone());
        self.route(pattern, move |params| {
            let fallback = loading.borrow().as_ref().map_or(Node::Empty, |f| f());
            let (load, page, key) = (load.clone(), page.clone(), key.clone());
            crate::resource::suspense(fallback, move || {
                let data = crate::ssr::resource(key, move || params.get(), move |p| load(p));
                // Browser: the page is built once, when the first value arrives.
                let loaded: Rc<dyn Fn() -> bool> = if cfg!(feature = "client") {
                    let loaded = crate::reactive::memo(move || data.with(|d| d.is_some()));
                    Rc::new(move || loaded.get())
                } else {
                    Rc::new(move || data.with(|d| d.is_some()))
                };
                fragment_dyn(move || {
                    if !loaded() {
                        return Vec::new();
                    }
                    untrack(|| {
                        let Some(first) = data.get() else {
                            return Vec::new();
                        };
                        let state = use_state(first);
                        watch(
                            move || data.get(),
                            move |next, _| {
                                if let Some(next) = next {
                                    if state.is_alive() && state.with(|s| s != next) {
                                        state.set(next.clone());
                                    }
                                }
                            },
                        );
                        vec![page(state, params)]
                    })
                })
            })
        })
    }

    /// The parameters of the first route matching `path` (the fallback's when none does), or
    /// `None` when nothing matches and there is no fallback.
    pub fn resolve(&self, path: &str) -> Option<Params> {
        self.table
            .iter()
            .enumerate()
            .find_map(|(route, r)| match_path(&r.pattern, path).map(|p| Params { route, ..p }))
            .or_else(|| {
                self.fallback.as_ref()?;
                let mut params = match_path("/*path", path)?;
                params.route = self.table.len();
                Some(params)
            })
    }

    /// Whether a route (not a fallback) matches `path`, nested tables included: answer 404
    /// when it is `false`.
    pub fn matches(&self, path: &str) -> bool {
        let Some(params) = self.resolve(path) else {
            return false;
        };
        match self.table.get(params.route) {
            Some(Route {
                nested: Some(nested),
                ..
            }) => nested.matches(&format!("/{}", params.get(REST).unwrap_or(""))),
            Some(_) => true,
            None => false,
        }
    }

    fn page(&self, params: State<Params>) -> Node {
        let route = untrack(|| params.with(|p| p.route));
        match self.table.get(route) {
            Some(r) => (r.view)(params),
            None if route == self.table.len() => self
                .fallback
                .as_ref()
                .map_or(Node::Empty, |view| view(params)),
            None => Node::Empty,
        }
    }

    fn wrap(&self, outlet: Node) -> Node {
        match &self.layout {
            Some(layout) => layout(outlet),
            None => outlet,
        }
    }

    /// The page of `path`, built once (for a server, or a static site), in its layout.
    pub fn render(&self, path: &str) -> Node {
        let page = match self.resolve(path) {
            Some(params) => self.page(use_state(params)),
            None => Node::Empty,
        };
        self.wrap(page)
    }

    /// The page of `path` with what a server answers: the HTTP status (200, 404 when no
    /// route matches, or what the page set with [`set_status`]) and the address given to
    /// [`redirect`] while the page was built.
    ///
    /// ```
    /// use active::{html::*, redirect, set_status, Routes};
    ///
    /// let signed_in = false;
    /// let routes = Routes::new()
    ///     .route("/", |_| h1().text("Home").into())
    ///     .route("/account", move |_| {
    ///         if !signed_in {
    ///             redirect("/login");
    ///         }
    ///         h1().text("Account").into()
    ///     })
    ///     .route("/gone", |_| {
    ///         set_status(410);
    ///         p().text("This page was removed").into()
    ///     })
    ///     .fallback(|_| p().text("Not found").into());
    ///
    /// assert_eq!(routes.respond("/").status, 200);
    /// let account = routes.respond("/account");
    /// assert_eq!((account.status, account.location.as_deref()), (302, Some("/login")));
    /// assert_eq!(routes.respond("/gone").status, 410);
    /// assert_eq!(routes.respond("/nope").status, 404);
    /// ```
    pub fn respond(&self, path: &str) -> PageResponse {
        let previous = RESPONSE.with(|r| r.replace(Some(ResponseMeta::default())));
        let body = self.render(path);
        let meta = RESPONSE.with(|r| r.replace(previous)).unwrap_or_default();
        let status = match (meta.status, &meta.location) {
            (Some(status), Some(_)) if (300..400).contains(&status) => status,
            (_, Some(_)) => 302,
            (Some(status), None) => status,
            (None, None) if self.matches(path) => 200,
            (None, None) => 404,
        };
        PageResponse {
            status,
            location: meta.location,
            body,
        }
    }

    /// The page of a path that changes (`move || active::location().get()`): rebuilt when
    /// another route matches, kept with its parameters updated while the same route does,
    /// inside a [`layout`](Routes::layout) built once.
    pub fn view(self, path: impl Fn() -> String + 'static) -> Node {
        view_of(Rc::new(self), move || (path(), Vec::new()))
    }
}

/// [`Routes::view`], the parameters of an enclosing table added to each page's.
fn view_of(
    routes: Rc<Routes>,
    path: impl Fn() -> (String, Vec<(String, String)>) + 'static,
) -> Node {
    let table = routes.clone();
    let pages = switch(
        move || {
            let (path, inherited) = path();
            let mut params = table.resolve(&path).unwrap_or(Params {
                route: NO_ROUTE,
                pairs: Vec::new(),
            });
            for (name, value) in inherited {
                if params.get(&name).is_none() {
                    params.pairs.push((name, value));
                }
            }
            params
        },
        |params| params.route,
        {
            let routes = routes.clone();
            move |params| routes.page(params)
        },
    );
    routes.wrap(pages)
}

/// What [`Routes::respond`] gives a server: the status, the redirect and the page.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PageResponse {
    /// The HTTP status: 200, 404 when no route matches, 302 after [`redirect`] (or the
    /// `3xx` given to [`set_status`]), or what the page set with [`set_status`].
    pub status: u16,
    /// The address given to [`redirect`]: send it as the `Location` header.
    pub location: Option<String>,
    /// The page (put it in a [`Document`](crate::Document)).
    pub body: Node,
}

#[derive(Default)]
struct ResponseMeta {
    status: Option<u16>,
    location: Option<String>,
}

thread_local! {
    /// While [`Routes::respond`] builds a page.
    static RESPONSE: RefCell<Option<ResponseMeta>> = const { RefCell::new(None) };
}

/// Sets the HTTP status of the page being built by [`Routes::respond`] (a 404 for a post
/// that does not exist, a 410, a 503 during maintenance). Elsewhere, and in the browser, it
/// does nothing.
pub fn set_status(status: u16) {
    RESPONSE.with(|r| {
        if let Some(meta) = r.borrow_mut().as_mut() {
            meta.status = Some(status);
        }
    });
}

/// Sends the visitor to `to` instead of the page being built (SvelteKit's `redirect`,
/// Next's `redirect()`): on the server, [`Routes::respond`] answers with a 302 (or the
/// `3xx` given to [`set_status`]) and `to` as the `Location`; in the browser, the router
/// goes to `to`, replacing the history entry, right after the page is built.
pub fn redirect(to: impl Into<String>) {
    let to = to.into();
    #[cfg(feature = "client")]
    {
        wasm_bindgen_futures::spawn_local(async move { crate::client::navigate_replace(&to) });
    }
    #[cfg(not(feature = "client"))]
    RESPONSE.with(|r| {
        if let Some(meta) = r.borrow_mut().as_mut() {
            meta.location = Some(to);
        }
    });
}

/// What a guard registered with [`before_navigate`] decides about a navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation {
    /// Let it happen (the next guard decides in turn).
    Continue,
    /// Stay on the current page.
    Cancel,
    /// Go to this address instead (`"/login?next=/account"`); the guards check it in turn.
    Redirect(String),
}

type Guard = Rc<dyn Fn(&str) -> Navigation>;

thread_local! {
    static GUARDS: std::cell::RefCell<Vec<(u64, Guard)>> = const { std::cell::RefCell::new(Vec::new()) };
    static NEXT_GUARD: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Registers a navigation guard (Vue Router's `beforeEach`, a Next.js middleware run in the
/// browser): before the router goes to another address — a link clicked,
/// [`navigate`](crate::navigate), the back and forward buttons — `guard` gets the address
/// (`"/account?tab=2"`) and lets it happen, cancels it or redirects it.
///
/// Guards run in the order they were registered; a redirect is checked by every guard again
/// (ten redirects in a row cancel the navigation). A guard registered while building a
/// component is removed with it, so a page can hold the visitor while a form has unsaved
/// changes. On the server, where nothing navigates, guards are never called.
///
/// With the back and forward buttons, the browser has already changed the address: a cancel
/// puts the current page's address back (as a new history entry), a redirect replaces the
/// entry.
///
/// ```
/// use active::{before_navigate, use_state, Navigation};
///
/// let signed_in = use_state(false);
/// before_navigate(move |to| {
///     if to.starts_with("/account") && !signed_in.get() {
///         Navigation::Redirect(format!("/login?next={to}"))
///     } else {
///         Navigation::Continue
///     }
/// });
/// ```
pub fn before_navigate(guard: impl Fn(&str) -> Navigation + 'static) {
    let id = NEXT_GUARD.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    GUARDS.with(|g| g.borrow_mut().push((id, Rc::new(guard))));
    crate::reactive::on_cleanup(move || {
        GUARDS.with(|g| g.borrow_mut().retain(|(i, _)| *i != id));
    });
}

/// The address to go to after the guards: `to`, a redirect, or `None` when cancelled.
#[cfg_attr(not(any(feature = "client", test)), allow(dead_code))]
pub(crate) fn check_navigation(to: &str) -> Option<String> {
    let mut to = to.to_string();
    for _ in 0..10 {
        let guards: Vec<Guard> =
            GUARDS.with(|g| g.borrow().iter().map(|(_, g)| g.clone()).collect());
        let mut redirect = None;
        for guard in guards {
            match untrack(|| guard(&to)) {
                Navigation::Continue => {}
                Navigation::Cancel => return None,
                Navigation::Redirect(next) => {
                    redirect = Some(next);
                    break;
                }
            }
        }
        match redirect {
            None => return Some(to),
            Some(next) if next == to => return Some(to),
            Some(next) => to = next,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::*;
    use std::cell::Cell;

    #[test]
    fn patterns_match_static_parameter_optional_and_rest_segments() {
        assert!(match_path("/", "/").is_some());
        assert!(match_path("/", "").is_some());
        assert!(match_path("/about", "/about/").is_some());
        assert!(match_path("/about", "/About").is_none());
        assert!(match_path("/about", "/about/team").is_none());

        let p = match_path("/users/:user/posts/:post", "/users/ada/posts/7#top").unwrap();
        assert_eq!(
            p.iter().collect::<Vec<_>>(),
            [("user", "ada"), ("post", "7")]
        );
        assert_eq!(p.parse::<u8>("post"), Some(7));
        assert_eq!(p.parse::<u8>("user"), None);

        let p = match_path("/:lang?/docs/:page", "/fr/docs/state").unwrap();
        assert_eq!((p.get("lang"), p.get("page")), (Some("fr"), Some("state")));
        let p = match_path("/:lang?/docs/:page", "/docs/state").unwrap();
        assert_eq!((p.get("lang"), p.get("page")), (None, Some("state")));

        assert_eq!(
            match_path("/files/*rest", "/files").unwrap().get("rest"),
            Some("")
        );
        assert!(match_path("/files/*", "/files/x/y").unwrap().is_empty());
        assert_eq!(
            match_path("/tags/:tag", "/tags/caf%C3%A9%2")
                .unwrap()
                .get("tag"),
            Some("café%2")
        );
    }

    #[test]
    fn the_first_matching_route_wins_and_the_fallback_comes_last() {
        let routes = Routes::new()
            .route("/posts/new", |_| p().text("new").into())
            .route("/posts/:id", |id| {
                p().text(untrack(|| id.get()).get("id").unwrap_or("").to_string())
                    .into()
            });
        assert_eq!(routes.render("/posts/new").render(), "<p>new</p>");
        assert_eq!(routes.render("/posts/3").render(), "<p>3</p>");
        assert_eq!(routes.render("/elsewhere").render(), "");
        assert!(routes.matches("/posts/3") && !routes.matches("/x"));
        let routes = routes.fallback(|_| p().text("404").into());
        assert_eq!(routes.render("/a/b").render(), "<p>404</p>");
        assert!(!routes.matches("/a/b"));
        assert_eq!(routes.resolve("/a/b").unwrap().get("path"), Some("a/b"));
    }

    #[test]
    fn a_page_is_kept_while_only_its_parameters_change() {
        let path = use_state("/posts/1".to_string());
        let built = Rc::new(Cell::new(0));
        let given: Rc<Cell<Option<State<Params>>>> = Rc::default();
        let (count, seen) = (built.clone(), given.clone());
        let view = Routes::new()
            .route("/", |_| h1().text("Home").into())
            .route("/posts/:id", move |params| {
                count.set(count.get() + 1);
                seen.set(Some(params));
                h1().text_dyn(move || params.with(|p| p.get("id").unwrap_or("").to_string()))
                    .into()
            })
            .view(move || path.get());
        assert_eq!(view.render(), "<!--[--><h1><!--#-->1</h1><!--]-->");
        let params = given.get().unwrap();
        path.set("/posts/2".into());
        assert_eq!(
            params.get().get("id"),
            Some("2"),
            "same route: parameters updated"
        );
        assert_eq!(built.get(), 1, "same route: the page is kept");
        path.set("/".into());
        assert_eq!(
            params.get().get("id"),
            Some("2"),
            "another route: the old page sees nothing"
        );
        assert_eq!(view.render(), "<!--[--><h1>Home</h1><!--]-->");
        path.set("/unknown".into());
        assert_eq!(view.render(), "<!--[--><!--]-->");
    }

    #[test]
    fn nested_routes_match_the_rest_of_the_path() {
        let settings = |params: State<Params>| {
            let rest = untrack(|| params.get())
                .get("rest")
                .unwrap_or("")
                .to_string();
            Routes::new()
                .route("/", |_| p().text("overview").into())
                .route("/profile", |_| p().text("profile").into())
                .render(&rest)
        };
        let routes = Routes::new().route("/settings/*rest", settings);
        assert_eq!(routes.render("/settings").render(), "<p>overview</p>");
        assert_eq!(
            routes.render("/settings/profile/").render(),
            "<p>profile</p>"
        );
    }

    #[test]
    fn guards_continue_cancel_redirect_and_leave_with_their_component() {
        use crate::reactive::scope;
        assert_eq!(check_navigation("/a").as_deref(), Some("/a"));
        let ((), owner) = scope(|| {
            before_navigate(|to| match to {
                "/private" => Navigation::Redirect("/login".into()),
                "/loop" => Navigation::Redirect("/loop2".into()),
                "/loop2" => Navigation::Redirect("/loop".into()),
                "/stay" => Navigation::Cancel,
                _ => Navigation::Continue,
            });
            before_navigate(|to| {
                if to == "/login" {
                    Navigation::Redirect("/login?from=guard".into())
                } else {
                    Navigation::Continue
                }
            });
        });
        assert_eq!(check_navigation("/").as_deref(), Some("/"));
        assert_eq!(
            check_navigation("/private").as_deref(),
            Some("/login?from=guard")
        );
        assert_eq!(check_navigation("/stay"), None);
        assert_eq!(check_navigation("/loop"), None);
        owner.dispose();
        assert_eq!(check_navigation("/stay").as_deref(), Some("/stay"));
    }

    #[test]
    fn layouts_stay_while_pages_change_and_nested_pages_inherit_parameters() {
        let path = use_state("/users/7".to_string());
        let layouts = Rc::new(Cell::new(0));
        let built = layouts.clone();
        let user = Routes::new()
            .layout(|outlet| section().child(outlet).into())
            .route("/", |params| {
                p().text_dyn(move || {
                    format!(
                        "user {}",
                        params.with(|p| p.get("id").unwrap_or("?").to_string())
                    )
                })
                .into()
            })
            .route("/posts/:post", |params| {
                let get = move |k: &str| params.with(|p| p.get(k).unwrap_or("?").to_string());
                p().text_dyn(move || format!("{} of {}", get("post"), get("id")))
                    .into()
            });
        let view = Routes::new()
            .layout(move |outlet| {
                built.set(built.get() + 1);
                main().child(outlet).into()
            })
            .route("/", |_| h1().text("Home").into())
            .nest("/users/:id", user)
            .view(move || path.get());
        assert_eq!(
            view.render(),
            "<main><!--[--><section><!--[--><p><!--#-->user 7</p><!--]--></section><!--]--></main>"
        );
        path.set("/users/7/posts/3".into());
        assert!(view.render().contains("3 of 7"), "{}", view.render());
        path.set("/".into());
        assert!(view.render().contains("<h1>Home</h1>"));
        assert_eq!(layouts.get(), 1, "the layout is built once");
    }

    #[test]
    fn data_routes_load_before_showing_their_page() {
        let routes = Routes::new()
            .loading(|| p().text("Loading").into())
            .route_with_data(
                "/n/:n",
                |params| {
                    let n: u32 = params.parse("n").unwrap_or(0);
                    async move { n * 2 }
                },
                |double, params| {
                    p().text_dyn(move || {
                        format!(
                            "{} -> {}",
                            params.with(|p| p.get("n").unwrap_or("").to_string()),
                            double.get()
                        )
                    })
                    .into()
                },
            )
            .route_with_data(
                "/never",
                |_| std::future::pending::<u8>(),
                |_, _| p().text("never").into(),
            );
        let html = routes.render("/n/4").render();
        assert!(html.contains("4 -&gt; 8"), "{html}");
        let html = routes.render("/never").render();
        assert!(
            html.contains("Loading") && !html.contains("never"),
            "{html}"
        );
    }

    #[test]
    fn respond_sets_status_and_location() {
        let routes = Routes::new()
            .route("/old", |_| {
                set_status(301);
                redirect("/new");
                Node::Empty
            })
            .route("/temp", |_| {
                redirect("/elsewhere");
                Node::Empty
            });
        let old = routes.respond("/old");
        assert_eq!((old.status, old.location.as_deref()), (301, Some("/new")));
        assert_eq!(routes.respond("/temp").status, 302);
        assert_eq!(routes.respond("/x").status, 404);
        // Outside `respond`: nothing recorded, nothing leaks into the next response.
        set_status(500);
        redirect("/y");
        assert_eq!(routes.respond("/x").location, None);
    }
}
