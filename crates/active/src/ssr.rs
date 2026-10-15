//! Server rendering that sends data along: out-of-order **streaming** of [`suspense`]
//! boundaries (React 18's `renderToPipeableStream`, Solid's and Marko's streaming, SvelteKit's
//! streamed promises) and [`resource`]s whose values travel with the HTML, so that the
//! browser does not load them again when it hydrates (Next's and Nuxt's payloads, Leptos's
//! serialized resources).
//!
//! - [`render`] renders a [`Document`] like [`Document::render`], plus the values of the
//!   [`resource`]s created by the page, as JSON in `<script type="application/json">`.
//! - [`stream`] sends the page in chunks: first the whole page with the fallback of every
//!   [`suspense`] still loading, then, as each one's data arrives, its content and a few bytes
//!   of JavaScript that put it in place — the visitor reads the page while the slow parts
//!   load, and the server never waits for the slowest request before sending the first byte.
//!
//! [`suspense`]: crate::suspense

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::data::Data;
use crate::document::Document;
use crate::reactive::{ReleaseHold, Scope, State, scope, untrack};
use crate::resource::Resource;
use crate::view::Node;

/// A pending future and what to do with its output: polled by the stream, `true` when done.
pub(crate) type Job = Box<dyn FnMut(&mut Context<'_>) -> bool>;

/// What a page records while it is built and rendered by [`render`] or [`stream`].
#[derive(Default)]
struct Capture {
    /// [`stream`]: suspense boundaries are deferred and pending futures kept.
    streaming: bool,
    tasks: Vec<Job>,
    boundaries: Vec<Boundary>,
    data: Vec<Shared>,
    next_id: u32,
}

/// A [`suspense`](crate::suspense) whose fallback was sent; its content follows once
/// `pending` is back to zero.
struct Boundary {
    id: u32,
    pending: State<usize>,
    content: Node,
}

/// A [`resource`] whose value is sent to the browser.
struct Shared {
    key: String,
    encode: Box<dyn Fn() -> Option<String>>,
    sent: bool,
}

thread_local! {
    static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) };
}

/// Runs `f` with `capture` installed, and gives it back.
fn with_capture<R>(capture: Capture, f: impl FnOnce() -> R) -> (R, Capture) {
    let previous = CAPTURE.with(|c| c.replace(Some(capture)));
    /// Puts the previous capture back, even when `f` panics.
    struct Restore(Option<Option<Capture>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(previous) = self.0.take() {
                CAPTURE.with(|c| *c.borrow_mut() = previous);
            }
        }
    }
    let mut restore = Restore(Some(previous));
    let result = f();
    let previous = restore.0.take().expect("restored once");
    let capture = CAPTURE.with(|c| c.replace(previous)).unwrap_or_default();
    (result, capture)
}

/// Server, while streaming: keeps a pending future to poll it later. Gives it back otherwise.
#[cfg_attr(feature = "client", allow(dead_code))]
pub(crate) fn capture_task(job: Job) -> Result<(), Job> {
    CAPTURE.with(|c| match c.borrow_mut().as_mut() {
        Some(capture) if capture.streaming => {
            capture.tasks.push(job);
            Ok(())
        }
        _ => Err(job),
    })
}

/// Server, while streaming: defers a suspense boundary still loading and returns its id.
#[cfg_attr(feature = "client", allow(dead_code))]
pub(crate) fn defer(pending: State<usize>, content: &Node) -> Option<u32> {
    CAPTURE.with(|c| {
        let mut c = c.borrow_mut();
        let capture = c.as_mut().filter(|c| c.streaming)?;
        capture.next_id += 1;
        let id = capture.next_id;
        capture.boundaries.push(Boundary {
            id,
            pending,
            content: content.clone(),
        });
        Some(id)
    })
}

/// Renders `build()`'s document, followed by the values of the [`resource`]s it created
/// (see the [module](self)): the browser hydrates the page without loading them again.
///
/// `build` is called here, so that the resources it creates are recorded: build the page
/// inside it. A resource whose value is not there yet when the page is rendered is not sent
/// (its [`suspense`](crate::suspense) shows the fallback, and the browser loads it).
///
/// ```
/// use active::{html::*, ssr, suspense, Document};
///
/// let html = ssr::render(|| {
///     let page = suspense(p().text("Loading…"), || {
///         let user = ssr::resource("user", || 7, |id| async move { format!("User {id}") });
///         h1().text_dyn(move || user.get().unwrap_or_default()).into()
///     });
///     Document::new(page)
/// });
/// assert!(html.contains("<h1><!--#-->User 7</h1>"));
/// assert!(html.ends_with(
///     r#"<script type="application/json" data-active-data="user">"User 7"</script></body></html>"#
/// ));
/// ```
pub fn render(build: impl FnOnce() -> Document) -> String {
    let _hold = crate::reactive::hold_release();
    let ((document, capture), page) = scope(|| with_capture(Capture::default(), build));
    let ((mut html, mut capture), rendering) =
        scope(|| with_capture(capture, || document.render()));
    let scripts = data_scripts(&mut capture);
    let end = if html.ends_with(END) {
        html.len() - END.len()
    } else {
        html.len()
    };
    html.insert_str(end, &scripts);
    rendering.dispose();
    page.dispose();
    html
}

const END: &str = "</body></html>";

/// Defines the function that puts streamed content in place of its fallback.
const SWAP: &str = "<script>function __activeSwap(i){var t=document.getElementById(\"active-s\"+i),p=document.querySelector('active-suspense[data-suspense=\"'+i+'\"]');if(t&&p){p.replaceWith(t.content);t.remove()}}</script>";

/// `<script type="application/json">` for each shared value that is ready and not sent yet.
fn data_scripts(capture: &mut Capture) -> String {
    let mut out = String::new();
    for shared in capture.data.iter_mut().filter(|s| !s.sent) {
        let Some(json) = (shared.encode)() else {
            continue;
        };
        shared.sent = true;
        out.push_str(r#"<script type="application/json" data-active-data=""#);
        crate::escape::escape_into(&mut out, &shared.key);
        out.push_str(r#"">"#);
        // `<` only appears inside JSON strings, where `<` means the same: nothing can
        // close the script element.
        out.push_str(&json.replace('<', "\\u003c"));
        out.push_str("</script>");
    }
    out
}

/// Starts streaming `build()`'s document: see [`Stream`].
///
/// The page is built and rendered at once: the first chunk is ready, with the fallback of
/// every [`suspense`](crate::suspense) whose data is still loading. The futures of the
/// [`resource`](crate::resource)s (plain or from this module) then run as the stream is
/// polled, and each boundary is sent as soon as its data is there, in the order the data
/// arrives (out of order).
///
/// ```
/// use active::{html::*, ssr, suspense, Document};
///
/// let mut stream = ssr::stream(|| {
///     let page = main()
///         .child(h1().text("Shop"))
///         .child(suspense(p().text("Loading…"), || {
///             let n = ssr::resource("count", || (), |()| async { 3u32 });
///             p().text_dyn(move || format!("{} items", n.get().unwrap_or(0))).into()
///         }));
///     Document::new(page)
/// });
/// // Every future here is ready at once: the page comes in one chunk, then the end.
/// let html = stream.now_or_never_all();
/// assert!(html.contains("<p><!--#-->3 items</p>"));
/// assert!(html.ends_with("</body></html>"));
/// ```
///
/// With a server: send each chunk as it comes (`while let Some(chunk) = stream.next_chunk().await`),
/// as the body of a response with `Content-Type: text/html; charset=utf-8`. Like the
/// states it holds, a stream belongs to the thread that created it (`!Send`): drive it on that
/// thread (a `tokio::task::LocalSet`, a single-threaded runtime) and send the chunks to the
/// response body through a channel.
pub fn stream(build: impl FnOnce() -> Document) -> Stream {
    let hold = crate::reactive::hold_release();
    let start = Capture {
        streaming: true,
        ..Capture::default()
    };
    let ((document, capture), page) = scope(|| with_capture(start, build));
    let ((html, mut capture), rendering) = scope(|| with_capture(capture, || document.render()));
    let mut shell = html.strip_suffix(END).unwrap_or(&html).to_string();
    if !capture.boundaries.is_empty() {
        shell.push_str("<style>active-suspense{display:contents}</style>");
        shell.push_str(SWAP);
    }
    shell.push_str(&data_scripts(&mut capture));
    Stream {
        inner: Some(Inner {
            capture,
            owners: vec![page, rendering],
            shell: Some(shell),
            _hold: hold,
        }),
    }
}

/// A page sent in chunks, made by [`stream`]: the first chunk is the page with fallbacks,
/// each next one the content of the boundaries whose data arrived, the last one the end of the
/// page. `None` after the last.
///
/// A chunk is a `String` of HTML to send as is. A stream dropped before its end (the
/// visitor left) stops its futures and releases its states.
#[must_use = "a stream sends nothing until it is polled"]
pub struct Stream {
    inner: Option<Inner>,
}

struct Inner {
    capture: Capture,
    owners: Vec<Scope>,
    shell: Option<String>,
    _hold: ReleaseHold,
}

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("finished", &self.inner.is_none())
            .field(
                "boundaries_left",
                &self
                    .inner
                    .as_ref()
                    .map_or(0, |i| i.capture.boundaries.len()),
            )
            .finish()
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Futures first: they may hold states.
        self.capture.tasks.clear();
        for owner in self.owners.drain(..) {
            owner.dispose();
        }
    }
}

impl Stream {
    /// The next chunk: `Ready(Some(html))`, `Pending` while the data of the boundaries left
    /// is loading (`cx`'s waker is called when one of their futures can make progress), or
    /// `Ready(None)` once the page is complete.
    pub fn poll_chunk(&mut self, cx: &mut Context<'_>) -> Poll<Option<String>> {
        let Some(inner) = self.inner.as_mut() else {
            return Poll::Ready(None);
        };
        if let Some(shell) = inner.shell.take() {
            return Poll::Ready(Some(shell));
        }
        let capture = std::mem::take(&mut inner.capture);
        let ((chunk, done), capture) = with_capture(capture, || {
            poll_tasks(cx);
            let chunk = ready_boundaries(&mut inner.owners);
            // Done when no boundary is left, or when the ones left wait for nothing that
            // runs (their fallback stays).
            let finished = CAPTURE.with(|c| {
                c.borrow()
                    .as_ref()
                    .is_none_or(|c| c.boundaries.is_empty() || c.tasks.is_empty())
            });
            (chunk, finished)
        });
        inner.capture = capture;
        let mut chunk = chunk;
        if done {
            chunk.push_str(&data_scripts(&mut inner.capture));
            chunk.push_str(END);
            self.inner = None;
            return Poll::Ready(Some(chunk));
        }
        if chunk.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Some(chunk))
        }
    }

    /// The next chunk, or `None` once the page is complete.
    pub fn next_chunk(&mut self) -> impl Future<Output = Option<String>> + '_ {
        std::future::poll_fn(move |cx| self.poll_chunk(cx))
    }

    /// Ends the page now (a time limit was reached): the chunks that are ready, then the end
    /// of the page; the boundaries still loading keep their fallback in the HTML, and the
    /// browser loads their data when it hydrates them.
    pub fn now_or_never_all(&mut self) -> String {
        let mut out = String::new();
        let mut cx = Context::from_waker(std::task::Waker::noop());
        while let Poll::Ready(Some(chunk)) = self.poll_chunk(&mut cx) {
            out.push_str(&chunk);
        }
        if let Some(inner) = self.inner.take() {
            let mut inner = inner;
            out.push_str(&data_scripts(&mut inner.capture));
            out.push_str(END);
        }
        out
    }
}

/// Polls the captured futures once each, keeping those still pending.
fn poll_tasks(cx: &mut Context<'_>) {
    let tasks = CAPTURE.with(|c| {
        c.borrow_mut()
            .as_mut()
            .map(|c| std::mem::take(&mut c.tasks))
            .unwrap_or_default()
    });
    let mut left = Vec::with_capacity(tasks.len());
    for mut task in tasks {
        if !task(cx) {
            left.push(task);
        }
    }
    CAPTURE.with(|c| {
        if let Some(c) = c.borrow_mut().as_mut() {
            // Futures started meanwhile come after.
            left.append(&mut c.tasks);
            c.tasks = left;
        }
    });
}

/// The HTML of the boundaries whose data is all there (boundaries inside them are deferred
/// in turn).
fn ready_boundaries(owners: &mut Vec<Scope>) -> String {
    let mut out = String::new();
    loop {
        let ready: Vec<Boundary> = CAPTURE.with(|c| {
            let mut c = c.borrow_mut();
            let Some(c) = c.as_mut() else {
                return Vec::new();
            };
            let (ready, waiting) = std::mem::take(&mut c.boundaries)
                .into_iter()
                .partition(|b| !b.pending.is_alive() || untrack(|| b.pending.get()) == 0);
            c.boundaries = waiting;
            ready
        });
        if ready.is_empty() {
            return out;
        }
        for boundary in ready {
            let (html, rendering) = scope(|| boundary.content.render());
            owners.push(rendering);
            let data = CAPTURE.with(|c| c.borrow_mut().as_mut().map(data_scripts));
            out.push_str(&data.unwrap_or_default());
            let id = boundary.id;
            out.push_str(&format!(
                r#"<template id="active-s{id}">{html}</template><script>__activeSwap({id})</script>"#
            ));
        }
        // A boundary inside one just sent may be ready too.
    }
}

/// A [`resource`](crate::resource) whose value is sent from the server to the browser under
/// `key`, so that the page hydrates with it instead of loading it again (Next's and Nuxt's
/// payloads).
///
/// - On the server, inside [`render`] or [`stream`], the value is written to the page as
///   JSON (see [`Data`]) once it is there.
/// - In the browser, the first load reads the value from the page if it is there (and
///   removes it from the page), without calling `fetch`; the next ones (a changed source,
///   [`Resource::refetch`]) call `fetch`.
///
/// Give each resource of a page its own `key` (`"user"`, `"posts:7"`).
///
/// ```
/// use active::ssr;
///
/// let posts = ssr::resource("posts", || 1u32, |page| async move { vec![format!("Post of page {page}")] });
/// assert_eq!(posts.get(), Some(vec!["Post of page 1".to_string()]));
/// ```
pub fn resource<S, T, F>(
    key: impl Into<String>,
    source: impl Fn() -> S + 'static,
    fetch: impl Fn(S) -> F + 'static,
) -> Resource<T>
where
    S: Clone + PartialEq + 'static,
    T: Data + 'static,
    F: Future<Output = T> + 'static,
{
    let key = key.into();
    #[cfg(feature = "client")]
    let embedded = Cell::new(crate::client::take_data(&key).and_then(|json| T::read_data(&json)));
    #[cfg(not(feature = "client"))]
    let embedded = Cell::new(None::<T>);
    let resource = crate::resource::resource(source, move |s| {
        let task: Pin<Box<dyn Future<Output = T>>> = match embedded.take() {
            Some(value) => Box::pin(std::future::ready(value)),
            None => Box::pin(fetch(s)),
        };
        task
    });
    let value = resource.state();
    CAPTURE.with(|c| {
        if let Some(capture) = c.borrow_mut().as_mut() {
            capture.data.push(Shared {
                key,
                encode: Box::new(move || {
                    if !value.is_alive() {
                        return None;
                    }
                    untrack(|| value.with(|v| v.as_ref().map(Data::to_data)))
                }),
                sent: false,
            });
        }
    });
    resource
}

#[cfg(all(test, not(feature = "client")))]
mod tests {
    use super::*;
    use crate::html::*;
    use crate::resource::suspense;
    use crate::view::text_dyn;
    use std::rc::Rc;
    use std::task::Waker;

    /// A future completed by hand, waking its task.
    #[derive(Clone, Default)]
    struct Later<T>(Rc<RefCell<(Option<T>, Option<Waker>)>>);

    impl<T> Later<T> {
        fn resolve(&self, v: T) {
            let waker = {
                let mut inner = self.0.borrow_mut();
                inner.0 = Some(v);
                inner.1.take()
            };
            if let Some(w) = waker {
                w.wake();
            }
        }
    }

    impl<T> Future for Later<T> {
        type Output = T;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            let mut inner = self.0.borrow_mut();
            match inner.0.take() {
                Some(v) => Poll::Ready(v),
                None => {
                    inner.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }

    /// Counts its wake-ups.
    struct Count(std::sync::atomic::AtomicUsize);
    impl std::task::Wake for Count {
        fn wake(self: std::sync::Arc<Self>) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn boundaries_stream_out_of_order_with_their_data() {
        let (slow, fast) = (Later::<String>::default(), Later::<String>::default());
        let (s, f) = (slow.clone(), fast.clone());
        let mut stream = stream(move || {
            let page = main()
                .child(h1().text("Title"))
                .child(suspense(p().text("Loading A"), move || {
                    let a = resource("a", || (), move |()| s.clone());
                    p().text_dyn(move || a.get().unwrap_or_default()).into()
                }))
                .child(suspense(p().text("Loading B"), move || {
                    let b = resource("b", || (), move |()| f.clone());
                    p().text_dyn(move || b.get().unwrap_or_default()).into()
                }));
            Document::new(page)
        });
        let counter = std::sync::Arc::new(Count(Default::default()));
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);
        let Poll::Ready(Some(shell)) = stream.poll_chunk(&mut cx) else {
            panic!("the shell is ready at once");
        };
        assert!(shell.starts_with("<!DOCTYPE html>"));
        assert!(shell.contains("<h1>Title</h1>"));
        assert!(shell.contains(
            r#"<!--[--><active-suspense data-suspense="1"><p>Loading A</p></active-suspense><!--]-->"#
        ));
        assert!(shell.contains(r#"<active-suspense data-suspense="2"><p>Loading B</p>"#));
        assert!(shell.contains("function __activeSwap"));
        assert!(!shell.contains("</body>"));
        assert_eq!(stream.poll_chunk(&mut cx), Poll::Pending);

        // B arrives first, and is sent first.
        fast.resolve("<B>".into());
        assert_eq!(counter.0.load(std::sync::atomic::Ordering::SeqCst), 1);
        let Poll::Ready(Some(chunk)) = stream.poll_chunk(&mut cx) else {
            panic!("B is ready");
        };
        assert_eq!(
            chunk,
            r#"<script type="application/json" data-active-data="b">"\u003cB>"</script><template id="active-s2"><p><!--#-->&lt;B&gt;</p></template><script>__activeSwap(2)</script>"#
        );
        assert_eq!(stream.poll_chunk(&mut cx), Poll::Pending);
        slow.resolve("A".into());
        let Poll::Ready(Some(last)) = stream.poll_chunk(&mut cx) else {
            panic!("A is ready");
        };
        assert!(last.contains(r#"<template id="active-s1"><p><!--#-->A</p></template>"#));
        assert!(last.ends_with("</body></html>"));
        assert_eq!(stream.poll_chunk(&mut cx), Poll::Ready(None));
    }

    #[test]
    fn nested_boundaries_follow_their_parent_and_a_stream_can_end_early() {
        let (outer, inner) = (Later::<u32>::default(), Later::<u32>::default());
        let (o, i) = (outer.clone(), inner.clone());
        let mut stream = stream(move || {
            Document::new(suspense("outer…", move || {
                let a = crate::resource(|| (), move |()| o.clone());
                let i = i.clone();
                div()
                    .child(text_dyn(move || a.get().unwrap_or(0).to_string()))
                    .child(suspense("inner…", move || {
                        let b = crate::resource(|| (), move |()| i.clone());
                        span()
                            .text_dyn(move || b.get().unwrap_or(0).to_string())
                            .into()
                    }))
                    .into()
            }))
        });
        let mut cx = Context::from_waker(Waker::noop());
        let Poll::Ready(Some(shell)) = stream.poll_chunk(&mut cx) else {
            panic!()
        };
        assert!(shell.contains("outer…") && !shell.contains("inner…"));
        outer.resolve(1);
        let Poll::Ready(Some(chunk)) = stream.poll_chunk(&mut cx) else {
            panic!()
        };
        assert!(chunk.contains(r#"<template id="active-s1"><div><!--#-->1<!--[--><active-suspense data-suspense="2">inner…</active-suspense><!--]--></div></template>"#), "{chunk}");
        // Time is up: the inner boundary keeps its fallback.
        let rest = stream.now_or_never_all();
        assert_eq!(rest, "</body></html>");
        assert_eq!(stream.poll_chunk(&mut cx), Poll::Ready(None));
        // Its future is gone with the stream.
        inner.resolve(2);
    }

    #[test]
    fn render_sends_ready_values_and_skips_loading_ones() {
        let html = render(|| {
            let ready = resource("ready", || 2u32, |n| async move { vec![n; 2] });
            let view = div()
                .child(text_dyn(move || format!("{:?}", ready.get())))
                .child(suspense("wait", move || {
                    let never = resource("never", || (), |()| std::future::pending::<u32>());
                    p().text_dyn(move || never.get().unwrap_or(0).to_string())
                        .into()
                }));
            Document::new(view)
        });
        assert!(html.contains(
            r#"<script type="application/json" data-active-data="ready">[2,2]</script></body></html>"#
        ));
        assert!(!html.contains("\"never\""));
        assert!(html.contains("wait"), "{html}");
        assert!(
            !html.contains("active-suspense"),
            "not streaming: plain fallback"
        );
    }

    #[test]
    fn a_stream_waits_for_its_futures_and_stops_them_when_dropped() {
        let before = crate::reactive::live_states();
        let mut stream = stream(|| {
            Document::new(suspense("stuck", || {
                let r = crate::resource(|| (), |()| std::future::pending::<u8>());
                p().text_dyn(move || r.get().unwrap_or(0).to_string())
                    .into()
            }))
        });
        let mut cx = Context::from_waker(Waker::noop());
        assert!(matches!(stream.poll_chunk(&mut cx), Poll::Ready(Some(_))));
        // The future never ends: the stream waits.
        assert_eq!(stream.poll_chunk(&mut cx), Poll::Pending);
        drop(stream);
        assert_eq!(
            crate::reactive::live_states(),
            before,
            "its states are released"
        );
    }
}
