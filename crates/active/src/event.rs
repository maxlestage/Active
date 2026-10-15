//! Browser events, with the same API on the server (where handlers never run), so one
//! component compiles for both targets.

/// The DOM element type given to [`on_mount`](crate::Element::on_mount): `web_sys::Element`
/// in the browser, an empty placeholder on the server.
#[cfg(feature = "client")]
pub type Dom = web_sys::Element;

/// The DOM element type given to [`on_mount`](crate::Element::on_mount): `web_sys::Element`
/// in the browser, an empty placeholder on the server.
#[cfg(not(feature = "client"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Dom;

/// A DOM event passed to handlers registered with [`on`](crate::Element::on).
#[derive(Debug, Clone)]
pub struct Event {
    #[cfg(feature = "client")]
    pub(crate) raw: web_sys::Event,
}

impl Event {
    /// The underlying `web_sys::Event`, for anything this wrapper does not cover.
    #[cfg(feature = "client")]
    pub fn raw(&self) -> &web_sys::Event {
        &self.raw
    }

    /// Cancels the browser's default action (following a link, submitting a form…).
    pub fn prevent_default(&self) {
        #[cfg(feature = "client")]
        self.raw.prevent_default();
    }

    /// Stops the event from reaching the listeners of the parent elements (Vue's `.stop`).
    pub fn stop_propagation(&self) {
        #[cfg(feature = "client")]
        self.raw.stop_propagation();
    }

    /// Whether the checkbox or radio button that fired the event is checked (`false` for any
    /// other target, and on the server).
    pub fn checked(&self) -> bool {
        #[cfg(feature = "client")]
        {
            use wasm_bindgen::JsCast;
            if let Some(input) = self
                .raw
                .target()
                .as_ref()
                .and_then(|t| t.dyn_ref::<web_sys::HtmlInputElement>())
            {
                return input.checked();
            }
        }
        false
    }

    /// The current value of the `<input>`, `<textarea>` or `<select>` that fired the event.
    pub fn value(&self) -> String {
        #[cfg(feature = "client")]
        {
            use wasm_bindgen::JsCast;
            let target = self.raw.target();
            if let Some(input) = target
                .as_ref()
                .and_then(|t| t.dyn_ref::<web_sys::HtmlInputElement>())
            {
                return input.value();
            }
            if let Some(area) = target
                .as_ref()
                .and_then(|t| t.dyn_ref::<web_sys::HtmlTextAreaElement>())
            {
                return area.value();
            }
            if let Some(select) = target
                .as_ref()
                .and_then(|t| t.dyn_ref::<web_sys::HtmlSelectElement>())
            {
                return select.value();
            }
        }
        String::new()
    }

    /// The key of a keyboard event (`"Enter"`, `"ArrowDown"`, `"k"`…), empty otherwise.
    pub fn key(&self) -> String {
        #[cfg(feature = "client")]
        {
            use wasm_bindgen::JsCast;
            if let Some(k) = self.raw.dyn_ref::<web_sys::KeyboardEvent>() {
                return k.key();
            }
        }
        String::new()
    }

    /// The element the listener is attached to.
    pub fn current_target(&self) -> Option<Dom> {
        #[cfg(feature = "client")]
        let target = {
            use wasm_bindgen::JsCast;
            self.raw
                .current_target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        };
        #[cfg(not(feature = "client"))]
        let target = None;
        target
    }
}
