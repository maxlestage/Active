//! Browser side of the docs search island.

use active::{Event, State};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{HtmlDialogElement, HtmlInputElement};

use super::{on, window};
use crate::routes::Lang;

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

/// Asks the server; it answers one result per line: `href \t title \t category`.
pub fn query(lang: Lang, q: String, results: Results, loaded: State<bool>) {
    let url = format!(
        "/api/search?lang={}&q={}",
        lang.code(),
        js_sys::encode_uri_component(&q)
    );
    spawn_local(async move {
        let Ok(response) = JsFuture::from(window().fetch_with_str(&url)).await else {
            return;
        };
        let Ok(response) = response.dyn_into::<web_sys::Response>() else {
            return;
        };
        let Ok(text) = response.text() else { return };
        let Ok(text) = JsFuture::from(text).await else {
            return;
        };
        let text = text.as_string().unwrap_or_default();
        let list = text
            .lines()
            .filter_map(|line| {
                let mut parts = line.split('\t');
                Some((
                    parts.next()?.to_string(),
                    parts.next()?.to_string(),
                    parts.next().unwrap_or("").to_string(),
                ))
            })
            .collect();
        results.set(list);
        loaded.set(true);
    });
}
