//! Baseline: the page written by hand into one `String`, every value escaped. No framework
//! can do less work, so the other results are shown relative to this one.

use std::fmt::Write;

use bench_common::{Product, SITE, TWITTER, bench_ssr, script_safe};

fn render(i: u32) -> String {
    let p = Product::new(i);
    let mut out = String::with_capacity(4096);
    out.push_str(r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/><title>"#);
    escape(&mut out, &p.name);
    out.push_str(" · ");
    escape(&mut out, SITE);
    out.push_str("</title>");
    meta(&mut out, "name", "description", &p.description);
    out.push_str(r#"<link rel="canonical" href=""#);
    escape(&mut out, &p.url);
    out.push_str(r#""/>"#);
    meta(&mut out, "property", "og:title", &p.name);
    meta(&mut out, "property", "og:description", &p.description);
    meta(&mut out, "property", "og:type", "website");
    meta(&mut out, "property", "og:url", &p.url);
    meta(&mut out, "property", "og:image", &p.image);
    meta(&mut out, "property", "og:site_name", SITE);
    meta(&mut out, "name", "twitter:card", "summary_large_image");
    meta(&mut out, "name", "twitter:site", TWITTER);
    out.push_str(r#"<script type="application/ld+json">"#);
    out.push_str(&script_safe(&p.json_ld));
    out.push_str("</script></head><body><h1>");
    escape(&mut out, &p.name);
    out.push_str("</h1><ul>");
    for (k, price) in p.options() {
        let _ = write!(out, "<li>Option {k} — {price} €</li>");
    }
    out.push_str("</ul></body></html>");
    out
}

fn meta(out: &mut String, attr: &str, key: &str, content: &str) {
    out.push_str("<meta ");
    out.push_str(attr);
    out.push_str("=\"");
    out.push_str(key);
    out.push_str("\" content=\"");
    escape(out, content);
    out.push_str(r#""/>"#);
}

fn escape(out: &mut String, s: &str) {
    let mut start = 0;
    for (i, byte) in s.bytes().enumerate() {
        let entity = match byte {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            b'\'' => "&#x27;",
            _ => continue,
        };
        out.push_str(&s[start..i]);
        out.push_str(entity);
        start = i + 1;
    }
    out.push_str(&s[start..]);
}

fn main() {
    bench_ssr(render);
}
