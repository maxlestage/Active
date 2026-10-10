//! Checks a rendered SSR page tag by tag, whatever the attribute order, quoting of void
//! elements (`<meta>` / `<meta/>`) or hydration markers a framework adds.

use crate::page::{Product, SITE, TWITTER, option_label};

/// Verifies the page rendered for product `i`: `<html lang="en">`, every SEO tag exactly
/// once with the right value and in `<head>`, the JSON-LD script, the `<h1>` and the 30
/// list items in `<body>`.
pub fn check_page(i: u32, html: &str) -> Result<(), String> {
    let p = Product::new(i);
    let tokens = tokenize(html)?;

    if !html
        .get(..15)
        .is_some_and(|s| s.eq_ignore_ascii_case("<!DOCTYPE html>"))
    {
        return Err("the page does not start with <!DOCTYPE html>".into());
    }
    let html_tag = one(&tokens, "html", |_| true)?;
    if attr(&tokens[html_tag], "lang") != Some("en") {
        return Err(r#"<html> has no lang="en""#.into());
    }
    let head_end = tokens
        .iter()
        .position(|t| matches!(t, Token::End(name) if name == "head"))
        .ok_or("no </head>")?;
    let body_start = tokens
        .iter()
        .position(|t| matches!(t, Token::Start { name, .. } if name == "body"))
        .ok_or("no <body>")?;
    let in_head = |index: usize| {
        if index < head_end {
            Ok(())
        } else {
            Err("a SEO tag is outside <head>".to_string())
        }
    };

    // Each SEO tag must appear exactly once in the whole document, inside <head>.
    let title = expect_text(&tokens, "title", |_| true, &p.title())?;
    in_head(title)?;
    let metas: [(&str, &str, &str); 9] = [
        ("name", "description", &p.description),
        ("property", "og:title", &p.name),
        ("property", "og:description", &p.description),
        ("property", "og:type", "website"),
        ("property", "og:url", &p.url),
        ("property", "og:image", &p.image),
        ("property", "og:site_name", SITE),
        ("name", "twitter:card", "summary_large_image"),
        ("name", "twitter:site", TWITTER),
    ];
    for (key_attr, key, content) in metas {
        let index = one(&tokens, "meta", |t| attr(t, key_attr) == Some(key))?;
        in_head(index)?;
        let found = attr(&tokens[index], "content");
        if found != Some(content) {
            return Err(format!(
                "<meta {key_attr}=\"{key}\"> content is {found:?}, expected {content:?}"
            ));
        }
    }
    let canonical = one(&tokens, "link", |t| attr(t, "rel") == Some("canonical"))?;
    in_head(canonical)?;
    if attr(&tokens[canonical], "href") != Some(p.url.as_str()) {
        return Err(format!(
            "canonical href is {:?}",
            attr(&tokens[canonical], "href")
        ));
    }
    let json_ld = expect_text(
        &tokens,
        "script",
        |t| attr(t, "type") == Some("application/ld+json"),
        &p.json_ld,
    )?;
    in_head(json_ld)?;

    let body = &tokens[body_start..];
    expect_text(body, "h1", |_| true, &p.name)?;
    let items = texts(body, "li", |_| true);
    let expected: Vec<String> = p
        .options()
        .map(|(k, price)| option_label(k, price))
        .collect();
    if items != expected {
        return Err(format!(
            "expected {} <li> {:?}…, found {} {:?}…",
            expected.len(),
            expected.first(),
            items.len(),
            items.first()
        ));
    }
    Ok(())
}

/// Attributes in source order: `(name, value)`.
type Attrs = Vec<(String, String)>;

#[derive(Debug)]
enum Token {
    Start { name: String, attrs: Attrs },
    End(String),
    Text(String),
}

/// A minimal HTML tokenizer: start tags with attributes, end tags, text (entities decoded),
/// raw text inside `<script>`/`<style>`. Comments and doctypes are skipped.
fn tokenize(html: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            let end = after.find("-->").ok_or("unclosed comment")?;
            rest = &after[end + 3..];
        } else if rest.starts_with("<!") {
            let end = rest.find('>').ok_or("unclosed doctype")?;
            rest = &rest[end + 1..];
        } else if let Some(after) = rest.strip_prefix("</") {
            let end = after.find('>').ok_or("unclosed end tag")?;
            tokens.push(Token::End(after[..end].trim().to_ascii_lowercase()));
            rest = &after[end + 1..];
        } else if rest.starts_with('<') && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            let (name, attrs, after) = start_tag(&rest[1..])?;
            rest = after;
            if name == "script" || name == "style" {
                let close = format!("</{name}");
                let end = rest.find(&close).ok_or("unclosed raw text element")?;
                tokens.push(Token::Start { name, attrs });
                tokens.push(Token::Text(rest[..end].to_string()));
                rest = &rest[end..];
            } else {
                tokens.push(Token::Start { name, attrs });
            }
        } else {
            let first = rest.chars().next().map_or(1, char::len_utf8);
            let end = rest[first..].find('<').map_or(rest.len(), |e| e + first);
            tokens.push(Token::Text(decode(&rest[..end])));
            rest = &rest[end..];
        }
    }
    Ok(tokens)
}

/// Parses `name attr="value" …>` (after the `<`); returns the rest after `>`.
fn start_tag(s: &str) -> Result<(String, Attrs, &str), String> {
    let name_end = s
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .ok_or("unclosed tag")?;
    let name = s[..name_end].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut rest = &s[name_end..];
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("/>").or_else(|| rest.strip_prefix('>')) {
            return Ok((name, attrs, after));
        }
        rest = rest.strip_prefix('/').unwrap_or(rest);
        let key_end = rest
            .find(|c: char| c.is_whitespace() || "=>/".contains(c))
            .ok_or("unclosed tag")?;
        let key = rest[..key_end].to_ascii_lowercase();
        rest = rest[key_end..].trim_start();
        let value = if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (value, after) = match after.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let end = after[1..].find(q).ok_or("unclosed attribute value")?;
                    (&after[1..=end], &after[end + 2..])
                }
                _ => {
                    let end = after
                        .find(|c: char| c.is_whitespace() || c == '>')
                        .unwrap_or(after.len());
                    after.split_at(end)
                }
            };
            rest = after;
            decode(value)
        } else {
            String::new()
        };
        if !key.is_empty() {
            attrs.push((key, value));
        }
    }
}

fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn attr<'a>(token: &'a Token, name: &str) -> Option<&'a str> {
    match token {
        Token::Start { attrs, .. } => attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str()),
        _ => None,
    }
}

fn starts<'a>(
    tokens: &'a [Token],
    tag: &'a str,
    filter: impl Fn(&Token) -> bool + 'a,
) -> impl Iterator<Item = usize> + 'a {
    tokens
        .iter()
        .enumerate()
        .filter(move |(_, t)| matches!(t, Token::Start { name, .. } if name == tag) && filter(t))
        .map(|(index, _)| index)
}

/// Index of the only `<tag>` matching `filter`.
fn one(tokens: &[Token], tag: &str, filter: impl Fn(&Token) -> bool) -> Result<usize, String> {
    let found: Vec<usize> = starts(tokens, tag, filter).collect();
    match found.as_slice() {
        [index] => Ok(*index),
        _ => Err(format!(
            "expected one matching <{tag}>, found {}",
            found.len()
        )),
    }
}

/// Text content of every `<tag>` matching `filter` (no nested elements expected).
fn texts(tokens: &[Token], tag: &str, filter: impl Fn(&Token) -> bool) -> Vec<String> {
    starts(tokens, tag, filter)
        .map(|start| {
            tokens[start + 1..]
                .iter()
                .take_while(|t| !matches!(t, Token::End(name) if name == tag))
                .filter_map(|t| match t {
                    Token::Text(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// Index of the only `<tag>` matching `filter`, checking its text.
fn expect_text(
    tokens: &[Token],
    tag: &str,
    filter: impl Fn(&Token) -> bool + Copy,
    expected: &str,
) -> Result<usize, String> {
    match texts(tokens, tag, filter).as_slice() {
        [text] if text == expected => one(tokens, tag, filter),
        [text] => Err(format!("<{tag}> is {text:?}, expected {expected:?}")),
        found => Err(format!("expected one <{tag}>, found {}", found.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(i: u32) -> String {
        let p = Product::new(i);
        let mut html = format!(
            r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><title>{}</title><meta content="{}" name="description"/><link href="{}" rel="canonical">"#,
            p.title(),
            p.description,
            p.url
        );
        for (k, v) in [
            ("og:title", &*p.name),
            ("og:description", &p.description),
            ("og:type", "website"),
            ("og:url", &p.url),
            ("og:image", &p.image),
            ("og:site_name", SITE),
        ] {
            html += &format!(r#"<meta property="{k}" content="{v}">"#);
        }
        html += &format!(
            r#"<meta name="twitter:card" content="summary_large_image"><meta name="twitter:site" content="{TWITTER}">"#
        );
        html += &format!(
            r#"<script type="application/ld+json">{}</script></head><body data-hk="1"><h1>{}<!----></h1><ul>"#,
            p.json_ld, p.name
        );
        for (k, price) in p.options() {
            html += &format!("<li>{}</li>", option_label(k, price));
        }
        html + "</ul><!></body></html>"
    }

    #[test]
    fn accepts_a_correct_page_in_any_attribute_order() {
        assert_eq!(check_page(7, &reference(7)), Ok(()));
    }

    #[test]
    fn rejects_duplicates_wrong_values_and_misplaced_tags() {
        let page = reference(7);
        let dup = page.replace("</head>", r#"<meta name="description" content="x"></head>"#);
        assert!(check_page(7, &dup).is_err());
        assert!(check_page(8, &page).is_err());
        let moved = page
            .replace(r#"<meta name="twitter:site" content="@acme">"#, "")
            .replace("<h1>", r#"<meta name="twitter:site" content="@acme"><h1>"#);
        assert!(check_page(7, &moved).is_err());
        assert!(check_page(7, &page.replace("<li>Option 3", "<li>Option 4")).is_err());
    }
}
