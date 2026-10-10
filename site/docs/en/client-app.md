---
title: Client-side apps
description: Build a single-page app that runs entirely in the browser: mount it with mount_to, route with location and navigate, and keep titles and meta tags up to date.
---

A client-side app (often called a single-page app) is rendered entirely in the browser. The server, or any static host, sends one small HTML page; the WebAssembly bundle then builds the whole interface, changes pages without reloading and updates the title and meta tags as the visitor moves around.

It is the right choice for tools, dashboards and anything behind a login, where search engines do not matter and the interface is interactive everywhere. For public content, prefer [server-rendered pages with islands](../islands/) or a [static site](../static-site/): they send complete HTML that every crawler can read. All three use the same components, so you can mix them.

## What you build

The repository contains a complete example, [`examples/spa`](https://github.com/maxlestage/Active/tree/master/examples/spa): a to-do list with two pages, a form, filters kept in the URL (`/?show=done`), a list saved in `localStorage` and a page title per route. Run it from the repository root:

```sh
cargo build -p spa --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --out-dir examples/spa/pkg target/wasm32-unknown-unknown/wasm-release/spa.wasm
cargo run -p spa
```

The sections below explain each part. The crate layout is the same as in [Installation](../installation/#one-crate-two-targets).

## The shell

The page the server sends is an ordinary [`Document`](../documents/) whose body is an empty container, plus the bundle:

```rust
use active::prelude::*;

/// SEO shared by every page.
pub fn site_seo() -> Seo {
    Seo::new()
        .site_name("Todos")
        .title_template("%s · Todos")
        .description("A to-do list written in Rust, running as WebAssembly.")
}

/// The same page for every URL: the app fills #app in the browser.
pub fn shell() -> String {
    Document::new(div().id("app").child(p().text("Loading…")))
        .lang("en")
        .seo(site_seo())
        .stylesheet("/app.css")
        .client("/pkg/spa.js", "/pkg/spa_bg.wasm")
        .render()
}
```

The "Loading…" text shows until the bundle has started. Because the shell already has the site's title and description, link previews and crawlers that do not run WebAssembly still see something meaningful.

## Mounting the app

The browser entry point mounts the root view into the container:

```rust
use active::prelude::*;

fn app() -> Node {
    h1().text("Hello from the browser").into()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app());
}
```

`mount_to(selector, view)` finds the first element matching the CSS selector, removes its content and builds the view inside it. If no element matches, it prints a warning in the console.

To add a view to a page without replacing anything (a widget inside an existing page), use `active::mount(&node, &element)`, which appends the view after the element's current children.

## Routing

`active::location()` returns a `State<String>` holding the current path (`"/"`, `"/about"`). Read it in a [dynamic region](../state/#lists-and-conditions-that-change) to show the page that matches:

```rust
use active::prelude::*;

fn home() -> Node {
    section().child(Seo::new().title("Home")).child(h1().text("Home")).into()
}

fn about() -> Node {
    section()
        .child(Seo::new().title("About").description("Who we are."))
        .child(h1().text("About"))
        .into()
}

fn not_found() -> Node {
    section().child(Seo::new().title("Not found").noindex(true)).child(h1().text("Not found")).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let link = move |href: &'static str, label: &'static str| {
        a().href(href).class_if("current", move || path.get() == href).text(label)
    };
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .child(nav().child(link("/", "Home")).child(link("/about", "About")))
        .child(el("main").children_dyn(move || match path.get().as_str() {
            "/" => vec![home()],
            "/about" => vec![about()],
            _ => vec![not_found()],
        }))
        .into()
}
```

The first call to `location()`, `query()` or `query_param()` starts the router:

- **Links just work.** Clicks on `<a href>` elements that point to the same site change the path without reloading the page. Links with a `target` (other than `_self`), a `download` attribute, a click with a modifier key (Ctrl, ⌘, Shift, Alt) or the middle button, links to other sites, and links to an anchor on the current page (`#section`) are left to the browser.
- **Back and forward work.** The browser's history buttons update the path and the query string.
- **Only the region is rebuilt.** When the path changes, the region that reads it is rebuilt; the navigation bar and everything else stay in place.

`location()`, `query()`, `query_param()` and `navigate()` exist only with the `client` feature, so code that calls them is compiled for the browser only (here behind `#[cfg(target_arch = "wasm32")]`). The page components themselves compile everywhere.

### Changing page from code

`active::navigate(to)` goes to another page from a handler, for example after a form is saved:

```rust
use active::prelude::*;

fn save_button() -> Element {
    button().text("Save").on_click(|_| {
        // … save, then:
        #[cfg(target_arch = "wasm32")]
        active::navigate("/saved");
    })
}
```

It adds an entry to the browser history, updates `location()` and `query()`, then scrolls to the top of the page, or to the element whose `id` matches a `#fragment` (`navigate("/docs#install")`).

### Paths with parameters

Match on the path's segments to extract parameters:

```rust
use active::prelude::*;

fn post(slug: &str) -> Node {
    article().child(Seo::new().title(slug.to_string())).child(h1().text(slug.to_string())).into()
}

fn route(path: &str) -> Vec<Node> {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => vec![h1().text("Blog").into()],
        ["posts", slug] => vec![post(slug)],
        _ => vec![h1().text("Not found").into()],
    }
}

assert!(Node::from(div().children(route("/posts/hello-rust"))).render().contains("<h1>hello-rust</h1>"));
```

In the app: `.children_dyn(move || route(&path.get()))`.

### Query strings

`active::query()` returns the query string without the `?` (`"q=rust&page=2"`) as a state, and `active::query_param(name)` returns one parameter, decoded, or `None`. Both follow the router: links, `navigate` and the history buttons update them. Going from `/search?q=a` to `/search?q=b` rebuilds the regions that read the query, while those that read only the path stay as they are.

Keep in the URL what should survive a reload or be shared in a link: a filter, a search, a page number. `examples/spa` keeps its filter there:

```rust
use active::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

/// The filter is part of the URL: `/`, `/?show=active`, `/?show=done`.
#[cfg(target_arch = "wasm32")]
fn filter() -> Filter {
    match active::query_param("show").as_deref() {
        Some("active") => Filter::Active,
        Some("done") => Filter::Done,
        _ => Filter::All,
    }
}

#[cfg(target_arch = "wasm32")]
fn filter_links() -> Element {
    let link = |value: Filter, href: &'static str, label: &'static str| {
        a().href(href).class_if("on", move || filter() == value).text(label)
    };
    nav()
        .child(link(Filter::All, "/", "All"))
        .child(link(Filter::Active, "/?show=active", "Active"))
        .child(link(Filter::Done, "/?show=done", "Done"))
}
```

The list reads `filter()` inside its `children_dyn`, so clicking a link rebuilds only the list. The fragment of the URL (`#…`) is not part of either state: the browser scrolls to anchors by itself.

## Titles and meta tags

Each page puts a `Seo` in its view, exactly as on the server. In a mounted app, `active` keeps the `<head>` in sync:

- when the app is mounted, the `Seo` values of the view are merged and written to the head;
- every time a dynamic region is rebuilt (a page change), its `Seo` values are merged **over those of its ancestors** and the head is updated: the title, the description, robots, canonical, alternates, Open Graph, Twitter, JSON-LD and the extra meta tags;
- when the new content has no `Seo`, the head goes back to the ancestors' values;
- when the merged values have neither a title nor a site name, the page title is emptied rather than left over from the previous page.

Put the site defaults (site name, title template, default description) in a `Seo` in the **root view**, as the router example above does, so that every page's title gets the template. The `Seo` given to `Document::seo` in the shell is only used by the server.

Other head elements (stylesheets, the charset, your own `<meta>` tags that `Seo` does not manage) are left alone.

To change the head from a handler, call `active::update_head(&seo)` with a complete `Seo`; it replaces the tags of the kinds listed above.

## Keeping data across pages

States created inside a page are released when the router leaves that page: a page that is shown again starts fresh. Data that must survive page changes goes in a state created **above** the router, and is passed to the pages:

```rust
use active::prelude::*;

#[derive(Clone)]
struct Todo {
    text: String,
    done: bool,
}

fn todo_page(todos: State<Vec<Todo>>) -> Node {
    let left = move || todos.with(|list| list.iter().filter(|t| !t.done).count());
    section().child(p().text_dyn(move || format!("{} left", left()))).into()
}

#[cfg(target_arch = "wasm32")]
fn app() -> Node {
    let path = active::location();
    let todos = use_state(Vec::<Todo>::new()); // survives page changes
    div()
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![todo_page(todos)],
            _ => vec![h1().text("Not found").into()],
        })
        .into()
}
```

To keep data between visits, save it in `localStorage` from an [effect](../state/#effects), as `examples/spa` does.

## Serving the app

Every URL of the app must return the shell, so that reloading `/about` or opening a shared link works. Requests for the bundle (`/pkg/…`) return the files.

With Axum, serve `/pkg` as files and answer the shell for everything else:

```rust
use axum::{Router, response::Html, routing::get};
use tower_http::services::ServeDir;

fn shell() -> String {
    String::from("<!DOCTYPE html>…") // the shell() function above
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .nest_service("/pkg", ServeDir::new("pkg"))
        .fallback(get(|| async { Html(shell()) }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`examples/spa` does the same with a server written with the standard library only. On a static host, write the shell to `index.html` (see [Static sites](../static-site/)) and configure a fallback:

| Host | Fallback to `index.html` |
| --- | --- |
| Netlify | a `_redirects` file containing `/* /index.html 200` |
| Cloudflare Pages | automatic when the site has no `404.html` |
| GitHub Pages | no rewrites: copy `index.html` to `404.html` (pages are then served with a 404 status) |

## Rendering the first page on the server

A router's pages can also be rendered on the server: the app takes the path as a `State`, created from the request on the server and from `location()` in the browser.

```rust
use active::prelude::*;

pub fn app(path: State<String>) -> Node {
    div()
        .child(Seo::new().site_name("Acme").title_template("%s · Acme"))
        .children_dyn(move || match path.get().as_str() {
            "/" => vec![section().child(Seo::new().title("Home")).text("Welcome").into()],
            _ => vec![section().child(Seo::new().title("Not found").noindex(true)).into()],
        })
        .into()
}

/// Server: the full HTML of `path`, with its title and meta tags.
pub fn page(path: &str) -> String {
    Document::new(div().id("app").child(app(use_state(path.to_string()))))
        .client("/pkg/app.js", "/pkg/app_bg.wasm")
        .render()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app(active::location()));
}

assert!(page("/").contains("<title>Home · Acme</title>"));
```

On the server, `Document` finds the `Seo` of the page shown by the dynamic region, so each URL gets its own title and description. In the browser, `mount_to` replaces the server's HTML with the live app. Unlike [islands](../islands/), the content is rebuilt rather than hydrated, which is simple and fine for most apps.

## Size

The whole `examples/spa` app (routing, forms, filters, storage) compiles to about 106 kB of WebAssembly, 42 kB gzipped, with the `wasm-release` profile. See [Performance](../performance/) for how to measure and reduce it.
