//! The active website: islands shared by the server and the browser, the Axum server
//! (native) and the browser bundle (wasm32).

pub mod i18n;
pub mod islands;
pub mod labels;
pub mod routes;
pub mod search;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

#[cfg(target_arch = "wasm32")]
mod client;
