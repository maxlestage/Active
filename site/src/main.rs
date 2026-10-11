//! `cargo run -p active-site`: the web server (build the browser bundle first, see
//! site/README.md).
//!
//! `cargo run -p active-site -- export dist`: the whole site as static files in `dist/`,
//! for GitHub Pages or any static host. `BASE_PATH` (e.g. `/Active`) prefixes every link,
//! `SITE_URL` (e.g. `https://you.github.io/Active`) makes the absolute URLs.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("export") => {
            let dir = args.next().unwrap_or_else(|| "dist".into());
            let base = std::env::var("BASE_PATH").unwrap_or_default();
            let site_url = std::env::var("SITE_URL").ok();
            let runtime = vitesse::tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            match runtime.block_on(active_site::server::export::export(
                std::path::Path::new(&dir),
                &base,
                site_url,
            )) {
                Ok(files) => {
                    println!("{files} files written to {dir}");
                    Ok(())
                }
                Err(error) => {
                    eprintln!("export failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!("unknown command {other:?}: run without arguments, or `export <dir>`");
            std::process::exit(2);
        }
        None => active_site::server::run(),
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
