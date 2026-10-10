//! Syntax highlighting with syntect: spans with `s-…` classes, coloured by app.css.

use std::sync::OnceLock;

use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// The label shown on a code block ("" for plain text).
pub fn label(lang: &str) -> &str {
    match lang {
        "sh" | "bash" | "shell" | "console" => "terminal",
        "" | "text" | "txt" | "plain" => "",
        other => other,
    }
}

/// `code` as HTML spans (already escaped).
pub fn highlight(code: &str, lang: &str) -> String {
    let set = syntaxes();
    let token = match lang {
        "sh" | "shell" | "console" | "terminal" => "bash",
        "rs" => "rust",
        "yml" => "yaml",
        other => other,
    };
    let syntax = set
        .find_syntax_by_token(token)
        .unwrap_or_else(|| set.find_syntax_plain_text());
    let mut html = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        set,
        ClassStyle::SpacedPrefixed { prefix: "s-" },
    );
    for line in LinesWithEndings::from(code) {
        if html
            .parse_html_for_line_which_includes_newline(line)
            .is_err()
        {
            break;
        }
    }
    html.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_rust_and_toml() {
        let rust = highlight("fn main() { let x = \"<hi>\"; }\n", "rust");
        assert!(
            rust.contains("s-keyword") || rust.contains("s-storage"),
            "{rust}"
        );
        assert!(rust.contains("&lt;hi&gt;"));
        let toml = highlight("[dependencies]\nactive = \"0.1\"\n", "toml");
        assert!(toml.contains("s-string"), "{toml}");
        assert_eq!(label("sh"), "terminal");
    }
}
