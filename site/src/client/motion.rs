//! Page motion that CSS cannot do alone. Scroll reveals, the opening curtain, the marquee
//! and page transitions are pure CSS; this adds the parts that need the pointer or numbers.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    Element, HtmlElement, IntersectionObserver, IntersectionObserverEntry, IntersectionObserverInit,
};

use super::{all, on, on_passive, reduced_motion, request_frame, window};
use crate::labels::{fixed, parse_decimal};

pub fn init() {
    let doc = window().document().unwrap();
    let root = doc.document_element().unwrap();
    let _ = root.class_list().add_1("hydrated");
    scroll_state(&root);
    counters();
    copy_buttons();
    toggles();
    toc();
    let fine = window()
        .match_media("(pointer: fine)")
        .ok()
        .flatten()
        .is_some_and(|m| m.matches());
    if fine && !reduced_motion() {
        cursor();
        magnetic();
        spotlight();
    }
}

fn observe(
    selector: &str,
    options: &IntersectionObserverInit,
    f: impl Fn(&IntersectionObserverEntry, &IntersectionObserver) + 'static,
) {
    let callback = Closure::<dyn Fn(js_sys::Array, IntersectionObserver)>::new(
        move |entries: js_sys::Array, io: IntersectionObserver| {
            for entry in entries.iter() {
                if let Ok(entry) = entry.dyn_into::<IntersectionObserverEntry>() {
                    f(&entry, &io);
                }
            }
        },
    );
    if let Ok(io) =
        IntersectionObserver::new_with_options(callback.as_ref().unchecked_ref(), options)
    {
        for el in all(selector) {
            io.observe(&el);
        }
    }
    callback.forget();
}

/// Header style and visibility, progress bar, and the manifesto words lighting up.
fn scroll_state(root: &Element) {
    let root: HtmlElement = root.clone().unchecked_into();
    let scrubs = if reduced_motion() {
        Vec::new()
    } else {
        all("[data-scrub]")
    };
    for el in &scrubs {
        let _ = el.class_list().add_1("scrubbing");
    }
    let last = Rc::new(Cell::new(0.0));
    let ticking = Rc::new(Cell::new(false));
    let update = move || {
        let w = window();
        let y = w.scroll_y().unwrap_or(0.0);
        let height = w
            .inner_height()
            .ok()
            .and_then(|h| h.as_f64())
            .unwrap_or(800.0);
        let _ = root.class_list().toggle_with_force("scrolled", y > 40.0);
        let _ = root
            .class_list()
            .toggle_with_force("hide-header", y > last.get() && y > 400.0);
        last.set(y);
        let max = f64::from(root.scroll_height()) - height;
        let style = root.style();
        let _ = style.set_property(
            "--progress",
            &fixed(if max > 0.0 { y / max } else { 0.0 }, 4),
        );
        let _ = style.set_property("--scroll", &fixed(y, 0));
        for el in &scrubs {
            let rect = el.get_bounding_client_rect();
            let p = (height * 0.85 - rect.top()) / (rect.height() + height * 0.45);
            let _ = el.style().set_property("--p", &fixed(p.clamp(0.0, 1.0), 4));
        }
    };
    update();
    let update = Rc::new(update);
    on_passive(&window(), "scroll", move |_: web_sys::Event| {
        if ticking.replace(true) {
            return;
        }
        let (update, ticking) = (update.clone(), ticking.clone());
        request_frame(move |_| {
            ticking.set(false);
            update();
            false
        });
    });
}

/// Numbers count up when they come into view (`data-count`, `data-decimals`).
fn counters() {
    let comma = window()
        .document()
        .and_then(|d| d.document_element())
        .and_then(|h| h.get_attribute("lang"))
        .is_some_and(|l| l != "en");
    let options = IntersectionObserverInit::new();
    options.set_threshold(&JsValue::from_f64(0.3));
    observe("[data-count]", &options, move |entry, io| {
        if !entry.is_intersecting() {
            return;
        }
        let el: HtmlElement = entry.target().unchecked_into();
        io.unobserve(&el);
        let target = el.dataset().get("count").map_or(0.0, |v| parse_decimal(&v));
        let decimals: u32 = el
            .dataset()
            .get("decimals")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let format = move |v: f64| {
            let s = fixed(v, decimals);
            if comma { s.replace('.', ",") } else { s }
        };
        if reduced_motion() {
            el.set_text_content(Some(&format(target)));
            return;
        }
        let start = Cell::new(None::<f64>);
        request_frame(move |now| {
            let t0 = *start.get().get_or_insert(now);
            start.set(Some(t0));
            let p = ((now - t0) / 1400.0).min(1.0);
            el.set_text_content(Some(&format(target * (1.0 - (1.0 - p).powi(4)))));
            p < 1.0
        });
    });
}

/// "Copy" buttons of code blocks.
fn copy_buttons() {
    let doc = window().document().unwrap();
    on(&doc, "click", |e: web_sys::MouseEvent| {
        let Some(button) = e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|t| t.closest(".copy").ok().flatten())
        else {
            return;
        };
        let code = button
            .parent_element()
            .and_then(|p| p.query_selector("pre").ok().flatten())
            .and_then(|p| p.text_content());
        let Some(code) = code else { return };
        let promise = window().navigator().clipboard().write_text(&code);
        wasm_bindgen_futures::spawn_local(async move {
            if wasm_bindgen_futures::JsFuture::from(promise).await.is_ok() {
                let _ = button.class_list().add_1("done");
                let done = Closure::once_into_js(move || {
                    let _ = button.class_list().remove_1("done");
                });
                let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
                    done.unchecked_ref(),
                    1400,
                );
            }
        });
    });
}

/// Mobile menu and docs sidebar.
fn toggles() {
    for (button, owner, class) in [
        (".burger", ".header", "open"),
        (".sidebar-toggle", ".sidebar", "open"),
    ] {
        for el in all(button) {
            let toggle = el.clone();
            on(&el, "click", move |_: web_sys::Event| {
                let Some(owner) = toggle.closest(owner).ok().flatten() else {
                    return;
                };
                let open = owner.class_list().toggle(class).unwrap_or(false);
                let _ = toggle.set_attribute("aria-expanded", if open { "true" } else { "false" });
            });
        }
    }
    // A link of the mobile menu closes it.
    for link in all(".nav .link") {
        on(&link, "click", |_: web_sys::Event| {
            for header in all(".header") {
                let _ = header.class_list().remove_1("open");
            }
        });
    }
}

/// "On this page": highlights the heading being read.
fn toc() {
    let links = all(".toc a");
    if links.is_empty() {
        return;
    }
    let options = IntersectionObserverInit::new();
    options.set_root_margin("-15% 0px -70% 0px");
    observe(".prose h2[id], .prose h3[id]", &options, move |entry, _| {
        if !entry.is_intersecting() {
            return;
        }
        let id = entry.target().id();
        for link in &links {
            let active = link.get_attribute("href").as_deref() == Some(&format!("#{id}"));
            let _ = link.class_list().toggle_with_force("active", active);
        }
    });
}

/// A dot and a trailing ring follow the pointer; they grow over links and buttons.
fn cursor() {
    let Some(el) = all(".cursor").into_iter().next() else {
        return;
    };
    let state = Rc::new(Cell::new((0.0f64, 0.0f64, 0.0f64, 0.0f64))); // x, y, trailing x, trailing y
    let s = state.clone();
    let target = el.clone();
    on_passive(&window(), "pointermove", move |e: web_sys::PointerEvent| {
        let (_, _, cx, cy) = s.get();
        s.set((e.client_x() as f64, e.client_y() as f64, cx, cy));
        let _ = target.class_list().add_1("on");
        let hover = e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|t| {
                t.closest("a, button, input, textarea, [data-cursor]")
                    .ok()
                    .flatten()
            })
            .is_some();
        let _ = target.class_list().toggle_with_force("hover", hover);
    });
    let target = el.clone();
    on(
        &window().document().unwrap(),
        "pointerleave",
        move |_: web_sys::Event| {
            let _ = target.class_list().remove_1("on");
        },
    );
    for (event, add) in [("pointerdown", true), ("pointerup", false)] {
        let target = el.clone();
        on(&window(), event, move |_: web_sys::Event| {
            let _ = target.class_list().toggle_with_force("down", add);
        });
    }
    request_frame(move |_| {
        let (x, y, mut cx, mut cy) = state.get();
        cx += (x - cx) * 0.2;
        cy += (y - cy) * 0.2;
        state.set((x, y, cx, cy));
        let style = el.style();
        let _ = style.set_property("--x", &format!("{}px", x as i32));
        let _ = style.set_property("--y", &format!("{}px", y as i32));
        let _ = style.set_property("--cx", &format!("{}px", cx as i32));
        let _ = style.set_property("--cy", &format!("{}px", cy as i32));
        true
    });
}

/// Buttons lean towards the pointer.
fn magnetic() {
    for el in all("[data-magnetic]") {
        let target = el.clone();
        on(&el, "pointermove", move |e: web_sys::PointerEvent| {
            let r = target.get_bounding_client_rect();
            let dx = (e.client_x() as f64 - (r.left() + r.width() / 2.0)) / r.width();
            let dy = (e.client_y() as f64 - (r.top() + r.height() / 2.0)) / r.height();
            let _ = target.style().set_property(
                "transform",
                &format!(
                    "translate({}px, {}px)",
                    (dx * 12.0) as i32,
                    (dy * 10.0) as i32
                ),
            );
        });
        let target = el.clone();
        on(&el, "pointerleave", move |_: web_sys::Event| {
            let _ = target.style().remove_property("transform");
        });
    }
}

/// Cards light up around the pointer.
fn spotlight() {
    on_passive(&window(), "pointermove", |e: web_sys::PointerEvent| {
        let card = e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|t| t.closest("[data-spot]").ok().flatten())
            .and_then(|c| c.dyn_into::<HtmlElement>().ok());
        if let Some(card) = card {
            let r = card.get_bounding_client_rect();
            let _ = card
                .style()
                .set_property("--mx", &format!("{}px", e.client_x() - r.left() as i32));
            let _ = card
                .style()
                .set_property("--my", &format!("{}px", e.client_y() - r.top() as i32));
        }
    });
}
