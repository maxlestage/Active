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
    let _ = std::fs::remove_dir_all(&dir);
}
