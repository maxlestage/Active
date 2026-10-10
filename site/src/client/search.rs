//! Browser side of the docs search island.

use std::cell::RefCell;
use std::rc::Rc;

use active::{Event, State};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{HtmlDialogElement, HtmlInputElement};

use super::{on, window};
use crate::routes::{Lang, link};
use crate::search::{Entry, search};

type Results = State<Vec<(String, String, String)>>;

fn dialog() -> Option<HtmlDialogElement> {
    window()
        .document()?
        .query_selector("dialog.search")
        .ok()
        .flatten()?
        .dyn_into()
        .ok()
}

/// Opens the dialog, focuses the field and shows the first pages.
pub fn open_dialog(lang: Lang, results: Results, loaded: State<bool>) {
    let Some(dialog) = dialog() else { return };
    if !dialog.open() {
        let _ = dialog.show_modal();
    }
    if let Some(input) = dialog
        .query_selector("input")
        .ok()
        .flatten()
        .and_then(|i| i.dyn_into::<HtmlInputElement>().ok())
    {
        let _ = input.focus();
        query(lang, input.value(), results, loaded);
    }
}

pub fn open(_e: &Event, lang: Lang, results: Results, loaded: State<bool>) {
    open_dialog(lang, results, loaded);
}

/// "/" and Ctrl/⌘ + K open the search from anywhere on the page.
pub fn shortcut(button: &web_sys::Element) {
    let button = button.clone();
    on(&window(), "keydown", move |e: web_sys::KeyboardEvent| {
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|t| t.closest("input, textarea").ok().flatten())
            .is_some();
        if (e.key() == "k" && (e.meta_key() || e.ctrl_key())) || (e.key() == "/" && !typing) {
            e.prevent_default();
            if let Some(b) = button.dyn_ref::<web_sys::HtmlElement>() {
                b.click();
            }
        }
    });
}

pub fn close_on_backdrop(e: &Event) {
    let (Some(dialog), Some(target)) = (dialog(), e.raw().target()) else {
        return;
    };
    if target.dyn_ref::<HtmlDialogElement>().is_some() {
        dialog.close();
    }
}

pub fn navigate(href: &str) {
    let _ = window().location().set_href(href);
}

thread_local! {
    /// The search index of the page's language, downloaded on first use.
    static INDEX: RefCell<Option<Rc<Vec<Entry>>>> = const { RefCell::new(None) };
    /// The latest query typed while the index downloads; `Some` while downloading.
    static PENDING: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Shows the pages matching `q`. The first call downloads `/search/{lang}.txt` (all pages
/// and headings of the language, a few kilobytes); then every key press is answered
/// locally, without network.
pub fn query(lang: Lang, q: String, results: Results, loaded: State<bool>) {
    if let Some(index) = INDEX.with(|i| i.borrow().clone()) {
        show(&index, &q, results, loaded);
        return;
    }
    let downloading = PENDING.with(|p| p.replace(Some(q)).is_some());
    if downloading {
        return;
    }
    let url = link(&format!("/search/{}.txt", lang.code()));
    spawn_local(async move {
        let text = fetch_text(&url).await.unwrap_or_default();
        let index = Rc::new(text.lines().filter_map(Entry::parse).collect::<Vec<_>>());
        INDEX.with(|i| *i.borrow_mut() = Some(index.clone()));
        let q = PENDING.with(|p| p.borrow_mut().take()).unwrap_or_default();
        show(&index, &q, results, loaded);
    });
}

fn show(index: &[Entry], q: &str, results: Results, loaded: State<bool>) {
    let list = search(index, q)
        .into_iter()
        .map(|e| (e.href.clone(), e.title.clone(), e.context.clone()))
        .collect();
    results.set(list);
    loaded.set(true);
}

async fn fetch_text(url: &str) -> Option<String> {
    let response = JsFuture::from(window().fetch_with_str(url)).await.ok()?;
    let response = response.dyn_into::<web_sys::Response>().ok()?;
    if !response.ok() {
        return None;
    }
    JsFuture::from(response.text().ok()?)
        .await
        .ok()?
        .as_string()
}
