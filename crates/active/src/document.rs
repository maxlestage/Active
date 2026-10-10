//! Full HTML pages: `<head>` with merged SEO tags, stylesheets and the WebAssembly loader.

use crate::escape::escape_into;
use crate::seo::{Seo, write_merged};
use crate::view::{Node, Str, collect_seo, render_collecting_seo, static_seo};

/// A complete HTML page.
///
/// Every [`Seo`] found in the body is merged into the page's `Seo` in tree order (the
/// deepest wins), so layouts and pages can each describe themselves without duplicates.
///
/// ```
/// use active::{html::{h1, main}, Document, Seo};
///
/// let site = Seo::new().site_name("Acme").title_template("%s · Acme");
/// let page = main().child(Seo::new().title("Contact")).child(h1().text("Contact"));
/// let html = Document::new(page).lang("en").seo(site).render();
/// assert!(html.starts_with(r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>"#));
/// assert!(html.contains("<title>Contact · Acme</title>"));
/// assert!(html.ends_with("<body><main><h1>Contact</h1></main></body></html>"));
/// ```
#[derive(Clone, Default)]
pub struct Document {
    lang: Option<Str>,
    viewport: Option<Str>,
    seo: Seo,
    stylesheets: Vec<Str>,
    head: Vec<Str>,
    client: Option<(Str, Str)>,
    html_attrs: Vec<(Str, Str)>,
    body_attrs: Vec<(Str, Str)>,
    body: Node,
}

impl Document {
    /// A page whose `<body>` is `body`.
    pub fn new(body: impl Into<Node>) -> Self {
        Self {
            body: body.into(),
            ..Self::default()
        }
    }

    /// `<html lang="…">`.
    #[must_use]
    pub fn lang(mut self, lang: impl Into<Str>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// The `content` of `<meta name="viewport">`, instead of the default
    /// `width=device-width, initial-scale=1` (e.g. add `viewport-fit=cover` to draw under the
    /// notch of a phone).
    #[must_use]
    pub fn viewport(mut self, content: impl Into<Str>) -> Self {
        self.viewport = Some(content.into());
        self
    }

    /// Base SEO tags, merged with the [`Seo`] values found in the body.
    #[must_use]
    pub fn seo(mut self, seo: Seo) -> Self {
        self.seo = seo;
        self
    }

    /// `<link rel="stylesheet" href="…"/>`.
    #[must_use]
    pub fn stylesheet(mut self, href: impl Into<Str>) -> Self {
        self.stylesheets.push(href.into());
        self
    }

    /// Trusted HTML appended to `<head>` (font preloads, `theme-color`, an icon…).
    #[must_use]
    pub fn head(mut self, html: impl Into<Str>) -> Self {
        self.head.push(html.into());
        self
    }

    /// Makes the page installable: `<link rel="manifest">` (serve
    /// [`Manifest::to_json`](crate::pwa::Manifest::to_json) at `href`), the theme color,
    /// the home screen title and the Apple touch icon (the largest PNG icon).
    #[must_use]
    pub fn manifest(self, href: &str, manifest: &crate::pwa::Manifest) -> Self {
        self.head(crate::pwa::head_tags(href, manifest))
    }

    /// Registers the service worker served at `url` (see
    /// [`ServiceWorker`](crate::pwa::ServiceWorker)) with a small inline script, once the
    /// page has loaded.
    #[must_use]
    pub fn service_worker(self, url: &str) -> Self {
        self.head(format!(
            "<script>{}</script>",
            crate::pwa::register_script(url)
        ))
    }

    /// Loads the browser bundle produced by `wasm-bindgen --target web --no-typescript`: `js` is the
    /// generated module, `wasm` the binary. Both are preloaded from `<head>`, so the
    /// binary downloads in parallel with the HTML instead of after the JavaScript glue.
    #[must_use]
    pub fn client(mut self, js: impl Into<Str>, wasm: impl Into<Str>) -> Self {
        self.client = Some((js.into(), wasm.into()));
        self
    }

    /// An attribute on `<html>` (e.g. `class`).
    #[must_use]
    pub fn html_attr(mut self, name: impl Into<Str>, value: impl Into<Str>) -> Self {
        self.html_attrs.push((name.into(), value.into()));
        self
    }

    /// An attribute on `<body>`.
    #[must_use]
    pub fn body_attr(mut self, name: impl Into<Str>, value: impl Into<Str>) -> Self {
        self.body_attrs.push((name.into(), value.into()));
        self
    }

    /// The merged SEO of the page: the document's `Seo`, then every `Seo` in the body.
    pub fn final_seo(&self) -> Seo {
        let mut found = Vec::new();
        collect_seo(&self.body, true, &mut found);
        found
            .iter()
            .fold(self.seo.clone(), |acc, seo| acc.merged(seo))
    }

    /// Renders the whole page.
    pub fn render(&self) -> String {
        crate::reactive::render_guard(|| {
            let mut layers = vec![&self.seo];
            if static_seo(&self.body, &mut layers) {
                // Every `Seo` is known before rendering: one pass, one buffer.
                let mut out = String::with_capacity(4096);
                self.write_head(&layers, &mut out);
                self.body.render_into(&mut out);
                out.push_str("</body></html>");
                out
            } else {
                // Dynamic regions: render the body first, collecting the `Seo` nodes met
                // (each region runs once), then the head.
                let mut body = String::with_capacity(4096);
                let found = render_collecting_seo(&self.body, &mut body);
                let layers: Vec<&Seo> = std::iter::once(&self.seo).chain(&found).collect();
                let mut out = String::with_capacity(body.len() + 2048);
                self.write_head(&layers, &mut out);
                out.push_str(&body);
                out.push_str("</body></html>");
                out
            }
        })
    }

    /// Everything before the body's content: doctype, `<head>` with the SEO `layers`
    /// merged, `<body …>`.
    fn write_head(&self, layers: &[&Seo], out: &mut String) {
        out.push_str("<!DOCTYPE html><html");
        if let Some(lang) = &self.lang {
            attr(out, "lang", lang);
        }
        for (name, value) in &self.html_attrs {
            attr(out, name, value);
        }
        out.push_str(r#"><head><meta charset="utf-8"/><meta name="viewport""#);
        attr(
            out,
            "content",
            self.viewport
                .as_deref()
                .unwrap_or("width=device-width, initial-scale=1"),
        );
        out.push_str("/>");
        write_merged(layers, out);
        out.push_str("<style>active-island{display:contents}</style>");
        for href in &self.stylesheets {
            out.push_str(r#"<link rel="stylesheet""#);
            attr(out, "href", href);
            out.push_str("/>");
        }
        if let Some((js, wasm)) = &self.client {
            out.push_str(r#"<link rel="modulepreload""#);
            attr(out, "href", js);
            out.push_str(r#"/><link rel="preload""#);
            attr(out, "href", wasm);
            out.push_str(r#" as="fetch" type="application/wasm" crossorigin=""/>"#);
            // The one line browsers need to start a WebAssembly module.
            out.push_str(r#"<script type="module">import init from "#);
            js_string(out, js);
            out.push_str(";init({module_or_path:");
            js_string(out, wasm);
            out.push_str("});</script>");
        }
        for html in &self.head {
            out.push_str(html);
        }
        out.push_str("</head><body");
        for (name, value) in &self.body_attrs {
            attr(out, name, value);
        }
        out.push('>');
    }
}

fn attr(out: &mut String, name: &str, value: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape_into(out, value);
    out.push('"');
}

/// A JavaScript string literal that cannot close the `<script>` element.
fn js_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '<' => out.push_str("\\u003c"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::{div, p};

    #[test]
    fn merges_every_seo_in_tree_order_without_duplicates() {
        let layout = Seo::new()
            .site_name("Acme")
            .title_template("%s · Acme")
            .description("Default")
            .noindex(true);
        let page = div()
            .child(Seo::new().title("Docs").description("All the docs"))
            .child(
                p().child(
                    Seo::new()
                        .title("Install")
                        .noindex(false)
                        .alternate("fr", "/fr/install"),
                ),
            );
        let html = Document::new(page).seo(layout).render();
        assert_eq!(html.matches("<title>").count(), 1);
        assert!(html.contains("<title>Install · Acme</title>"));
        assert_eq!(html.matches(r#"name="description""#).count(), 1);
        assert!(html.contains(r#"content="All the docs""#));
        assert!(!html.contains("robots"));
        assert!(html.contains(r#"hreflang="fr""#));
    }

    #[test]
    fn finds_the_seo_of_pages_shown_by_a_router() {
        let path = crate::use_state("/about".to_string());
        let app = div()
            .child(Seo::new().title_template("%s · Acme"))
            .children_dyn(move || match path.get().as_str() {
                "/about" => vec![p().child(Seo::new().title("About")).text("Us").into()],
                _ => vec![],
            });
        let html = Document::new(app).render();
        assert!(html.contains("<title>About · Acme</title>"));
        assert!(html.contains("<p>Us</p>"));
    }

    #[test]
    fn evaluates_each_dynamic_region_once() {
        let runs = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = runs.clone();
        let page = div().children_dyn(move || {
            counter.set(counter.get() + 1);
            vec![Seo::new().title("Once").into()]
        });
        let html = Document::new(page).render();
        assert!(html.contains("<title>Once</title>"));
        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn loads_the_browser_bundle_and_preloads_the_binary() {
        let html = Document::new(p().text("x"))
            .client("/pkg/app.js", "/pkg/app_bg.wasm")
            .stylesheet("/app.css")
            .render();
        assert!(html.contains(r#"<link rel="stylesheet" href="/app.css"/>"#));
        assert!(html.contains(r#"<link rel="modulepreload" href="/pkg/app.js"/>"#));
        assert!(html.contains(r#"<link rel="preload" href="/pkg/app_bg.wasm" as="fetch" type="application/wasm" crossorigin=""/>"#));
        assert!(html.contains(r#"<script type="module">import init from "/pkg/app.js";init({module_or_path:"/pkg/app_bg.wasm"});</script>"#));
    }

    #[test]
    fn the_viewport_can_be_replaced() {
        let default = Document::new(p().text("x")).render();
        assert!(
            default.contains(
                r#"<meta name="viewport" content="width=device-width, initial-scale=1"/>"#
            )
        );
        let cover = Document::new(p().text("x"))
            .viewport("width=device-width, initial-scale=1, viewport-fit=cover")
            .render();
        assert_eq!(cover.matches(r#"name="viewport""#).count(), 1);
        assert!(
            cover.contains(r#"content="width=device-width, initial-scale=1, viewport-fit=cover""#)
        );
    }

    #[test]
    fn script_urls_cannot_break_out() {
        let mut out = String::new();
        js_string(&mut out, "a\"</script><b>");
        assert_eq!(out, r#""a\"\u003c/script>\u003cb>""#);
    }
}
