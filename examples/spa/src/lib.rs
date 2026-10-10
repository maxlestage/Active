//! A to-do list rendered entirely in the browser, with two pages and a router.

use active::prelude::*;

#[cfg(target_arch = "wasm32")]
mod app;

/// SEO shared by every page. The app repeats it in its root view so that the browser keeps
/// the title template when the router changes page.
pub fn site_seo() -> Seo {
    Seo::new()
        .site_name("Todos")
        .title_template("%s · Todos")
        .description("A to-do list written in Rust with active, running as WebAssembly.")
}

/// The page the server sends for every path: SEO tags, styles, an empty `#app` and the
/// WebAssembly bundle that fills it.
pub fn shell() -> String {
    Document::new(div().id("app").child(p().class("loading").text("Loading…")))
        .lang("en")
        .seo(site_seo())
        .head(STYLE)
        .client("/pkg/spa.js", "/pkg/spa_bg.wasm")
        .render()
}

const STYLE: &str = "<style>\
body{font:17px/1.5 system-ui,sans-serif;margin:0;background:#f6f5f2;color:#1b1b1f}\
.app{max-width:34rem;margin:3rem auto;padding:0 1rem}\
nav{display:flex;gap:1rem;margin-bottom:2rem}nav a{color:inherit}nav a.on{font-weight:700}\
form{display:flex;gap:.5rem}input[type=text]{flex:1;padding:.5rem;font:inherit}\
button{font:inherit;padding:.4rem .8rem;cursor:pointer}\
ul{list-style:none;padding:0}li{display:flex;gap:.6rem;align-items:center;padding:.35rem 0}\
li.done span{text-decoration:line-through;opacity:.55}li button{margin-left:auto}\
.bar{display:flex;gap:.8rem;align-items:center}.bar .left{margin-right:auto}\
.bar a{color:inherit}.bar a.on{font-weight:700}\
</style>";
