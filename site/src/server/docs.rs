//! The documentation: categories, pages in three languages and search.

use std::collections::HashMap;
use std::path::Path;

use super::markdown::{Doc, render};
use crate::i18n::t;
use crate::routes::{Lang, link};
use crate::search::{Entry, norm};

/// Categories (in reading order) and their pages. Titles come from the i18n dictionary
/// (`docs.categories`, same order) and from each page's frontmatter.
pub const CATEGORIES: [&[&str]; 7] = [
    &["introduction", "installation", "quick-start"],
    &["components", "state", "events"],
    &["seo", "documents", "structured-data"],
    &["islands", "client-app", "client-build", "performance"],
    &["axum", "static-site", "pwa"],
    &["deploy-heroku", "deploy-pages", "publishing"],
    &["api", "benchmarks", "faq"],
];

pub struct Docs {
    pages: HashMap<(Lang, &'static str), Doc>,
    slugs: Vec<&'static str>,
}

impl Docs {
    /// Reads `dir/{lang}/{slug}.md`. A page missing in a language falls back to English;
    /// a page missing in English is left out of the site.
    pub fn load(dir: &Path) -> Docs {
        let mut pages = HashMap::new();
        for lang in Lang::ALL {
            for slug in CATEGORIES.iter().flat_map(|c| c.iter()) {
                if let Ok(src) =
                    std::fs::read_to_string(dir.join(lang.code()).join(format!("{slug}.md")))
                {
                    pages.insert((lang, *slug), render(&src));
                }
            }
        }
        let slugs = CATEGORIES
            .iter()
            .flat_map(|c| c.iter().copied())
            .filter(|s| pages.contains_key(&(Lang::En, *s)))
            .collect();
        Docs { pages, slugs }
    }

    /// Pages that exist, in reading order.
    pub fn slugs(&self) -> &[&'static str] {
        &self.slugs
    }

    pub fn get(&self, lang: Lang, slug: &str) -> Option<&Doc> {
        let slug = self.slugs.iter().find(|s| **s == slug)?;
        self.pages
            .get(&(lang, *slug))
            .or_else(|| self.pages.get(&(Lang::En, *slug)))
    }

    pub fn title<'a>(&'a self, lang: Lang, slug: &'a str) -> &'a str {
        self.get(lang, slug).map_or(slug, |d| d.title.as_str())
    }

    /// Category index of a page.
    pub fn category(slug: &str) -> usize {
        CATEGORIES
            .iter()
            .position(|c| c.contains(&slug))
            .unwrap_or(0)
    }

    pub fn neighbours(&self, slug: &str) -> (Option<&'static str>, Option<&'static str>) {
        let i = self.slugs.iter().position(|s| *s == slug);
        let prev = i
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.slugs.get(i).copied());
        let next = i.and_then(|i| self.slugs.get(i + 1).copied());
        (prev, next)
    }

    /// The search index of `lang`: every page, then its headings, in reading order.
    pub fn search_index(&self, lang: Lang) -> Vec<Entry> {
        let categories = &t(lang).docs.categories;
        let mut entries = Vec::new();
        for slug in &self.slugs {
            let Some(doc) = self.get(lang, slug) else {
                continue;
            };
            let href = link(&format!("/{}/docs/{slug}/", lang.code()));
            let category = categories[Docs::category(slug)].title;
            entries.push(Entry {
                page: true,
                href: href.clone(),
                title: doc.title.clone(),
                context: category.to_string(),
                words: norm(&format!("{} {} {category}", doc.title, doc.description)),
                title_words: norm(&doc.title),
            });
            for h in &doc.toc {
                entries.push(Entry {
                    page: false,
                    href: format!("{href}#{}", h.id),
                    title: h.text.clone(),
                    context: doc.title.clone(),
                    words: norm(&format!("{} {}", h.text, doc.title)),
                    title_words: norm(&h.text),
                });
            }
        }
        entries
    }
}
