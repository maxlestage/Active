//! Static files and the browser bundle, read once at startup and served from memory.

use std::collections::HashMap;
use std::path::Path;

pub struct Asset {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    /// Short content hash, used in `?v=` so files can be cached forever.
    pub version: String,
}

#[derive(Default)]
pub struct Assets {
    files: HashMap<String, Asset>,
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

/// FNV-1a, enough to tell versions apart.
pub fn hash(bytes: &[u8]) -> String {
    let h = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{h:016x}")[..10].to_string()
}

impl Assets {
    /// Loads every file of `dir` (recursively) under the URL prefix `prefix`.
    pub fn load(&mut self, dir: &Path, prefix: &str) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                self.load(&path, &format!("{prefix}/{name}"));
            } else if let Ok(bytes) = std::fs::read(&path) {
                let url = format!("{prefix}/{name}");
                let content_type = content_type(&name);
                let version = hash(&bytes);
                self.files.insert(
                    url,
                    Asset {
                        bytes,
                        content_type,
                        version,
                    },
                );
            }
        }
    }

    pub fn get(&self, url: &str) -> Option<&Asset> {
        self.files.get(url)
    }

    /// The link to `url`, behind the base path: `url?v=<hash>` for files that exist.
    pub fn url(&self, url: &str) -> String {
        let link = crate::routes::link(url);
        match self.files.get(url) {
            Some(asset) => format!("{link}?v={}", asset.version),
            None => link,
        }
    }

    /// Every file, with its URL (static export).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Asset)> {
        self.files.iter().map(|(url, asset)| (url.as_str(), asset))
    }
}
