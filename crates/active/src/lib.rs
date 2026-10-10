#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

mod document;
mod escape;
mod event;
mod island;
mod reactive;
mod seo;
mod view;

#[cfg(feature = "client")]
mod client;

pub use document::Document;
pub use event::{Dom, Event};
pub use island::{IslandFn, island};
pub use reactive::{Scope, State, effect, scope, use_state};
pub use seo::Seo;
pub use view::{Element, Node, Str, el, fragment, fragment_dyn, html, keyed, text, text_dyn};

#[cfg(feature = "client")]
pub use client::{hydrate, location, mount, mount_to, navigate, query, query_param, update_head};

/// Everything a component file usually needs: `use active::prelude::*;`.
pub mod prelude {
    pub use crate::html::*;
    pub use crate::{
        Document, Element, Event, Node, Seo, State, effect, el, fragment, fragment_dyn, island,
        keyed, text, text_dyn, use_state,
    };
    #[cfg(feature = "client")]
    pub use crate::{location, navigate, query_param};
}
