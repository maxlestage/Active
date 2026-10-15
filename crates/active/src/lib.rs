#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

mod context;
mod document;
mod escape;
mod event;
mod island;
pub mod pwa;
mod reactive;
mod router;
mod seo;
mod view;

#[cfg(feature = "client")]
mod client;

/// The version of this crate, e.g. `"0.1.0"`: handy for a footer or an `X-Powered-By` header.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use context::{expect_context, provide, use_context};
pub use document::Document;
pub use event::{Dom, Event};
pub use island::{IslandFn, island};
pub use reactive::{
    Scope, State, Watcher, effect, memo, on_cleanup, scope, untrack, use_state, watch,
    watch_immediate,
};
pub use router::{Params, Routes, match_path};
pub use seo::Seo;
pub use view::{
    Element, Node, Str, el, fragment, fragment_dyn, html, indexed, keyed, switch, text, text_dyn,
};

#[cfg(feature = "client")]
pub use client::{hydrate, location, mount, mount_to, navigate, query, query_param, update_head};

/// Everything a component file usually needs: `use active::prelude::*;`.
pub mod prelude {
    pub use crate::html::*;
    pub use crate::{
        Document, Element, Event, Node, Seo, State, effect, el, expect_context, fragment,
        fragment_dyn, indexed, island, keyed, match_path, memo, on_cleanup, provide, switch, text,
        text_dyn, untrack, use_context, use_state, watch, watch_immediate,
    };
    #[cfg(feature = "client")]
    pub use crate::{location, navigate, query_param};
}
