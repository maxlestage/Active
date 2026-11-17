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

/// Whether `byte` must be escaped, without a table lookup, so that a fixed block of
/// bytes compiles to a few vector comparisons.
#[inline(always)]
fn is_special(byte: u8) -> bool {
    (byte == b'&') | (byte == b'<') | (byte == b'>') | (byte == b'"') | (byte == b'\'')
}

/// The index of the first byte of `bytes` that must be escaped. Whole blocks of 16 bytes
/// are tested at once (the usual case: none of them is special), then the byte is found
/// inside the block, or in the last bytes.
#[inline]
fn find_special(bytes: &[u8]) -> Option<usize> {
    let (blocks, rest) = bytes.as_chunks::<16>();
    let mut offset = 0;
    for block in blocks {
        if block.iter().fold(false, |any, &b| any | is_special(b)) {
            return block
                .iter()
                .position(|&b| SPECIAL[b as usize])
                .map(|i| offset + i);
        }
        offset += block.len();
    }
    rest.iter()
        .position(|&b| SPECIAL[b as usize])
        .map(|i| offset + i)
}

/// Escapes `&`, `<`, `>`, `"` and `'` in HTML text and attribute values.
pub(crate) fn escape<W: Write>(w: &mut W, s: &str) -> fmt::Result {
    let bytes = s.as_bytes();
    let mut start = 0;
    while let Some(i) = find_special(&bytes[start..]) {
        let i = start + i;
        w.write_str(&s[start..i])?;
        w.write_str(entity(bytes[i]))?;
        start = i + 1;
    }
    w.write_str(&s[start..])
}

/// [`escape`] into a `String`: the common case, text without special characters, is one
/// scan and one copy.
pub(crate) fn escape_into(out: &mut String, s: &str) {
    let bytes = s.as_bytes();
    let Some(first) = find_special(bytes) else {
        out.push_str(s);
        return;
    };
    out.reserve(s.len() + 8);
    let mut start = 0;
    let mut next = Some(first);
    while let Some(i) = next {
        let i = start + i;
        out.push_str(&s[start..i]);
        out.push_str(entity(bytes[i]));
        start = i + 1;
        next = find_special(&bytes[start..]);
    }
    out.push_str(&s[start..]);
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

    /// The block scan gives the same result as escaping byte by byte, wherever the special
    /// characters fall: inside a block, on its edges, in the last bytes, several in a row.
    #[test]
    fn block_scan_matches_byte_by_byte() {
        fn reference(s: &str) -> String {
            let mut out = String::new();
            for c in s.chars() {
                match c {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    '"' => out.push_str("&quot;"),
                    '\'' => out.push_str("&#x27;"),
                    c => out.push(c),
                }
            }
            out
        }
        let mut cases = Vec::new();
        for len in 0..=50 {
            let base: String = "abcdé€0123456789xyz".chars().cycle().take(len).collect();
            cases.push(base.clone());
            for (at, _) in base.char_indices() {
                for special in ['&', '<', '>', '"', '\''] {
                    let mut s = base.clone();
                    s.insert(at, special);
                    cases.push(s.clone());
                    s.push(special);
                    cases.push(s);
                }
            }
            cases.push("<&>\"'".repeat(len));
        }
        for s in &cases {
            let mut out = String::new();
            escape_into(&mut out, s);
            assert_eq!(out, reference(s), "{s:?}");
            let mut fmt = String::new();
            escape(&mut fmt, s).unwrap();
            assert_eq!(fmt, reference(s), "{s:?}");
        }
    }
}
