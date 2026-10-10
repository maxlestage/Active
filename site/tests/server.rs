//! The production server, end to end (without the network): routes, SEO tags, caching.
#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use active_site::server::{Site, app, site_dir};
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode, header};
use tower::ServiceExt;

async fn get(path: &str, headers: &[(&str, &str)]) -> Response<Body> {
    let mut request = Request::get(path).header("host", "example.test");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let site = Arc::new(Site::load(&site_dir()));
    app(site)
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn text(response: Response<Body>) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 10_000_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

fn header_of(response: &Response<Body>, name: header::HeaderName) -> &str {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
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
        assert!(
            header_of(&response, "strict-transport-security".parse().unwrap()).contains("max-age")
        );
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
async fn sitemap_robots_and_search_use_absolute_urls() {
    let sitemap = text(get("/sitemap.xml", &[]).await).await;
    assert!(sitemap.contains("<loc>http://example.test/fr/docs/</loc>"));
    assert!(sitemap.contains(r#"hreflang="x-default""#));
    let robots = text(get("/robots.txt", &[]).await).await;
    assert!(robots.contains("Sitemap: http://example.test/sitemap.xml"));
    let search = text(get("/api/search?lang=en&q=", &[]).await).await;
    assert!(search.lines().count() >= 3);
    assert!(
        search
            .lines()
            .all(|l| l.starts_with("/en/docs/") && l.split('\t').count() == 3)
    );
}
