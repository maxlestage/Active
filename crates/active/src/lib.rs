#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

use std::fmt::{self, Write};

/// SEO `<head>` tags for one page: title, description, robots, canonical, hreflang
/// alternates, Open Graph, Twitter and JSON-LD.
///
/// Build it with chained setters (each takes `self` by value and returns it), then
/// render with [`to_html`](Seo::to_html), [`write_html`](Seo::write_html) or
/// [`Display`](std::fmt::Display). Keep site-wide values in one `Seo` and
/// [`clone`](Clone::clone) it per page, like `createSeo(defaults)` in active-react.
///
/// The output is byte-for-byte what React 19 renders for active-react's `<Seo>`, except
/// that the attribute is spelled `hreflang` (React prints `hrefLang`) and JSON-LD scripts
/// come last instead of where the component sits in the body.
///
/// ```
/// use active::Seo;
///
/// let html = Seo::new().title("Home").description("Hello").to_html();
/// assert_eq!(
///     html,
///     r#"<title>Home</title><meta name="description" content="Hello"/><meta property="og:title" content="Home"/><meta property="og:description" content="Hello"/><meta property="og:type" content="website"/><meta name="twitter:card" content="summary"/>"#
/// );
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[must_use = "`Seo` is a builder: setters return the updated value"]
pub struct Seo {
    title: Option<String>,
    title_template: Option<String>,
    description: Option<String>,
    canonical: Option<String>,
    image: Option<String>,
    og_type: Option<String>,
    site_name: Option<String>,
    locale: Option<String>,
    twitter: Option<String>,
    base_url: Option<String>,
    noindex: bool,
    nofollow: bool,
    alternates: Vec<(String, String)>,
    json_ld: Vec<String>,
    meta: Vec<Meta>,
}

/// An extra `<meta>` tag: `<meta {attr}="{key}" content="{content}"/>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Meta {
    attr: &'static str,
    key: String,
    content: String,
}

impl Seo {
    /// An empty `Seo`. Renders only `og:type` (`website`) and `twitter:card` (`summary`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Page title (`<title>` and `og:title`). [`title_template`](Seo::title_template) is
    /// applied to `<title>` only.
    ///
    /// An empty title renders no `<title>`/`og:title` and does not fall back to
    /// [`site_name`](Seo::site_name), like `title ?? siteName` in active-react.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Title template: the first `%s` is replaced by [`title`](Seo::title), e.g.
    /// `"%s · Acme"`. Unused when no title is set (the site name is shown as is).
    pub fn title_template(mut self, template: impl Into<String>) -> Self {
        self.title_template = Some(template.into());
        self
    }

    /// `<meta name="description">` and `og:description`.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Canonical URL: `<link rel="canonical">` and `og:url`. A relative path is resolved
    /// against [`base_url`](Seo::base_url).
    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical = Some(url.into());
        self
    }

    /// Share image (`og:image`); also switches `twitter:card` to `summary_large_image`.
    /// A relative path is resolved against [`base_url`](Seo::base_url).
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(url.into());
        self
    }

    /// `og:type`, `"website"` when not set (React prop: `type`).
    pub fn og_type(mut self, og_type: impl Into<String>) -> Self {
        self.og_type = Some(og_type.into());
        self
    }

    /// `og:site_name`, and the `<title>` / `og:title` when no title is set.
    pub fn site_name(mut self, site_name: impl Into<String>) -> Self {
        self.site_name = Some(site_name.into());
        self
    }

    /// `og:locale`, e.g. `"fr_FR"`.
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Twitter/X handle for `twitter:site`, e.g. `"@acme"`.
    pub fn twitter(mut self, handle: impl Into<String>) -> Self {
        self.twitter = Some(handle.into());
        self
    }

    /// Origin used to resolve relative URLs (canonical, image, alternates), e.g.
    /// `"https://acme.com"`. One trailing `/` is ignored.
    ///
    /// URLs with a scheme (`https:`, `mailto:`…) or starting with `//` are kept as is.
    ///
    /// ```
    /// let html = active::Seo::new().base_url("https://acme.com/").canonical("blog").to_html();
    /// assert!(html.contains(r#"<link rel="canonical" href="https://acme.com/blog"/>"#));
    /// ```
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Adds `noindex` to `<meta name="robots">` (`index` otherwise).
    pub fn noindex(mut self, noindex: bool) -> Self {
        self.noindex = noindex;
        self
    }

    /// Adds `nofollow` to `<meta name="robots">` (`follow` otherwise).
    pub fn nofollow(mut self, nofollow: bool) -> Self {
        self.nofollow = nofollow;
        self
    }

    /// Adds `<link rel="alternate" hreflang="{lang}" href="{url}"/>`. Call it once per
    /// language; tags keep the call order. Calling it again with the same `lang` replaces
    /// the URL in place (like a key of the `alternates` object in active-react).
    ///
    /// A relative `url` is resolved against [`base_url`](Seo::base_url); an empty one is
    /// skipped (React does not hoist a `<link>` without `href` either).
    ///
    /// ```
    /// let html = active::Seo::new()
    ///     .base_url("https://acme.com")
    ///     .alternate("en", "/en")
    ///     .alternate("fr", "/fr")
    ///     .alternate("x-default", "/")
    ///     .to_html();
    /// assert!(html.starts_with(concat!(
    ///     r#"<link rel="alternate" hreflang="en" href="https://acme.com/en"/>"#,
    ///     r#"<link rel="alternate" hreflang="fr" href="https://acme.com/fr"/>"#,
    ///     r#"<link rel="alternate" hreflang="x-default" href="https://acme.com/"/>"#,
    /// )));
    /// ```
    pub fn alternate(mut self, lang: impl Into<String>, url: impl Into<String>) -> Self {
        let (lang, url) = (lang.into(), url.into());
        match self.alternates.iter_mut().find(|(l, _)| *l == lang) {
            Some(slot) => slot.1 = url,
            None => self.alternates.push((lang, url)),
        }
        self
    }

    /// Adds a `<script type="application/ld+json">` with this JSON (already serialized,
    /// e.g. with `serde_json::to_string`). Call it once per object.
    ///
    /// The JSON is inserted as is, except that every `<` becomes `\u003c`: still the same
    /// JSON, but it can never close the `<script>` tag.
    ///
    /// ```
    /// let html = active::Seo::new().json_ld(r#"{"name":"</script>"}"#).to_html();
    /// assert!(html.ends_with(r#"<script type="application/ld+json">{"name":"\u003c/script>"}</script>"#));
    /// ```
    pub fn json_ld(mut self, json: impl Into<String>) -> Self {
        self.json_ld.push(json.into());
        self
    }

    /// Adds `<meta name="{name}" content="{content}"/>` after the built-in tags.
    pub fn meta_name(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.meta.push(Meta {
            attr: "name",
            key: name.into(),
            content: content.into(),
        });
        self
    }

    /// Adds `<meta property="{property}" content="{content}"/>` after the built-in tags.
    pub fn meta_property(
        mut self,
        property: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.meta.push(Meta {
            attr: "property",
            key: property.into(),
            content: content.into(),
        });
        self
    }

    /// Renders the tags to a new `String` (no whitespace between tags).
    #[must_use]
    pub fn to_html(&self) -> String {
        let mut out = String::new();
        self.write_html(&mut out);
        out
    }

    /// Appends the tags to `out`, e.g. a page buffer you are already writing to.
    ///
    /// ```
    /// let mut page = String::from("<head>");
    /// active::Seo::new().title("Hi").write_html(&mut page);
    /// page.push_str("</head>");
    /// assert!(page.starts_with("<head><title>Hi</title>"));
    /// ```
    pub fn write_html(&self, out: &mut String) {
        out.reserve(self.size_hint());
        // Writing to a `String` never fails.
        let _ = self.render(out);
    }

    /// Rough length of the output, used to reserve the buffer once.
    fn size_hint(&self) -> usize {
        let len = |s: &Option<String>| s.as_ref().map_or(0, String::len);
        let base = len(&self.base_url);
        let alternates: usize = self
            .alternates
            .iter()
            .map(|(lang, url)| lang.len() + url.len() + base + 48)
            .sum();
        let json: usize = self.json_ld.iter().map(|j| j.len() + 45).sum();
        let meta: usize = self
            .meta
            .iter()
            .map(|m| m.key.len() + m.content.len() + 30)
            .sum();
        // Markup of all built-in tags (~490 bytes), plus each value once per tag using it.
        let twice = len(&self.title) + len(&self.site_name) + len(&self.description);
        let once =
            len(&self.title_template) + len(&self.og_type) + len(&self.locale) + len(&self.twitter);
        let urls = 2 * (len(&self.canonical) + base) + len(&self.image) + base;
        512 + 2 * twice + once + urls + alternates + json + meta
    }

    fn render<W: Write>(&self, w: &mut W) -> fmt::Result {
        let base = self.base_url.as_deref();
        // `title ?? siteName`: an empty title does not fall back to the site name.
        let name = set(self.title.as_deref().or(self.site_name.as_deref()));
        let canonical = set(self.canonical.as_deref()).map(|url| Url::new(url, base));
        let image = set(self.image.as_deref()).map(|url| Url::new(url, base));

        // `title && titleTemplate ? titleTemplate.replace("%s", title) : name`
        let template = set(self.title.as_deref()).zip(set(self.title_template.as_deref()));
        if let Some((title, template)) = template {
            w.write_str("<title>")?;
            match template.split_once("%s") {
                Some((before, after)) => {
                    escape(w, before)?;
                    escape(w, title)?;
                    escape(w, after)?;
                }
                None => escape(w, template)?,
            }
            w.write_str("</title>")?;
        } else if let Some(name) = name {
            w.write_str("<title>")?;
            escape(w, name)?;
            w.write_str("</title>")?;
        }
        if let Some(description) = set(self.description.as_deref()) {
            meta(w, "name", "description", description)?;
        }
        if self.noindex || self.nofollow {
            let index = if self.noindex { "noindex" } else { "index" };
            let follow = if self.nofollow { "nofollow" } else { "follow" };
            write!(w, r#"<meta name="robots" content="{index},{follow}"/>"#)?;
        }
        if let Some(url) = canonical {
            url_tag(w, r#"<link rel="canonical" href=""#, url)?;
        }
        for (lang, url) in self.alternates.iter().filter(|(_, url)| !url.is_empty()) {
            w.write_str(r#"<link rel="alternate" hreflang=""#)?;
            escape(w, lang)?;
            url_tag(w, r#"" href=""#, Url::new(url, base))?;
        }
        if let Some(name) = name {
            meta(w, "property", "og:title", name)?;
        }
        if let Some(description) = set(self.description.as_deref()) {
            meta(w, "property", "og:description", description)?;
        }
        meta(
            w,
            "property",
            "og:type",
            self.og_type.as_deref().unwrap_or("website"),
        )?;
        if let Some(url) = canonical {
            url_tag(w, r#"<meta property="og:url" content=""#, url)?;
        }
        if let Some(url) = image {
            url_tag(w, r#"<meta property="og:image" content=""#, url)?;
        }
        if let Some(site_name) = set(self.site_name.as_deref()) {
            meta(w, "property", "og:site_name", site_name)?;
        }
        if let Some(locale) = set(self.locale.as_deref()) {
            meta(w, "property", "og:locale", locale)?;
        }
        let card = if image.is_some() {
            "summary_large_image"
        } else {
            "summary"
        };
        meta(w, "name", "twitter:card", card)?;
        if let Some(twitter) = set(self.twitter.as_deref()) {
            meta(w, "name", "twitter:site", twitter)?;
        }
        for m in &self.meta {
            meta(w, m.attr, &m.key, &m.content)?;
        }
        for json in self.json_ld.iter().filter(|j| !j.is_empty()) {
            w.write_str(r#"<script type="application/ld+json">"#)?;
            let mut parts = json.split('<');
            if let Some(first) = parts.next() {
                w.write_str(first)?;
            }
            for part in parts {
                w.write_str("\\u003c")?;
                w.write_str(part)?;
            }
            w.write_str("</script>")?;
        }
        Ok(())
    }
}

/// Same as [`Seo::to_html`], written straight to the formatter: `format!("<head>{seo}</head>")`.
impl fmt::Display for Seo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render(f)
    }
}

/// JS truthiness for an optional string: `None` and `""` are "not set".
fn set(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.is_empty())
}

/// `<meta {attr}="{key}" content="{content}"/>`, escaped.
fn meta<W: Write>(w: &mut W, attr: &str, key: &str, content: &str) -> fmt::Result {
    w.write_str("<meta ")?;
    w.write_str(attr)?;
    w.write_str("=\"")?;
    escape(w, key)?;
    w.write_str("\" content=\"")?;
    escape(w, content)?;
    w.write_str("\"/>")
}

/// `{open}{url}"/>`, with `open` ending in an attribute's opening quote.
fn url_tag<W: Write>(w: &mut W, open: &str, url: Url<'_>) -> fmt::Result {
    w.write_str(open)?;
    url.write(w)?;
    w.write_str("\"/>")
}

/// Escapes text and attribute values exactly like React (`escapeTextForBrowser`).
fn escape<W: Write>(w: &mut W, s: &str) -> fmt::Result {
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
        w.write_str(&s[start..i])?;
        w.write_str(entity)?;
        start = i + 1;
    }
    w.write_str(&s[start..])
}

/// A URL resolved against the base URL, kept as borrowed parts: `{base}{sep}{path}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Url<'a> {
    base: &'a str,
    sep: &'a str,
    path: &'a str,
}

impl<'a> Url<'a> {
    /// `abs(url, base)` from active-react: `url` unchanged if empty, without base, or
    /// absolute (`scheme:` or `//`); otherwise joined to `base` minus one trailing `/`.
    fn new(url: &'a str, base: Option<&'a str>) -> Self {
        match base {
            Some(base) if !url.is_empty() && !base.is_empty() && !is_absolute(url) => {
                let base = base.strip_suffix('/').unwrap_or(base);
                let sep = if url.starts_with('/') { "" } else { "/" };
                Url {
                    base,
                    sep,
                    path: url,
                }
            }
            _ => Url {
                base: "",
                sep: "",
                path: url,
            },
        }
    }

    fn write<W: Write>(self, w: &mut W) -> fmt::Result {
        escape(w, self.base)?;
        w.write_str(self.sep)?;
        escape(w, self.path)
    }
}

/// `/^[a-z][a-z\d+.-]*:|^\/\//i`
fn is_absolute(url: &str) -> bool {
    if url.starts_with("//") {
        return true;
    }
    let mut bytes = url.bytes();
    if !bytes.next().is_some_and(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    for b in bytes {
        match b {
            b':' => return true,
            b'+' | b'.' | b'-' => {}
            _ if b.is_ascii_alphanumeric() => {}
            _ => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs(url: &str, base: Option<&str>) -> String {
        let mut out = String::new();
        Url::new(url, base).write(&mut out).unwrap();
        out
    }

    #[test]
    fn resolves_relative_urls() {
        let base = Some("https://acme.com");
        assert_eq!(abs("/p", base), "https://acme.com/p");
        assert_eq!(abs("p", base), "https://acme.com/p");
        assert_eq!(abs("/p", Some("https://acme.com/")), "https://acme.com/p");
        // Only one trailing slash is removed, like `base.slice(0, -1)`.
        assert_eq!(abs("p", Some("https://acme.com//")), "https://acme.com//p");
        assert_eq!(abs("/", base), "https://acme.com/");
        assert_eq!(abs("?q=1", base), "https://acme.com/?q=1");
    }

    #[test]
    fn keeps_urls_without_base_or_absolute() {
        assert_eq!(abs("/p", None), "/p");
        assert_eq!(abs("/p", Some("")), "/p");
        assert_eq!(abs("", Some("https://acme.com")), "");
        for url in [
            "https://x.dev/",
            "HTTP://X",
            "mailto:a@b.c",
            "a+b-c.d:z",
            "x1:y",
            "//cdn.acme.com/a.png",
        ] {
            assert_eq!(abs(url, Some("https://acme.com")), url, "{url}");
        }
    }

    #[test]
    fn scheme_detection_matches_the_js_regex() {
        assert!(is_absolute("data:image/png;base64,AA"));
        assert!(is_absolute("Z:"));
        assert!(!is_absolute("1abc:x"));
        assert!(!is_absolute("+a:x"));
        assert!(!is_absolute("a b:x"));
        assert!(!is_absolute("a/b:x"));
        assert!(!is_absolute("é:x"));
        assert!(!is_absolute("abc"));
        assert!(!is_absolute("/a:b"));
        assert!(!is_absolute(""));
    }

    #[test]
    fn escapes_like_react() {
        let mut out = String::new();
        escape(&mut out, r#"<a href="x">Tom & 'Jerry'</a> é"#).unwrap();
        assert_eq!(
            out,
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#x27;Jerry&#x27;&lt;/a&gt; é"
        );
    }

    #[test]
    fn size_hint_covers_typical_pages() {
        let seo = Seo::new()
            .site_name("Acme")
            .title_template("%s · Acme")
            .title("Contact")
            .description("Write to us.")
            .canonical("/contact")
            .base_url("https://acme.com")
            .image("/og.png")
            .og_type("article")
            .locale("fr_FR")
            .twitter("@acme")
            .noindex(true)
            .nofollow(true)
            .alternate("en", "/en/contact")
            .json_ld(r#"{"@type":"Organization"}"#)
            .meta_name("theme-color", "#000");
        let (hint, len) = (seo.size_hint(), seo.to_html().len());
        assert!(hint >= len && hint < 2 * len, "{hint} vs {len}");
        let empty = Seo::new();
        assert!(empty.size_hint() >= empty.to_html().len());
    }
}
