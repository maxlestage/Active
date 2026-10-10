//! `cargo run -p active-site` — build the browser bundle first (see site/README.md).

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    active_site::server::run().await;
}

#[cfg(target_arch = "wasm32")]
fn main() {}
