//! A query cache, the best of TanStack Query (React Query) and SWR: [`use_query`] loads data
//! under a key and shares it with every component that asks for the same key, [`invalidate`]
//! reloads it after a change, [`use_mutation`] sends changes, and [`set_query_data`] makes
//! optimistic updates.
//!
//! - **Shared and deduplicated**: ten components asking for `"users/7"` make one request and
//!   show the same value.
//! - **Stale-while-revalidate**: data already in the cache is shown at once, and reloaded in
//!   the background when it is older than its `stale_time` (0 by default: on every mount).
//! - **Invalidation by prefix**: after saving a user, `invalidate("users")` reloads
//!   `"users"`, `"users/7"` and `"users?page=2"` wherever they are shown.
//! - **Safe on the server**: each server render gets its own cache, so one visitor's data
//!   never reaches another; [`QueryClient`] can be [`provide`](crate::provide)d to share one
//!   cache across a page's components. In the browser, the page has one cache.
//!
//! Keys are strings: write them like paths (`"users"`, `"users/7"`, `"users/7/posts"`), so
//! that a prefix names a family.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::context::use_context;
use crate::reactive::{State, batch, effect, on_cleanup, root_state, untrack, use_state, watch};
use crate::resource::{Pending, Task, spawn};

/// A cache of queries. The browser has one for the whole page; on the server, provide one
/// around a page (`provide(QueryClient::new(), || page())`) for its components to share
/// their data during that render.
#[derive(Clone, Default)]
pub struct QueryClient(Rc<RefCell<HashMap<(String, TypeId), Slot>>>);

impl std::fmt::Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryClient")
            .field("queries", &self.0.borrow().len())
            .finish()
    }
}

/// A cache entry, seen without its type.
#[derive(Clone)]
struct Slot {
    entry: Rc<dyn Any>,
    users: Rc<Cell<usize>>,
    /// Marks the entry stale, and reloads it if a component shows it.
    invalidate: Rc<dyn Fn()>,
}

/// Starts a request for an entry's key.
type Fetcher<T> = Rc<dyn Fn() -> Task<T>>;

struct Entry<T> {
    data: State<Option<T>>,
    fetching: State<bool>,
    /// When the data arrived, in milliseconds (`None`: never, or invalidated).
    updated: Cell<Option<f64>>,
    /// Only the latest request may write its result.
    generation: Cell<u64>,
    fetch: RefCell<Option<Fetcher<T>>>,
    users: Rc<Cell<usize>>,
}

impl QueryClient {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    fn entry<T: 'static>(&self, key: &str) -> Rc<Entry<T>> {
        let id = (key.to_owned(), TypeId::of::<T>());
        if let Some(slot) = self.0.borrow().get(&id) {
            return slot.entry.clone().downcast().expect("keyed by type");
        }
        let users = Rc::new(Cell::new(0));
        let entry = Rc::new(Entry::<T> {
            data: root_state(None),
            fetching: root_state(false),
            updated: Cell::new(None),
            generation: Cell::new(0),
            fetch: RefCell::new(None),
            users: users.clone(),
        });
        let weak = Rc::downgrade(&entry);
        let invalidate: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(e) = weak.upgrade() {
                e.updated.set(None);
                if e.users.get() > 0 {
                    start(&e);
                }
            }
        });
        self.0.borrow_mut().insert(
            id,
            Slot {
                entry: entry.clone(),
                users,
                invalidate,
            },
        );
        entry
    }

    /// Reloads every query whose key starts with `prefix` and that a component shows; the
    /// others are only marked stale, and reload when next shown. `""` invalidates everything.
    pub fn invalidate(&self, prefix: &str) {
        let matching: Vec<Rc<dyn Fn()>> = self
            .0
            .borrow()
            .iter()
            .filter(|((key, _), _)| key.starts_with(prefix))
            .map(|(_, slot)| slot.invalidate.clone())
            .collect();
        batch(|| matching.iter().for_each(|f| f()));
    }

    /// Replaces the data of `key` right away (an optimistic update), as if it had just loaded.
    pub fn set_query_data<T: 'static>(&self, key: &str, value: T) {
        let entry = self.entry::<T>(key);
        entry.generation.set(entry.generation.get() + 1);
        entry.updated.set(Some(now()));
        batch(|| {
            entry.data.set(Some(value));
            entry.fetching.set(false);
        });
    }

    /// The data of `key` in the cache, if any (a clone; it does not subscribe).
    pub fn get_query_data<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let slot = self
            .0
            .borrow()
            .get(&(key.to_owned(), TypeId::of::<T>()))
            .cloned()?;
        let entry: Rc<Entry<T>> = slot.entry.downcast().ok()?;
        untrack(|| entry.data.get())
    }

    /// Forgets the queries that no component shows (their data reloads when next shown).
    pub fn clear_unused(&self) {
        self.0.borrow_mut().retain(|_, slot| slot.users.get() > 0);
    }
}

/// The cache in use here: the provided [`QueryClient`], else the page's (browser), else a
/// fresh one (server, so that visitors never share data).
fn current_client() -> QueryClient {
    if let Some(client) = use_context::<QueryClient>() {
        return client;
    }
    #[cfg(feature = "client")]
    {
        thread_local! {
            static PAGE: QueryClient = QueryClient::new();
        }
        PAGE.with(Clone::clone)
    }
    #[cfg(not(feature = "client"))]
    QueryClient::new()
}

fn now() -> f64 {
    #[cfg(feature = "client")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(feature = "client"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
    }
}

/// Starts a request for `entry` (the previous one, if still running, is ignored).
fn start<T: 'static>(entry: &Rc<Entry<T>>) {
    let Some(fetch) = entry.fetch.borrow().clone() else {
        return;
    };
    let id = entry.generation.get() + 1;
    entry.generation.set(id);
    let mut task = fetch();
    let finish = {
        let entry = entry.clone();
        move |value: T| {
            if entry.generation.get() != id {
                return;
            }
            entry.updated.set(Some(now()));
            batch(|| {
                entry.data.set(Some(value));
                entry.fetching.set(false);
            });
        }
    };
    match task.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(value) => finish(value),
        Poll::Pending => {
            untrack(|| {
                if !entry.fetching.get() {
                    entry.fetching.set(true);
                }
            });
            spawn(task, Box::new(finish));
        }
    }
}

/// Options of [`use_query_with`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct QueryOptions {
    /// How long loaded data counts as fresh: shown without reloading. `0` (the default)
    /// reloads in the background each time a component starts showing the query.
    pub stale_time: Duration,
}

impl QueryOptions {
    /// Data stays fresh for `time` (`Duration::MAX`: loaded once, until invalidated).
    pub fn stale_time(mut self, time: Duration) -> Self {
        self.stale_time = time;
        self
    }
}

/// Data loaded by [`use_query`], shared by key. `Copy`, like a [`State`].
pub struct Query<T> {
    key: State<String>,
    client: State<QueryClient>,
    _type: std::marker::PhantomData<fn() -> T>,
}

impl<T> Clone for Query<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Query<T> {}

impl<T> std::fmt::Debug for Query<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query").field("key", &self.key).finish()
    }
}

impl<T: 'static> Query<T> {
    fn entry(self) -> Rc<Entry<T>> {
        let key = self.key.get();
        untrack(|| self.client.get()).entry::<T>(&key)
    }

    /// The data (a clone), or `None` before it first loaded. Reading it in a view or an
    /// effect subscribes it: the view updates when the data or the key changes.
    pub fn get(self) -> Option<T>
    where
        T: Clone,
    {
        self.entry().data.get()
    }

    /// Reads the data through a reference (`None` before it first loaded).
    pub fn with<R>(self, f: impl FnOnce(Option<&T>) -> R) -> R {
        self.entry().data.with(|v| f(v.as_ref()))
    }

    /// No data yet, and a request is running: show a placeholder.
    pub fn loading(self) -> bool {
        let entry = self.entry();
        entry.fetching.get() && entry.data.with(Option::is_none)
    }

    /// A request is running, the first one or a background reload: show a discreet
    /// indicator next to the data.
    pub fn fetching(self) -> bool {
        self.entry().fetching.get()
    }

    /// Reloads now (a "Refresh" button).
    pub fn refetch(self) {
        let entry = untrack(|| self.entry());
        start(&entry);
    }

    /// Replaces the data right away for every component that shows it (an optimistic
    /// update).
    pub fn set(self, value: T) {
        let key = untrack(|| self.key.get());
        untrack(|| self.client.get()).set_query_data(&key, value);
    }

    /// The current key.
    pub fn key(self) -> String {
        self.key.get()
    }
}

/// Loads data under a key, shared with every component that asks for the same key:
/// TanStack Query's `useQuery`, SWR's `useSWR`.
///
/// `key` may read states: when it changes, the query switches to the new key (loading it if
/// the cache does not have it fresh). `fetch` is called with the key and returns a future;
/// make `T` a `Result` to keep the errors. Inside a [`suspense`](crate::suspense), the
/// fallback is shown until the first data arrives.
///
/// ```
/// use active::{html::*, use_query, Node, QueryClient, provide};
///
/// async fn load_user(key: String) -> Result<String, String> {
///     // `gloo_net::http::Request::get(&format!("/api/{key}")).send().await` …
///     Ok(format!("Ada ({key})"))
/// }
///
/// let html = provide(QueryClient::new(), || {
///     let user = use_query(|| "users/7".to_string(), load_user);
///     let again = use_query(|| "users/7".to_string(), load_user); // shared: no second request
///     Node::from(p().text_dyn(move || match user.get() {
///         None => "Loading…".into(),
///         Some(Ok(name)) => name,
///         Some(Err(e)) => format!("Error: {e}"),
///     }))
///     .render()
/// });
/// assert_eq!(html, "<p><!--#-->Ada (users/7)</p>");
/// ```
pub fn use_query<T, F>(
    key: impl Fn() -> String + 'static,
    fetch: impl Fn(String) -> F + 'static,
) -> Query<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    use_query_with(key, QueryOptions::default(), fetch)
}

/// [`use_query`] with [`QueryOptions`]: how long the data stays fresh.
pub fn use_query_with<T, F>(
    key: impl Fn() -> String + 'static,
    options: QueryOptions,
    fetch: impl Fn(String) -> F + 'static,
) -> Query<T>
where
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let client = current_client();
    let fetch = Rc::new(fetch);
    let stale_ms = options.stale_time.as_secs_f64() * 1000.0;
    let attach = {
        let client = client.clone();
        move |key: &str| -> Rc<Entry<T>> {
            let entry = client.entry::<T>(key);
            let (f, k) = (fetch.clone(), key.to_owned());
            *entry.fetch.borrow_mut() = Some(Rc::new(move || Box::pin(f(k.clone())) as Task<T>));
            entry.users.set(entry.users.get() + 1);
            let fresh = entry.updated.get().is_some_and(|t| now() - t < stale_ms);
            let running = untrack(|| entry.fetching.get());
            if !fresh && !running {
                start(&entry);
            }
            entry
        }
    };
    let first = untrack(&key);
    let current = Rc::new(RefCell::new(attach(&first)));
    let key_state = use_state(first);
    {
        let current = current.clone();
        watch(key, move |k, _| {
            let old = current.borrow().clone();
            old.users.set(old.users.get().saturating_sub(1));
            *current.borrow_mut() = attach(k);
            key_state.set(k.clone());
        });
    }
    {
        let current = current.clone();
        on_cleanup(move || {
            let e = current.borrow();
            e.users.set(e.users.get().saturating_sub(1));
        });
    }
    let query = Query {
        key: key_state,
        client: use_state(client),
        _type: std::marker::PhantomData,
    };
    // In a `suspense`, counted until the first data arrives.
    if let Some(pending) = use_context::<Pending>()
        && untrack(|| current.borrow().data.with(Option::is_none))
    {
        let counted = Rc::new(Cell::new(true));
        pending.add(1);
        let c = counted.clone();
        effect(move || {
            if c.get() && query.with(|d| d.is_some()) {
                c.set(false);
                pending.add(-1);
            }
        });
        on_cleanup(move || {
            if counted.replace(false) {
                pending.add(-1);
            }
        });
    }
    query
}

/// Reloads the queries whose key starts with `prefix` (see [`QueryClient::invalidate`]), in
/// the cache in use here: call it from a component, an event handler, or after a change.
pub fn invalidate(prefix: &str) {
    current_client().invalidate(prefix);
}

/// Replaces the data of `key` right away (see [`QueryClient::set_query_data`]).
pub fn set_query_data<T: 'static>(key: &str, value: T) {
    current_client().set_query_data(key, value);
}

/// A change sent to the server by [`use_mutation`]. `Copy`, like a [`State`].
pub struct Mutation<I, O> {
    run: State<Rc<dyn Fn(I)>>,
    pending: State<bool>,
    result: State<Option<O>>,
    invalidates: State<Vec<String>>,
}

impl<I, O> Clone for Mutation<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<I, O> Copy for Mutation<I, O> {}

impl<I, O> std::fmt::Debug for Mutation<I, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutation")
            .field("pending", &self.pending)
            .finish()
    }
}

impl<I: 'static, O: 'static> Mutation<I, O> {
    /// Sends the change. While it runs, [`pending`](Mutation::pending) is `true`; when it
    /// ends, its output is in [`result`](Mutation::result) and the queries named by
    /// [`invalidates`](Mutation::invalidates) reload.
    pub fn mutate(self, input: I) {
        let run = untrack(|| self.run.get());
        run(input);
    }

    /// Reloads the queries starting with `prefix` once the change is done (TanStack's
    /// `invalidateQueries` in `onSuccess`). Can be called several times.
    pub fn invalidates(self, prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        self.invalidates.update(|list| list.push(prefix));
        self
    }

    /// The change is being sent: disable the button, show a spinner.
    pub fn pending(self) -> bool {
        self.pending.get()
    }

    /// The output of the last change (a clone), `None` before the first one ended.
    pub fn result(self) -> Option<O>
    where
        O: Clone,
    {
        self.result.get()
    }
}

/// Sends changes (a form, a "Delete" button): TanStack Query's `useMutation`.
///
/// `send` takes the input and returns a future; make `O` a `Result` to keep the errors.
/// Chain [`invalidates`](Mutation::invalidates) to reload the queries the change affects,
/// and use [`set_query_data`] or [`Query::set`] before `mutate` for an optimistic update.
///
/// ```
/// use active::{html::*, use_mutation, use_query, Node, QueryClient, provide};
///
/// provide(QueryClient::new(), || {
///     let todos = use_query(|| "todos".to_string(), |_| async { vec!["Write".to_string()] });
///     let add = use_mutation(|title: String| async move {
///         // `Request::post("/api/todos").json(&title)?.send().await` …
///         Ok::<_, String>(title)
///     })
///     .invalidates("todos");
///     let form = button().on_click(move |_| add.mutate("Read".into())).text("Add");
///     assert_eq!(todos.get().map(|t| t.len()), Some(1));
///     # let _ = form;
/// });
/// ```
pub fn use_mutation<I, O, F>(send: impl Fn(I) -> F + 'static) -> Mutation<I, O>
where
    I: 'static,
    O: 'static,
    F: Future<Output = O> + 'static,
{
    let client = current_client();
    let pending = use_state(false);
    let result = use_state(None::<O>);
    let invalidates = use_state(Vec::<String>::new());
    let latest = Rc::new(Cell::new(0u64));
    let run: Rc<dyn Fn(I)> = Rc::new(move |input: I| {
        let id = latest.get() + 1;
        latest.set(id);
        let mut task: Task<O> = Box::pin(send(input));
        let finish = {
            let (latest, client) = (latest.clone(), client.clone());
            move |out: O| {
                if !result.is_alive() {
                    return;
                }
                if latest.get() == id {
                    batch(|| {
                        result.set(Some(out));
                        pending.set(false);
                    });
                }
                for prefix in untrack(|| invalidates.get()) {
                    client.invalidate(&prefix);
                }
            }
        };
        match task.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(out) => finish(out),
            Poll::Pending => {
                untrack(|| pending.set(true));
                spawn(task, Box::new(finish));
            }
        }
    });
    Mutation {
        run: use_state(run),
        pending,
        result,
        invalidates,
    }
}

#[cfg(all(test, not(feature = "client")))]
mod tests {
    use super::*;
    use crate::context::provide;
    use crate::html::*;
    use crate::reactive::scope;
    use crate::resource::tests_support::Later;
    use crate::view::Node;

    type Calls = Rc<RefCell<Vec<(String, Later<String>)>>>;

    fn loader(calls: &Calls) -> impl Fn(String) -> Later<String> + 'static {
        let calls = calls.clone();
        move |key| {
            let later = Later::default();
            calls.borrow_mut().push((key, later.clone()));
            later
        }
    }

    #[test]
    fn components_asking_for_the_same_key_share_one_request() {
        let calls = Calls::default();
        let client = QueryClient::new();
        provide(client.clone(), || {
            let a = use_query(|| "users/1".into(), loader(&calls));
            let b = use_query(|| "users/1".into(), loader(&calls));
            assert_eq!(calls.borrow().len(), 1);
            assert!(a.loading() && b.loading());
            calls.borrow()[0].1.resolve("Ada".into());
            assert_eq!((a.get(), b.get()), (Some("Ada".into()), Some("Ada".into())));
            assert!(!a.loading() && !a.fetching());
            assert_eq!(
                client.get_query_data::<String>("users/1"),
                Some("Ada".into())
            );
        });
    }

    #[test]
    fn cached_data_shows_at_once_and_revalidates_in_the_background() {
        let calls = Calls::default();
        let client = QueryClient::new();
        provide(client.clone(), || {
            let (first, owner) = scope(|| use_query(|| "posts".into(), loader(&calls)));
            calls.borrow()[0].1.resolve("v1".into());
            assert_eq!(first.get(), Some("v1".into()));
            owner.dispose();
            // A new component: the cached value at once, and a background reload.
            let again = use_query(|| "posts".into(), loader(&calls));
            assert_eq!(again.get(), Some("v1".into()));
            assert!(again.fetching() && !again.loading());
            assert_eq!(calls.borrow().len(), 2);
            calls.borrow()[1].1.resolve("v2".into());
            assert_eq!(again.get(), Some("v2".into()));
        });
    }

    #[test]
    fn fresh_data_is_not_reloaded_within_its_stale_time() {
        let calls = Calls::default();
        provide(QueryClient::new(), || {
            let opts = QueryOptions::default().stale_time(Duration::from_secs(60));
            let a = use_query_with(|| "cfg".into(), opts, loader(&calls));
            calls.borrow()[0].1.resolve("x".into());
            let b = use_query_with(|| "cfg".into(), opts, loader(&calls));
            assert_eq!(calls.borrow().len(), 1);
            assert_eq!(b.get(), a.get());
        });
    }

    #[test]
    fn invalidate_reloads_the_shown_queries_of_a_prefix() {
        let calls = Calls::default();
        let client = QueryClient::new();
        provide(client.clone(), || {
            let list = use_query(|| "users".into(), loader(&calls));
            let one = use_query(|| "users/7".into(), loader(&calls));
            let other = use_query(|| "posts".into(), loader(&calls));
            for (_, l) in calls.borrow().clone() {
                l.resolve("0".into());
            }
            client.invalidate("users");
            let keys: Vec<String> = calls.borrow()[3..].iter().map(|(k, _)| k.clone()).collect();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains(&"users".into()) && keys.contains(&"users/7".into()));
            assert!(list.fetching() && one.fetching() && !other.fetching());
            // The previous data stays on screen meanwhile.
            assert_eq!(one.get(), Some("0".into()));
        });
    }

    #[test]
    fn the_key_can_change_and_late_answers_are_ignored() {
        let calls = Calls::default();
        provide(QueryClient::new(), || {
            let id = use_state(1);
            let q = use_query(move || format!("users/{}", id.get()), loader(&calls));
            id.set(2);
            assert_eq!(q.key(), "users/2");
            let (one, two) = (calls.borrow()[0].1.clone(), calls.borrow()[1].1.clone());
            two.resolve("two".into());
            one.resolve("one".into());
            assert_eq!(q.get(), Some("two".into()));
            id.set(1);
            assert_eq!(q.get(), Some("one".into()));
        });
    }

    #[test]
    fn optimistic_updates_and_mutations_that_invalidate() {
        let calls = Calls::default();
        let client = QueryClient::new();
        provide(client.clone(), || {
            let todos = use_query(|| "todos".into(), loader(&calls));
            calls.borrow()[0].1.resolve("a".into());
            let saved = Later::<u32>::default();
            let s = saved.clone();
            let add = use_mutation(move |_title: String| s.clone()).invalidates("todos");
            todos.set("a, b".into());
            add.mutate("b".into());
            assert!(add.pending());
            assert_eq!(todos.get(), Some("a, b".into()));
            saved.resolve(2);
            assert!(!add.pending());
            assert_eq!(add.result(), Some(2));
            // The list reloads after the change.
            assert_eq!(calls.borrow().len(), 2);
            assert!(todos.fetching());
        });
    }

    #[test]
    fn suspense_waits_for_the_first_data_of_a_query() {
        let calls = Calls::default();
        let load = loader(&calls);
        provide(QueryClient::new(), || {
            let (page, owner) = scope(|| {
                crate::suspense(p().text("Loading"), move || {
                    let q = use_query(|| "slow".into(), load);
                    span().text_dyn(move || q.get().unwrap_or_default()).into()
                })
            });
            let html = || {
                let (html, rendering) = scope(|| page.render());
                rendering.dispose();
                html
            };
            assert!(html().contains("Loading"));
            calls.borrow()[0].1.resolve("ready".into());
            assert!(html().contains("ready"), "{}", html());
            owner.dispose();
        });
    }

    #[test]
    fn without_a_provided_client_the_server_shares_nothing() {
        let calls = Calls::default();
        let _a = use_query(|| "me".into(), loader(&calls));
        let _b = use_query(|| "me".into(), loader(&calls));
        assert_eq!(calls.borrow().len(), 2);
        let _ = Node::from(p());
    }
}
