//! Docs search, shared by both sides: the server builds one index per language (a small
//! text file, so the site also works as static files), the browser downloads it once and
//! filters it on every key press.

/// One searchable target: a page, or a heading inside a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Whole pages rank above their headings.
    pub page: bool,
    pub href: String,
    pub title: String,
    /// Category of a page, page title of a heading.
    pub context: String,
    /// Everything a query can match, normalized with [`norm`].
    pub words: String,
    /// The title normalized: pages whose title contains the first word come first.
    pub title_words: String,
}

/// At most this many results.
pub const MAX: usize = 12;

impl Entry {
    /// One line of the index: `p|h \t href \t title \t context \t words \t title words`.
    pub fn line(&self) -> String {
        let clean = |s: &str| s.replace(['\t', '\n'], " ");
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            if self.page { "p" } else { "h" },
            clean(&self.href),
            clean(&self.title),
            clean(&self.context),
            clean(&self.words),
            clean(&self.title_words),
        )
    }

    pub fn parse(line: &str) -> Option<Entry> {
        let mut f = line.split('\t');
        Some(Entry {
            page: f.next()? == "p",
            href: f.next()?.to_string(),
            title: f.next()?.to_string(),
            context: f.next()?.to_string(),
            words: f.next()?.to_string(),
            title_words: f.next()?.to_string(),
        })
    }
}

/// Every entry matching all the words of `query` (accents and case ignored), best first:
/// pages whose title starts the match, other pages, then headings. An empty query lists
/// the pages in reading order.
pub fn search<'a>(entries: &'a [Entry], query: &str) -> Vec<&'a Entry> {
    let query = norm(query);
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return entries.iter().filter(|e| e.page).take(MAX).collect();
    }
    let mut hits: Vec<(u8, &Entry)> = entries
        .iter()
        .filter(|e| words.iter().all(|w| e.words.contains(w)))
        .map(|e| {
            let score = match (e.page, e.title_words.contains(words[0])) {
                (true, true) => 3,
                (true, false) => 2,
                (false, _) => 1,
            };
            (score, e)
        })
        .collect();
    // Stable: equal scores keep the reading order.
    hits.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    hits.into_iter().take(MAX).map(|(_, e)| e).collect()
}

/// Lowercase, accents removed, everything else kept (spaces separate words).
pub fn norm(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        match deaccent(c) {
            Some(ascii) => out.push_str(ascii),
            None => out.push(c),
        }
    }
    out
}

/// The ASCII spelling of an accented Latin letter, `None` for any other character.
pub fn deaccent(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' => "a",
        'ç' | 'ć' | 'č' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ę' | 'ě' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'ī' => "i",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' => "u",
        'ý' | 'ÿ' => "y",
        'š' | 'ś' => "s",
        'ž' | 'ź' | 'ż' => "z",
        'ř' => "r",
        'ł' => "l",
        'œ' => "oe",
        'æ' => "ae",
        'ß' => "ss",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(page: bool, href: &str, title: &str, words: &str) -> Entry {
        Entry {
            page,
            href: href.into(),
            title: title.into(),
            context: "ctx".into(),
            words: norm(words),
            title_words: norm(title),
        }
    }

    #[test]
    fn ranks_titles_then_pages_then_headings() {
        let index = [
            entry(true, "/a/", "Islands", "islands hydrate the page"),
            entry(false, "/a/#routing", "Routing", "routing islands"),
            entry(
                true,
                "/b/",
                "Client-side apps",
                "apps with a router and islands",
            ),
        ];
        let hrefs =
            |q: &str| -> Vec<&str> { search(&index, q).iter().map(|e| e.href.as_str()).collect() };
        assert_eq!(hrefs("ISLANDS"), ["/a/", "/b/", "/a/#routing"]);
        assert_eq!(hrefs("router island"), ["/b/"]);
        assert_eq!(hrefs(""), ["/a/", "/b/"]);
        assert!(hrefs("nothing").is_empty());
    }

    #[test]
    fn ignores_accents_and_round_trips_lines() {
        assert_eq!(norm("Déployer Œuvre ÇA"), "deployer oeuvre ca");
        let e = entry(true, "/fr/docs/x/", "Déployer\tsur Heroku", "déployer");
        let back = Entry::parse(e.line().trim_end()).unwrap();
        assert_eq!(back.title, "Déployer sur Heroku");
        assert_eq!(back.words, "deployer");
    }
}
