---
title: Static sites
description: Generate every page of an active site as HTML files at build time, with a sitemap and robots.txt, host them anywhere for free, and keep the islands interactive.
---

When your pages do not depend on the request (a blog, documentation, a portfolio, a landing page), you do not need a server at all. A small Rust program renders every page to an HTML file once, at build time, and any static host serves the files. Pages keep their complete SEO tags, and [islands](../islands/) still hydrate in the browser, because hydration only needs the HTML and the bundle.

## How it works

```text
cargo build --lib --target wasm32-unknown-unknown …   →  pkg/            the islands' bundle
cargo run --release                                    →  dist/index.html
                                                          dist/posts/hello-rust/index.html
                                                          dist/pkg/…, dist/sitemap.xml, dist/robots.txt
```

The crate layout is the usual one ([Installation](../installation/#one-crate-two-targets)): pages and islands in `src/lib.rs`, and `src/main.rs` is now a generator instead of a server. It needs no dependencies besides `active` (and `wasm-bindgen` for the browser).

## The pages

```rust
use active::prelude::*;

/// Public address of the site, without a trailing slash.
pub const ORIGIN: &str = "https://example.com";
/// Path the site is published under: "" at the root of a domain, "/my-blog" for a
/// GitHub Pages project site (https://<user>.github.io/my-blog/).
pub const BASE: &str = "";

pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

pub static POSTS: [Post; 2] = [
    Post { slug: "hello-rust", title: "Hello, Rust", summary: "Why this blog is written in Rust." },
    Post { slug: "islands", title: "Islands", summary: "Interactive parts on a static page." },
];

/// A link inside the site, with the base path.
fn url(path: &str) -> String {
    format!("{BASE}{path}")
}

/// Island: a like button, interactive once the bundle has loaded.
pub fn likes(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(0));
    button().on_click(move |_| n.update(|n| *n += 1)).text_dyn(move || format!("♥ {}", n.get())).into()
}

fn document(content: Element) -> String {
    let body = div()
        .child(header().child(a().href(url("/")).text("My blog")))
        .child(main().child(content));
    Document::new(body)
        .lang("en")
        .seo(
            Seo::new()
                .site_name("My blog")
                .title_template("%s · My blog")
                .base_url(format!("{ORIGIN}{BASE}")),
        )
        .stylesheet(url("/static/app.css"))
        .client(url("/pkg/my_blog.js"), url("/pkg/my_blog_bg.wasm"))
        .render()
}

fn home() -> String {
    document(
        section()
            .child(Seo::new().title("Home").description("Notes about Rust and the web.").canonical("/"))
            .child(h1().text("My blog"))
            .child(ul().children(POSTS.iter().map(|post| {
                li().child(a().href(url(&format!("/posts/{}/", post.slug))).text(post.title))
            }))),
    )
}

fn post_page(post: &Post) -> String {
    document(
        article()
            .child(
                Seo::new()
                    .title(post.title)
                    .description(post.summary)
                    .canonical(format!("/posts/{}/", post.slug))
                    .og_type("article"),
            )
            .child(h1().text(post.title))
            .child(p().text(post.summary))
            .child(island("likes", "0", likes)),
    )
}

/// The 404 page that static hosts show for unknown URLs.
pub fn not_found() -> String {
    document(
        section()
            .child(Seo::new().title("Page not found").noindex(true))
            .child(h1().text("Page not found")),
    )
}

/// Every page of the site, as (path, HTML).
pub fn pages() -> Vec<(String, String)> {
    let mut pages = vec![("/".to_string(), home())];
    for post in &POSTS {
        pages.push((format!("/posts/{}/", post.slug), post_page(post)));
    }
    pages
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("likes", likes)]);
}
```

Each page is rendered with its own `Document`, so each gets its own title, description and canonical URL. Paths end with `/` and are written as `index.html` files in a folder of the same name, which every static host serves at that clean URL.

## The generator

`src/main.rs` writes the pages, copies the bundle and static files, and adds the sitemap and `robots.txt`:

```rust
use std::fs;
use std::io;
use std::path::Path;

use my_blog::{BASE, ORIGIN};

fn main() -> io::Result<()> {
    let out = Path::new("dist");
    if out.exists() {
        fs::remove_dir_all(out)?;
    }

    // 1. One index.html per page.
    let pages = my_blog::pages();
    for (path, html) in &pages {
        let dir = out.join(path.trim_start_matches('/'));
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }
    fs::write(out.join("404.html"), my_blog::not_found())?;

    // 2. The islands' bundle and the static files.
    copy_dir(Path::new("pkg"), &out.join("pkg"))?;
    copy_dir(Path::new("static"), &out.join("static"))?;

    // 3. Sitemap and robots.txt, with absolute URLs.
    let mut sitemap = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for (path, _) in &pages {
        sitemap.push_str(&format!("<url><loc>{ORIGIN}{BASE}{path}</loc></url>"));
    }
    sitemap.push_str("</urlset>\n");
    fs::write(out.join("sitemap.xml"), sitemap)?;
    fs::write(
        out.join("robots.txt"),
        format!("User-agent: *\nAllow: /\n\nSitemap: {ORIGIN}{BASE}/sitemap.xml\n"),
    )?;

    println!("{} pages written to dist/", pages.len());
    Ok(())
}

/// Copies the files of `from` (recursively) into `to`. A missing `from` is skipped.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(from) else {
        return Ok(());
    };
    fs::create_dir_all(to)?;
    for entry in entries {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
```

## Build

Build the bundle first, then run the generator:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
cargo run --release
```

`dist/` now contains the whole site. To look at it locally, serve the folder with any static file server, for example:

```sh
python3 -m http.server --directory dist 8000
```

and open [http://localhost:8000](http://localhost:8000). (Opening the files directly with `file://` does not work: browsers refuse to load WebAssembly modules from local files.)

A site without islands needs no bundle: drop `.client(...)`, the `start` function and the first two commands.

## Sitemap and robots.txt

Search engines discover pages through `sitemap.xml`, which lists every URL with its absolute address, and find the sitemap through `robots.txt`. The generator above writes both from the same list of pages, so they never get out of sync. Submit the sitemap's address in [Google Search Console](https://search.google.com/search-console) and Bing Webmaster Tools to speed up indexing.

For a multilingual site, add an `<xhtml:link rel="alternate" hreflang="…">` element per language inside each `<url>`, as this website's server does in `site/src/server/mod.rs`.

## Hosting

`dist/` is plain files: any static host works, often for free.

### GitHub Pages

1. In the repository, open **Settings**, **Pages**, and set **Source** to **GitHub Actions**.
2. Set `BASE` to `"/<repository>"` (or `""` with a custom domain or a `<user>.github.io` repository), and `ORIGIN` to `https://<user>.github.io`.
3. Add this workflow as `.github/workflows/pages.yml`:

```yaml
name: Pages

on:
  push:
    branches: [master]

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
      - run: |
          cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
          wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
          cargo run --release
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Every push to `master` rebuilds and publishes the site. This works from the GitHub app too: edit a file, commit, and the site updates a few minutes later.

### Netlify, Cloudflare Pages and others

Build `dist/` in CI (as above) and deploy the folder with the host's action or command-line tool, or drag and drop `dist/` in the host's dashboard for a one-off deploy. These hosts serve `404.html` for unknown URLs and set the `application/wasm` content type automatically.

## Installable and offline

A static site can also be installed on the home screen and keep working offline: write `manifest.webmanifest` and `sw.js` next to the pages with `active::pwa`, and add `.manifest(...)` and `.service_worker(...)` to each `Document`. See [Installable apps (PWA)](../pwa/#static-sites-and-base-paths).

## Updating the content

The site changes only when you rebuild it. Content usually lives in the repository (Rust data as above, or Markdown files read by the generator with a crate such as [`pulldown-cmark`](https://docs.rs/pulldown-cmark) and inserted with `.html(...)`), so a commit triggers a rebuild through CI. When pages must change per request or per visitor, use a server instead: see [Axum](../axum/).
