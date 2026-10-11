//! The web server (Vitesse): pages in three languages, docs search, static files, and the
//! web app manifest and service worker that make the site an installable app.

pub mod assets;
pub mod docs;
pub mod export;
pub mod highlight;
pub mod markdown;
pub mod pages;
pub mod pwa;
pub mod snippets;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vitesse::SameSite;
use vitesse::prelude::*;

use crate::routes::{Lang, Page, Route};
use assets::Assets;
use docs::Docs;

/// Bench numbers shown on the home page (bench/RESULTS.md).
pub const BENCH_SIZE: pages::Bars = &[
    ("Active", 36.8, true, false),
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
    ("Active", 5.58, true, false),
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
    /// Version of the service worker: a hash of everything it saves ([`pwa::version`]).
    pub pwa_version: String,
}

impl Site {
    /// Reads docs, static files and the browser bundle from `dir` (the `site/` folder);
    /// the public address comes from `SITE_URL`.
    pub fn load(dir: &Path) -> Site {
        Site::load_with(dir, std::env::var("SITE_URL").ok())
    }

    /// Like [`load`](Site::load), with the public address given (`None`: taken from each
    /// request). The base path (`routes::set_base`) is appended to it if missing.
    pub fn load_with(dir: &Path, site_url: Option<String>) -> Site {
        let mut assets = Assets::default();
        assets.load(&dir.join("static"), "/static");
        assets.load(&dir.join("pkg"), "/pkg");
        let mut site = Site {
            docs: Docs::load(&dir.join("docs")),
            assets,
            pwa_version: String::new(),
            site_url: site_url
                .map(|u| u.trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty())
                .map(|u| {
                    let base = crate::routes::base();
                    if u.ends_with(base) {
                        u
                    } else {
                        format!("{u}{base}")
                    }
                }),
        };
        site.pwa_version = pwa::version(&site);
        site
    }

    /// Every page of the site, in every language.
    pub fn routes(&self) -> Vec<Route> {
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
            routes.extend(self.docs.slugs().iter().map(|s| Route {
                lang,
                page: Page::Doc(s.to_string()),
            }));
        }
        routes
    }

    /// The public address of the site: `SITE_URL`, else the one the request was sent to.
    fn origin(&self, req: &Request) -> String {
        if let Some(url) = &self.site_url {
            return url.clone();
        }
        let header = |name: &str| {
            req.header(name)
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

/// `/`: the home page of the visitor's language.
fn root(_site: &Site, req: &Request) -> Response {
    let accept = req.header("accept-language").unwrap_or("");
    Redirect::to(crate::routes::link(&format!(
        "/{}/",
        pick_lang(accept).code()
    )))
    .into_response()
    .header("vary", "Accept-Language")
    .header("cache-control", "no-cache")
}

/// Every page: `/{lang}/`, `/{lang}/docs/`, `/{lang}/docs/{slug}/`, or a 404.
fn page(site: &Site, req: &Request) -> Response {
    let path = req.path();
    let accept = req.header("accept-language").unwrap_or("");
    let route = Route::parse(path, site.docs.slugs()).unwrap_or(Route {
        lang: pick_lang(accept),
        page: Page::NotFound,
    });
    if route.page != Page::NotFound && !path.ends_with('/') {
        let query = req
            .query_string()
            .map(|q| format!("?{q}"))
            .unwrap_or_default();
        return Redirect::permanent(format!("{path}/{query}")).into_response();
    }
    html(site, &route, req)
}

/// `/offline/`: the page the service worker shows for a page not saved on the device.
fn offline(site: &Site, req: &Request) -> Response {
    let route = Route {
        lang: Lang::En,
        page: Page::Offline,
    };
    html(site, &route, req)
}

/// The HTML page of `route` (404 for `Page::NotFound`).
fn html(site: &Site, route: &Route, req: &Request) -> Response {
    // The opening curtain plays on the first home page of a browser session.
    let seen = req.cookie("active-intro") == Some("1");
    let intro = route.page == Page::Home && !seen;
    let origin = site.origin(req);
    let html = pages::render(&pages::Ctx {
        site,
        route,
        origin: &origin,
        intro,
    });
    let status = if route.page == Page::NotFound {
        404
    } else {
        200
    };
    let response = res::status(status)
        .header("cache-control", "no-cache")
        .html(html);
    if intro {
        response.cookie(
            Cookie::new("active-intro", "1")
                .path("/")
                .same_site(SameSite::Lax),
        )
    } else {
        response
    }
}

/// `/{lang}/manifest.webmanifest`: name, icons and colors of the installed app, which
/// opens in that language (`/manifest.webmanifest`: English).
fn manifest(_site: &Site, req: &Request) -> Response {
    let lang = match req.param("lang") {
        None => Lang::En,
        Some(code) => match Lang::parse(code) {
            Some(lang) => lang,
            None => return res::status(404),
        },
    };
    res::send(pwa::manifest(lang).to_json())
        .header("content-type", "application/manifest+json")
        .header("cache-control", "no-cache")
}

/// `/sw.js`: the offline service worker. Never cached by HTTP, so that browsers see a new
/// version as soon as it is deployed.
fn service_worker(site: &Site, _req: &Request) -> Response {
    res::send(pwa::worker(site).to_js())
        .header("content-type", "text/javascript; charset=utf-8")
        .header("cache-control", "no-cache")
}

/// `/search/{lang}.txt`: the search index of a language, one entry per line (see
/// [`crate::search::Entry`]). The browser downloads it once and searches locally.
fn search_index(site: &Site, req: &Request) -> Response {
    let file = req.param("file").unwrap_or("");
    let Some(lang) = file.strip_suffix(".txt").and_then(Lang::parse) else {
        return res::status(404);
    };
    let body: String = site
        .docs
        .search_index(lang)
        .iter()
        .map(|e| e.line())
        .collect();
    res::text(body).header("cache-control", "public, max-age=300")
}

/// A static file by its URL. Versioned URLs (`?v=`) never change: browsers keep them a year.
fn asset(site: &Site, url: &str, req: &Request) -> Response {
    match site.assets.get(url) {
        Some(a) => {
            let cache = if req.query_string().is_some_and(|q| q.starts_with("v=")) {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=3600"
            };
            res::send(a.bytes.clone())
                .header("content-type", a.content_type)
                .header("cache-control", cache)
        }
        None => res::status(404),
    }
}

/// `/static/…` and `/pkg/…`.
fn files(site: &Site, req: &Request) -> Response {
    asset(site, req.path(), req)
}

/// The files browsers look for at the root: the favicon, the Open Graph image, the Apple
/// touch icon.
fn root_file(site: &Site, req: &Request) -> Response {
    asset(site, &format!("/static{}", req.path()), req)
}

fn sitemap(site: &Site, req: &Request) -> Response {
    let origin = site.origin(req);
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">\n",
    );
    for route in site.routes() {
        xml.push_str(&format!("<url><loc>{origin}{}</loc>", route.path()));
        for lang in Lang::ALL {
            xml.push_str(&format!(
                r#"<xhtml:link rel="alternate" hreflang="{}" href="{origin}{}"/>"#,
                lang.code(),
                route.with_lang(lang).path()
            ));
        }
        xml.push_str(&format!(
            r#"<xhtml:link rel="alternate" hreflang="x-default" href="{origin}{}"/></url>"#,
            route.with_lang(Lang::En).path()
        ));
        xml.push('\n');
    }
    xml.push_str("</urlset>\n");
    res::send(xml).header("content-type", "application/xml; charset=utf-8")
}

fn robots(site: &Site, req: &Request) -> Response {
    let origin = site.origin(req);
    res::text(format!(
        "User-agent: *\nAllow: /\n\nSitemap: {origin}/sitemap.xml\n"
    ))
}

/// Routes a handler that only needs the site and the request.
fn route(app: &mut App, path: &str, site: &Arc<Site>, handler: fn(&Site, &Request) -> Response) {
    let site = site.clone();
    app.get(path, move |req: Request| {
        let site = site.clone();
        async move { handler(&site, &req) }
    });
}

/// The application, ready to serve or to test: every response compressed for the browsers
/// that accept it and sent with the usual security headers.
pub fn app(site: Arc<Site>) -> App {
    let mut app = App::new();
    app.middleware(middleware::compress());
    app.middleware(middleware::helmet());
    route(&mut app, "/", &site, root);
    route(&mut app, "/search/:file", &site, search_index);
    route(&mut app, "/sitemap.xml", &site, sitemap);
    route(&mut app, "/robots.txt", &site, robots);
    route(&mut app, "/manifest.webmanifest", &site, manifest);
    route(&mut app, "/:lang/manifest.webmanifest", &site, manifest);
    route(&mut app, "/sw.js", &site, service_worker);
    route(&mut app, "/offline", &site, offline);
    route(&mut app, "/static/*path", &site, files);
    route(&mut app, "/pkg/*path", &site, files);
    for file in ["/favicon.svg", "/og.png", "/apple-touch-icon.png"] {
        route(&mut app, file, &site, root_file);
    }
    app.fallback(move |req: Request| {
        let site = site.clone();
        async move { page(&site, &req) }
    });
    app
}

/// The `site/` folder: `SITE_DIR`, or the crate folder when run with cargo.
pub fn site_dir() -> PathBuf {
    std::env::var_os("SITE_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

/// `cargo run -p active-site`: the site on `PORT` (Heroku) or 3000, on every core, stopped
/// by Ctrl+C locally and by SIGTERM on Heroku.
pub fn run() -> io::Result<()> {
    crate::routes::set_base(&std::env::var("BASE_PATH").unwrap_or_default());
    let site = Arc::new(Site::load(&site_dir()));
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000u16);
    println!("active site on http://localhost:{port}");
    app(site).run(port)
}
