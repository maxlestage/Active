//! active: the page is a component tree; its `Seo` is merged into `<head>` by `Document`
//! with the site-wide `Seo` (template, site name, Twitter handle).

use active::prelude::*;
use bench_common::{Product, SITE, TITLE_TEMPLATE, TWITTER, bench_ssr, option_label};

/// The product page component: SEO data and content side by side.
fn product_page(p: Product) -> Node {
    let options = p
        .options()
        .map(|(k, price)| li().text(option_label(k, price)));
    let seo = Seo::new()
        .title(p.name.clone())
        .description(p.description)
        .canonical(p.url)
        .image(p.image)
        .json_ld(p.json_ld);
    fragment([
        Node::from(seo),
        h1().text(p.name).into(),
        ul().children(options).into(),
    ])
}

fn main() {
    // Site-wide values, built once like in a real server and cloned per page.
    let site = Seo::new()
        .site_name(SITE)
        .title_template(TITLE_TEMPLATE)
        .twitter(TWITTER);
    bench_ssr(|i| {
        Document::new(product_page(Product::new(i)))
            .lang("en")
            .seo(site.clone())
            .render()
    });
}
