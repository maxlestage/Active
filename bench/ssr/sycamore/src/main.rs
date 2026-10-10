//! Sycamore 0.9: the whole document as one component, rendered with `render_to_string`.
//! Sycamore has no head management, so the tags are plain elements (`"property"` is quoted:
//! it is not an HTML attribute). Its server rendering always adds hydration keys
//! (`data-hk`).

use bench_common::{Product, SITE, TWITTER, bench_ssr, option_label, script_safe};
use sycamore::prelude::*;

#[component(inline_props)]
fn ProductPage(i: u32) -> View {
    // Values are prepared first: an expression with a call in an attribute would become a
    // reactive closure.
    let p = Product::new(i);
    let options = p
        .options()
        .map(|(k, price)| view! { li { (option_label(k, price)) } })
        .collect::<Vec<_>>();
    let Product {
        name,
        description,
        url,
        image,
        json_ld,
        ..
    } = p;
    let title = format!("{name} · {SITE}");
    let json_ld = script_safe(&json_ld);
    let (og_title, og_description, og_url) = (name.clone(), description.clone(), url.clone());
    view! {
        html(lang="en") {
            head {
                meta(charset="utf-8")
                meta(name="viewport", content="width=device-width, initial-scale=1")
                title { (title) }
                meta(name="description", content=description)
                link(rel="canonical", href=url)
                meta("property"="og:title", content=og_title)
                meta("property"="og:description", content=og_description)
                meta("property"="og:type", content="website")
                meta("property"="og:url", content=og_url)
                meta("property"="og:image", content=image)
                meta("property"="og:site_name", content=SITE)
                meta(name="twitter:card", content="summary_large_image")
                meta(name="twitter:site", content=TWITTER)
                script(r#type="application/ld+json", dangerously_set_inner_html=json_ld)
            }
            body {
                h1 { (name) }
                ul { (options) }
            }
        }
    }
}

fn main() {
    bench_ssr(|i| {
        let body = sycamore::render_to_string(|| view! { ProductPage(i=i) });
        let mut html = String::with_capacity(body.len() + 15);
        html.push_str("<!DOCTYPE html>");
        html.push_str(&body);
        html
    });
}
