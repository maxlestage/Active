//! SEO `<head>` tags.

use std::fmt::{self, Write};

use crate::escape::escape;

/// SEO `<head>` tags for one page: title, description, robots, canonical, hreflang
/// alternates, Open Graph, Twitter and JSON-LD.
///
/// Build it with chained setters (each takes `self` by value and returns it), then
/// render with [`to_html`](Seo::to_html), [`write_html`](Seo::write_html) or
/// [`Display`](std::fmt::Display). Keep site-wide values in one `Seo` and
/// [`clone`](Clone::clone) it per page, or let the [`Document`](crate::Document) merge them.
///
/// Put one in a view with [`Node::from`](crate::Node) (or `.child(seo)`): when the page is
/// rendered by a [`Document`](crate::Document), every `Seo` found in the tree is
/// [merged](Seo::merge) in tree order, so a layout can set site defaults and the page
/// override them — no duplicate tags.
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
    noindex: Option<bool>,
    nofollow: Option<bool>,
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
    /// [`site_name`](Seo::site_name).
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

    /// `og:type`, `"website"` when not set.
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
        self.noindex = Some(noindex);
        self
    }

    /// Adds `nofollow` to `<meta name="robots">` (`follow` otherwise).
    pub fn nofollow(mut self, nofollow: bool) -> Self {
        self.nofollow = Some(nofollow);
        self
    }

    /// Adds `<link rel="alternate" hreflang="{lang}" href="{url}"/>`. Call it once per
    /// language; tags keep the call order. Calling it again with the same `lang` replaces
    /// the URL in place.
    ///
    /// A relative `url` is resolved against [`base_url`](Seo::base_url); an empty one is
    /// skipped.
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

    /// Applies `other` on top of `self`: every value set in `other` wins, alternates and
    /// extra meta tags are replaced by key (language, name/property), JSON-LD is appended.
    ///
    /// This is what lets several `Seo` values live in one page (layout, then page, then a
    /// deeper component): the deepest value wins and no tag is duplicated.
    ///
    /// ```
    /// use active::Seo;
    ///
    /// let site = Seo::new().site_name("Acme").title_template("%s · Acme").noindex(true);
    /// let page = Seo::new().title("Contact").noindex(false);
    /// let html = site.merged(&page).to_html();
    /// assert!(html.starts_with("<title>Contact · Acme</title>"));
    /// assert!(!html.contains("robots"));
    /// ```
    pub fn merge(&mut self, other: &Seo) {
        let fields = [
            (&mut self.title, &other.title),
            (&mut self.title_template, &other.title_template),
            (&mut self.description, &other.description),
            (&mut self.canonical, &other.canonical),
            (&mut self.image, &other.image),
            (&mut self.og_type, &other.og_type),
            (&mut self.site_name, &other.site_name),
            (&mut self.locale, &other.locale),
            (&mut self.twitter, &other.twitter),
            (&mut self.base_url, &other.base_url),
        ];
        for (mine, theirs) in fields {
            if theirs.is_some() {
                mine.clone_from(theirs);
            }
        }
        if other.noindex.is_some() {
            self.noindex = other.noindex;
        }
        if other.nofollow.is_some() {
            self.nofollow = other.nofollow;
        }
        for (lang, url) in &other.alternates {
            match self.alternates.iter_mut().find(|(l, _)| l == lang) {
                Some(slot) => slot.1.clone_from(url),
                None => self.alternates.push((lang.clone(), url.clone())),
            }
        }
        for m in &other.meta {
            match self
                .meta
                .iter_mut()
                .find(|x| x.attr == m.attr && x.key == m.key)
            {
                Some(slot) => slot.content.clone_from(&m.content),
                None => self.meta.push(m.clone()),
            }
        }
        self.json_ld.extend(other.json_ld.iter().cloned());
    }

    /// [`merge`](Seo::merge) returning a new value.
    pub fn merged(&self, other: &Seo) -> Seo {
        let mut out = self.clone();
        out.merge(other);
        out
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
        write_merged(&[self], out);
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
        Merged(&[self]).render(w)
    }
}

/// Writes the tags of `layers` merged like [`Seo::merge`] (later layers win), without
/// copying any of them: how a [`Document`](crate::Document) writes its head.
pub(crate) fn write_merged(layers: &[&Seo], out: &mut String) {
    out.reserve(layers.iter().map(|l| l.size_hint()).sum());
    // Writing to a `String` never fails.
    let _ = Merged(layers).render(out);
}

/// Several `Seo`, read as one.
struct Merged<'a>(&'a [&'a Seo]);

impl<'a> Merged<'a> {
    /// The value of the last layer that sets the field.
    fn get(&self, field: impl Fn(&'a Seo) -> &'a Option<String>) -> Option<&'a str> {
        self.0.iter().rev().find_map(|l| field(l).as_deref())
    }

    fn flag(&self, field: impl Fn(&'a Seo) -> Option<bool>) -> Option<bool> {
        self.0.iter().rev().find_map(|l| field(l))
    }

    /// Alternates in first-seen order, each with the URL of the last layer setting it.
    fn alternates(&self) -> impl Iterator<Item = (&'a str, &'a str)> + '_ {
        let layers = self.0;
        layers.iter().enumerate().flat_map(move |(i, layer)| {
            layer.alternates.iter().filter_map(move |(lang, _)| {
                let seen = layers[..i]
                    .iter()
                    .any(|l| l.alternates.iter().any(|(l, _)| l == lang));
                let url = layers.iter().rev().find_map(|l| {
                    l.alternates
                        .iter()
                        .find(|(l, _)| l == lang)
                        .map(|(_, u)| u.as_str())
                })?;
                (!seen).then_some((lang.as_str(), url))
            })
        })
    }

    /// Extra meta tags, merged the same way as alternates.
    fn meta(&self) -> impl Iterator<Item = (&'static str, &'a str, &'a str)> + '_ {
        let layers = self.0;
        let same = |a: &Meta, b: &Meta| a.attr == b.attr && a.key == b.key;
        layers.iter().enumerate().flat_map(move |(i, layer)| {
            layer.meta.iter().filter_map(move |m| {
                let seen = layers[..i]
                    .iter()
                    .any(|l| l.meta.iter().any(|x| same(x, m)));
                let content = layers
                    .iter()
                    .rev()
                    .find_map(|l| l.meta.iter().find(|x| same(x, m)))?;
                (!seen).then_some((m.attr, m.key.as_str(), content.content.as_str()))
            })
        })
    }

    fn render<W: Write>(&self, w: &mut W) -> fmt::Result {
        let base = self.get(|s| &s.base_url);
        // `title ?? siteName`: an empty title does not fall back to the site name.
        let name = set(self.get(|s| &s.title).or(self.get(|s| &s.site_name)));
        let canonical = set(self.get(|s| &s.canonical)).map(|url| Url::new(url, base));
        let image = set(self.get(|s| &s.image)).map(|url| Url::new(url, base));

        // `title && titleTemplate ? titleTemplate.replace("%s", title) : name`
        let template = set(self.get(|s| &s.title)).zip(set(self.get(|s| &s.title_template)));
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
        if let Some(description) = set(self.get(|s| &s.description)) {
            meta(w, "name", "description", description)?;
        }
        let (noindex, nofollow) = (
            self.flag(|s| s.noindex) == Some(true),
            self.flag(|s| s.nofollow) == Some(true),
        );
        if noindex || nofollow {
            let index = if noindex { "noindex" } else { "index" };
            let follow = if nofollow { "nofollow" } else { "follow" };
            write!(w, r#"<meta name="robots" content="{index},{follow}"/>"#)?;
        }
        if let Some(url) = canonical {
            url_tag(w, r#"<link rel="canonical" href=""#, url)?;
        }
        for (lang, url) in self.alternates().filter(|(_, url)| !url.is_empty()) {
            w.write_str(r#"<link rel="alternate" hreflang=""#)?;
            escape(w, lang)?;
            url_tag(w, r#"" href=""#, Url::new(url, base))?;
        }
        if let Some(name) = name {
            meta(w, "property", "og:title", name)?;
        }
        if let Some(description) = set(self.get(|s| &s.description)) {
            meta(w, "property", "og:description", description)?;
        }
        meta(
            w,
            "property",
            "og:type",
            self.get(|s| &s.og_type).unwrap_or("website"),
        )?;
        if let Some(url) = canonical {
            url_tag(w, r#"<meta property="og:url" content=""#, url)?;
        }
        if let Some(url) = image {
            url_tag(w, r#"<meta property="og:image" content=""#, url)?;
        }
        if let Some(site_name) = set(self.get(|s| &s.site_name)) {
            meta(w, "property", "og:site_name", site_name)?;
        }
        if let Some(locale) = set(self.get(|s| &s.locale)) {
            meta(w, "property", "og:locale", locale)?;
        }
        let card = if image.is_some() {
            "summary_large_image"
        } else {
            "summary"
        };
        meta(w, "name", "twitter:card", card)?;
        if let Some(twitter) = set(self.get(|s| &s.twitter)) {
            meta(w, "name", "twitter:site", twitter)?;
        }
        for (attr, key, content) in self.meta() {
            meta(w, attr, key, content)?;
        }
        let json_ld = self.0.iter().flat_map(|l| &l.json_ld);
        for json in json_ld.filter(|j| !j.is_empty()) {
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

/// `None` and `""` are "not set".
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

/// A URL resolved against the base URL, kept as borrowed parts: `{base}{sep}{path}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Url<'a> {
    base: &'a str,
    sep: &'a str,
    path: &'a str,
}

impl<'a> Url<'a> {
    /// `url` unchanged if empty, without base, or
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
    fn writing_layers_equals_merging_them() {
        let site = Seo::new()
            .site_name("Acme")
            .title_template("%s · Acme")
            .base_url("https://acme.com")
            .alternate("en", "/en")
            .alternate("fr", "/fr")
            .meta_name("theme-color", "#000")
            .json_ld("{}");
        let page = Seo::new()
            .title("Chairs")
            .canonical("/chairs")
            .alternate("fr", "/fr/chaises")
            .alternate("es", "/es/sillas")
            .meta_name("theme-color", "#fff")
            .meta_property("og:image:alt", "A chair")
            .noindex(true)
            .json_ld(r#"{"a":1}"#);
        let item = Seo::new().nofollow(true).description("Oak");
        let mut layered = String::new();
        super::write_merged(&[&site, &page, &item], &mut layered);
        assert_eq!(layered, site.merged(&page).merged(&item).to_html());
        assert_eq!(layered.matches("hreflang").count(), 3);
        assert!(layered.contains(r#"hreflang="fr" href="https://acme.com/fr/chaises""#));
    }

    #[test]
    fn escapes_html_special_characters() {
        use crate::escape::escape;
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
