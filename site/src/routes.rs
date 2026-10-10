//! URLs of the site: `/{lang}/`, `/{lang}/docs/`, `/{lang}/docs/{slug}/`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Fr, Lang::Es];

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    pub fn parse(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Docs,
    Doc(String),
    NotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub lang: Lang,
    pub page: Page,
}

impl Route {
    /// `slugs` lists the documentation pages that exist.
    pub fn parse(path: &str, slugs: &[&str]) -> Option<Route> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let lang = Lang::parse(parts.first()?)?;
        let page = match parts[1..] {
            [] => Page::Home,
            ["docs"] => Page::Docs,
            ["docs", slug] if slugs.contains(&slug) => Page::Doc(slug.to_string()),
            _ => Page::NotFound,
        };
        Some(Route { lang, page })
    }

    pub fn href(&self) -> String {
        let l = self.lang.code();
        match &self.page {
            Page::Home => format!("/{l}/"),
            Page::Docs => format!("/{l}/docs/"),
            Page::Doc(slug) => format!("/{l}/docs/{slug}/"),
            Page::NotFound => format!("/{l}/404/"),
        }
    }

    /// The same page in another language.
    pub fn with_lang(&self, lang: Lang) -> Route {
        Route {
            lang,
            page: self.page.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_builds_urls() {
        let slugs = ["seo", "api"];
        let doc = Route::parse("/fr/docs/seo/", &slugs).unwrap();
        assert_eq!(
            doc,
            Route {
                lang: Lang::Fr,
                page: Page::Doc("seo".into())
            }
        );
        assert_eq!(doc.href(), "/fr/docs/seo/");
        assert_eq!(doc.with_lang(Lang::Es).href(), "/es/docs/seo/");
        assert_eq!(Route::parse("/en/", &slugs).unwrap().page, Page::Home);
        assert_eq!(Route::parse("/en/docs", &slugs).unwrap().page, Page::Docs);
        assert_eq!(
            Route::parse("/en/docs/nope/", &slugs).unwrap().page,
            Page::NotFound
        );
        assert_eq!(Route::parse("/de/", &slugs), None);
    }
}
