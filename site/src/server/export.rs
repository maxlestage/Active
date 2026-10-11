//! `active-site export <dir>`: the whole site as static files, for GitHub Pages or any
//! static host. Every file comes from the server's own handlers, called in memory through
//! Vitesse's test client, so both are identical.
//!
//! What a static host cannot do is replaced: the root page and `404.html` go to the visitor's
//! language with a script (and link the languages), the opening curtain is left out (it plays
//! once per visit thanks to a cookie), and each language has its "not found" page.
//!
//! The site stays an installable app: `manifest.webmanifest`, `sw.js` and the offline page
//! are written too, every URL in them behind the base path.

use std::io;
use std::path::Path;
use std::sync::Arc;

use vitesse::test::TestClient;

use super::{Site, app, pages, site_dir};
use crate::routes::{Lang, set_base};

/// Writes the site into `out`, links behind `base` (`""` or `/Active`), absolute URLs
/// (canonical, sitemap, Open Graph) on `site_url`. Returns the number of files written.
pub async fn export(out: &Path, base: &str, site_url: Option<String>) -> io::Result<usize> {
    set_base(base);
    let site = Arc::new(Site::load_with(&site_dir(), site_url));
    if site.assets.get("/pkg/active_site_bg.wasm").is_none() {
        eprintln!(
            "warning: site/pkg is missing, the pages will not be interactive (see site/README.md)"
        );
    }
    let client = TestClient::new(app(site.clone()));
    let get = |path: &str, expected: u16| {
        let client = &client;
        let path = path.to_string();
        async move {
            // The curtain plays once per visit thanks to this cookie: never in static pages.
            let response = client
                .get(&path)
                .header("host", "localhost")
                .header("cookie", "active-intro=1")
                .await;
            if response.status().as_u16() != expected {
                return Err(io::Error::other(format!(
                    "GET {path}: {} instead of {expected}",
                    response.status()
                )));
            }
            Ok(response.bytes().to_vec())
        }
    };
    let mut written = 0;
    let mut write = |path: &str, bytes: &[u8]| -> io::Result<()> {
        let file = out.join(path.trim_start_matches('/'));
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file, bytes)?;
        written += 1;
        Ok(())
    };

    for route in site.routes() {
        let html = get(&route.path(), 200).await?;
        write(&format!("{}index.html", route.path()), &html)?;
    }
    let origin = site
        .site_url
        .clone()
        .unwrap_or_else(|| "http://localhost".into());
    // The visitor's language, chosen in the browser: at the root and on a missing address
    // (the static host serves `404.html`), then the "not found" page of each language.
    write("index.html", pages::language_picker(&origin, "").as_bytes())?;
    write(
        "404.html",
        pages::language_picker(&origin, "404/").as_bytes(),
    )?;
    for lang in Lang::ALL {
        let path = format!("/{}/404/", lang.code());
        write(&format!("{path}index.html"), &get(&path, 404).await?)?;
    }
    for lang in Lang::ALL {
        let index = format!("/search/{}.txt", lang.code());
        write(&index, &get(&index, 200).await?)?;
    }
    for file in [
        "/sitemap.xml",
        "/robots.txt",
        "/manifest.webmanifest",
        "/sw.js",
        "/offline/",
    ] {
        let path = if file.ends_with('/') {
            format!("{file}index.html")
        } else {
            file.to_string()
        };
        write(&path, &get(file, 200).await?)?;
    }
    for (url, asset) in site.assets.iter() {
        write(url, &asset.bytes)?;
    }
    // Served at the root by the server too.
    for file in ["favicon.svg", "og.png", "apple-touch-icon.png"] {
        if let Some(asset) = site.assets.get(&format!("/static/{file}")) {
            write(file, &asset.bytes)?;
        }
    }
    // Files are served as they are: no Jekyll processing on GitHub Pages.
    write(".nojekyll", b"")?;
    Ok(written)
}
