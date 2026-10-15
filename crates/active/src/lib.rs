#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

mod boundary;
mod context;
mod document;
mod escape;
mod event;
pub mod forms;
mod island;
pub mod pwa;
mod reactive;
mod resource;
mod router;
mod seo;
mod view;

#[cfg(feature = "client")]
mod client;

/// The version of this crate, e.g. `"0.1.0"`: handy for a footer or an `X-Powered-By` header.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use boundary::{ErrorBoundary, error_boundary, report_error};
pub use context::{expect_context, provide, use_context};
pub use document::Document;
pub use event::{Dom, Event};
pub use island::{Hydrate, IslandFn, island, island_with};
pub use reactive::{
    Scope, State, Watcher, batch, effect, memo, on_cleanup, scope, untrack, use_state, watch,
    watch_immediate,
};
pub use resource::{Resource, resource, suspense};
pub use router::{Navigation, Params, Routes, before_navigate, match_path};
pub use seo::Seo;
pub use view::{
    Element, Node, Str, el, fragment, fragment_dyn, html, indexed, keyed, portal, switch, text,
    text_dyn,
};

#[cfg(feature = "client")]
pub use client::{
    hydrate, location, mount, mount_to, navigate, navigate_replace, query, query_param, update_head,
};

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
