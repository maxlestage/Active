//! Asynchronous data: [`resource`] (Solid's `createResource`, React Query's basics) and
//! [`suspense`], a fallback shown while the resources of a view load (React's `<Suspense>`).

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use crate::context::{provide, use_context};
use crate::reactive::{State, batch, on_cleanup, untrack, use_state, watch};
use crate::view::{Node, fragment_dyn};

type Task<T> = Pin<Box<dyn Future<Output = T>>>;

/// Data loaded asynchronously by [`resource`]: `None` until the first value arrives, then the
/// latest value. `Copy`, like a [`State`], so it can be moved into any closure.
pub struct Resource<T> {
    value: State<Option<T>>,
    loading: State<bool>,
    refetch: State<Rc<dyn Fn()>>,
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Resource<T> {}

impl<T> std::fmt::Debug for Resource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource")
            .field("value", &self.value)
            .finish()
    }
}

impl<T: 'static> Resource<T> {
    /// The latest value (a clone), or `None` before the first one arrived. Reading it in a
    /// view or an effect subscribes it.
    pub fn get(self) -> Option<T>
    where
        T: Clone,
    {
        self.value.get()
    }

    /// Reads the latest value through a reference (`None` before the first one).
    pub fn with<R>(self, f: impl FnOnce(Option<&T>) -> R) -> R {
        self.value.with(|v| f(v.as_ref()))
    }

    /// Whether a load is in progress: the first one, or a reload after the source changed or
    /// [`refetch`](Resource::refetch). The previous value stays readable meanwhile, so a view
    /// can keep showing it, dimmed, while the next one loads (React's `useTransition`).
    pub fn loading(self) -> bool {
        self.loading.get()
    }

    /// Loads again with the current source (a "Retry" or "Refresh" button).
    pub fn refetch(self) {
        let run = untrack(|| self.refetch.get());
        run();
    }

    /// Replaces the value right away, without loading (an optimistic update: show the new
    /// title at once, while the request that saves it runs).
    pub fn set(self, value: T) {
        self.value.set(Some(value));
    }

    /// The state holding the value, for [`memo`](crate::memo)s and [`watch`]es.
    pub fn state(self) -> State<Option<T>> {
        self.value
    }
}

/// The resources of a [`suspense`] that have not loaded their first value yet.
#[derive(Clone, Copy)]
struct Pending(State<usize>);

impl Pending {
    fn add(self, n: isize) {
        if self.0.is_alive() {
            self.0
                .update(|count| *count = count.saturating_add_signed(n));
        }
    }
}

/// Loads data asynchronously from a source that may change: Solid's `createResource`.
///
/// `fetch` is called with the value of `source` and returns a future (an HTTP request, a
/// timer, a read from IndexedDB…); its output becomes the resource's value. When a state read
/// by `source` changes, `fetch` is called again with the new source (the result of an
/// earlier, slower request is ignored). Make `T` a `Result` to keep the errors.
///
/// - In the browser (feature `client`), the future runs on the browser's event loop.
/// - On the server, there is no event loop: the future is polled once, so a value that is
///   already there (a cache, data the page was rendered with, `async { value }`) is rendered
///   at once; a real request stays loading, and [`suspense`] renders its fallback.
///
/// ```
/// use active::{html::*, resource, use_state, Node};
///
/// async fn load_user(id: u32) -> Result<String, String> {
///     // `gloo_net::http::Request::get(&format!("/api/users/{id}")).send().await` …
///     Ok(format!("User {id}"))
/// }
///
/// let id = use_state(7);
/// let user = resource(move || id.get(), load_user);
/// let view = p().text_dyn(move || match user.get() {
///     None => "Loading…".into(),
///     Some(Ok(name)) => name,
///     Some(Err(e)) => format!("Error: {e}"),
/// });
/// assert_eq!(Node::from(view).render(), "<p><!--#-->User 7</p>");
/// ```
pub fn resource<S, T, F>(
    source: impl Fn() -> S + 'static,
    fetch: impl Fn(S) -> F + 'static,
) -> Resource<T>
where
    S: Clone + PartialEq + 'static,
    T: 'static,
    F: Future<Output = T> + 'static,
{
    let value = use_state(None::<T>);
    let loading = use_state(false);
    let pending = use_context::<Pending>();
    // Counted in the enclosing `suspense` while the first value is awaited.
    let counted = Rc::new(Cell::new(false));
    {
        let counted = counted.clone();
        on_cleanup(move || {
            if counted.replace(false) {
                if let Some(p) = pending {
                    p.add(-1);
                }
            }
        });
    }
    // Only the latest request may write its result.
    let latest = Rc::new(Cell::new(0u64));
    let load = Rc::new(move |s: S| {
        let id = latest.get() + 1;
        latest.set(id);
        let mut task: Task<T> = Box::pin(fetch(s));
        let finish = {
            let (latest, counted) = (latest.clone(), counted.clone());
            move |v: T| {
                if latest.get() != id || !value.is_alive() {
                    return;
                }
                batch(|| {
                    value.set(Some(v));
                    loading.set(false);
                });
                if counted.replace(false) {
                    if let Some(p) = pending {
                        p.add(-1);
                    }
                }
            }
        };
        match task.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(v) => finish(v),
            Poll::Pending => {
                untrack(|| {
                    if !loading.get() {
                        loading.set(true);
                    }
                    if value.with(Option::is_none) && !counted.replace(true) {
                        if let Some(p) = pending {
                            p.add(1);
                        }
                    }
                });
                spawn(task, Box::new(finish));
            }
        }
    });
    let refetch: Rc<dyn Fn()> = {
        let source = Rc::new(source);
        // The first load, then one per change of the source (browser only).
        load(untrack(|| source()));
        let (reload, followed) = (load.clone(), source.clone());
        watch(move || followed(), move |s, _| reload(s.clone()));
        Rc::new(move || load(untrack(|| source())))
    };
    Resource {
        value,
        loading,
        refetch: use_state(refetch),
    }
}

/// Browser: runs `task` on the event loop, then `done` with its output.
#[cfg(feature = "client")]
fn spawn<T: 'static>(task: Task<T>, done: Box<dyn FnOnce(T)>) {
    wasm_bindgen_futures::spawn_local(async move { done(task.await) });
}

/// Server: no event loop. While [`ssr::stream`](crate::ssr::stream) streams a page, the
/// future is kept and polled by the stream; otherwise the value stays loading.
#[cfg(not(feature = "client"))]
fn spawn<T: 'static>(task: Task<T>, done: Box<dyn FnOnce(T)>) {
    let mut task = Some((task, done));
    let job: crate::ssr::Job = Box::new(move |cx: &mut Context<'_>| {
        let Some((t, _)) = task.as_mut() else {
            return true;
        };
        match t.as_mut().poll(cx) {
            Poll::Ready(v) => {
                let (_, done) = task.take().expect("polled once");
                done(v);
                true
            }
            Poll::Pending => false,
        }
    });
    if let Err(job) = crate::ssr::capture_task(job) {
        #[cfg(test)]
        TASKS.with(|t| t.borrow_mut().push(job));
        #[cfg(not(test))]
        drop(job);
    }
}

#[cfg(all(test, not(feature = "client")))]
thread_local! {
    static TASKS: std::cell::RefCell<Vec<crate::ssr::Job>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Tests: polls the pending tasks once each, keeping those still pending.
#[cfg(all(test, not(feature = "client")))]
pub(crate) fn run_tasks() {
    let tasks = TASKS.with(|t| std::mem::take(&mut *t.borrow_mut()));
    let mut left = Vec::new();
    let mut cx = Context::from_waker(Waker::noop());
    for mut task in tasks {
        if !task(&mut cx) {
            left.push(task);
        }
    }
    TASKS.with(|t| t.borrow_mut().extend(left));
}

/// Shows `fallback` while a [`resource`] created by `view` is loading its **first** value,
/// then the view: React's `<Suspense>`, Solid's `<Suspense>`.
///
/// `view` is built once, right away (so that its resources start loading in parallel); the
/// page shows `fallback` in its place until every resource created inside, however deep,
/// has a value. Later reloads (a changed source, [`Resource::refetch`]) keep the view on
/// screen with its previous data: show a discreet indicator with
/// [`Resource::loading`] instead.
///
/// On the server, a resource whose value is not ready at once leaves the fallback in the
/// HTML; the browser then loads the data and shows the view.
///
/// ```
/// use active::{html::*, resource, suspense, use_state, Node};
///
/// fn profile() -> Node {
///     let id = use_state(1);
///     let user = resource(move || id.get(), |id| async move { format!("User {id}") });
///     h2().text_dyn(move || user.get().unwrap_or_default()).into()
/// }
///
/// let page = suspense(p().text("Loading…"), profile);
/// assert_eq!(page.render(), "<!--[--><h2><!--#-->User 1</h2><!--]-->");
/// ```
pub fn suspense(fallback: impl Into<Node>, view: impl FnOnce() -> Node) -> Node {
    let pending = use_state(0usize);
    let content = provide(Pending(pending), view);
    let fallback = fallback.into();
    fragment_dyn(move || {
        if pending.get() > 0 {
            // Streaming on the server: the fallback is sent now, the content later.
            #[cfg(not(feature = "client"))]
            if let Some(id) = crate::ssr::defer(pending, &content) {
                return vec![
                    crate::view::el("active-suspense")
                        .attr("data-suspense", id.to_string())
                        .child(fallback.clone())
                        .into(),
                ];
            }
            vec![fallback.clone()]
        } else {
            vec![content.clone()]
        }
    })
}

/// Test helpers shared with other modules.
#[cfg(all(test, not(feature = "client")))]
pub(crate) mod tests_support {
    use super::*;
    use std::cell::RefCell;

    /// A future completed by hand.
    #[derive(Clone, Default)]
    pub(crate) struct Later<T>(Rc<RefCell<Option<T>>>);

    impl<T> Later<T> {
        pub(crate) fn resolve(&self, v: T) {
            *self.0.borrow_mut() = Some(v);
            run_tasks();
        }
    }

    impl<T> Future for Later<T> {
        type Output = T;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
            match self.0.borrow_mut().take() {
                Some(v) => Poll::Ready(v),
                None => Poll::Pending,
            }
        }
    }
}

#[cfg(all(test, not(feature = "client")))]
mod tests {
    use super::*;
    use crate::html::*;
    use crate::reactive::scope;
    use std::cell::RefCell;

    type Requests = Rc<RefCell<Vec<(u32, Later<String>)>>>;

    /// A future completed by hand.
    #[derive(Clone, Default)]
    struct Later<T>(Rc<RefCell<Option<T>>>);

    impl<T> Later<T> {
        fn resolve(&self, v: T) {
            *self.0.borrow_mut() = Some(v);
            run_tasks();
        }
    }

    impl<T> Future for Later<T> {
        type Output = T;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
            match self.0.borrow_mut().take() {
                Some(v) => Poll::Ready(v),
                None => Poll::Pending,
            }
        }
    }

    #[test]
    fn a_ready_future_gives_its_value_at_once() {
        let r = resource(|| 2, |n| async move { n * 10 });
        assert_eq!((r.get(), r.loading()), (Some(20), false));
    }

    #[test]
    fn a_resource_reloads_when_its_source_changes_and_ignores_stale_results() {
        let id = use_state(1);
        let requests: Requests = Rc::default();
        let log = requests.clone();
        let r = resource(
            move || id.get(),
            move |id| {
                let later = Later::default();
                log.borrow_mut().push((id, later.clone()));
                later
            },
        );
        assert_eq!((r.get(), r.loading()), (None, true));
        id.set(2);
        assert_eq!(requests.borrow().len(), 2);
        // The first request ends after the second one started: ignored.
        let (first, second) = {
            let reqs = requests.borrow();
            (reqs[0].1.clone(), reqs[1].1.clone())
        };
        second.resolve("two".into());
        assert_eq!((r.get(), r.loading()), (Some("two".into()), false));
        first.resolve("one".into());
        assert_eq!(r.get(), Some("two".into()));
        r.set("edited".into());
        assert_eq!(r.get(), Some("edited".into()));
    }

    #[test]
    fn refetch_reloads_and_keeps_the_previous_value_meanwhile() {
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let next: Rc<RefCell<Option<Later<u32>>>> = Rc::default();
        let n = next.clone();
        let r = resource(
            || (),
            move |()| {
                c.set(c.get() + 1);
                let later = Later::default();
                *n.borrow_mut() = Some(later.clone());
                later
            },
        );
        next.borrow().clone().unwrap().resolve(1);
        r.refetch();
        assert_eq!((calls.get(), r.get(), r.loading()), (2, Some(1), true));
        next.borrow().clone().unwrap().resolve(2);
        assert_eq!((r.get(), r.loading()), (Some(2), false));
    }

    #[test]
    fn suspense_shows_the_fallback_until_every_first_value_is_there() {
        let (a, b) = (Later::<u32>::default(), Later::<u32>::default());
        let (la, lb) = (a.clone(), b.clone());
        let reloads: Rc<RefCell<Option<Resource<u32>>>> = Rc::default();
        let keep = reloads.clone();
        let (page, owner) = scope(|| {
            suspense(p().text("Loading"), move || {
                let ra = resource(|| (), move |()| la.clone());
                let rb = resource(|| (), move |()| lb.clone());
                *keep.borrow_mut() = Some(ra);
                span()
                    .text_dyn(move || {
                        format!("{}+{}", ra.get().unwrap_or(0), rb.get().unwrap_or(0))
                    })
                    .into()
            })
        });
        // Rendered inside a scope, like a component: the server's release of the states
        // unused by two renders in a row does not apply.
        let html = || {
            let (html, rendering) = scope(|| page.render());
            rendering.dispose();
            html
        };
        assert!(html().contains("Loading"));
        a.resolve(1);
        assert!(html().contains("Loading"));
        b.resolve(2);
        assert!(html().contains("1+2"), "{}", html());
        // A reload keeps the content.
        reloads.borrow().unwrap().refetch();
        assert!(html().contains("1+2"));
        owner.dispose();
    }
}
