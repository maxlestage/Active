//! A minimal static file server for the browser benchmark: one thread per connection,
//! `Connection: close`, no caching.
//!
//! Pages are deliberately not cross-origin isolated: COOP/COEP headers would give
//! `performance.now()` a finer resolution (5 µs instead of 100 µs), but they make Chromium
//! swap to a fresh renderer process on every navigation, which roughly doubles the load
//! times and is not what typical sites do.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};

/// Serves the files of `root` on `127.0.0.1` (a free port); `/` is `index.html`.
pub fn serve(root: PathBuf) -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let root = root.clone();
            std::thread::spawn(move || {
                let _ = respond(stream, &root);
            });
        }
    });
    Ok(addr)
}

fn respond(mut stream: TcpStream, root: &Path) -> std::io::Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    // Read the headers too, so the browser never sees its request cut off.
    let mut header = String::new();
    while reader.read_line(&mut header)? > 2 {
        header.clear();
    }

    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let path = path.split('?').next().unwrap_or("/");
    let file = match path {
        "/" => root.join("index.html"),
        p if !p.contains("..") => root.join(p.trim_start_matches('/')),
        _ => PathBuf::new(),
    };
    let (status, body) = match std::fs::read(&file) {
        Ok(body) => ("200 OK", body),
        Err(_) => ("404 Not Found", b"not found".to_vec()),
    };
    let content_type = match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        _ => "text/plain",
    };
    let head = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}
