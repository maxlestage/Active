//! HTML escaping shared by every renderer.

use std::fmt::{self, Write};

/// The bytes that must be escaped in HTML text and attribute values.
const SPECIAL: [bool; 256] = {
    let mut table = [false; 256];
    table[b'&' as usize] = true;
    table[b'<' as usize] = true;
    table[b'>' as usize] = true;
    table[b'"' as usize] = true;
    table[b'\'' as usize] = true;
    table
};

fn entity(byte: u8) -> &'static str {
    match byte {
        b'&' => "&amp;",
        b'<' => "&lt;",
        b'>' => "&gt;",
        b'"' => "&quot;",
        _ => "&#x27;",
    }
}

/// Escapes `&`, `<`, `>`, `"` and `'` in HTML text and attribute values.
pub(crate) fn escape<W: Write>(w: &mut W, s: &str) -> fmt::Result {
    let mut start = 0;
    for (i, &byte) in s.as_bytes().iter().enumerate() {
        if SPECIAL[byte as usize] {
            w.write_str(&s[start..i])?;
            w.write_str(entity(byte))?;
            start = i + 1;
        }
    }
    w.write_str(&s[start..])
}

/// [`escape`] into a `String`: the common case, text without special characters, is one
/// scan and one copy.
pub(crate) fn escape_into(out: &mut String, s: &str) {
    let bytes = s.as_bytes();
    match bytes.iter().position(|&b| SPECIAL[b as usize]) {
        None => out.push_str(s),
        Some(first) => {
            out.reserve(s.len() + 8);
            out.push_str(&s[..first]);
            let mut start = first;
            for (i, &byte) in bytes.iter().enumerate().skip(first) {
                if SPECIAL[byte as usize] {
                    out.push_str(&s[start..i]);
                    out.push_str(entity(byte));
                    start = i + 1;
                }
            }
            out.push_str(&s[start..]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_five_special_characters_only() {
        let mut out = String::new();
        escape_into(&mut out, r#"Tom & "Jerry" <b>'s</b> — €"#);
        assert_eq!(
            out,
            "Tom &amp; &quot;Jerry&quot; &lt;b&gt;&#x27;s&lt;/b&gt; — €"
        );
        let mut plain = String::new();
        escape_into(&mut plain, "Option 3 — 12 €");
        assert_eq!(plain, "Option 3 — 12 €");
        let mut fmt = String::new();
        escape(&mut fmt, "a<b").unwrap();
        assert_eq!(fmt, "a&lt;b");
    }
}
