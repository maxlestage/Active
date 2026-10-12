//! The static export for GitHub Pages: every page as a file, links behind the base path.
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use active_site::server::export::export;

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[tokio::test]
async fn exports_every_page_behind_the_base_path() {
    let dir = std::env::temp_dir().join(format!("active-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let files = export(&dir, "/Active/", Some("https://user.github.io".into()))
        .await
        .expect("export");
    assert!(files > 70, "{files} files");

    // Pages: links behind the base path, absolute URLs on the public address.
    let page = read(&dir, "fr/docs/seo/index.html");
    assert!(page.contains(r#"<html lang="fr" data-base="/Active">"#));
    assert!(
        page.contains(
            r#"<link rel="canonical" href="https://user.github.io/Active/fr/docs/seo/"/>"#
        )
    );
    assert!(page.contains(r#"href="/Active/fr/docs/quick-start/""#));
    assert!(page.contains(r#"href="/Active/static/app.css?v="#));
    assert!(
        page.contains(r#"import init from "/Active/pkg/active_site.js"#)
            || !dir.join("pkg").exists()
    );
    assert!(
        !page.contains(r#"href="/fr/"#),
        "a link without the base path"
    );
    assert!(!page.contains("class=\"intro\""));

    // What a static host cannot compute.
    let root = read(&dir, "index.html");
    assert!(root.contains(r#"content="0; url=/Active/en/""#));
    // The address people share most: its card has the image.
    assert!(
        root.contains(
            r#"<meta property="og:image" content="https://user.github.io/Active/og.png"/>"#
        )
    );
    assert!(read(&dir, "404.html").contains(r#"<meta name="robots" content="noindex,follow"/>"#));
    assert!(
        read(&dir, "sitemap.xml").contains("<loc>https://user.github.io/Active/es/docs/</loc>")
    );
    assert!(
        read(&dir, "search/es.txt")
            .lines()
            .all(|l| l.contains("\t/Active/es/docs/"))
    );
    assert!(dir.join("static/app.css").exists());
    assert!(dir.join("favicon.svg").exists());
    assert!(dir.join(".nojekyll").exists());

    // Still an installable app that works offline, every URL behind the base path.
    let exists = |url: &str| {
        let path = url
            .strip_prefix("/Active/")
            .unwrap_or_else(|| panic!("{url} without the base path"));
        let path = path.split('?').next().unwrap();
        let file = if path.is_empty() || path.ends_with('/') {
            format!("{path}index.html")
        } else {
            path.to_string()
        };
        assert!(dir.join(&file).is_file(), "{url}: no {file}");
    };
    let manifest: serde_json::Value =
        serde_json::from_str(&read(&dir, "manifest.webmanifest")).expect("valid JSON");
    assert_eq!(manifest["start_url"], "/Active/en/");
    let french: serde_json::Value =
        serde_json::from_str(&read(&dir, "fr/manifest.webmanifest")).expect("valid JSON");
    assert_eq!(french["start_url"], "/Active/fr/");
    assert_eq!(manifest["scope"], "/Active/");
    assert_eq!(manifest["id"], "/Active/");
    exists(manifest["start_url"].as_str().unwrap());
    for icon in manifest["icons"].as_array().unwrap() {
        exists(icon["src"].as_str().unwrap());
    }
    let worker = read(&dir, "sw.js");
    assert!(worker.contains(r#"const OFFLINE = "/Active/offline/";"#));
    let list = worker
        .lines()
        .find_map(|l| l.strip_prefix("const PRECACHE = "))
        .expect("the precache list");
    let precache: Vec<String> = serde_json::from_str(list.trim_end_matches(';')).expect("JSON");
    for url in [
        "/Active/en/",
        "/Active/fr/",
        "/Active/es/",
        "/Active/offline/",
    ] {
        assert!(precache.iter().any(|u| u == url), "{url}");
    }
    assert!(
        precache
            .iter()
            .any(|u| u.starts_with("/Active/static/app.css?v="))
    );
    for url in &precache {
        exists(url);
    }
    assert!(page.contains(r#"<link rel="manifest" href="/Active/fr/manifest.webmanifest"/>"#));
    assert!(page.contains(r#"navigator.serviceWorker.register("/Active/sw.js")"#));
    assert!(read(&dir, "offline/index.html").contains("Vous êtes hors ligne."));
    let _ = std::fs::remove_dir_all(&dir);
}
