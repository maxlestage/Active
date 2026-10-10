//! `Seo`: which tags are rendered, in which order, escaped and resolved how.

use active::Seo;

/// Site-wide defaults, cloned by every page.
fn site() -> Seo {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com/")
        .image("/og.png")
}

#[test]
fn renders_every_tag_in_order() {
    let html = Seo::new()
        .title("Produit")
        .description("Un super produit")
        .canonical("https://acme.com/p")
        .image("https://acme.com/p.png")
        .og_type("product")
        .locale("fr_FR")
        .twitter("@acme")
        .site_name("Acme")
        .to_html();
    assert_eq!(
        html,
        concat!(
            "<title>Produit</title>",
            r#"<meta name="description" content="Un super produit"/>"#,
            r#"<link rel="canonical" href="https://acme.com/p"/>"#,
            r#"<meta property="og:title" content="Produit"/>"#,
            r#"<meta property="og:description" content="Un super produit"/>"#,
            r#"<meta property="og:type" content="product"/>"#,
            r#"<meta property="og:url" content="https://acme.com/p"/>"#,
            r#"<meta property="og:image" content="https://acme.com/p.png"/>"#,
            r#"<meta property="og:site_name" content="Acme"/>"#,
            r#"<meta property="og:locale" content="fr_FR"/>"#,
            r#"<meta name="twitter:card" content="summary_large_image"/>"#,
            r#"<meta name="twitter:site" content="@acme"/>"#,
        )
    );
}

#[test]
fn merges_site_defaults_applies_the_template_and_resolves_urls() {
    let h = site()
        .twitter("@acme")
        .title("Contact")
        .canonical("/contact")
        .alternate("en", "/en/contact")
        .alternate("fr", "contact")
        .to_html();
    assert!(h.contains("<title>Contact · Acme</title>"));
    assert!(h.contains(r#"<meta property="og:title" content="Contact"/>"#));
    assert!(h.contains(r#"<link rel="canonical" href="https://acme.com/contact"/>"#));
    assert!(
        h.contains(r#"<link rel="alternate" hreflang="en" href="https://acme.com/en/contact"/>"#)
    );
    assert!(h.contains(r#"<link rel="alternate" hreflang="fr" href="https://acme.com/contact"/>"#));
    assert!(h.contains(r#"<meta property="og:image" content="https://acme.com/og.png"/>"#));
    assert!(h.contains(r#"<meta name="twitter:site" content="@acme"/>"#));
}

#[test]
fn falls_back_to_site_name_and_lets_a_page_override_defaults() {
    let defaults = Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .noindex(true);
    let h = defaults.clone().noindex(false).nofollow(true).to_html();
    assert!(h.contains("<title>Acme</title>"));
    assert!(h.contains(r#"<meta name="robots" content="index,nofollow"/>"#));
    // The defaults are untouched.
    assert!(
        defaults
            .to_html()
            .contains(r#"<meta name="robots" content="noindex,follow"/>"#)
    );
}

#[test]
fn keeps_absolute_and_protocol_relative_urls_untouched() {
    let h = Seo::new()
        .base_url("https://acme.com")
        .image("//cdn.acme.com/a.png")
        .canonical("https://x.dev/")
        .to_html();
    assert!(h.contains(r#"content="//cdn.acme.com/a.png""#));
    assert!(h.contains(r#"href="https://x.dev/""#));
}

#[test]
fn serialises_json_ld_safely() {
    let html = Seo::new()
        .json_ld(r#"{"@type":"Thing","name":"</script><script>alert(1)</script>"}"#)
        .to_html();
    assert!(html.contains(
        r#"<script type="application/ld+json">{"@type":"Thing","name":"\u003c/script>\u003cscript>alert(1)\u003c/script>"}</script>"#
    ));
    assert_eq!(html.matches("<script").count(), 1);
}

#[test]
fn supports_extra_meta_tags() {
    let h = Seo::new()
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "Logo")
        .to_html();
    assert!(h.ends_with(
        r##"<meta name="theme-color" content="#000"/><meta property="og:image:alt" content="Logo"/>"##
    ));
}

#[test]
fn starts_with_title_and_description() {
    let head = Seo::new().title("Hello").description("World").to_html();
    assert!(head.starts_with(r#"<title>Hello</title><meta name="description" content="World"/>"#));
    assert!(!head.contains("<div"));
}

// Every option at once, in the documented order.

#[test]
fn full_example_renders_every_tag_in_order() {
    let site = Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
        .locale("fr_FR")
        .twitter("@acme");
    let html = site
        .clone()
        .title("Contact")
        .description("Write to us.")
        .canonical("/contact")
        .og_type("article")
        .noindex(false)
        .nofollow(false)
        .alternate("en", "/en/contact")
        .json_ld(r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#)
        .meta_name("theme-color", "#000")
        .meta_property("og:image:alt", "Logo")
        .to_html();
    assert_eq!(
        html,
        concat!(
            "<title>Contact · Acme</title>",
            r#"<meta name="description" content="Write to us."/>"#,
            r#"<link rel="canonical" href="https://acme.com/contact"/>"#,
            r#"<link rel="alternate" hreflang="en" href="https://acme.com/en/contact"/>"#,
            r#"<meta property="og:title" content="Contact"/>"#,
            r#"<meta property="og:description" content="Write to us."/>"#,
            r#"<meta property="og:type" content="article"/>"#,
            r#"<meta property="og:url" content="https://acme.com/contact"/>"#,
            r#"<meta property="og:image" content="https://acme.com/og.png"/>"#,
            r#"<meta property="og:site_name" content="Acme"/>"#,
            r#"<meta property="og:locale" content="fr_FR"/>"#,
            r#"<meta name="twitter:card" content="summary_large_image"/>"#,
            r#"<meta name="twitter:site" content="@acme"/>"#,
            r##"<meta name="theme-color" content="#000"/>"##,
            r#"<meta property="og:image:alt" content="Logo"/>"#,
            r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Organization","name":"Acme"}</script>"#,
        )
    );
}

#[test]
fn escapes_text_and_attributes() {
    let html = Seo::new()
        .title(r#"<"Tom" & 'Jerry'>"#)
        .description(r#"a<b>&"'"#)
        .canonical("https://x.dev/?a=1&b=2")
        .site_name("S&P")
        .to_html();
    assert_eq!(
        html,
        concat!(
            "<title>&lt;&quot;Tom&quot; &amp; &#x27;Jerry&#x27;&gt;</title>",
            r#"<meta name="description" content="a&lt;b&gt;&amp;&quot;&#x27;"/>"#,
            r#"<link rel="canonical" href="https://x.dev/?a=1&amp;b=2"/>"#,
            r#"<meta property="og:title" content="&lt;&quot;Tom&quot; &amp; &#x27;Jerry&#x27;&gt;"/>"#,
            r#"<meta property="og:description" content="a&lt;b&gt;&amp;&quot;&#x27;"/>"#,
            r#"<meta property="og:type" content="website"/>"#,
            r#"<meta property="og:url" content="https://x.dev/?a=1&amp;b=2"/>"#,
            r#"<meta property="og:site_name" content="S&amp;P"/>"#,
            r#"<meta name="twitter:card" content="summary"/>"#,
        )
    );
}

#[test]
fn empty_builder_renders_og_type_and_twitter_card_only() {
    let expected = concat!(
        r#"<meta property="og:type" content="website"/>"#,
        r#"<meta name="twitter:card" content="summary"/>"#,
    );
    assert_eq!(Seo::new().to_html(), expected);
    assert_eq!(Seo::default(), Seo::new());
    // Empty strings count as "not set".
    let empty = Seo::new()
        .description("")
        .canonical("")
        .image("")
        .site_name("")
        .locale("")
        .twitter("")
        .base_url("")
        .title_template("")
        .json_ld("");
    assert_eq!(empty.to_html(), expected);
}

#[test]
fn empty_title_does_not_fall_back_to_site_name() {
    // No title: the site name is used.
    assert_eq!(
        Seo::new().title("").site_name("Acme").to_html(),
        concat!(
            r#"<meta property="og:type" content="website"/>"#,
            r#"<meta property="og:site_name" content="Acme"/>"#,
            r#"<meta name="twitter:card" content="summary"/>"#,
        )
    );
}

#[test]
fn empty_og_type_is_kept() {
    // No type: `website`.
    assert!(
        Seo::new()
            .og_type("")
            .to_html()
            .starts_with(r#"<meta property="og:type" content=""/>"#)
    );
}

#[test]
fn robots_combinations() {
    let robots = |noindex, nofollow| Seo::new().noindex(noindex).nofollow(nofollow).to_html();
    assert!(!robots(false, false).contains("robots"));
    assert!(robots(true, false).starts_with(r#"<meta name="robots" content="noindex,follow"/>"#));
    assert!(robots(false, true).starts_with(r#"<meta name="robots" content="index,nofollow"/>"#));
    assert!(robots(true, true).starts_with(r#"<meta name="robots" content="noindex,nofollow"/>"#));
}

#[test]
fn template_replaces_the_first_placeholder_only() {
    let title = |template| Seo::new().title("A").title_template(template).to_html();
    assert!(
        title("Fixed")
            .starts_with(r#"<title>Fixed</title><meta property="og:title" content="A"/>"#)
    );
    assert!(title("%s|%s").starts_with("<title>A|%s</title>"));
    assert!(title("%s").starts_with("<title>A</title>"));
    // The title is inserted literally (no `$&`-style patterns).
    assert!(
        Seo::new()
            .title("Save $& more")
            .title_template("%s · Acme")
            .to_html()
            .starts_with("<title>Save $&amp; more · Acme</title>")
    );
}

#[test]
fn resolves_relative_urls_against_the_base_url() {
    let h = Seo::new()
        .base_url("https://a.com/")
        .canonical("HTTP://X")
        .image("mailto:x")
        .to_html();
    assert!(h.starts_with(r#"<link rel="canonical" href="HTTP://X"/>"#));
    assert!(h.contains(r#"<meta property="og:image" content="mailto:x"/>"#));

    let h = Seo::new()
        .base_url("https://a.com//")
        .canonical("p")
        .to_html();
    assert!(h.starts_with(r#"<link rel="canonical" href="https://a.com//p"/>"#));

    let h = Seo::new()
        .base_url("https://a.com")
        .canonical("1abc:x")
        .image("a+b-c.d:z")
        .to_html();
    assert!(h.starts_with(r#"<link rel="canonical" href="https://a.com/1abc:x"/>"#));
    assert!(h.contains(r#"<meta property="og:image" content="a+b-c.d:z"/>"#));
}

#[test]
fn alternates_keep_order_skip_empty_urls_and_replace_duplicates() {
    let h = Seo::new()
        .base_url("https://a.com")
        .alternate("x-default", "/")
        .alternate("en", "")
        .alternate("fr", "fr")
        .alternate("de", "/de")
        .alternate("x-default", "/en")
        .to_html();
    assert!(h.starts_with(concat!(
        r#"<link rel="alternate" hreflang="x-default" href="https://a.com/en"/>"#,
        r#"<link rel="alternate" hreflang="fr" href="https://a.com/fr"/>"#,
        r#"<link rel="alternate" hreflang="de" href="https://a.com/de"/>"#,
        r#"<meta property="og:type""#,
    )));
}

#[test]
fn extra_meta_with_empty_values_is_kept() {
    assert!(
        Seo::new()
            .meta_name("", "")
            .meta_property("p", "")
            .to_html()
            .ends_with(r#"<meta name="" content=""/><meta property="p" content=""/>"#)
    );
}

#[test]
fn one_script_per_json_ld_call() {
    let h = Seo::new()
        .json_ld(r#"{"a":1}"#)
        .json_ld(r#"{"b":"<"}"#)
        .to_html();
    assert!(h.ends_with(concat!(
        r#"<meta name="twitter:card" content="summary"/>"#,
        r#"<script type="application/ld+json">{"a":1}</script>"#,
        r#"<script type="application/ld+json">{"b":"\u003c"}</script>"#,
    )));
}

#[test]
fn display_and_write_html_match_to_html() {
    let seo = site().title("Contact").canonical("/contact").json_ld("{}");
    let html = seo.to_html();
    assert_eq!(seo.to_string(), html);
    assert_eq!(
        format!("<head>{seo}</head>"),
        format!("<head>{html}</head>")
    );
    let mut buf = String::from("<head>");
    seo.write_html(&mut buf);
    buf.push_str("</head>");
    assert_eq!(buf, format!("<head>{html}</head>"));
}
