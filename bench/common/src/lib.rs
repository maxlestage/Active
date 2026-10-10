//! What every benchmark shares, so that all frameworks render exactly the same thing and are
//! measured exactly the same way.
//!
//! - [`Product`]: the data of the SSR page (11 SEO tags, one JSON-LD script, 30 list items).
//! - [`check_page`]: verifies a rendered page, tag by tag.
//! - [`bench_ssr`]: the timing loop of every `ssr-*` binary.
//! - [`counter_page`], [`COUNTER_START`], [`mark`]: the counter app used for the WASM size and
//!   browser benchmarks.

mod check;
mod counter;
mod page;
mod timing;

pub use check::check_page;
pub use counter::{COUNTER_START, bundle_js, bundle_wasm, counter_page, mark};
pub use page::{OPTIONS, Product, SITE, TITLE_TEMPLATE, TWITTER, option_label, script_safe};
pub use timing::bench_ssr;
