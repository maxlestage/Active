//! The interactive parts of the site. Each island renders on the server like any view and,
//! compiled to WebAssembly, hydrates in the browser from the same code.

use active::prelude::*;

use crate::labels::{fixed, labels};
use crate::routes::Lang;

fn lang(props: &str) -> Lang {
    Lang::parse(props).unwrap_or(Lang::En)
}

/// Every island, registered under the name used by [`island`].
pub const ISLANDS: [(&str, active::IslandFn); 4] = [
    ("dust", dust),
    ("tabs", tabs),
    ("playground", playground),
    ("search", search),
];

/// Hero background: particles simulated in Rust gather into the word "active".
pub fn dust(props: &str) -> Node {
    let d = labels(lang(props));
    let particles = use_state(0usize);
    let fps = use_state(0u32);
    let live = use_state(false);
    fragment([
        canvas()
            .class("dust")
            .class_if("live", move || live.get())
            .attr("aria-hidden", "true")
            .on_mount(move |_canvas| {
                #[cfg(target_arch = "wasm32")]
                crate::client::dust::start(_canvas, particles, fps, live);
            }),
        p().class("hud")
            .attr("aria-hidden", "true")
            .child(span().class("tag").text("Rust → wasm"))
            .text(" ")
            .text(d.dust)
            .text(" · ")
            .text_dyn(move || match particles.get() {
                0 => "—".into(),
                n => n.to_string(),
            })
            .text(" ")
            .text(d.particles)
            .text(" · ")
            .text_dyn(move || match fps.get() {
                0 => "—".into(),
                n => n.to_string(),
            })
            .text(" fps"),
    ])
}

/// Highlighted code of the home page tabs. The server renders it; in the browser the
/// panels keep the server HTML (raw HTML is never touched by hydration).
fn snippet(index: usize) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    return crate::server::snippets::html(index);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = index;
        String::new()
    }
}

/// Code examples with tabs.
pub fn tabs(props: &str) -> Node {
    let d = labels(lang(props));
    let tab = use_state(0usize);
    div()
        .class("window")
        .attr("data-reveal", "")
        .child(
            div()
                .class("tabs")
                .attr("role", "tablist")
                .attr_dyn("style", move || format!("--tab:{}", tab.get()))
                .children(d.tabs.iter().enumerate().map(|(i, label)| {
                    button()
                        .attr("role", "tab")
                        .attr("type", "button")
                        .attr_dyn("aria-selected", move || (tab.get() == i).to_string())
                        .on_click(move |_| tab.set(i))
                        .text(*label)
                }))
                .child(span().class("indicator").attr("aria-hidden", "true")),
        )
        .children((0..d.tabs.len()).map(|i| {
            div()
                .class("panel code")
                .attr("role", "tabpanel")
                .bool_attr("hidden", move || tab.get() != i)
                .child(
                    button()
                        .class("copy")
                        .attr("type", "button")
                        .attr("aria-label", "Copy")
                        .text("Copy"),
                )
                .child(div().html(snippet(i)))
        }))
        .into()
}

const PLAY_ORIGIN: &str = "https://acme.com";

/// The head tags of the playground page, by the same `active::Seo` used everywhere.
pub fn playground_head(title: &str, description: &str, path: &str) -> String {
    let json = format!(
        r#"{{"@context":"https://schema.org","@type":"WebPage","name":"{}","description":"{}"}}"#,
        json_escape(title),
        json_escape(description)
    );
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url(PLAY_ORIGIN)
        .image("/og.png")
        .title(title)
        .description(description)
        .canonical(path)
        .json_ld(json)
        .to_html()
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// One tag per line with light colouring; the input is already escaped by `Seo`.
fn pretty(html: &str) -> String {
    let mut out = String::with_capacity(html.len() * 2);
    for (n, line) in html.replace("><", ">\n<").lines().enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let escaped = line.replace('&', "&amp;").replace('<', "&lt;");
        // Attribute names and values, then tag names (attributes first: the tag markup
        // inserted afterwards must not be matched as an attribute).
        let mut colored = String::new();
        let mut rest = escaped.as_str();
        while let Some(eq) = rest.find("=\"") {
            let name_start = rest[..eq].rfind(' ').map_or(0, |s| s + 1);
            let Some(end) = rest[eq + 2..].find('"') else {
                break;
            };
            colored.push_str(&rest[..name_start]);
            colored.push_str(&format!(
                "<i class=\"at\">{}</i>=<u class=\"st\">\"{}\"</u>",
                &rest[name_start..eq],
                &rest[eq + 2..eq + 2 + end]
            ));
            rest = &rest[eq + 3 + end..];
        }
        colored.push_str(rest);
        out.push_str(&tag_names(&colored));
    }
    out
}

fn tag_names(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(i) = rest.find("&lt;") {
        out.push_str(&rest[..i + 4]);
        rest = &rest[i + 4..];
        let slash = rest.starts_with('/');
        if slash {
            out.push('/');
            rest = &rest[1..];
        }
        let len = rest
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        out.push_str(&format!("<b class=\"tg\">{}</b>", &rest[..len]));
        rest = &rest[len..];
    }
    out.push_str(rest);
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}…", s.chars().take(max - 2).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Live SEO playground: edit a page, see its head tags, a search result and a social card.
pub fn playground(props: &str) -> Node {
    let p = labels(lang(props));
    let title = use_state(p.title_value.to_string());
    let description = use_state(p.description_value.to_string());
    let path = use_state(p.path_value.to_string());
    // Where the output was computed: the server first, then Rust/wasm in the browser.
    let took = use_state(None::<f64>);
    let (server, browser) = (p.server, p.browser);

    let output = move || {
        let start = now();
        let html = title.with(|t| description.with(|d| path.with(|p| playground_head(t, d, p))));
        if let Some(start) = start {
            let ms = now().unwrap_or(start) - start;
            if took.with(|t| t.is_some()) {
                took.update(|t| *t = Some(ms));
            }
        }
        pretty(&html)
    };

    let field = |state: State<String>, max: &'static str| {
        input()
            .attr("value", state.get())
            .attr("maxlength", max)
            .on_input(move |e| {
                if took.with(Option::is_none) {
                    took.set(Some(0.0));
                }
                state.set(e.value());
            })
    };

    div()
        .class("play")
        .attr("data-reveal", "")
        .child(
            form()
                .class("play-form")
                .on("submit", |e| e.prevent_default())
                .child(
                    label()
                        .child(span().text(p.title))
                        .child(field(title, "90")),
                )
                .child(
                    label().child(span().text(p.description)).child(
                        textarea()
                            .attr("rows", "3")
                            .attr("maxlength", "220")
                            .on_input(move |e| {
                                if took.with(Option::is_none) {
                                    took.set(Some(0.0));
                                }
                                description.set(e.value());
                            })
                            .text(p.description_value),
                    ),
                )
                .child(label().child(span().text(p.path)).child(field(path, "80")))
                .child(
                    div()
                        .class("previews")
                        .child(
                            div()
                                .class("serp")
                                .attr("aria-label", p.google)
                                .child(small().text(p.google))
                                .child(span().class("serp-url").text_dyn(move || {
                                    format!(
                                        "acme.com › {}",
                                        path.get().trim_start_matches('/').replace('/', " › ")
                                    )
                                }))
                                .child(span().class("serp-title").text_dyn(move || {
                                    let t = title.get();
                                    truncate(
                                        &if t.is_empty() {
                                            "Acme".into()
                                        } else {
                                            format!("{t} · Acme")
                                        },
                                        60,
                                    )
                                }))
                                .child(
                                    span()
                                        .class("serp-desc")
                                        .text_dyn(move || truncate(&description.get(), 155)),
                                ),
                        )
                        .child(
                            div()
                                .class("social")
                                .attr("aria-label", p.social)
                                .child(small().text(p.social))
                                .child(
                                    div()
                                        .class("social-img")
                                        .attr("aria-hidden", "true")
                                        .child(span().text("active")),
                                )
                                .child(span().class("social-domain").text("acme.com"))
                                .child(span().class("social-title").text_dyn(move || {
                                    let t = title.get();
                                    if t.is_empty() { "Acme".into() } else { t }
                                })),
                        ),
                ),
        )
        .child(
            div()
                .class("play-out")
                .child(
                    div().class("play-bar").child(span().text(p.output)).child(
                        span()
                            .class("took")
                            .child(span().class("tag").text("Rust"))
                            .text(" ")
                            .text_dyn(move || match took.get() {
                                None => server.to_string(),
                                Some(ms) if ms < 0.01 => format!("{browser} · <0.01 ms"),
                                Some(ms) => format!("{browser} · {} ms", fixed(ms, 2)),
                            }),
                    ),
                )
                .child(
                    pre()
                        .attr("aria-live", "polite")
                        .child(code().html_dyn(output)),
                ),
        )
        .into()
}

/// Milliseconds since page load in the browser; `None` on the server.
fn now() -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    return web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now());
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Docs search: a button opening a dialog; results come from the language's search index.
pub fn search(props: &str) -> Node {
    let lang = lang(props);
    let d = labels(lang);
    let results = use_state(Vec::<(String, String, String)>::new());
    let active = use_state(0usize);
    let loaded = use_state(false);

    let go = move || {
        if let Some(href) = results.with(|r| r.get(active.get()).map(|r| r.0.clone())) {
            #[cfg(target_arch = "wasm32")]
            crate::client::search::navigate(&href);
            let _ = href;
        }
    };

    fragment([
        button()
            .class("search-btn")
            .attr("type", "button")
            .on_click(move |_e| {
                #[cfg(target_arch = "wasm32")]
                crate::client::search::open(&_e, lang, results, loaded);
            })
            .on_mount(move |_button| {
                #[cfg(target_arch = "wasm32")]
                crate::client::search::shortcut(_button);
            })
            .child(
                svg()
                    .attr("viewBox", "0 0 24 24")
                    .attr("aria-hidden", "true")
                    .attr("width", "16")
                    .attr("height", "16")
                    .child(
                        circle()
                            .attr("cx", "11")
                            .attr("cy", "11")
                            .attr("r", "7")
                            .attr("fill", "none")
                            .attr("stroke", "currentColor")
                            .attr("stroke-width", "2"),
                    )
                    .child(
                        path()
                            .attr("d", "m20 20-4-4")
                            .attr("stroke", "currentColor")
                            .attr("stroke-width", "2")
                            .attr("stroke-linecap", "round"),
                    ),
            )
            .child(span().text(d.search))
            .child(kbd().text("/")),
        dialog()
            .class("search")
            .attr("aria-label", d.search)
            .on_click(|_e| {
                #[cfg(target_arch = "wasm32")]
                crate::client::search::close_on_backdrop(&_e);
            })
            .child(
                div()
                    .class("search-box")
                    .child(
                        input()
                            .attr("type", "search")
                            .attr("placeholder", d.search_hint)
                            .attr("aria-label", d.search)
                            .on_input(move |e| {
                                active.set(0);
                                #[cfg(target_arch = "wasm32")]
                                crate::client::search::query(lang, e.value(), results, loaded);
                                let _ = e;
                            })
                            .on("keydown", move |e| match e.key().as_str() {
                                "ArrowDown" => {
                                    e.prevent_default();
                                    let n = results.with(Vec::len);
                                    active.update(|a| *a = (*a + 1).min(n.saturating_sub(1)));
                                }
                                "ArrowUp" => {
                                    e.prevent_default();
                                    active.update(|a| *a = a.saturating_sub(1));
                                }
                                "Enter" => {
                                    e.prevent_default();
                                    go();
                                }
                                _ => {}
                            }),
                    )
                    .child(ul().attr("role", "listbox").children_dyn(move || {
                        let list = results.get();
                        if list.is_empty() && loaded.get() {
                            return vec![li().class("empty").text(d.no_results).into()];
                        }
                        list.into_iter()
                            .enumerate()
                            .map(|(i, (href, title, sub))| {
                                li().attr("role", "option")
                                    .attr_dyn("aria-selected", move || {
                                        (active.get() == i).to_string()
                                    })
                                    .child(
                                        a().href(href)
                                            .on("mouseenter", move |_| active.set(i))
                                            .child(strong().text(title))
                                            .child(span().text(sub)),
                                    )
                                    .into()
                            })
                            .collect()
                    })),
            ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playground_output_is_colored_and_escaped() {
        let html = pretty(&playground_head("A <b> & \"c\"", "d", "/x"));
        assert!(html.starts_with("&lt;<b class=\"tg\">title</b>>A &amp;lt;b&amp;gt; &amp;amp; &amp;quot;c&amp;quot; · Acme&lt;/<b class=\"tg\">title</b>>"));
        assert!(html.contains("<i class=\"at\">rel</i>=<u class=\"st\">\"canonical\"</u>"));
    }

    #[test]
    fn islands_render_on_the_server() {
        for (name, island_fn) in ISLANDS {
            let html = active::island(name, "fr", island_fn).render();
            assert!(
                html.starts_with(&format!(
                    r#"<active-island data-island="{name}" data-props="fr">"#
                )),
                "{name}"
            );
        }
        let play = active::island("playground", "en", playground).render();
        assert!(play.contains("rendered on the server"));
        assert!(play.contains("title</b>>Handmade oak chair · Acme"));
    }
}
