//! Every page of the site, written with active.

use active::prelude::*;

use super::Site;
use super::docs::{CATEGORIES, Docs};
use super::pwa;
use crate::i18n::t;
use crate::routes::{Lang, Page, Route, base, link};

pub const GITHUB: &str = "https://github.com/maxlestage/Active";
pub const HEROKU_DEPLOY: &str =
    "https://heroku.com/deploy?template=https://github.com/maxlestage/Active";

/// What a request needs to render a page.
pub struct Ctx<'a> {
    pub site: &'a Site,
    pub route: &'a Route,
    /// Absolute origin for canonical and Open Graph URLs, e.g. `https://example.com`.
    pub origin: &'a str,
    /// Show the opening curtain (first visit of the session, decided by the server).
    pub intro: bool,
}

/// Splits a title into masked words for staggered reveals; `*text*` becomes an accent.
fn split(root: Element, text: &str, start: usize) -> Element {
    let mut i = start;
    let mut root = root.class("split");
    for (n, part) in text.split('*').enumerate() {
        if part.is_empty() {
            continue;
        }
        let mut group = Vec::new();
        for (k, word) in part.split(' ').enumerate() {
            if k > 0 {
                group.push(Node::from(" "));
            }
            if !word.is_empty() {
                group.push(
                    span()
                        .class("w")
                        .child(span().style(format!("--i:{i}")).text(word.to_string()))
                        .into(),
                );
                i += 1;
            }
        }
        root = if n % 2 == 1 {
            root.child(em().children(group))
        } else {
            root.children(group)
        };
    }
    root
}

/// A title whose words rise one by one when it scrolls into view.
fn heading(tag: fn() -> Element, text: &str, class: &str) -> Element {
    split(
        tag().class(class.to_string()).attr("data-reveal", ""),
        text,
        0,
    )
}

fn arrow() -> Element {
    svg()
        .class("arrow")
        .attr("viewBox", "0 0 24 24")
        .attr("aria-hidden", "true")
        .child(
            path()
                .attr("d", "M5 12h14M13 6l6 6-6 6")
                .attr("fill", "none")
                .attr("stroke", "currentColor")
                .attr("stroke-width", "1.8")
                .attr("stroke-linecap", "round")
                .attr("stroke-linejoin", "round"),
        )
}

fn logo() -> Element {
    span()
        .class("logo")
        .child(
            svg()
                .attr("viewBox", "0 0 32 32")
                .attr("aria-hidden", "true")
                .child(
                    circle()
                        .attr("cx", "16")
                        .attr("cy", "16")
                        .attr("r", "12.5")
                        .attr("fill", "none")
                        .attr("stroke", "currentColor")
                        .attr("stroke-width", "2.2"),
                )
                .child(
                    circle()
                        .attr("cx", "24.8")
                        .attr("cy", "7.2")
                        .attr("r", "3.4")
                        .attr("fill", "var(--accent)"),
                ),
        )
        .child(span().text("active"))
}

fn github_icon() -> Element {
    svg().attr("viewBox", "0 0 24 24").attr("aria-hidden", "true").attr("width", "18").attr("height", "18").child(path().attr("fill", "currentColor").attr(
        "d",
        "M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.53-1.33-1.28-1.69-1.28-1.69-1.05-.72.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.71 1.26 3.37.96.1-.75.4-1.26.73-1.55-2.56-.29-5.25-1.28-5.25-5.7 0-1.26.45-2.29 1.19-3.1-.12-.29-.52-1.47.11-3.06 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.77.11 3.06.74.81 1.19 1.84 1.19 3.1 0 4.43-2.7 5.4-5.27 5.69.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z",
    ))
}

fn alternates(seo: Seo, route: &Route) -> Seo {
    let seo = Lang::ALL.into_iter().fold(seo, |seo, l| {
        seo.alternate(l.code(), route.with_lang(l).path())
    });
    seo.alternate("x-default", route.with_lang(Lang::En).path())
}

fn lang_switch(route: &Route) -> Element {
    div()
        .class("langs")
        .attr("role", "group")
        .attr("aria-label", t(route.lang).footer.language)
        .children(Lang::ALL.map(|l| {
            let link = a()
                .href(route.with_lang(l).href())
                .attr("hreflang", l.code())
                .attr("lang", l.code())
                .attr("title", t(l).lang_name)
                .text(l.code());
            if l == route.lang {
                link.attr("aria-current", "true")
            } else {
                link
            }
        }))
}

fn header(route: &Route) -> Element {
    let d = t(route.lang);
    let home = link(&format!("/{}/", route.lang.code()));
    let links = [
        (link(&format!("/{}/docs/", route.lang.code())), d.nav.docs),
        (format!("{home}#features"), d.nav.features),
        (format!("{home}#playground"), d.nav.playground),
        (format!("{home}#benchmarks"), d.nav.benchmarks),
    ];
    el("header")
        .id("top")
        .class("header")
        .child(a().class("skip").href("#main").text(d.skip))
        .child(
            a().href(home.clone())
                .class("brand")
                .attr("aria-label", "active")
                .attr("data-magnetic", "")
                .child(logo()),
        )
        .child(
            nav()
                .class("nav")
                .attr("aria-label", "Main")
                .children(links.map(|(href, label)| {
                    a().href(href)
                        .class("link")
                        .child(span().attr("data-text", label).text(label))
                })),
        )
        .child(
            div()
                .class("actions")
                .child(lang_switch(route))
                .child(
                    a().class("icon")
                        .href(GITHUB)
                        .attr("aria-label", "GitHub")
                        .attr("target", "_blank")
                        .attr("rel", "noopener noreferrer")
                        .child(github_icon()),
                )
                .child(
                    a().class("btn small")
                        .href(link(&format!("/{}/docs/quick-start/", route.lang.code())))
                        .attr("data-magnetic", "")
                        .text(d.nav.start),
                )
                .child(
                    button()
                        .class("burger")
                        .attr("type", "button")
                        .attr("aria-expanded", "false")
                        .attr("aria-label", d.nav.menu)
                        .child(span())
                        .child(span()),
                ),
        )
}

/// (href, label, opens a new tab)
type FooterLink<'a> = (String, &'a str, bool);

fn footer(route: &Route) -> Element {
    let d = t(route.lang);
    let f = &d.footer;
    let l = route.lang.code();
    let doc = |slug: &str| link(&format!("/{l}/docs/{slug}/"));
    let columns: [(&str, Vec<FooterLink<'_>>); 3] = [
        (
            f.product,
            vec![
                (link(&format!("/{l}/docs/")), d.nav.docs, false),
                (doc("quick-start"), f.quick_start, false),
                (doc("api"), f.api, false),
                (doc("benchmarks"), f.benchmarks, false),
            ],
        ),
        (
            f.learn,
            vec![
                (doc("components"), f.components, false),
                (doc("seo"), f.seo, false),
                (doc("islands"), f.islands, false),
                (doc("faq"), f.faq, false),
            ],
        ),
        (
            f.project,
            vec![
                (GITHUB.to_string(), "GitHub", true),
                (
                    "https://crates.io/crates/active".to_string(),
                    "crates.io",
                    true,
                ),
                ("https://docs.rs/active".to_string(), "docs.rs", true),
                (doc("deploy-heroku"), f.deploy, false),
            ],
        ),
    ];
    let link = |(href, label, external): FooterLink<'_>| {
        let link = a().href(href).text(label.to_string());
        li().child(if external {
            link.attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
        } else {
            link
        })
    };
    el("footer")
        .class("footer")
        .child(
            a().class("footer-cta")
                .href(doc("quick-start"))
                .attr("data-reveal", "")
                .attr("data-cursor", "")
                .child(span().text(f.start))
                .child(arrow()),
        )
        .child(
            div()
                .class("footer-grid")
                .child(
                    div()
                        .class("footer-brand")
                        .attr("data-reveal", "")
                        .child(logo())
                        .child(p().text(f.tagline)),
                )
                .children(columns.into_iter().map(|(title, links)| {
                    nav()
                        .attr("aria-label", title)
                        .attr("data-reveal", "")
                        .child(h2().text(title))
                        .child(ul().children(links.into_iter().map(link)))
                }))
                .child(
                    nav()
                        .attr("aria-label", f.language)
                        .attr("data-reveal", "")
                        .child(h2().text(f.language))
                        .child(ul().children(Lang::ALL.map(|lang| {
                            let link = a()
                                .href(route.with_lang(lang).href())
                                .attr("hreflang", lang.code())
                                .attr("lang", lang.code())
                                .text(t(lang).lang_name);
                            li().child(if lang == route.lang {
                                link.attr("aria-current", "true")
                            } else {
                                link
                            })
                        }))),
                ),
        )
        .child(
            div()
                .class("wordmark")
                .attr("aria-hidden", "true")
                .children(
                    "active"
                        .chars()
                        .enumerate()
                        .map(|(i, c)| span().style(format!("--i:{i}")).text(c.to_string())),
                ),
        )
        .child(
            div()
                .class("footer-bottom")
                .child(span().text(format!("© 2026 active · {}", f.license)))
                .child(span().text(f.built))
                .child(a().href("#top").class("top").text(format!("{} ↑", f.top))),
        )
}

/// Makes a page installable: the manifest link with the theme color and the home screen
/// icon, and the registration of the offline service worker (see [`pwa`]).
fn installable(doc: Document) -> Document {
    doc.manifest(&link("/manifest.webmanifest"), &pwa::manifest())
        .service_worker(&link("/sw.js"))
}

/// A complete HTML page around `content`.
pub fn document(ctx: &Ctx<'_>, kind: &str, content: Node) -> String {
    let route = ctx.route;
    let assets = &ctx.site.assets;
    let body = fragment([
        Node::from(div().class("progress").attr("aria-hidden", "true")),
        header(route).into(),
        el("main")
            .id("main")
            .class(format!("page page-{kind}"))
            .child(content)
            .into(),
        footer(route).into(),
        div()
            .class("cursor")
            .attr("aria-hidden", "true")
            .child(span().class("dot"))
            .child(span().class("ring"))
            .into(),
        div().class("grain").attr("aria-hidden", "true").into(),
    ]);
    let site = Seo::new()
        .site_name("active")
        .title_template("%s — active")
        .base_url(ctx.origin)
        .image("/og.png")
        .locale(t(route.lang).locale);
    let mut doc = installable(
        Document::new(body)
            .lang(route.lang.code())
            .seo(site)
            .stylesheet(assets.url("/static/app.css"))
            .head(format!(
                concat!(
                    r#"<meta name="color-scheme" content="dark"/>"#,
                    r#"<link rel="icon" href="{base}/favicon.svg" type="image/svg+xml"/>"#,
                    // Same URL as in app.css (no version query), so the preload is reused.
                    r#"<link rel="preload" href="{base}/static/fonts/geist-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin=""/>"#,
                ),
                base = base()
            )),
    )
    .client(assets.url("/pkg/active_site.js"), assets.url("/pkg/active_site_bg.wasm"));
    if ctx.intro {
        doc = doc.html_attr("class", "intro");
    }
    if !base().is_empty() {
        // Read by the browser bundle to build its URLs.
        doc = doc.html_attr("data-base", base());
    }
    doc.render()
}

fn curtain() -> Element {
    // Pure CSS: rendered only on the first visit of a session (see `Ctx::intro`).
    div()
        .class("curtain")
        .attr("aria-hidden", "true")
        .child(
            svg()
                .attr("viewBox", "0 0 120 120")
                .child(
                    circle()
                        .class("ring")
                        .attr("cx", "60")
                        .attr("cy", "60")
                        .attr("r", "52")
                        .attr("pathLength", "100"),
                )
                .child(
                    circle()
                        .class("dot")
                        .attr("cx", "96.8")
                        .attr("cy", "23.2")
                        .attr("r", "7"),
                ),
        )
        .child(span().class("count"))
        .child(span().class("word").text("active"))
}

fn hero(lang: Lang) -> Element {
    let d = &t(lang).home;
    section()
        .class("hero")
        .attr("aria-labelledby", "hero-title")
        .child(island("dust", lang.code(), crate::islands::dust))
        .child(div().class("hero-glow").attr("aria-hidden", "true"))
        .child(
            div()
                .class("hero-content")
                .child(
                    p().class("eyebrow hero-in")
                        .style("--i:0")
                        .child(span().class("pulse").attr("aria-hidden", "true"))
                        .text(d.eyebrow),
                )
                .child(
                    h1().id("hero-title").class("hero-title").children(
                        d.hero
                            .iter()
                            .enumerate()
                            .map(|(i, line)| split(span().class("line"), line, i * 3)),
                    ),
                )
                .child(
                    div()
                        .class("hero-bottom")
                        .child(p().class("lead hero-in").style("--i:6").text(d.lead))
                        .child(
                            div()
                                .class("hero-actions hero-in")
                                .style("--i:7")
                                .child(
                                    a().class("btn")
                                        .href(link(&format!("/{}/docs/quick-start/", lang.code())))
                                        .attr("data-magnetic", "")
                                        .text(d.cta)
                                        .child(arrow()),
                                )
                                .child(
                                    a().class("btn ghost")
                                        .href(GITHUB)
                                        .attr("target", "_blank")
                                        .attr("rel", "noopener noreferrer")
                                        .attr("data-magnetic", "")
                                        .text(d.github),
                                ),
                        ),
                ),
        )
        .child(
            a().class("scroll-hint")
                .href("#manifesto")
                .attr("aria-label", d.scroll)
                .child(span().text(d.scroll))
                .child(i()),
        )
}

fn marquee(lang: Lang) -> Element {
    let words = t(lang).home.marquee;
    let row = |hidden: bool| {
        let list = ul().children(words.map(|w| {
            li().text(w)
                .child(span().class("star").attr("aria-hidden", "true").text("✦"))
        }));
        if hidden {
            list.attr("aria-hidden", "true")
        } else {
            list
        }
    };
    div()
        .class("marquee")
        .child(div().class("track").child(row(false)).child(row(true)))
}

fn section_head(kicker: String, title: &str, sub: Option<&str>) -> Element {
    div()
        .class("section-head")
        .child(span().class("kicker").attr("data-reveal", "").text(kicker))
        .child(heading(h2, title, "title"))
        .child(sub.map(|s| p().class("sub").attr("data-reveal", "").text(s.to_string())))
}

/// Benchmark rows: (name, value, is active, is baseline).
pub type Bars = &'static [(&'static str, f64, bool, bool)];

fn bars(title: &str, rows: Bars, unit: &str, lang: Lang) -> Element {
    let max = rows.iter().map(|r| r.1).fold(0.0, f64::max);
    let number = |v: f64| {
        let s = if v >= 100.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.1}")
        };
        if lang == Lang::En {
            s
        } else {
            s.replace('.', ",")
        }
    };
    figure()
        .class("bars")
        .attr("data-reveal", "")
        .child(figcaption().text(title.to_string()))
        .children(rows.iter().map(|(name, value, us, base)| {
            let mut class = String::from("bar");
            if *us {
                class.push_str(" us");
            }
            if *base {
                class.push_str(" base");
            }
            div()
                .class(class)
                .style(format!("--w:{:.3}", value / max))
                .child(span().class("name").text(name.to_string()))
                .child(span().class("track").child(span().class("fill")))
                .child(
                    span()
                        .class("value")
                        .text(format!("{} {unit}", number(*value))),
                )
        }))
}

fn home(ctx: &Ctx<'_>) -> Node {
    let lang = ctx.route.lang;
    let d = t(lang);
    let h = &d.home;
    let seo = alternates(Seo::new().title(h.title).title_template("%s").description(h.description).canonical(format!("/{}/", lang.code())), ctx.route)
        .json_ld(format!(
            r#"{{"@context":"https://schema.org","@type":"SoftwareSourceCode","name":"active","description":"{}","codeRepository":"{GITHUB}","programmingLanguage":"Rust","license":"https://opensource.org/licenses/MIT","url":"{}/{}/"}}"#,
            json(h.description),
            ctx.origin,
            lang.code()
        ))
        .json_ld(format!(r#"{{"@context":"https://schema.org","@type":"WebSite","name":"active","url":"{}/","inLanguage":"{}"}}"#, ctx.origin, lang.code()));
    let manifesto_words = h.manifesto.split_whitespace().count();
    fragment([
        Node::from(seo),
        curtain().into(),
        hero(lang).into(),
        marquee(lang).into(),
        section()
            .id("manifesto")
            .class("manifesto")
            .attr("data-scrub", "")
            .style(format!("--n:{manifesto_words}"))
            .child(split(p().class("big"), h.manifesto, 0))
            .into(),
        section()
            .id("features")
            .class("section features")
            .child(section_head(
                format!("01 — {}", d.nav.features),
                h.features_title,
                None,
            ))
            .child(
                div()
                    .class("cards")
                    .children(h.features.iter().enumerate().map(|(i, f)| {
                        article()
                            .class("card")
                            .attr("data-reveal", "")
                            .attr("data-spot", "")
                            .style(format!("--d:{}ms", i * 90))
                            .child(span().class("num").text(format!("0{}", i + 1)))
                            .child(h3().text(f.title))
                            .child(p().text(f.text))
                            .child(span().class("rule"))
                    })),
            )
            .child(div().class("stats").children(h.stats.iter().map(|s| {
                let shown = if lang == Lang::En {
                    format!("{:.*}", s.decimals, s.value)
                } else {
                    format!("{:.*}", s.decimals, s.value).replace('.', ",")
                };
                div()
                    .class("stat")
                    .attr("data-reveal", "")
                    .child(
                        strong()
                            .text(s.prefix)
                            .child(
                                span()
                                    .attr("data-count", s.value.to_string())
                                    .attr("data-decimals", s.decimals.to_string())
                                    .text(shown),
                            )
                            .text(s.suffix),
                    )
                    .child(span().text(s.label))
            })))
            .into(),
        section()
            .class("section code-section")
            .child(section_head(
                "02 — API".into(),
                h.code_title,
                Some(h.code_text),
            ))
            .child(island("tabs", lang.code(), crate::islands::tabs))
            .into(),
        section()
            .id("playground")
            .class("section playground")
            .child(section_head(
                format!("03 — {}", d.nav.playground),
                h.play_title,
                Some(h.play_text),
            ))
            .child(island(
                "playground",
                lang.code(),
                crate::islands::playground,
            ))
            .into(),
        section()
            .id("benchmarks")
            .class("section bench")
            .child(section_head(
                format!("04 — {}", d.nav.benchmarks),
                h.bench_title,
                Some(h.bench_text),
            ))
            .child(
                div()
                    .class("bench-grid")
                    .child(bars(h.bench_size, super::BENCH_SIZE, "kB", lang))
                    .child(bars(h.bench_ssr, super::BENCH_SSR, "µs", lang)),
            )
            .child(
                a().class("more")
                    .href(link(&format!("/{}/docs/benchmarks/", lang.code())))
                    .attr("data-reveal", "")
                    .text(h.bench_link)
                    .text(" ")
                    .child(arrow()),
            )
            .into(),
        section()
            .class("section steps")
            .child(section_head("05 — Start".into(), h.steps_title, None))
            .child(
                ol().children(
                    h.steps
                        .iter()
                        .zip([
                            "cargo add active",
                            "fn page() -> Node",
                            "active::hydrate(&ISLANDS)",
                        ])
                        .enumerate()
                        .map(|(i, (s, code_line))| {
                            li().attr("data-reveal", "")
                                .style(format!("--d:{}ms", i * 120))
                                .child(span().class("num").text((i + 1).to_string()))
                                .child(h3().text(s.title))
                                .child(p().text(s.text))
                                .child(code().text(code_line))
                        }),
                ),
            )
            .into(),
        section()
            .class("cta")
            .child(heading(h2, h.cta_title, "title huge"))
            .child(p().attr("data-reveal", "").text(h.cta_text))
            .child(
                div()
                    .class("cta-actions")
                    .attr("data-reveal", "")
                    .child(
                        a().class("btn")
                            .href(link(&format!("/{}/docs/", lang.code())))
                            .attr("data-magnetic", "")
                            .text(h.cta_docs)
                            .text(" ")
                            .child(arrow()),
                    )
                    .child(
                        a().class("btn ghost")
                            .href(HEROKU_DEPLOY)
                            .attr("target", "_blank")
                            .attr("rel", "noopener noreferrer")
                            .attr("data-magnetic", "")
                            .text(h.cta_deploy),
                    ),
            )
            .into(),
    ])
}

fn json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn sidebar(ctx: &Ctx<'_>, current: Option<&str>) -> Element {
    let lang = ctx.route.lang;
    let d = &t(lang).docs;
    let docs = &ctx.site.docs;
    aside()
        .class("sidebar")
        .child(island("search", lang.code(), crate::islands::search))
        .child(
            div()
                .class("sidebar-nav")
                .child(
                    button()
                        .class("sidebar-toggle")
                        .attr("type", "button")
                        .attr("aria-expanded", "false")
                        .text(d.title)
                        .child(span().attr("aria-hidden", "true").text("+")),
                )
                .child(nav().attr("aria-label", d.title).children(
                    CATEGORIES.iter().enumerate().filter_map(|(i, pages)| {
                        let pages: Vec<_> =
                            pages.iter().filter(|p| docs.slugs().contains(p)).collect();
                        (!pages.is_empty()).then(|| {
                            div()
                                .class("group")
                                .child(h2().text(d.categories[i].title))
                                .child(ul().children(pages.into_iter().map(|slug| {
                                    let link = a()
                                        .href(link(&format!("/{}/docs/{slug}/", lang.code())))
                                        .text(docs.title(lang, slug).to_string());
                                    li().child(if Some(*slug) == current {
                                        link.attr("aria-current", "page")
                                    } else {
                                        link
                                    })
                                })))
                        })
                    }),
                )),
        )
}

fn docs_index(ctx: &Ctx<'_>) -> Node {
    let lang = ctx.route.lang;
    let d = &t(lang).docs;
    let docs = &ctx.site.docs;
    let seo = alternates(
        Seo::new()
            .title(d.title)
            .description(d.description)
            .canonical(format!("/{}/docs/", lang.code())),
        ctx.route,
    );
    div()
        .class("docs docs-index")
        .child(seo)
        .child(
            el("header")
                .class("docs-hero")
                .child(span().class("kicker").text("active · Rust"))
                .child(heading(h1, d.title, "title"))
                .child(p().class("lead").text(d.intro))
                .child(island("search", lang.code(), crate::islands::search)),
        )
        .child(
            div()
                .class("categories")
                .children(CATEGORIES.iter().enumerate().filter_map(|(i, pages)| {
                    let pages: Vec<_> = pages.iter().filter(|p| docs.slugs().contains(p)).collect();
                    (!pages.is_empty()).then(|| {
                        section()
                            .class("category")
                            .attr("data-reveal", "")
                            .attr("data-spot", "")
                            .style(format!("--d:{}ms", i * 70))
                            .child(span().class("num").text(format!("0{}", i + 1)))
                            .child(h2().text(d.categories[i].title))
                            .child(p().text(d.categories[i].text))
                            .child(ul().children(pages.into_iter().map(|slug| {
                                let doc = docs.get(lang, slug);
                                li().child(
                                    a().href(link(&format!("/{}/docs/{slug}/", lang.code())))
                                        .child(strong().text(
                                            doc.map_or(slug.to_string(), |d| d.title.clone()),
                                        ))
                                        .child(span().text(
                                            doc.map_or(String::new(), |d| d.description.clone()),
                                        ))
                                        .child(arrow()),
                                )
                            })))
                    })
                })),
        )
        .into()
}

fn doc_page(ctx: &Ctx<'_>, slug: &str) -> Node {
    let lang = ctx.route.lang;
    let d = &t(lang).docs;
    let docs = &ctx.site.docs;
    let Some(doc) = docs.get(lang, slug) else {
        return not_found(lang);
    };
    let (prev, next) = docs.neighbours(slug);
    let category = d.categories[Docs::category(slug)].title;
    let seo = alternates(
        Seo::new()
            .title(doc.title.clone())
            .title_template("%s — active docs")
            .description(doc.description.clone())
            .canonical(format!("/{}/docs/{slug}/", lang.code()))
            .og_type("article"),
        ctx.route,
    )
    .json_ld(format!(
        r#"{{"@context":"https://schema.org","@type":"TechArticle","headline":"{}","description":"{}","inLanguage":"{}"}}"#,
        json(&doc.title),
        json(&doc.description),
        lang.code()
    ))
    .json_ld(format!(
        r#"{{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{{"@type":"ListItem","position":1,"name":"{}","item":"{o}/{l}/docs/"}},{{"@type":"ListItem","position":2,"name":"{}","item":"{o}/{l}/docs/{slug}/"}}]}}"#,
        json(d.title),
        json(&doc.title),
        o = ctx.origin,
        l = lang.code()
    ));
    let pager_link =
        |slug: Option<&str>, label: String, class: &'static str, rel: &'static str| -> Node {
            match slug {
                Some(s) => a()
                    .href(link(&format!("/{}/docs/{s}/", lang.code())))
                    .attr("rel", rel)
                    .class(class)
                    .child(small().text(label))
                    .child(span().text(docs.title(lang, s).to_string()))
                    .into(),
                None => span().into(),
            }
        };
    div()
        .class("docs")
        .child(seo)
        .child(sidebar(ctx, Some(slug)))
        .child(
            article()
                .class("doc")
                .child(
                    nav()
                        .class("crumbs")
                        .attr("aria-label", "Breadcrumb")
                        .child(
                            a().href(link(&format!("/{}/docs/", lang.code())))
                                .text(d.title),
                        )
                        .child(span().attr("aria-hidden", "true").text("/"))
                        .child(span().text(category)),
                )
                .child(h1().class("doc-title").text(doc.title.clone()))
                .child(p().class("doc-lead").text(doc.description.clone()))
                .child(div().class("prose").html(doc.html.clone()))
                .child(
                    a().class("edit")
                        .href(link(&format!(
                            "{GITHUB}/edit/master/site/docs/{}/{slug}.md",
                            lang.code()
                        )))
                        .attr("target", "_blank")
                        .attr("rel", "noopener noreferrer")
                        .text(d.edit),
                )
                .child(
                    nav()
                        .class("pager")
                        .attr("aria-label", "Pagination")
                        .child(pager_link(prev, format!("← {}", d.prev), "prev", "prev"))
                        .child(pager_link(next, format!("{} →", d.next), "next", "next")),
                ),
        )
        .child(nav().class("toc").attr("aria-label", d.on_this_page).child(
            (!doc.toc.is_empty()).then(|| {
                fragment([
                    Node::from(h2().text(d.on_this_page)),
                    ul().children(doc.toc.iter().map(|h| {
                        li().class(format!("d{}", h.depth))
                            .child(a().href(link(&format!("#{}", h.id))).text(h.text.clone()))
                    }))
                    .into(),
                ])
            }),
        ))
        .into()
}

fn not_found(lang: Lang) -> Node {
    let d = &t(lang).not_found;
    section()
        .class("notfound")
        .child(Seo::new().title(d.title).description(d.text).noindex(true))
        .child(
            span()
                .class("code404")
                .attr("aria-hidden", "true")
                .text("404"),
        )
        .child(heading(h1, d.title, "title"))
        .child(p().text(d.text))
        .child(
            a().class("btn")
                .href(link(&format!("/{}/", lang.code())))
                .attr("data-magnetic", "")
                .text(d.back),
        )
        .into()
}

/// The page the service worker shows when the network is down and the page asked for is
/// not saved on the device. It speaks the three languages, since the worker has only one
/// such page, and links the home pages, which are always saved.
fn offline() -> Node {
    let en = &t(Lang::En).offline;
    let home = |lang: Lang, class: &str| {
        a().class(class.to_string())
            .href(link(&format!("/{}/", lang.code())))
            .attr("hreflang", lang.code())
            .attr("lang", lang.code())
            .attr("data-magnetic", "")
            .text(t(lang).offline.back)
            .child(arrow())
    };
    section()
        .class("notfound offline")
        .child(
            Seo::new()
                .title(en.title.replace('*', ""))
                .description(en.text)
                .noindex(true),
        )
        .child(
            span()
                .class("code404")
                .attr("aria-hidden", "true")
                .text("offline"),
        )
        .child(heading(h1, en.title, "title"))
        .child(p().text(en.text))
        .child(home(Lang::En, "btn"))
        .child(
            div()
                .class("offline-more")
                .children([Lang::Fr, Lang::Es].map(|lang| {
                    let d = &t(lang).offline;
                    div()
                        .attr("lang", lang.code())
                        .child(p().class("offline-title").text(d.title.replace('*', "")))
                        .child(p().text(d.text))
                        .child(home(lang, "btn ghost small"))
                })),
        )
        .into()
}

/// The root page of a static export, where no server reads `Accept-Language`: it sends
/// the visitor to the English home page at once and links the three languages.
pub fn language_picker(origin: &str) -> String {
    let seo = Seo::new()
        .title("active")
        .description(t(Lang::En).home.description)
        .base_url(origin)
        .canonical("/en/")
        .noindex(true);
    let body = main()
        .class("doc-body")
        .child(h1().text("active"))
        .child(ul().children(Lang::ALL.map(|l| {
            li().child(
                a().href(link(&format!("/{}/", l.code())))
                    .attr("hreflang", l.code())
                    .attr("lang", l.code())
                    .text(t(l).lang_name),
            )
        })));
    installable(Document::new(body).lang("en").seo(seo).head(format!(
        r#"<meta http-equiv="refresh" content="0; url={}/en/"/>"#,
        base()
    )))
    .render()
}

/// Renders the page of `ctx.route`.
pub fn render(ctx: &Ctx<'_>) -> String {
    let (kind, content) = match &ctx.route.page {
        Page::Home => ("home", home(ctx)),
        Page::Docs => ("docs", docs_index(ctx)),
        Page::Doc(slug) => ("doc", doc_page(ctx, slug)),
        Page::NotFound => ("404", not_found(ctx.route.lang)),
        Page::Offline => ("offline", offline()),
    };
    document(ctx, kind, content)
}
