//! A dependency-free server: `cargo run -p hello`, then open http://localhost:3000.
//! Build the browser part first (see README.md). The page is installable: the server
//! also serves the web app manifest, the service worker and the offline page.
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use active::pwa::{ServiceWorker, version};

/// The files of the example, by their URL: the browser bundle (`/pkg/`) and the icons
/// (`/static/`).
fn file(url: &str) -> Option<Vec<u8>> {
    if url.contains("..") {
        return None;
    }
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR")).to_string() + url).ok()
}

fn content_type(url: &str) -> &'static str {
    match url.rsplit('.').next() {
        Some("wasm") => "application/wasm",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

/// The service worker: at installation it saves the page, the browser bundle and the
/// icons (those that exist: a missing file would cancel the installation), and shows
/// `/offline/` for anything else when the network is down. Its version is a hash of
/// what it saves: change a file, and the next visit installs a new worker.
fn worker() -> ServiceWorker {
    let mut urls = vec!["/".to_string()];
    let mut parts = vec![hello::page().into_bytes()];
    for url in [
        "/pkg/hello.js",
        "/pkg/hello_bg.wasm",
        "/static/icon.svg",
        "/static/icon-192.png",
        "/static/icon-512.png",
        "/static/icon-maskable-512.png",
    ] {
        if let Some(bytes) = file(url) {
            urls.push(url.to_string());
            parts.push(bytes);
        }
    }
    ServiceWorker::new("hello", version(&parts))
        .precache(urls)
        .offline_page("/offline/")
}

fn not_found() -> (&'static str, &'static str, Vec<u8>) {
    ("404 Not Found", "text/plain", b"not found".to_vec())
}

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).expect("bind");
    println!("http://localhost:{port}");
    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || {
            let mut stream = stream;
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                return;
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let html = "text/html; charset=utf-8";
            let (status, kind, body) = match path.as_str() {
                "/" => ("200 OK", html, hello::page().into_bytes()),
                "/offline/" => ("200 OK", html, hello::offline().into_bytes()),
                "/manifest.webmanifest" => (
                    "200 OK",
                    "application/manifest+json",
                    hello::manifest().to_json().into_bytes(),
                ),
                "/sw.js" => (
                    "200 OK",
                    "text/javascript; charset=utf-8",
                    worker().to_js().into_bytes(),
                ),
                p if p.starts_with("/pkg/") || p.starts_with("/static/") => match file(p) {
                    Some(bytes) => ("200 OK", content_type(p), bytes),
                    None => not_found(),
                },
                _ => not_found(),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\ncache-control: no-cache\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(&body));
        });
    }
}
