//! [`Data`]: values that travel from the server to the browser (and to `localStorage`) as
//! JSON, without a serialization dependency.

use std::collections::{BTreeMap, HashMap};

/// A value that can be written as JSON and read back: what [`ssr::resource`] sends from the
/// server to the browser (so that hydration does not load it again) and what
/// [`persisted`](crate::persisted) keeps in `localStorage`.
///
/// Implemented for strings, numbers, `bool`, `char`, `()`, `Option`, `Vec`, `Result`
/// (`{"Ok": …}` / `{"Err": …}`), tuples of up to four values and maps with string keys —
/// the same JSON as `serde_json` writes for them. For your own types, write the two methods
/// by hand or with `serde_json`:
///
/// ```ignore
/// impl active::Data for User {
///     fn write_data(&self, out: &mut String) {
///         out.push_str(&serde_json::to_string(self).unwrap_or_default());
///     }
///     fn read_data(json: &str) -> Option<Self> {
///         serde_json::from_str(json).ok()
///     }
/// }
/// ```
///
/// ```
/// use active::Data;
///
/// let value: Vec<(String, Option<u32>)> = vec![("a\"b".into(), Some(1)), ("c".into(), None)];
/// let json = value.to_data();
/// assert_eq!(json, r#"[["a\"b",1],["c",null]]"#);
/// assert_eq!(Vec::<(String, Option<u32>)>::read_data(&json), Some(value));
/// ```
///
/// [`ssr::resource`]: crate::ssr::resource
pub trait Data: Sized {
    /// Appends the JSON of the value to `out`.
    fn write_data(&self, out: &mut String);

    /// Reads a value back from its JSON (`None` when it does not fit).
    fn read_data(json: &str) -> Option<Self>;

    /// The JSON of the value.
    fn to_data(&self) -> String {
        let mut out = String::new();
        self.write_data(&mut out);
        out
    }
}

/// Appends `s` as a JSON string.
pub(crate) fn json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Reads a JSON string (the whole of `json`, spaces around allowed).
fn read_string(json: &str) -> Option<String> {
    let s = json.trim();
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let hex = |chars: &mut std::str::Chars<'_>| {
                    let code: String = chars.by_ref().take(4).collect();
                    u32::from_str_radix(&code, 16).ok()
                };
                let high = hex(&mut chars)?;
                let code = if (0xD800..0xDC00).contains(&high) {
                    // A surrogate pair: `😀`.
                    if chars.next()? != '\\' || chars.next()? != 'u' {
                        return None;
                    }
                    let low = hex(&mut chars)?;
                    0x10000 + ((high - 0xD800) << 10) + (low.checked_sub(0xDC00)?)
                } else {
                    high
                };
                out.push(char::from_u32(code)?);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// The end (exclusive) of the JSON value starting at `start` (spaces skipped first).
fn value_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = skip_spaces(bytes, start);
    match *bytes.get(i)? {
        b'"' => {
            i += 1;
            loop {
                match *bytes.get(i)? {
                    b'\\' => i += 2,
                    b'"' => return Some(i + 1),
                    _ => i += 1,
                }
            }
        }
        b'[' | b'{' => {
            let mut depth = 0usize;
            loop {
                match *bytes.get(i)? {
                    b'"' => {
                        i = value_end(bytes, i)?;
                        continue;
                    }
                    b'[' | b'{' => depth += 1,
                    b']' | b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
        _ => {
            while i < bytes.len()
                && !matches!(bytes[i], b',' | b']' | b'}' | b':')
                && !bytes[i].is_ascii_whitespace()
            {
                i += 1;
            }
            Some(i)
        }
    }
}

fn skip_spaces(bytes: &[u8], mut i: usize) -> usize {
    while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

/// The elements of a JSON array, as slices of `json`.
fn read_array(json: &str) -> Option<Vec<&str>> {
    let s = json.trim();
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'[') || bytes.last() != Some(&b']') {
        return None;
    }
    let mut items = Vec::new();
    let mut i = skip_spaces(bytes, 1);
    if bytes.get(i) == Some(&b']') {
        return (i == bytes.len() - 1).then_some(items);
    }
    loop {
        let end = value_end(bytes, i)?;
        items.push(s.get(i..end)?);
        i = skip_spaces(bytes, end);
        match bytes.get(i)? {
            b',' => i += 1,
            b']' if i == bytes.len() - 1 => return Some(items),
            _ => return None,
        }
    }
}

/// The members of a JSON object: keys decoded, values as slices of `json`.
fn read_object(json: &str) -> Option<Vec<(String, &str)>> {
    let s = json.trim();
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'{') || bytes.last() != Some(&b'}') {
        return None;
    }
    let mut members = Vec::new();
    let mut i = skip_spaces(bytes, 1);
    if bytes.get(i) == Some(&b'}') {
        return (i == bytes.len() - 1).then_some(members);
    }
    loop {
        let key_end = value_end(bytes, i)?;
        let key = read_string(s.get(i..key_end)?)?;
        i = skip_spaces(bytes, key_end);
        if bytes.get(i) != Some(&b':') {
            return None;
        }
        let start = skip_spaces(bytes, i + 1);
        let end = value_end(bytes, start)?;
        members.push((key, s.get(start..end)?));
        i = skip_spaces(bytes, end);
        match bytes.get(i)? {
            b',' => i = skip_spaces(bytes, i + 1),
            b'}' if i == bytes.len() - 1 => return Some(members),
            _ => return None,
        }
    }
}

impl Data for String {
    fn write_data(&self, out: &mut String) {
        json_string(out, self);
    }
    fn read_data(json: &str) -> Option<Self> {
        read_string(json)
    }
}

impl Data for char {
    fn write_data(&self, out: &mut String) {
        json_string(out, self.encode_utf8(&mut [0; 4]));
    }
    fn read_data(json: &str) -> Option<Self> {
        let s = read_string(json)?;
        let mut chars = s.chars();
        let c = chars.next()?;
        chars.next().is_none().then_some(c)
    }
}

impl Data for bool {
    fn write_data(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }
    fn read_data(json: &str) -> Option<Self> {
        match json.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }
}

impl Data for () {
    fn write_data(&self, out: &mut String) {
        out.push_str("null");
    }
    fn read_data(json: &str) -> Option<Self> {
        (json.trim() == "null").then_some(())
    }
}

macro_rules! integers {
    ($($t:ty),*) => {$(
        impl Data for $t {
            fn write_data(&self, out: &mut String) {
                out.push_str(&self.to_string());
            }
            fn read_data(json: &str) -> Option<Self> {
                json.trim().parse().ok()
            }
        }
    )*};
}
integers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

macro_rules! floats {
    ($($t:ty),*) => {$(
        impl Data for $t {
            /// Not finite (`NaN`, infinities): `null`, as in JSON.
            fn write_data(&self, out: &mut String) {
                if self.is_finite() {
                    out.push_str(&self.to_string());
                } else {
                    out.push_str("null");
                }
            }
            fn read_data(json: &str) -> Option<Self> {
                match json.trim() {
                    "null" => Some(<$t>::NAN),
                    n => n.parse().ok(),
                }
            }
        }
    )*};
}
floats!(f32, f64);

/// `None` is `null`, `Some(v)` is `v` (as `serde_json` does).
impl<T: Data> Data for Option<T> {
    fn write_data(&self, out: &mut String) {
        match self {
            Some(v) => v.write_data(out),
            None => out.push_str("null"),
        }
    }
    fn read_data(json: &str) -> Option<Self> {
        if json.trim() == "null" {
            Some(None)
        } else {
            T::read_data(json).map(Some)
        }
    }
}

impl<T: Data> Data for Vec<T> {
    fn write_data(&self, out: &mut String) {
        out.push('[');
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            v.write_data(out);
        }
        out.push(']');
    }
    fn read_data(json: &str) -> Option<Self> {
        read_array(json)?.into_iter().map(T::read_data).collect()
    }
}

impl<T: Data> Data for Box<T> {
    fn write_data(&self, out: &mut String) {
        (**self).write_data(out);
    }
    fn read_data(json: &str) -> Option<Self> {
        T::read_data(json).map(Box::new)
    }
}

/// `{"Ok": value}` or `{"Err": error}`, as `serde_json` writes a `Result`.
impl<T: Data, E: Data> Data for Result<T, E> {
    fn write_data(&self, out: &mut String) {
        match self {
            Ok(v) => {
                out.push_str("{\"Ok\":");
                v.write_data(out);
            }
            Err(e) => {
                out.push_str("{\"Err\":");
                e.write_data(out);
            }
        }
        out.push('}');
    }
    fn read_data(json: &str) -> Option<Self> {
        let members = read_object(json)?;
        match members.as_slice() {
            [(key, value)] if key == "Ok" => T::read_data(value).map(Ok),
            [(key, value)] if key == "Err" => E::read_data(value).map(Err),
            _ => None,
        }
    }
}

macro_rules! tuples {
    ($(($($name:ident $i:tt),+))*) => {$(
        impl<$($name: Data),+> Data for ($($name,)+) {
            fn write_data(&self, out: &mut String) {
                out.push('[');
                $(
                    if $i > 0 {
                        out.push(',');
                    }
                    self.$i.write_data(out);
                )+
                out.push(']');
            }
            fn read_data(json: &str) -> Option<Self> {
                let items = read_array(json)?;
                let mut items = items.into_iter();
                let value = ($($name::read_data(items.next()?)?,)+);
                items.next().is_none().then_some(value)
            }
        }
    )*};
}
tuples! {
    (A 0)
    (A 0, B 1)
    (A 0, B 1, C 2)
    (A 0, B 1, C 2, D 3)
}

fn write_map<'a, V: Data + 'a>(
    out: &mut String,
    entries: impl Iterator<Item = (&'a String, &'a V)>,
) {
    out.push('{');
    for (i, (key, value)) in entries.enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_string(out, key);
        out.push(':');
        value.write_data(out);
    }
    out.push('}');
}

/// A JSON object.
impl<V: Data> Data for BTreeMap<String, V> {
    fn write_data(&self, out: &mut String) {
        write_map(out, self.iter());
    }
    fn read_data(json: &str) -> Option<Self> {
        read_object(json)?
            .into_iter()
            .map(|(k, v)| Some((k, V::read_data(v)?)))
            .collect()
    }
}

/// A JSON object (keys sorted, so that the same map always gives the same JSON).
impl<V: Data, S: std::hash::BuildHasher + Default> Data for HashMap<String, V, S> {
    fn write_data(&self, out: &mut String) {
        let mut entries: Vec<_> = self.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        write_map(out, entries.into_iter());
    }
    fn read_data(json: &str) -> Option<Self> {
        read_object(json)?
            .into_iter()
            .map(|(k, v)| Some((k, V::read_data(v)?)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<T: Data + PartialEq + std::fmt::Debug>(value: T, json: &str) {
        assert_eq!(value.to_data(), json);
        assert_eq!(T::read_data(json), Some(value));
    }

    #[test]
    fn values_round_trip_as_serde_json_writes_them() {
        round_trip(
            "tab\t\"quote\" \\ é 😀".to_string(),
            r#""tab\t\"quote\" \\ é 😀""#,
        );
        round_trip(-42i64, "-42");
        round_trip(2.5f64, "2.5");
        round_trip(true, "true");
        round_trip('x', r#""x""#);
        round_trip((), "null");
        round_trip(Some(3u8), "3");
        round_trip(None::<u8>, "null");
        round_trip(vec![vec![1, 2], vec![]], "[[1,2],[]]");
        round_trip(Ok::<u32, String>(1), r#"{"Ok":1}"#);
        round_trip(Err::<u32, String>("no".into()), r#"{"Err":"no"}"#);
        round_trip((1u8, "a".to_string(), false), r#"[1,"a",false]"#);
        let map: BTreeMap<String, Vec<u8>> =
            [("b".to_string(), vec![2]), ("a".to_string(), vec![])].into();
        round_trip(map, r#"{"a":[],"b":[2]}"#);
        let hash: HashMap<String, u8> = [("z".to_string(), 1), ("y".to_string(), 2)].into();
        round_trip(hash, r#"{"y":2,"z":1}"#);
    }

    #[test]
    fn reading_accepts_spaces_and_escapes_and_rejects_what_does_not_fit() {
        assert_eq!(
            Vec::<String>::read_data(r#" [ "a" , "\u00e9\ud83d\ude00\/" ] "#),
            Some(vec!["a".into(), "é😀/".into()])
        );
        assert_eq!(
            Result::<Vec<u8>, String>::read_data(r#" { "Ok" : [ 1 , 2 ] } "#),
            Some(Ok(vec![1, 2]))
        );
        assert!(f64::read_data("null").unwrap().is_nan());
        assert_eq!(f64::NAN.to_data(), "null");
        assert_eq!(Vec::<u8>::read_data("[1,]"), None);
        assert_eq!(Vec::<u8>::read_data("[1 2]"), None);
        assert_eq!(Vec::<u8>::read_data("[300]"), None);
        assert_eq!(<(u8, u8)>::read_data("[1,2,3]"), None);
        assert_eq!(String::read_data(r#""\x""#), None);
        assert_eq!(char::read_data(r#""ab""#), None);
        assert_eq!(
            BTreeMap::<String, String>::read_data(r#"{"a":"}","b":"]"}"#),
            Some([("a".into(), "}".into()), ("b".into(), "]".into())].into())
        );
    }
}
