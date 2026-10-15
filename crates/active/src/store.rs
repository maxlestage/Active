//! Stores that outlive the page: [`persisted`] keeps a state in the browser's
//! `localStorage` (Svelte's `persisted` stores, VueUse's `useLocalStorage`, Zustand's
//! `persist`), in sync between the visitor's tabs.

use crate::data::Data;
use crate::reactive::{State, use_state};

/// Where [`persisted_in`] keeps a state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Storage {
    /// `localStorage`: kept after the browser is closed, shared by the tabs of the site.
    #[default]
    Local,
    /// `sessionStorage`: kept while the tab is open (a reload included), one per tab.
    Session,
}

/// A [`State`] kept in `localStorage` under `key`: a theme, a cart, the last search, a
/// draft. Its first value is the stored one (or `initial` when nothing valid is stored);
/// every change is written back as JSON (see [`Data`]), and a change made in another tab of
/// the site updates it here too.
///
/// On the server, and where storage is unavailable (private mode, blocked cookies), it is a
/// plain state holding `initial`: the page is rendered with it, and the browser shows the
/// stored value once it hydrates.
///
/// ```
/// use active::{html::*, persisted, Node};
///
/// let dark = persisted("theme-dark", false);
/// let toggle = button()
///     .on_click(move |_| dark.update(|d| *d = !*d))
///     .text_dyn(move || if dark.get() { "Light".into() } else { "Dark".into() });
/// assert_eq!(Node::from(toggle).render(), "<button><!--#-->Dark</button>");
/// ```
pub fn persisted<T: Data + 'static>(key: impl Into<String>, initial: T) -> State<T> {
    persisted_in(Storage::Local, key, initial)
}

/// [`persisted`] in the [`Storage`] of your choice (`sessionStorage` for what should not
/// outlive the tab: a wizard's step, a scroll position).
pub fn persisted_in<T: Data + 'static>(
    storage: Storage,
    key: impl Into<String>,
    initial: T,
) -> State<T> {
    let key = key.into();
    #[cfg(feature = "client")]
    {
        let session = storage == Storage::Session;
        let stored = crate::client::storage_get(session, &key).and_then(|j| T::read_data(&j));
        let state = use_state(stored.unwrap_or(initial));
        let written = key.clone();
        crate::reactive::effect(move || {
            let json = state.with(T::to_data);
            crate::client::storage_set(session, &written, &json);
        });
        if !session {
            crate::client::on_storage(key, move |json| {
                let Some(value) = json.and_then(|j| T::read_data(&j)) else {
                    return;
                };
                if state.is_alive() && state.with(|v| v.to_data() != value.to_data()) {
                    state.set(value);
                }
            });
        }
        state
    }
    #[cfg(not(feature = "client"))]
    {
        let _ = (storage, key);
        use_state(initial)
    }
}
