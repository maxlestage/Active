//! maud: compile-time HTML templates. A template engine, not a component framework: the
//! fastest idiomatic way to write HTML in Rust, shown as a second baseline.

use bench_common::{Product, SITE, TWITTER, bench_ssr, script_safe};
use maud::{DOCTYPE, PreEscaped, html};

fn render(i: u32) -> String {
    let p = Product::new(i);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (p.name) " · " (SITE) }
                meta name="description" content=(p.description);
                link rel="canonical" href=(p.url);
                meta property="og:title" content=(p.name);
                meta property="og:description" content=(p.description);
                meta property="og:type" content="website";
                meta property="og:url" content=(p.url);
                meta property="og:image" content=(p.image);
                meta property="og:site_name" content=(SITE);
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:site" content=(TWITTER);
                script type="application/ld+json" { (PreEscaped(script_safe(&p.json_ld))) }
            }
            body {
                h1 { (p.name) }
                ul {
                    @for (k, price) in p.options() {
                        li { "Option " (k) " — " (price) " €" }
                    }
                }
            }
        }
    }
    .into_string()
}

fn main() {
    bench_ssr(render);
}
