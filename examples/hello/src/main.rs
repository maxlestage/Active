//! A dependency-free server: `cargo run -p hello`, then open http://localhost:3000.
//! Build the browser part first (see README.md).
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

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
            let (status, kind, body) = match path.as_str() {
                "/" => (
                    "200 OK",
                    "text/html; charset=utf-8",
                    hello::page().into_bytes(),
                ),
                p if p.starts_with("/pkg/") && !p.contains("..") => {
                    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/").to_string() + &p[5..];
                    let kind = if p.ends_with(".wasm") {
                        "application/wasm"
                    } else {
                        "text/javascript"
                    };
                    match std::fs::read(file) {
                        Ok(bytes) => ("200 OK", kind, bytes),
                        Err(_) => ("404 Not Found", "text/plain", b"not found".to_vec()),
                    }
                }
                _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream
                .write_all(head.as_bytes())
                .and_then(|()| stream.write_all(&body));
        });
    }
}
