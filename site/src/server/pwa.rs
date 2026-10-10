//! The site as an installable app (PWA), built with `active::pwa`: the web app manifest
//! (`/manifest.webmanifest`) and the offline service worker (`/sw.js`), every URL behind
//! the base path.

use active::pwa::{Manifest, ServiceWorker};

use super::Site;
use super::assets::hash;
use super::pages;
use crate::i18n::t;
use crate::routes::{Lang, Page, Route, link};

/// Background of the site and of its icons: the status bar and the splash screen.
pub const THEME: &str = "#0b0b0c";

/// The icons of the manifest, in `site/static/` (rendered from `favicon.svg`).
const ICONS: [(&str, &str, bool); 3] = [
    ("/static/icon-192.png", "192x192", false),
    ("/static/icon-512.png", "512x512", false),
    ("/static/icon-maskable-512.png", "512x512", true),
];

/// The web app manifest. The app opens on the English home page: a page that answers 200
/// and is saved on the device (`/` is a redirect, which a service worker cannot replay
/// offline), with the language switch one tap away.
pub fn manifest() -> Manifest {
    let manifest = Manifest::new("active")
        .short_name("active")
        .description(t(Lang::En).home.description)
        .id(link("/"))
        .start_url(link("/en/"))
        .scope(link("/"))
        .theme_color(THEME)
        .background_color(THEME)
        .lang("en")
        .categories(["developer", "education"]);
    ICONS
        .iter()
        .fold(manifest, |manifest, &(src, sizes, maskable)| {
            if maskable {
                manifest.maskable_icon(link(src), sizes, "image/png")
            } else {
                manifest.icon(link(src), sizes, "image/png")
            }
        })
        .icon(link("/favicon.svg"), "any", "image/svg+xml")
}

/// The pages saved on the device when the worker installs: the three home pages (the
/// offline page is added by `offline_page`).
fn saved_pages() -> Vec<Route> {
    Lang::ALL
        .into_iter()
        .map(|lang| Route {
            lang,
            page: Page::Home,
        })
        .collect()
}

fn offline() -> Route {
    Route {
        lang: Lang::En,
        page: Page::Offline,
    }
}

/// Every URL the worker precaches: the home pages, then the files they load, with the
/// exact URLs the pages use (`?v=` included, the cache matches query strings): the
/// stylesheet, the WebAssembly bundle, the fonts (linked from the stylesheet without a
/// version) and the icons. Files that do not exist (no `site/pkg` yet) are left out, so
/// the installation never fails on a 404.
pub fn precache(site: &Site) -> Vec<String> {
    let mut urls: Vec<String> = saved_pages().iter().map(Route::href).collect();
    for file in [
        "/static/app.css",
        "/pkg/active_site.js",
        "/pkg/active_site_bg.wasm",
    ] {
        if site.assets.get(file).is_some() {
            urls.push(site.assets.url(file));
        }
    }
    let mut fonts: Vec<&str> = site
        .assets
        .iter()
        .map(|(url, _)| url)
        .filter(|url| url.starts_with("/static/fonts/") && url.ends_with(".woff2"))
        .collect();
    fonts.sort_unstable();
    urls.extend(fonts.into_iter().map(link));
    // The favicon is served at the root of the site, like in the pages.
    for (file, url) in ICONS
        .iter()
        .map(|(src, ..)| (*src, link(src)))
        .chain([("/static/favicon.svg", link("/favicon.svg"))])
    {
        if site.assets.get(file).is_some() {
            urls.push(url);
        }
    }
    urls
}

/// The version of the worker: a hash of everything it saves (the pages rendered, the
/// files) and of the manifest. Any change makes a new worker, which replaces the cache.
pub fn version(site: &Site) -> String {
    let mut all = manifest().to_json();
    let origin = site.site_url.as_deref().unwrap_or("http://localhost");
    for route in saved_pages().iter().chain([&offline()]) {
        all.push_str(&pages::render(&pages::Ctx {
            site,
            route,
            origin,
            intro: false,
        }));
    }
    for url in precache(site) {
        all.push('\n');
        all.push_str(&url);
    }
    let mut files: Vec<(&str, &str)> = site
        .assets
        .iter()
        .map(|(url, asset)| (url, asset.version.as_str()))
        .collect();
    files.sort_unstable();
    for (url, version) in files {
        all.push_str(&format!("\n{url} {version}"));
    }
    hash(all.as_bytes())
}

/// The service worker: pages from the network first (the saved copy offline, else the
/// offline page), files from the cache at once and refreshed in the background.
pub fn worker(site: &Site) -> ServiceWorker {
    ServiceWorker::new("active-site", &site.pwa_version)
        .precache(precache(site))
        .offline_page(offline().href())
}
