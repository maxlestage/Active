//! The web server (Axum): pages in three languages, docs search, static files.

pub mod assets;
pub mod docs;
pub mod highlight;
pub mod markdown;
pub mod pages;
pub mod snippets;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use tower_http::compression::CompressionLayer;

use crate::routes::{Lang, Page, Route};
use assets::Assets;
use docs::Docs;

/// Bench numbers shown on the home page (bench/RESULTS.md).
pub const BENCH_SIZE: pages::Bars = &[
    ("active", 36.8, true, false),
    ("sycamore", 45.8, false, false),
    ("leptos", 60.9, false, false),
    ("yew", 96.3, false, false),
    ("dioxus", 170.4, false, false),
];
/// Server render of the same product page, µs (bench/RESULTS.md).
pub const BENCH_SSR: pages::Bars = &[
    ("maud", 1.37, false, false),
    ("write! (baseline)", 2.01, false, true),
    ("leptos (manual <head>)", 5.19, false, false),
    ("active", 5.58, true, false),
    ("leptos + leptos_meta", 13.44, false, false),
    ("yew", 17.17, false, false),
    ("dioxus", 24.22, false, false),
    ("sycamore", 33.01, false, false),
];

pub struct Site {
    pub docs: Docs,
    pub assets: Assets,
    /// Public origin if configured (`SITE_URL`), else taken from each request.
    pub site_url: Option<String>,
}

impl Site {
    /// Reads docs, static files and the browser bundle from `dir` (the `site/` folder).
    pub fn load(dir: &Path) -> Site {
        let mut assets = Assets::default();
        assets.load(&dir.join("static"), "/static");
        assets.load(&dir.join("pkg"), "/pkg");
        Site {
            docs: Docs::load(&dir.join("docs")),
            assets,
            site_url: std::env::var("SITE_URL")
                .ok()
                .map(|u| u.trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty()),
        }
    }

    fn origin(&self, headers: &HeaderMap) -> String {
        if let Some(url) = &self.site_url {
            return url.clone();
        }
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.split(',').next().unwrap_or("").trim().to_string())
        };
        let host = header("x-forwarded-host")
            .or_else(|| header("host"))
            .unwrap_or_else(|| "localhost".into());
        if !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
        {
            return "http://localhost".into();
        }
        let proto = if header("x-forwarded-proto").as_deref() == Some("https") {
            "https"
        } else {
            "http"
        };
        format!("{proto}://{host}")
    }
}

/// Preferred supported language from `Accept-Language`, English by default.
pub fn pick_lang(header: &str) -> Lang {
    let mut ranked: Vec<(Lang, f32)> = header
        .split(',')
        .filter_map(|part| {
            let mut pieces = part.trim().split(";q=");
            let tag = pieces.next()?.get(..2)?.to_ascii_lowercase();
            let q = pieces
                .next()
                .and_then(|q| q.trim().parse().ok())
                .unwrap_or(1.0);
            Some((Lang::parse(&tag)?, q))
        })
        .filter(|(_, q)| *q > 0.0)
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked.first().map_or(Lang::En, |r| r.0)
}

fn security(headers: &mut HeaderMap, https: bool) {
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("SAMEORIGIN"));
    if https {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000"),
        );
    }
}

fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .is_some_and(|v| v == "https")
}

type Shared = State<Arc<Site>>;

async fn root(headers: HeaderMap) -> Response {
    let accept = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut response = (
        StatusCode::FOUND,
        [(header::LOCATION, format!("/{}/", pick_lang(accept).code()))],
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::VARY, HeaderValue::from_static("Accept-Language"));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    security(response.headers_mut(), is_https(&headers));
    response
}

/// Every page: `/{lang}/`, `/{lang}/docs/`, `/{lang}/docs/{slug}/`, or a 404.
async fn page(State(site): Shared, uri: Uri, headers: HeaderMap) -> Response {
    let path = uri.path();
    let accept = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let route = Route::parse(path, site.docs.slugs()).unwrap_or(Route {
        lang: pick_lang(accept),
        page: Page::NotFound,
    });
    if route.page != Page::NotFound && !path.ends_with('/') {
        let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
        return (
            StatusCode::MOVED_PERMANENTLY,
            [(header::LOCATION, format!("{path}/{query}"))],
        )
            .into_response();
    }
    // The opening curtain plays on the first home page of a browser session.
    let seen = headers
        .get(header::COOKIE)
        .and_then(|c| c.to_str().ok())
        .is_some_and(|c| c.contains("active-intro=1"));
    let intro = route.page == Page::Home && !seen;
    let origin = site.origin(&headers);
    let html = pages::render(&pages::Ctx {
        site: &site,
        route: &route,
        origin: &origin,
        intro,
    });
    let status = if route.page == Page::NotFound {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::OK
    };
    let mut response = (status, html).into_response();
    let h = response.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if intro {
        h.insert(
            header::SET_COOKIE,
            HeaderValue::from_static("active-intro=1; Path=/; SameSite=Lax"),
        );
    }
    security(h, is_https(&headers));
    response
}

/// Value of `key` in a query string, percent-decoded.
fn query_param(query: &str, key: &str) -> Option<String> {
    let value = query.split('&').find_map(|pair| {
        pair.split_once('=')
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v)
    })?;
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                match u8::from_str_radix(
                    std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("zz"),
                    16,
                ) {
                    Ok(b) => {
                        out.push(b);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    Some(String::from_utf8_lossy(&out).into_owned())
}

/// `/api/search?lang=fr&q=…`: one result per line, `href \t title \t context`.
async fn search(State(site): Shared, RawQuery(query): RawQuery) -> Response {
    let query = query.unwrap_or_default();
    let lang = query_param(&query, "lang")
        .as_deref()
        .and_then(Lang::parse)
        .unwrap_or(Lang::En);
    let body: String = site
        .docs
        .search(lang, query_param(&query, "q").as_deref().unwrap_or(""))
        .into_iter()
        .map(|(href, title, sub)| {
            format!(
                "{href}\t{}\t{}\n",
                title.replace(['\t', '\n'], " "),
                sub.replace(['\t', '\n'], " ")
            )
        })
        .collect();
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=300"),
        ],
        body,
    )
        .into_response()
}

fn asset(site: &Site, url: &str, uri: &Uri) -> Response {
    match site.assets.get(url) {
        Some(a) => {
            let cache = if uri.query().is_some_and(|q| q.starts_with("v=")) {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=3600"
            };
            (
                [
                    (header::CONTENT_TYPE, a.content_type),
                    (header::CACHE_CONTROL, cache),
                ],
                a.bytes.clone(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn files(State(site): Shared, uri: Uri) -> Response {
    asset(&site, uri.path(), &uri)
}

async fn root_file(State(site): Shared, uri: Uri) -> Response {
    asset(&site, &format!("/static{}", uri.path()), &uri)
}

async fn sitemap(State(site): Shared, headers: HeaderMap) -> Response {
    let origin = site.origin(&headers);
    let mut routes = Vec::new();
    for lang in Lang::ALL {
        routes.push(Route {
            lang,
            page: Page::Home,
        });
        routes.push(Route {
            lang,
            page: Page::Docs,
        });
        routes.extend(site.docs.slugs().iter().map(|s| Route {
            lang,
            page: Page::Doc(s.to_string()),
        }));
    }
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for route in routes {
        xml.push_str(&format!("<url><loc>{origin}{}</loc>", route.href()));
        for lang in Lang::ALL {
            xml.push_str(&format!(
                r#"<xhtml:link rel="alternate" hreflang="{}" href="{origin}{}"/>"#,
                lang.code(),
                route.with_lang(lang).href()
            ));
        }
        xml.push_str(&format!(
            r#"<xhtml:link rel="alternate" hreflang="x-default" href="{origin}{}"/></url>"#,
            route.with_lang(Lang::En).href()
        ));
        xml.push('\n');
    }
    xml.push_str("</urlset>\n");
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

async fn robots(State(site): Shared, headers: HeaderMap) -> Response {
    let origin = site.origin(&headers);
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!("User-agent: *\nAllow: /\n\nSitemap: {origin}/sitemap.xml\n"),
    )
        .into_response()
}

/// The application, ready to serve or to test.
pub fn app(site: Arc<Site>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/api/search", get(search))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
        .route("/static/{*path}", get(files))
        .route("/pkg/{*path}", get(files))
        .route("/favicon.svg", get(root_file))
        .route("/og.png", get(root_file))
        .route("/apple-touch-icon.png", get(root_file))
        .fallback(get(page))
        .layer(CompressionLayer::new())
        .with_state(site)
}

/// The `site/` folder: `SITE_DIR`, or the crate folder when run with cargo.
pub fn site_dir() -> PathBuf {
    std::env::var_os("SITE_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

/// Ctrl+C locally, SIGTERM on Heroku.
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = term.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

pub async fn run() {
    let site = Arc::new(Site::load(&site_dir()));
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000u16);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind");
    println!("active site on http://localhost:{port}");
    axum::serve(listener, app(site))
        .with_graceful_shutdown(shutdown())
        .await
        .expect("server");
}
