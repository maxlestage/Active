//! Leptos 0.8, two ways (first argument):
//!
//! - `meta` (default, idiomatic for SEO): the page declares its head tags with leptos_meta's
//!   `<Title>`, `<Meta>`, `<Link>` and `<Script>`; the shell has `<MetaTags/>` and
//!   `ServerMetaContextOutput::inject_meta_context` puts them in `<head>`, as leptos_axum does.
//! - `plain`: the same tags written as plain elements in the shell's `<head>`.
//!
//! Each page renders in its own reactive `Owner`, like one request on a server.

use bench_common::{Product, SITE, TWITTER, bench_ssr, option_label, script_safe};
use futures::StreamExt;
use leptos::prelude::*;
use leptos_meta::{Link, Meta, MetaTags, Script, ServerMetaContext, Title, provide_meta_context};

/// The body, shared by both variants.
#[component]
fn Content(name: String, options: Vec<(u32, u32)>) -> impl IntoView {
    view! {
        <h1>{name}</h1>
        <ul>{options.into_iter().map(|(k, price)| view! { <li>{option_label(k, price)}</li> }).collect_view()}</ul>
    }
}

/// leptos_meta: the page component declares its own head tags.
#[component]
fn MetaPage(p: Product) -> impl IntoView {
    let options = p.options().collect::<Vec<_>>();
    view! {
        <Title formatter=|title| format!("{title} · {SITE}") text=p.name.clone()/>
        <Meta name="description" content=p.description.clone()/>
        <Link rel="canonical" href=p.url.clone()/>
        <Meta property="og:title" content=p.name.clone()/>
        <Meta property="og:description" content=p.description/>
        <Meta property="og:type" content="website"/>
        <Meta property="og:url" content=p.url/>
        <Meta property="og:image" content=p.image/>
        <Meta property="og:site_name" content=SITE/>
        <Meta name="twitter:card" content="summary_large_image"/>
        <Meta name="twitter:site" content=TWITTER/>
        <Script type_="application/ld+json">{script_safe(&p.json_ld)}</Script>
        <Content name=p.name options/>
    }
}

#[component]
fn MetaShell(i: u32) -> impl IntoView {
    provide_meta_context();
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <MetaTags/>
            </head>
            <body>
                <MetaPage p=Product::new(i)/>
            </body>
        </html>
    }
}

/// Plain elements: the whole document in one view.
#[component]
fn PlainShell(i: u32) -> impl IntoView {
    let p = Product::new(i);
    let options = p.options().collect::<Vec<_>>();
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>{format!("{} · {SITE}", p.name)}</title>
                <meta name="description" content=p.description.clone()/>
                <link rel="canonical" href=p.url.clone()/>
                {og("og:title", p.name.clone())}
                {og("og:description", p.description)}
                {og("og:type", "website")}
                {og("og:url", p.url)}
                {og("og:image", p.image)}
                {og("og:site_name", SITE)}
                <meta name="twitter:card" content="summary_large_image"/>
                <meta name="twitter:site" content=TWITTER/>
                <script type="application/ld+json">{script_safe(&p.json_ld)}</script>
            </head>
            <body>
                <Content name=p.name options/>
            </body>
        </html>
    }
}

/// `<meta property="…" content="…">`: `property` is not an HTML attribute, so the view
/// macro has no method for it; leptos_meta's `<Meta>` builds it the same way.
fn og(property: &'static str, content: impl Into<String>) -> impl IntoView {
    leptos::html::meta()
        .attr("property", property)
        .content(content.into())
}

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio runtime");
    let _guard = runtime.enter();
    any_spawner::Executor::init_tokio().expect("Leptos executor");

    match std::env::args().nth(1).as_deref().unwrap_or("meta") {
        "meta" => bench_ssr(|i| {
            Owner::new().with(|| {
                let (meta, tags) = ServerMetaContext::new();
                provide_context(meta);
                let html = view! { <MetaShell i/> }.to_html();
                runtime.block_on(async {
                    let page = tags
                        .inject_meta_context(futures::stream::iter([html]))
                        .await;
                    page.collect::<String>().await
                })
            })
        }),
        "plain" => bench_ssr(|i| Owner::new().with(|| view! { <PlainShell i/> }.to_html())),
        other => panic!("unknown variant {other:?}: expected `meta` or `plain`"),
    }
}
