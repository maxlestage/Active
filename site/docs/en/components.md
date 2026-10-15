---
title: Components and views
description: Build views with one function per HTML tag, compose them into components that are plain Rust functions, and render them to HTML on the server or in the browser.
---

A view in `active` is a tree of nodes built with ordinary function calls: `div()` creates a `<div>`, `.class("card")` adds a class, `.child(...)` appends a child. A component is a Rust function that returns a view. There is no macro and no template language, so the compiler, rust-analyzer and rustfmt understand everything you write.

All examples on this page start with:

```rust
use active::prelude::*;
```

The prelude brings the tag functions (`div`, `p`, `a`…), `Element`, `Node`, `Seo`, `Document`, `use_state`, `effect`, `island`, `text`, `text_dyn`, `fragment` and `fragment_dyn` into scope.

## Elements

Every common HTML tag has a function with the same name in `active::html`: `div()`, `section()`, `h1()`, `a()`, `button()`, `input()`, `table()`, `video()`… Each returns an `Element` that you configure with chained methods:

```rust
use active::prelude::*;

let link = a().href("/docs/").class("button primary").text("Read the docs");
assert_eq!(
    Node::from(link).render(),
    r#"<a class="button primary" href="/docs/">Read the docs</a>"#
);
```

A few names differ from the tag to stay valid Rust or avoid clashes:

| Function | Tag |
| --- | --- |
| `text_svg()` | SVG `<text>` |
| `use_svg()` | SVG `<use>` |
| `linear_gradient()` | SVG `<linearGradient>` |

For any other tag, including custom elements, use `el`:

```rust
use active::prelude::*;

let widget = el("my-widget").attr("mode", "compact");
assert_eq!(Node::from(widget).render(), r#"<my-widget mode="compact"></my-widget>"#);
```

There is no `html()`, `head()` or `body()` tag function: [`Document`](../documents/) writes those around your view.

## Text

`.text(...)` appends a text child, and the free function `text(...)` creates a text node. Text is always escaped, so user input can never inject markup:

```rust
use active::prelude::*;

let para = p().text("Tom & Jerry <3");
assert_eq!(Node::from(para).render(), "<p>Tom &amp; Jerry &lt;3</p>");
```

Text arguments accept `&'static str`, `String` and `Cow<'static, str>` (exported as `active::Str`). A borrowed `&str` that does not live for the whole program must be turned into a `String` first, which is why examples write `name.to_string()` or `format!(...)`:

```rust
use active::prelude::*;

fn greeting(name: &str) -> Element {
    p().text(format!("Hello, {name}!"))
}
```

When two texts are adjacent, the server writes an empty comment `<!---->` between them so the browser keeps them as two separate text nodes. You never need to handle it yourself.

## Attributes

| Method | Result |
| --- | --- |
| `.attr(name, value)` | `name="value"` (escaped) |
| `.id(id)` | `id="…"` |
| `.class(classes)` | adds classes; call it several times to add more |
| `.style(css)` | `style="…"` |
| `.href(url)` | `href="…"` |
| `.bool_attr(name, f)` | a boolean attribute such as `disabled`, present while `f()` returns `true` |

```rust
use active::prelude::*;

let field = input()
    .attr("type", "email")
    .attr("name", "email")
    .attr("placeholder", "you@example.com")
    .bool_attr("required", || true);
assert_eq!(
    Node::from(field).render(),
    r#"<input type="email" name="email" placeholder="you@example.com" required=""/>"#
);
```

Setting an attribute again replaces its value, whichever method set it (`attr`, `attr_dyn`, `bool_attr`, `id`, `href`…), so a component can return an element and its caller can still override an attribute. Classes are the exception: they add up. Every `.class(...)` (and `.attr("class", ...)`, which does the same) adds to one `class` attribute, written first:

```rust
use active::prelude::*;

fn button_link(href: &str) -> Element {
    a().class("button").href(href.to_string()).attr("role", "button")
}

let link = button_link("/old").href("/new").attr("class", "primary");
assert_eq!(
    Node::from(link).render(),
    r#"<a class="button primary" href="/new" role="button"></a>"#
);
```

Attributes, classes and even boolean attributes can follow the page's state: see [State](../state/#binding-state-to-the-view).

### Dynamic styles

`style_dyn(property, f)` sets one CSS property from states, like Vue's `:style="{ width: … }"`. It combines with a `style` set before it and with other `style_dyn`; an empty string leaves the property out. For classes, `class_if(class, f)` plays the role of Vue's `:class="{ active: … }"`.

```rust
use active::prelude::*;

let progress = use_state(40);
let bar = div()
    .class("bar")
    .style("height: 4px")
    .style_dyn("width", move || format!("{}%", progress.get()));
assert_eq!(
    Node::from(bar).render(),
    r#"<div class="bar" style="height: 4px; width: 40%"></div>"#
);
```

## Children

`.child(x)` appends one child and `.children(iter)` appends several. Anything that converts into a `Node` can be a child:

| Value | Becomes |
| --- | --- |
| an `Element` (`p()`, `div()…`) | that element |
| `&'static str`, `String` | escaped text |
| a `Seo` | SEO tags for the `<head>` (nothing in the body) |
| `Option<T>` | the value, or nothing for `None` |
| `Vec<T>`, `[T; N]` | the items, side by side |
| a `Node` | itself |

```rust
use active::prelude::*;

let list = ul()
    .class("menu")
    .children(["Home", "Docs", "Blog"].map(|label| li().text(label)));
assert_eq!(
    Node::from(list).render(),
    r#"<ul class="menu"><li>Home</li><li>Docs</li><li>Blog</li></ul>"#
);
```

## Fragments

`fragment(...)` groups nodes without a wrapper element, which is useful when a component returns several siblings:

```rust
use active::prelude::*;

fn title_and_lead() -> Node {
    fragment([h1().text("Pricing"), p().class("lead").text("Simple plans.")])
}

assert_eq!(title_and_lead().render(), r#"<h1>Pricing</h1><p class="lead">Simple plans.</p>"#);
```

## Conditions

Because `Option<T>` converts into a node, `bool::then` is the simplest way to show something conditionally:

```rust
use active::prelude::*;

fn badge(is_new: bool) -> Element {
    span()
        .class("product")
        .text("Oak chair")
        .child(is_new.then(|| em().text(" new")))
}

assert_eq!(Node::from(badge(false)).render(), r#"<span class="product">Oak chair</span>"#);
```

When the branches produce different kinds of nodes, convert each branch with `.into()`:

```rust
use active::prelude::*;

fn price(cents: Option<u32>) -> Node {
    match cents {
        Some(c) => strong().text(format!("{}.{:02} €", c / 100, c % 100)).into(),
        None => "Sold out".into(),
    }
}
```

These conditions are decided once, when the view is built. For a condition that changes in the browser, use [`children_dyn`](../state/#lists-and-conditions-that-change).

## Lists

Map an iterator to elements and pass it to `.children`:

```rust
use active::prelude::*;

struct Post {
    slug: String,
    title: String,
}

fn post_list(posts: &[Post]) -> Element {
    ul().class("posts").children(posts.iter().map(|post| {
        li().child(a().href(format!("/blog/{}/", post.slug)).text(post.title.clone()))
    }))
}
```

## Components

A component is a function that returns a view. Its arguments are its props:

```rust
use active::prelude::*;

/// A card with a title, a text and an optional link.
fn card(title: &str, body: &str, link: Option<&str>) -> Element {
    article()
        .class("card")
        .child(h2().text(title.to_string()))
        .child(p().text(body.to_string()))
        .child(link.map(|href| a().href(href.to_string()).text("Read more")))
}

fn features() -> Element {
    section()
        .class("features")
        .child(card("Fast", "Rendered on the server.", None))
        .child(card("Searchable", "Complete SEO tags.", Some("/docs/seo/")))
}
```

### Element or Node?

- Return **`Element`** when the component is one element. Callers can keep chaining: `card(...).class("wide").id("first")`.
- Return **`Node`** when it can be several siblings (a `fragment`), different kinds of nodes, or nothing. [Islands](../islands/) also return `Node`.

Turn an `Element` into a `Node` with `.into()` or `Node::from(element)`.

### Props structs

When a component takes many values, group them in a struct:

```rust
use active::prelude::*;

pub struct ButtonProps<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub primary: bool,
}

pub fn link_button(props: ButtonProps<'_>) -> Element {
    a().href(props.href.to_string())
        .class("button")
        .class(if props.primary { "primary" } else { "" })
        .text(props.label.to_string())
}
```

An empty class string is ignored, so `.class(if … { "primary" } else { "" })` is a convenient way to add a class conditionally.

### Layouts and children

A layout is a component that takes other views as arguments:

```rust
use active::prelude::*;

fn layout(title: &str, content: impl Into<Node>) -> Node {
    fragment([
        Node::from(header().child(a().href("/").text("Acme"))),
        el("main").child(h1().text(title.to_string())).child(content).into(),
        footer().text("© Acme").into(),
    ])
}

let page = layout("About", p().text("We make furniture."));
assert!(page.render().contains("<main><h1>About</h1><p>We make furniture.</p></main>"));
```

## Raw HTML

`.html(...)` sets an element's content to a string of HTML that is inserted without escaping, for example Markdown rendered by another crate:

```rust
use active::prelude::*;

let article = div().class("prose").html("<p>Rendered <em>Markdown</em></p>");
assert_eq!(
    Node::from(article).render(),
    r#"<div class="prose"><p>Rendered <em>Markdown</em></p></div>"#
);
```

`.html(...)` replaces any children added before, and a `.child(...)` added after it replaces the HTML. Use one or the other on a given element.

> **Warning:** Raw HTML is not escaped. Only pass HTML you produced or sanitized yourself, never text typed by users.

## SVG

SVG elements work like HTML ones, and in the browser they are created in the SVG namespace automatically:

```rust
use active::prelude::*;

fn check_icon() -> Element {
    svg()
        .attr("viewBox", "0 0 24 24")
        .attr("width", "16")
        .attr("height", "16")
        .attr("aria-hidden", "true")
        .child(path().attr("d", "M5 12l5 5L20 7").attr("fill", "none").attr("stroke", "currentColor"))
}
```

## Rendering

- `Node::render()` returns the HTML of a view as a `String`, and `render_into(&mut out)` appends it to an existing buffer.
- `Document::new(view).render()` returns a complete page with `<head>` and SEO tags; see [Documents](../documents/).
- In the browser, the same views are [hydrated](../islands/) or [mounted](../client-app/).

Void elements (`img`, `input`, `br`, `meta`…) are written as `<img …/>` and ignore children.

A view can be rendered as many times as you like, and printing a `Node` with `{:?}` shows its HTML. On the server, the [states](../state/#on-the-server) it uses are released automatically once later renders no longer need them.

## Things to know

- **`main` in a binary.** In a file that defines `fn main`, that function hides the `main()` tag function of the prelude. Write `active::html::main()` or `el("main")` there. In a library file, `main()` works as expected.
- **Short tag names.** The prelude includes one-letter tags (`a`, `b`, `i`, `p`, `q`, `s`, `u`). A local variable with the same name hides the function: `for (i, item) in …` followed by `i()` will not compile. Rename the variable or call `active::html::i()`.
- **Attribute names are written as is.** Values are escaped, names are not: never build an attribute name from user input.
