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

/// The special bytes of a block of 16, as a bit mask: bit `i` is set when `block[i]`
/// must be escaped. It compiles to a few vector comparisons and one `movemask`, so a
/// block is tested only once, however many special bytes it holds.
#[inline(always)]
fn special_mask(block: &[u8; 16]) -> u16 {
    let mut mask = 0;
    for (i, &byte) in block.iter().enumerate() {
        mask |= u16::from(is_special(byte)) << i;
    }
    mask
}

/// Escapes `&`, `<`, `>`, `"` and `'` in HTML text and attribute values.
///
/// Whole blocks of 16 bytes are tested at once and their special bytes taken from the
/// mask, one after the other (none is looked for again byte by byte, however dense they
/// are), then the last bytes are looked up one by one.
pub(crate) fn escape<W: Write>(w: &mut W, s: &str) -> fmt::Result {
    let bytes = s.as_bytes();
    let (blocks, rest) = bytes.as_chunks::<16>();
    let mut start = 0;
    for (n, block) in blocks.iter().enumerate() {
        let mut mask = special_mask(block);
        while mask != 0 {
            let i = n * 16 + mask.trailing_zeros() as usize;
            w.write_str(&s[start..i])?;
            w.write_str(entity(bytes[i]))?;
            start = i + 1;
            mask &= mask - 1;
        }
    }
    let offset = blocks.len() * 16;
    for (j, &byte) in rest.iter().enumerate() {
        if SPECIAL[byte as usize] {
            let i = offset + j;
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
    let (blocks, rest) = bytes.as_chunks::<16>();
    let mut start = 0;
    for (n, block) in blocks.iter().enumerate() {
        let mut mask = special_mask(block);
        if mask != 0 && start == 0 {
            out.reserve(s.len() + 8);
        }
        while mask != 0 {
            let i = n * 16 + mask.trailing_zeros() as usize;
            out.push_str(&s[start..i]);
            out.push_str(entity(bytes[i]));
            start = i + 1;
            mask &= mask - 1;
        }
    }
    let offset = blocks.len() * 16;
    for (j, &byte) in rest.iter().enumerate() {
        if SPECIAL[byte as usize] {
            let i = offset + j;
            out.push_str(&s[start..i]);
            out.push_str(entity(byte));
            start = i + 1;
        }
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
    /// characters fall: inside a block, on its edges, in the last bytes, several in a row,
    /// one in every few bytes or all of them.
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
            // Dense runs: a special character every `every` bytes, from every offset,
            // up to all of them (several in one block, in the last bytes, across blocks).
            for every in 1..=9 {
                for offset in 0..every {
                    let s: String = (0..len)
                        .map(|i| {
                            if i % every == offset {
                                ['&', '"', '<', '>', '\''][i % 5]
                            } else {
                                ['a', 'Z', '0', ' '][i % 4]
                            }
                        })
                        .collect();
                    cases.push(s);
                }
            }
        }
        // Two special characters at every pair of positions, over three blocks and a few
        // last bytes, between multi-byte characters.
        let base: String = "aé€b".chars().cycle().take(40).collect();
        let positions: Vec<usize> = base
            .char_indices()
            .map(|(at, _)| at)
            .chain([base.len()])
            .collect();
        for (n, &first) in positions.iter().enumerate() {
            for &second in &positions[n..] {
                let mut s = base.clone();
                s.insert(second, '<');
                s.insert(first, '&');
                cases.push(s);
            }
        }
        // Pseudo-random strings of every density.
        let mut seed = 0x2545_f491_u32;
        for _ in 0..2000 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let len = (seed % 100) as usize;
            let density = seed >> 28;
            let s: String = (0..len)
                .map(|_| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    if seed % 16 < density {
                        ['&', '"', '<', '>', '\''][(seed >> 8) as usize % 5]
                    } else {
                        ['x', 'é', '€', '7', ' '][(seed >> 8) as usize % 5]
                    }
                })
                .collect();
            cases.push(s);
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
