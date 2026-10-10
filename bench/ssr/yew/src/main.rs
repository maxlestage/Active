//! Yew 0.23: the whole document as one function component, rendered by
//! `LocalServerRenderer` (the single-thread form of `ServerRenderer`: `ServerRenderer` only
//! moves the same work to a worker thread) on a current-thread Tokio runtime. Yew has no
//! head management, so the tags are plain elements. Rendering is hydratable (Yew's
//! default), which adds a few comment markers.

use bench_common::{Product, SITE, TWITTER, bench_ssr, option_label, script_safe};
use yew::LocalServerRenderer;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
struct Props {
    i: u32,
}

#[function_component]
fn ProductPage(props: &Props) -> Html {
    let p = Product::new(props.i);
    let options = p
        .options()
        .map(|(k, price)| html! { <li>{ option_label(k, price) }</li> });
    // Text children are escaped, so JSON needs raw HTML. Hydratable rendering wraps raw HTML
    // in `<!--<#>-->` markers: inside `<script>` they would become part of the JSON, so the
    // whole element is the raw HTML.
    let json_ld = format!(
        r#"<script type="application/ld+json">{}</script>"#,
        script_safe(&p.json_ld)
    );
    html! {
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>{ p.title() }</title>
                <meta name="description" content={p.description.clone()}/>
                <link rel="canonical" href={p.url.clone()}/>
                <meta property="og:title" content={p.name.clone()}/>
                <meta property="og:description" content={p.description}/>
                <meta property="og:type" content="website"/>
                <meta property="og:url" content={p.url}/>
                <meta property="og:image" content={p.image}/>
                <meta property="og:site_name" content={SITE}/>
                <meta name="twitter:card" content="summary_large_image"/>
                <meta name="twitter:site" content={TWITTER}/>
                { Html::from_html_unchecked(json_ld.into()) }
            </head>
            <body>
                <h1>{ p.name }</h1>
                <ul>
                    { for options }
                </ul>
            </body>
        </html>
    }
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio runtime");
    bench_ssr(|i| {
        let mut html = String::with_capacity(4096);
        html.push_str("<!DOCTYPE html>");
        runtime.block_on(
            LocalServerRenderer::<ProductPage>::with_props(Props { i }).render_to_string(&mut html),
        );
        html
    });
}
