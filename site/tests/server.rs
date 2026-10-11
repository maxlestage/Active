//! The production server, end to end (without the network, through Vitesse's test
//! client): routes, SEO tags, caching.
#![cfg(not(target_arch = "wasm32"))]

use std::sync::{Arc, OnceLock};

use active_site::server::{Site, app, site_dir};
use vitesse::test::{TestClient, TestResponse};
use vitesse::{StatusCode, header};

/// The site, loaded once for every test.
fn site() -> Arc<Site> {
    static SITE: OnceLock<Arc<Site>> = OnceLock::new();
    SITE.get_or_init(|| Arc::new(Site::load(&site_dir())))
        .clone()
}

async fn get(path: &str, headers: &[(&str, &str)]) -> TestResponse {
    let client = TestClient::new(app(site()));
    let mut request = client.get(path).header("host", "example.test");
    for (name, value) in headers {
        request = request.header(name, value);
    }
    request.await
}

async fn text(response: TestResponse) -> String {
    response.text()
}

fn header_of(response: &TestResponse, name: impl AsRef<str>) -> &str {
    response.header(name.as_ref()).unwrap_or("")
}

#[tokio::test]
async fn root_redirects_to_the_visitor_language() {
    let response = get("/", &[("accept-language", "de-DE,es;q=0.8,en;q=0.5")]).await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(header_of(&response, header::LOCATION), "/es/");
    let response = get("/", &[]).await;
    assert_eq!(header_of(&response, header::LOCATION), "/en/");
}

#[tokio::test]
async fn home_pages_have_complete_seo_tags_in_every_language() {
    for lang in ["en", "fr", "es"] {
        let response = get(&format!("/{lang}/"), &[("x-forwarded-proto", "https")]).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(header_of(&response, "strict-transport-security").contains("max-age"));
        let html = text(response).await;
        assert!(
            html.starts_with(&format!(r#"<!DOCTYPE html><html lang="{lang}""#)),
            "{lang}"
        );
        assert_eq!(html.matches("<title>").count(), 1);
        assert!(html.contains(&format!(
            r#"<link rel="canonical" href="https://example.test/{lang}/"/>"#
        )));
        for alt in ["en", "fr", "es", "x-default"] {
            assert!(
                html.contains(&format!(r#"hreflang="{alt}""#)),
                "{lang} {alt}"
            );
        }
        assert!(
            html.contains(r#"<meta property="og:image" content="https://example.test/og.png"/>"#)
        );
        assert!(html.contains(r#"as="fetch" type="application/wasm""#));
        assert!(html.contains(r#"<active-island data-island="dust""#));
        // The playground output is rendered on the server by the same Rust code.
        assert!(html.contains(r#"<b class="tg">title</b>"#));
    }
}

#[tokio::test]
async fn every_page_ends_with_the_signature_footer() {
    for (lang, rights, here) in [
        ("en", "All rights reserved", "You are here"),
        ("fr", "Tous droits réservés", "Vous êtes ici"),
        ("es", "Todos los derechos reservados", "Estás aquí"),
    ] {
        let html = text(get(&format!("/{lang}/docs/"), &[]).await).await;
        let footer = html.split("<footer").nth(1).expect("a footer");
        // The author's signature, the legal line and the license.
        assert!(footer.contains(r#"class="credits-name""#), "{lang}");
        assert!(footer.contains("Maxime Nathan Lestage"), "{lang}");
        assert!(footer.contains(rights), "{lang}");
        assert!(footer.contains(r#"href="/static/LICENSE.txt""#), "{lang}");
        assert!(!html.contains("MIT"), "{lang}");
        // The source is on GitHub, the only way to get the crate until it is on crates.io.
        assert!(
            html.contains("https://github.com/maxlestage/Active"),
            "{lang}"
        );
        // The install command and the version of the crate.
        assert!(
            footer.contains("cargo add active --git https://github.com/maxlestage/Active"),
            "{lang}"
        );
        assert!(footer.contains(&format!("v{}", active::VERSION)), "{lang}");
        // The six projects of the ecosystem, this one marked as the current page.
        for project in [
            "Active",
            "Vitesse",
            "Securite",
            "Memoire",
            "Paquets",
            "FerroSwift",
        ] {
            assert!(
                footer.contains(&format!(
                    r#"href="https://maxlestage.github.io/{project}/""#
                )),
                "{lang} {project}"
            );
        }
        assert!(footer.contains(r#"aria-current="page""#), "{lang}");
        assert!(footer.contains(here), "{lang}");
    }
    // The author is named in the metadata too.
    let html = text(get("/en/", &[]).await).await;
    assert!(html.contains(r#"<meta name="author" content="Maxime Nathan Lestage"/>"#));
    assert!(html.contains(r#""author":{"@type":"Person","name":"Maxime Nathan Lestage""#));
    // The license is served by the site, identical to the repository's.
    let response = get("/static/LICENSE.txt", &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(header_of(&response, header::CONTENT_TYPE).starts_with("text/plain"));
    assert_eq!(text(response).await, include_str!("../../LICENSE"));
}

#[tokio::test]
async fn the_opening_curtain_plays_once_per_session() {
    let first = get("/en/", &[]).await;
    assert!(header_of(&first, header::SET_COOKIE).starts_with("active-intro=1"));
    assert!(
        text(first)
            .await
            .contains(r#"<html lang="en" class="intro">"#)
    );
    let again = get("/en/", &[("cookie", "active-intro=1")]).await;
    assert!(again.headers().get(header::SET_COOKIE).is_none());
    assert!(!text(again).await.contains(r#"class="intro""#));
}

#[tokio::test]
async fn docs_pages_render_with_their_navigation() {
    let index = text(get("/fr/docs/", &[]).await).await;
    assert!(index.contains(r#"<html lang="fr""#));
    let first_page = index
        .split(r#"href="/fr/docs/"#)
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .find(|slug| !slug.is_empty())
        .expect("a docs link");
    let page = get(&format!("/fr/docs/{first_page}"), &[]).await;
    assert_eq!(page.status(), StatusCode::OK);
    let html = text(page).await;
    assert!(html.contains(r#"<h1 class="doc-title">"#));
    assert!(html.contains(r#""@type":"BreadcrumbList""#));
    assert!(html.contains(r#"<meta property="og:type" content="article"/>"#));
}

#[tokio::test]
async fn directories_get_a_trailing_slash() {
    let response = get("/en/docs?x=1", &[]).await;
    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(header_of(&response, header::LOCATION), "/en/docs/?x=1");
}

#[tokio::test]
async fn unknown_pages_answer_404_in_the_right_language() {
    let response = get("/es/nope", &[]).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let html = text(response).await;
    assert!(html.contains(r#"<html lang="es""#));
    assert!(html.contains(r#"<meta name="robots" content="noindex,follow"/>"#));
    let unknown = get("/nope", &[("accept-language", "fr")]).await;
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    assert!(text(unknown).await.contains(r#"<html lang="fr""#));
}

#[tokio::test]
async fn versioned_assets_are_cached_forever_and_compressed() {
    let html = text(get("/en/", &[]).await).await;
    let css = html
        .split(r#"href="/static/app.css?v="#)
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap();
    let response = get(
        &format!("/static/app.css?v={css}"),
        &[("accept-encoding", "br, gzip")],
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(header_of(&response, header::CACHE_CONTROL).contains("immutable"));
    assert_eq!(header_of(&response, header::CONTENT_ENCODING), "br");
    let missing = get("/static/../Cargo.toml", &[]).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sitemap_and_robots_use_absolute_urls() {
    let sitemap = text(get("/sitemap.xml", &[]).await).await;
    assert!(sitemap.contains("<loc>http://example.test/fr/docs/</loc>"));
    assert!(sitemap.contains(r#"hreflang="x-default""#));
    let robots = text(get("/robots.txt", &[]).await).await;
    assert!(robots.contains("Sitemap: http://example.test/sitemap.xml"));
}

#[tokio::test]
async fn the_search_index_is_searched_like_in_the_browser() {
    let index = text(get("/search/fr.txt", &[]).await).await;
    let entries: Vec<_> = index
        .lines()
        .map(|l| active_site::search::Entry::parse(l).expect("an index line"))
        .collect();
    assert!(entries.iter().filter(|e| e.page).count() >= 20);
    assert!(entries.iter().all(|e| e.href.starts_with("/fr/docs/")));
    let hits = active_site::search::search(&entries, "déployer heroku");
    assert!(hits[0].href.ends_with("/deploy-heroku/"), "{:?}", hits[0]);
    assert_eq!(
        get("/search/de.txt", &[]).await.status(),
        StatusCode::NOT_FOUND
    );
}

/// Width and height of a PNG, from its IHDR chunk.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "a PNG file");
    let be = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[tokio::test]
async fn the_manifest_describes_the_installed_app() {
    let response = get("/manifest.webmanifest", &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_of(&response, header::CONTENT_TYPE),
        "application/manifest+json"
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&text(response).await).expect("valid JSON");
    assert_eq!(manifest["name"], "Active");
    // A page that answers 200 (`/` redirects), saved by the service worker.
    assert_eq!(manifest["start_url"], "/en/");
    // Each language has its manifest: the installed app opens in that language.
    for lang in ["fr", "es"] {
        let localized: serde_json::Value = serde_json::from_str(
            &text(get(&format!("/{lang}/manifest.webmanifest"), &[]).await).await,
        )
        .expect("valid JSON");
        assert_eq!(localized["start_url"], format!("/{lang}/"));
        assert_eq!(localized["lang"], lang);
    }
    assert_eq!(
        get("/de/manifest.webmanifest", &[]).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(manifest["scope"], "/");
    assert_eq!(manifest["display"], "standalone");
    assert_eq!(manifest["theme_color"], "#0b0b0c");
    let icons = manifest["icons"].as_array().expect("icons");
    for (size, purpose) in [
        ("192x192", "any"),
        ("512x512", "any"),
        ("512x512", "maskable"),
    ] {
        assert!(
            icons
                .iter()
                .any(|i| i["sizes"] == size && i["purpose"] == purpose && i["type"] == "image/png"),
            "{size} {purpose}"
        );
    }
    for icon in icons {
        let src = icon["src"].as_str().unwrap();
        let response = get(src, &[]).await;
        assert_eq!(response.status(), StatusCode::OK, "{src}");
        assert_eq!(
            header_of(&response, header::CONTENT_TYPE),
            icon["type"].as_str().unwrap()
        );
        if let Some((w, h)) = icon["sizes"].as_str().unwrap().split_once('x') {
            assert_eq!(
                png_size(response.bytes()),
                (w.parse().unwrap(), h.parse().unwrap()),
                "{src}"
            );
        }
    }
}

#[tokio::test]
async fn the_service_worker_saves_the_home_pages_and_what_they_load() {
    let response = get("/sw.js", &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header_of(&response, header::CONTENT_TYPE),
        "text/javascript; charset=utf-8"
    );
    assert_eq!(header_of(&response, header::CACHE_CONTROL), "no-cache");
    let js = text(response).await;
    let site = Site::load(&site_dir());
    assert!(js.contains(&format!(
        r#"const CACHE = "active-site-{}";"#,
        site.pwa_version
    )));
    assert!(js.contains(r#"const OFFLINE = "/offline/";"#));
    let list = js
        .lines()
        .find_map(|l| l.strip_prefix("const PRECACHE = "))
        .expect("the precache list");
    let urls: Vec<String> = serde_json::from_str(list.trim_end_matches(';')).expect("JSON");
    for url in [
        "/en/",
        "/fr/",
        "/es/",
        "/offline/",
        "/static/icon-192.png",
        "/static/icon-512.png",
        "/static/icon-maskable-512.png",
        "/favicon.svg",
    ] {
        assert!(urls.iter().any(|u| u == url), "{url} in {urls:?}");
    }
    assert!(
        urls.iter()
            .any(|u| u.starts_with("/static/fonts/") && u.ends_with(".woff2"))
    );
    // The exact URLs the pages load (the cache matches query strings).
    let html = text(get("/en/", &[("cookie", "active-intro=1")]).await).await;
    let mut files = vec!["/static/app.css"];
    if site_dir().join("pkg/active_site_bg.wasm").exists() {
        files.extend(["/pkg/active_site.js", "/pkg/active_site_bg.wasm"]);
    }
    for file in files {
        let versioned = site.assets.url(file);
        assert!(versioned.contains("?v="), "{versioned}");
        assert!(html.contains(&versioned), "{versioned} in the page");
        assert!(urls.contains(&versioned), "{versioned} precached");
    }
    // Every URL answers 200, or the installation would fail.
    for url in &urls {
        assert_eq!(get(url, &[]).await.status(), StatusCode::OK, "{url}");
    }
    // Same files, same version.
    assert_eq!(Site::load(&site_dir()).pwa_version, site.pwa_version);
}

#[tokio::test]
async fn every_page_is_installable() {
    for (path, lang) in [
        ("/en/", "en"),
        ("/fr/docs/", "fr"),
        ("/es/docs/pwa/", "es"),
        ("/en/nope", "en"),
        ("/offline/", "en"),
    ] {
        let html = text(get(path, &[]).await).await;
        let link = format!(r#"<link rel="manifest" href="/{lang}/manifest.webmanifest"/>"#);
        assert_eq!(html.matches(&link).count(), 1, "{path}");
        assert_eq!(
            html.matches(r##"<meta name="theme-color" content="#0b0b0c"/>"##)
                .count(),
            1,
            "{path}"
        );
        assert_eq!(
            html.matches(r#"<link rel="apple-touch-icon" href="/static/icon-512.png"/>"#)
                .count(),
            1,
            "{path}"
        );
        assert!(
            html.contains(r#"navigator.serviceWorker.register("/sw.js")"#),
            "{path}"
        );
    }
}

#[tokio::test]
async fn the_offline_page_speaks_the_three_languages() {
    let response = get("/offline/", &[]).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains(r#"<meta name="robots" content="noindex"#));
    assert!(html.contains("<title>You are offline. — Active</title>"));
    for (lang, text) in [
        ("en", "Home page"),
        ("fr", "Vous êtes hors ligne."),
        ("es", "Estás sin conexión."),
    ] {
        assert!(html.contains(text), "{text}");
        // Each language links its home page, which the worker always saves.
        assert!(html.contains(&format!(r#"href="/{lang}/" hreflang="{lang}""#)));
    }
    let sitemap = text(get("/sitemap.xml", &[]).await).await;
    assert!(!sitemap.contains("/offline/"));
}

#[tokio::test]
async fn the_pwa_guide_is_in_the_docs_in_every_language() {
    for (lang, title) in [
        ("en", "Installable apps (PWA)"),
        ("fr", "Applications installables (PWA)"),
        ("es", "Apps instalables (PWA)"),
    ] {
        let index = text(get(&format!("/{lang}/docs/"), &[]).await).await;
        assert!(
            index.contains(&format!(r#"href="/{lang}/docs/pwa/""#)),
            "{lang}"
        );
        let page = get(&format!("/{lang}/docs/pwa/"), &[]).await;
        assert_eq!(page.status(), StatusCode::OK);
        let html = text(page).await;
        assert!(
            html.contains(&format!("<title>{title} — Active docs</title>")),
            "{lang}"
        );
        assert!(html.contains(&format!(r#"<html lang="{lang}""#)));
    }
    let entries: Vec<_> = text(get("/search/fr.txt", &[]).await)
        .await
        .lines()
        .map(|l| active_site::search::Entry::parse(l).expect("an index line"))
        .collect();
    let hits = active_site::search::search(&entries, "application installable hors ligne");
    assert!(hits[0].href.starts_with("/fr/docs/pwa/"), "{:?}", hits[0]);
}
