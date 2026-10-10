//! `active-site export <dir>`: the whole site as static files, for GitHub Pages or any
//! static host. Every file comes from the server's own handlers, so both are identical.
//!
//! What a static host cannot do is replaced: the root page links the languages instead of
//! reading `Accept-Language`, the opening curtain is left out (it plays once per visit
//! thanks to a cookie), and `404.html` is the English "not found" page.
//!
//! The site stays an installable app: `manifest.webmanifest`, `sw.js` and the offline page
//! are written too, every URL in them behind the base path.

use std::io;
use std::path::Path;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

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
        let html = get(&site, &route.path(), StatusCode::OK).await?;
        write(&format!("{}index.html", route.path()), &html)?;
    }
    write(
        "404.html",
        &get(&site, "/en/404/", StatusCode::NOT_FOUND).await?,
    )?;
    let origin = site
        .site_url
        .clone()
        .unwrap_or_else(|| "http://localhost".into());
    write("index.html", pages::language_picker(&origin).as_bytes())?;
    for lang in Lang::ALL {
        let index = format!("/search/{}.txt", lang.code());
        write(&index, &get(&site, &index, StatusCode::OK).await?)?;
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
        write(&path, &get(&site, file, StatusCode::OK).await?)?;
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

/// The body of `GET path`, which must answer `expected`.
async fn get(site: &Arc<Site>, path: &str, expected: StatusCode) -> io::Result<Vec<u8>> {
    let request = Request::get(path)
        .header("host", "localhost")
        // The curtain plays once per visit thanks to this cookie: never in static pages.
        .header("cookie", "active-intro=1")
        .body(Body::empty())
        .map_err(io::Error::other)?;
    let response = app(site.clone())
        .oneshot(request)
        .await
        .map_err(io::Error::other)?;
    if response.status() != expected {
        return Err(io::Error::other(format!(
            "GET {path}: {} instead of {expected}",
            response.status()
        )));
    }
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(io::Error::other)?;
    Ok(bytes.to_vec())
}
