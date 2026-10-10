//! Documentation pages: Markdown with a small frontmatter, rendered once at startup.

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html};

use super::highlight::{highlight, label};

pub struct Doc {
    pub title: String,
    pub description: String,
    pub html: String,
    pub toc: Vec<TocItem>,
}

pub struct TocItem {
    pub depth: u8,
    pub id: String,
    pub text: String,
}

/// `title:` and `description:` between `---` lines at the top of the file.
fn frontmatter(src: &str) -> (Option<&str>, Option<&str>, &str) {
    let Some(rest) = src
        .strip_prefix("---\n")
        .or_else(|| src.strip_prefix("---\r\n"))
    else {
        return (None, None, src);
    };
    let Some(end) = rest.find("\n---") else {
        return (None, None, src);
    };
    let (head, body) = (&rest[..end], &rest[end + 4..]);
    let body = body.strip_prefix('\r').unwrap_or(body);
    let body = body.strip_prefix('\n').unwrap_or(body);
    let field = |name: &str| {
        head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then(|| value.trim().trim_matches(|c| c == '"' || c == '\''))
        })
    };
    (field("title"), field("description"), body)
}

/// Heading ids: lowercase ASCII, accents removed, other characters collapsed to `-`.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in crate::search::norm(text).chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

enum Buffer<'a> {
    Heading(u8, Vec<Event<'a>>),
    Quote(Vec<Event<'a>>),
}

/// Renders a page: heading anchors and table of contents, highlighted code with a copy
/// button, scrollable tables, callouts (`> **Note:**`, `> **Tip:**`, `> **Warning:**` and
/// their French/Spanish names) and external links opening in a new tab.
pub fn render(src: &str) -> Doc {
    let (title, description, body) = frontmatter(src);
    let mut out: Vec<Event<'_>> = Vec::new();
    let mut stack: Vec<Buffer<'_>> = Vec::new();
    let mut toc = Vec::new();
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut code: Option<(String, String)> = None;
    let mut external = Vec::new();

    fn emit<'a>(stack: &mut [Buffer<'a>], out: &mut Vec<Event<'a>>, event: Event<'a>) {
        match stack.last_mut() {
            Some(Buffer::Heading(_, events) | Buffer::Quote(events)) => events.push(event),
            None => out.push(event),
        }
    }

    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    for event in Parser::new_ext(body, options) {
        if let Some((lang, text)) = code.as_mut() {
            match event {
                Event::Text(t) => text.push_str(&t),
                Event::End(TagEnd::CodeBlock) => {
                    let html = format!(
                        r#"<div class="code" data-lang="{}"><button class="copy" type="button" aria-label="Copy">Copy</button><pre><code>{}</code></pre></div>"#,
                        escape(label(lang)),
                        highlight(text, lang)
                    );
                    code = None;
                    emit(&mut stack, &mut out, Event::Html(html.into()));
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_string()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                stack.push(Buffer::Heading(level as u8, Vec::new()))
            }
            Event::End(TagEnd::Heading(_)) => {
                let Some(Buffer::Heading(depth, events)) = stack.pop() else {
                    continue;
                };
                let text: String = events
                    .iter()
                    .filter_map(|e| match e {
                        Event::Text(t) | Event::Code(t) => Some(t.as_ref()),
                        _ => None,
                    })
                    .collect();
                let mut id = slugify(&text);
                if id.is_empty() {
                    id = "section".into();
                }
                match seen.iter_mut().find(|(s, _)| *s == id) {
                    Some((_, n)) => {
                        *n += 1;
                        id = format!("{id}-{n}");
                    }
                    None => seen.push((id.clone(), 0)),
                }
                let mut inner = String::new();
                html::push_html(&mut inner, events.into_iter());
                if depth == 2 || depth == 3 {
                    toc.push(TocItem {
                        depth,
                        id: id.clone(),
                        text,
                    });
                }
                let heading = format!(
                    r##"<h{depth} id="{id}"><a class="anchor" href="#{id}" aria-hidden="true" tabindex="-1">#</a>{inner}</h{depth}>"##
                );
                emit(&mut stack, &mut out, Event::Html(heading.into()));
            }
            Event::Start(Tag::BlockQuote(_)) => stack.push(Buffer::Quote(Vec::new())),
            Event::End(TagEnd::BlockQuote(_)) => {
                let Some(Buffer::Quote(events)) = stack.pop() else {
                    continue;
                };
                let mut inner = String::new();
                html::push_html(&mut inner, events.into_iter());
                let word = inner
                    .strip_prefix("<p><strong>")
                    .map(|rest| {
                        rest.split(|c: char| !c.is_alphabetic())
                            .next()
                            .unwrap_or("")
                            .to_lowercase()
                    })
                    .unwrap_or_default();
                let class = match word.as_str() {
                    "tip" | "astuce" | "consejo" => "tip",
                    "warning" | "attention" | "advertencia" => "warning",
                    _ => "note",
                };
                emit(
                    &mut stack,
                    &mut out,
                    Event::Html(
                        format!(r#"<blockquote class="callout {class}">{inner}</blockquote>"#)
                            .into(),
                    ),
                );
            }
            Event::Start(Tag::Table(alignments)) => {
                emit(
                    &mut stack,
                    &mut out,
                    Event::Html(r#"<div class="table">"#.into()),
                );
                emit(&mut stack, &mut out, Event::Start(Tag::Table(alignments)));
            }
            Event::End(TagEnd::Table) => {
                emit(&mut stack, &mut out, Event::End(TagEnd::Table));
                emit(&mut stack, &mut out, Event::Html("</div>".into()));
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let is_external =
                    dest_url.starts_with("http://") || dest_url.starts_with("https://");
                external.push(is_external);
                if is_external {
                    let title = if title.is_empty() {
                        String::new()
                    } else {
                        format!(r#" title="{}""#, escape(&title))
                    };
                    let open = format!(
                        r#"<a href="{}"{title} target="_blank" rel="noopener noreferrer">"#,
                        escape(&dest_url)
                    );
                    emit(&mut stack, &mut out, Event::Html(open.into()));
                } else {
                    emit(
                        &mut stack,
                        &mut out,
                        Event::Start(Tag::Link {
                            link_type,
                            dest_url,
                            title,
                            id,
                        }),
                    );
                }
            }
            Event::End(TagEnd::Link) => {
                let event = if external.pop() == Some(true) {
                    Event::Html(CowStr::from("</a>"))
                } else {
                    Event::End(TagEnd::Link)
                };
                emit(&mut stack, &mut out, event);
            }
            other => emit(&mut stack, &mut out, other),
        }
    }
    let mut html_out = String::with_capacity(body.len() * 2);
    html::push_html(&mut html_out, out.into_iter());
    Doc {
        title: title.unwrap_or_default().to_string(),
        description: description.unwrap_or_default().to_string(),
        html: html_out,
        toc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_the_documented_rules() {
        assert_eq!(
            slugify("Ce que fournit le dépôt"),
            "ce-que-fournit-le-depot"
        );
        assert_eq!(
            slugify("Hidratación y WebAssembly"),
            "hidratacion-y-webassembly"
        );
        assert_eq!(slugify("The `<Seo>` component!"), "the-seo-component");
        assert_eq!(slugify("  --a__b--  "), "a-b");
    }

    #[test]
    fn renders_headings_code_tables_callouts_and_links() {
        let doc = render(concat!(
            "---\ntitle: Hello\ndescription: \"World\"\n---\n",
            "Intro with [a link](https://x.dev) and [docs](../seo/).\n\n",
            "## Install it\n\n```toml\nactive = \"0.1\"\n```\n\n",
            "### Install it\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n",
            "> **Tip:** read [this](https://y.dev).\n"
        ));
        assert_eq!(
            (doc.title.as_str(), doc.description.as_str()),
            ("Hello", "World")
        );
        assert!(doc.html.contains(
            r#"<a href="https://x.dev" target="_blank" rel="noopener noreferrer">a link</a>"#
        ));
        assert!(doc.html.contains(r#"<a href="../seo/">docs</a>"#));
        assert!(
            doc.html
                .contains(r##"<h2 id="install-it"><a class="anchor" href="#install-it""##)
        );
        assert!(doc.html.contains(r#"<h3 id="install-it-1">"#));
        assert!(
            doc.html
                .contains(r#"<div class="code" data-lang="toml"><button class="copy""#)
        );
        assert!(doc.html.contains(r#"<div class="table"><table>"#));
        assert!(doc.html.contains(r#"<blockquote class="callout tip"><p><strong>Tip:</strong> read <a href="https://y.dev" target="_blank""#));
        assert_eq!(doc.toc.len(), 2);
    }
}
