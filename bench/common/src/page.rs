//! The SSR page: the same product page for every framework.
//!
//! `<head>`: `<title>` with a template, description, canonical, 6 Open Graph tags, 2 Twitter
//! tags and a JSON-LD `Product`. `<body>`: an `<h1>` and a `<ul>` of [`OPTIONS`] items.

/// Site name (`og:site_name`, title suffix).
pub const SITE: &str = "Acme";
/// Title template, for the frameworks that have one (`%s` is the page title).
pub const TITLE_TEMPLATE: &str = "%s · Acme";
/// `twitter:site`.
pub const TWITTER: &str = "@acme";
/// Number of `<li>` in the body.
pub const OPTIONS: u32 = 30;

/// The data of product page `i`. Building it is part of the measured work, identical for
/// every framework.
#[derive(Clone, Debug)]
pub struct Product {
    /// Page number, varies on every iteration.
    pub i: u32,
    /// `Produit {i}`: the `<h1>` and `og:title` (the `<title>` adds the site name).
    pub name: String,
    /// Meta description and `og:description`.
    pub description: String,
    /// Canonical URL and `og:url`.
    pub url: String,
    /// `og:image` (so `twitter:card` is `summary_large_image`).
    pub image: String,
    /// Serialized JSON-LD object.
    pub json_ld: String,
}

impl Product {
    /// The data of page `i`.
    pub fn new(i: u32) -> Self {
        Self {
            i,
            name: format!("Produit {i}"),
            description: format!("Description du produit {i}, livré en 24h partout en France."),
            url: format!("https://acme.com/p/{i}"),
            image: format!("https://acme.com/img/{i}.png"),
            json_ld: format!(
                r#"{{"@context":"https://schema.org","@type":"Product","name":"Produit {i}","offers":{{"@type":"Offer","price":{i},"priceCurrency":"EUR"}}}}"#
            ),
        }
    }

    /// The full `<title>`: `Produit {i} · Acme`.
    pub fn title(&self) -> String {
        format!("{} · {SITE}", self.name)
    }

    /// `(k, price)` for each list item.
    pub fn options(&self) -> impl Iterator<Item = (u32, u32)> + use<> {
        let i = self.i;
        (0..OPTIONS).map(move |k| (k, i * k))
    }
}

/// JSON made safe to put as is inside `<script>`: every `<` becomes `\u003c` (still the same
/// JSON, but it can never close the element). `active` does this itself; the other
/// implementations call this function.
pub fn script_safe(json: &str) -> String {
    json.replace('<', "\\u003c")
}

/// The text of one list item: `Option {k} — {price} €`.
pub fn option_label(k: u32, price: u32) -> String {
    format!("Option {k} — {price} €")
}
