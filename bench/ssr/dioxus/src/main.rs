//! Dioxus 0.7 with dioxus-ssr: `head` and `body` rendered by one component (`meta`, `title`…
//! are ordinary rsx elements; dioxus' `document::Title`/`Meta` components need the fullstack
//! integration). dioxus-html has no `html` element, so `<html lang="en">` is written around
//! the output, like dioxus' server does with its HTML template. One `VirtualDom` per page,
//! one `Renderer` reused across pages (its template cache is what dioxus' server keeps in
//! its renderer pool).

use bench_common::{Product, SITE, TWITTER, bench_ssr, script_safe};
use dioxus::prelude::*;

#[component]
fn ProductPage(i: u32) -> Element {
    let p = Product::new(i);
    let json_ld = script_safe(&p.json_ld);
    rsx! {
        head {
            meta { charset: "utf-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1" }
            title { "{p.name} · {SITE}" }
            meta { name: "description", content: "{p.description}" }
            link { rel: "canonical", href: "{p.url}" }
            meta { property: "og:title", content: "{p.name}" }
            meta { property: "og:description", content: "{p.description}" }
            meta { property: "og:type", content: "website" }
            meta { property: "og:url", content: "{p.url}" }
            meta { property: "og:image", content: "{p.image}" }
            meta { property: "og:site_name", content: SITE }
            meta { name: "twitter:card", content: "summary_large_image" }
            meta { name: "twitter:site", content: TWITTER }
            script { r#type: "application/ld+json", dangerous_inner_html: json_ld }
        }
        body {
            h1 { "{p.name}" }
            ul {
                for (k, price) in p.options() {
                    li { "Option {k} — {price} €" }
                }
            }
        }
    }
}

fn main() {
    let mut renderer = dioxus_ssr::Renderer::new();
    bench_ssr(|i| {
        let mut dom = VirtualDom::new_with_props(ProductPage, ProductPageProps { i });
        dom.rebuild_in_place();
        let mut html = String::with_capacity(4096);
        html.push_str(r#"<!DOCTYPE html><html lang="en">"#);
        renderer
            .render_to(&mut html, &dom)
            .expect("writing to a String");
        html.push_str("</html>");
        html
    });
}
