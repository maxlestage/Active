//! A dependency-free server for the client-side app: `cargo run -p spa`, then open
//! http://localhost:3000. Build the browser bundle first (see README.md).
//!
//! Every path gets the same page; the app's router shows the right view.
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}")).expect("bind");
    println!("http://localhost:{port}");
    let shell = spa::shell();
    for stream in listener.incoming().flatten() {
        let shell = shell.clone();
        std::thread::spawn(move || {
            let mut stream = stream;
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                return;
            }
            let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, kind, body) = match path.strip_prefix("/pkg/") {
                Some(file) if !file.contains("..") => {
                    let kind = if file.ends_with(".wasm") {
                        "application/wasm"
                    } else {
                        "text/javascript"
                    };
                    match std::fs::read(
                        concat!(env!("CARGO_MANIFEST_DIR"), "/pkg/").to_string() + file,
                    ) {
                        Ok(bytes) => ("200 OK", kind, bytes),
                        Err(_) => ("404 Not Found", "text/plain", b"not found".to_vec()),
                    }
                }
                _ => ("200 OK", "text/html; charset=utf-8", shell.into_bytes()),
            };
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        });
    }
}
