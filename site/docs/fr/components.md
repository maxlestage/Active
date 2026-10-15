---
title: Composants et vues
description: Construisez des vues avec une fonction par balise HTML, assemblez-les en composants qui sont de simples fonctions Rust, et rendez-les en HTML partout.
---

Dans `active`, une vue est un arbre de nœuds construit avec des appels de fonctions ordinaires : `div()` crée un `<div>`, `.class("card")` ajoute une classe, `.child(...)` ajoute un enfant. Un composant est une fonction Rust qui renvoie une vue. Il n'y a ni macro ni langage de templates : le compilateur, rust-analyzer et rustfmt comprennent tout ce que vous écrivez.

Tous les exemples de cette page commencent par :

```rust
use active::prelude::*;
```

Le prélude importe les fonctions de balise (`div`, `p`, `a`…), `Element`, `Node`, `Seo`, `Document`, `use_state`, `effect`, `island`, `text`, `text_dyn`, `fragment` et `fragment_dyn`.

## Éléments

Chaque balise HTML courante a une fonction du même nom dans `active::html` : `div()`, `section()`, `h1()`, `a()`, `button()`, `input()`, `table()`, `video()`… Chacune renvoie un `Element` que vous configurez avec des méthodes chaînées :

```rust
use active::prelude::*;

let link = a().href("/docs/").class("button primary").text("Read the docs");
assert_eq!(
    Node::from(link).render(),
    r#"<a class="button primary" href="/docs/">Read the docs</a>"#
);
```

Quelques noms diffèrent de la balise, pour rester du Rust valide ou éviter des conflits :

| Fonction | Balise |
| --- | --- |
| `text_svg()` | SVG `<text>` |
| `use_svg()` | SVG `<use>` |
| `linear_gradient()` | SVG `<linearGradient>` |

Pour toute autre balise, y compris les éléments personnalisés, utilisez `el` :

```rust
use active::prelude::*;

let widget = el("my-widget").attr("mode", "compact");
assert_eq!(Node::from(widget).render(), r#"<my-widget mode="compact"></my-widget>"#);
```

Il n'existe pas de fonction `html()`, `head()` ou `body()` : [`Document`](../documents/) écrit ces balises autour de votre vue.

## Texte

`.text(...)` ajoute un enfant texte, et la fonction libre `text(...)` crée un nœud texte. Le texte est toujours échappé, si bien qu'une saisie utilisateur ne peut jamais injecter de balisage :

```rust
use active::prelude::*;

let para = p().text("Tom & Jerry <3");
assert_eq!(Node::from(para).render(), "<p>Tom &amp; Jerry &lt;3</p>");
```

Les arguments texte acceptent `&'static str`, `String` et `Cow<'static, str>` (exporté sous le nom `active::Str`). Un `&str` emprunté qui ne vit pas pendant toute la durée du programme doit d'abord être converti en `String`, c'est pourquoi les exemples écrivent `name.to_string()` ou `format!(...)` :

```rust
use active::prelude::*;

fn greeting(name: &str) -> Element {
    p().text(format!("Hello, {name}!"))
}
```

Quand deux textes sont adjacents, le serveur écrit entre eux un commentaire vide `<!---->` pour que le navigateur les conserve comme deux nœuds texte distincts. Vous n'avez jamais à vous en occuper.

## Attributs

| Méthode | Résultat |
| --- | --- |
| `.attr(name, value)` | `name="value"` (échappé) |
| `.id(id)` | `id="…"` |
| `.class(classes)` | ajoute des classes ; appelez-la plusieurs fois pour en ajouter d'autres |
| `.style(css)` | `style="…"` |
| `.href(url)` | `href="…"` |
| `.bool_attr(name, f)` | un attribut booléen comme `disabled`, présent tant que `f()` renvoie `true` |

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

Définir de nouveau un attribut remplace sa valeur, quelle que soit la méthode qui l'a défini (`attr`, `attr_dyn`, `bool_attr`, `id`, `href`…) : un composant peut donc renvoyer un élément dont l'appelant pourra encore remplacer un attribut. Les classes font exception : elles s'additionnent. Chaque `.class(...)` (et `.attr("class", ...)`, qui fait la même chose) ajoute à un unique attribut `class`, écrit en premier :

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

Les attributs, les classes et même les attributs booléens peuvent suivre l'état de la page : voir [État](../state/#lier-l-etat-a-la-vue).

### Styles dynamiques

`style_dyn(propriete, f)` règle une propriété CSS à partir d'états, comme `:style="{ width: … }"` dans Vue. Elle se combine avec un `style` posé avant et avec d'autres `style_dyn` ; une chaîne vide omet la propriété. Pour les classes, `class_if(classe, f)` joue le rôle de `:class="{ active: … }"`.

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

## Enfants

`.child(x)` ajoute un enfant et `.children(iter)` en ajoute plusieurs. Tout ce qui se convertit en `Node` peut être un enfant :

| Valeur | Devient |
| --- | --- |
| un `Element` (`p()`, `div()…`) | cet élément |
| `&'static str`, `String` | du texte échappé |
| un `Seo` | des balises SEO pour le `<head>` (rien dans le corps) |
| `Option<T>` | la valeur, ou rien pour `None` |
| `Vec<T>`, `[T; N]` | les éléments, côte à côte |
| un `Node` | lui-même |

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

`fragment(...)` regroupe des nœuds sans élément englobant, ce qui est utile quand un composant renvoie plusieurs nœuds frères :

```rust
use active::prelude::*;

fn title_and_lead() -> Node {
    fragment([h1().text("Pricing"), p().class("lead").text("Simple plans.")])
}

assert_eq!(title_and_lead().render(), r#"<h1>Pricing</h1><p class="lead">Simple plans.</p>"#);
```

## Conditions

Comme `Option<T>` se convertit en nœud, `bool::then` est le moyen le plus simple d'afficher quelque chose sous condition :

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

Quand les branches produisent des nœuds de types différents, convertissez chaque branche avec `.into()` :

```rust
use active::prelude::*;

fn price(cents: Option<u32>) -> Node {
    match cents {
        Some(c) => strong().text(format!("{}.{:02} €", c / 100, c % 100)).into(),
        None => "Sold out".into(),
    }
}
```

Ces conditions sont évaluées une seule fois, à la construction de la vue. Pour une condition qui change dans le navigateur, utilisez [`children_dyn`](../state/#listes-et-conditions-qui-changent).

## Listes

Transformez un itérateur en éléments et passez-le à `.children` :

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

## Composants

Un composant est une fonction qui renvoie une vue. Ses arguments sont ses props :

```rust
use active::prelude::*;

/// Une carte avec un titre, un texte et un lien facultatif.
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

### Element ou Node ?

- Renvoyez un **`Element`** quand le composant est un seul élément. Les appelants peuvent continuer à chaîner : `card(...).class("wide").id("first")`.
- Renvoyez un **`Node`** quand il peut s'agir de plusieurs nœuds frères (un `fragment`), de nœuds de types différents, ou de rien du tout. Les [îlots](../islands/) renvoient aussi un `Node`.

Transformez un `Element` en `Node` avec `.into()` ou `Node::from(element)`.

### Structures de props

Quand un composant reçoit beaucoup de valeurs, regroupez-les dans une structure :

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

Une chaîne de classe vide est ignorée : `.class(if … { "primary" } else { "" })` est donc un moyen pratique d'ajouter une classe sous condition.

### Mises en page et enfants

Une mise en page (layout) est un composant qui reçoit d'autres vues en arguments :

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

## HTML brut

`.html(...)` remplace le contenu d'un élément par une chaîne de HTML insérée sans échappement, par exemple du Markdown rendu par un autre crate :

```rust
use active::prelude::*;

let article = div().class("prose").html("<p>Rendered <em>Markdown</em></p>");
assert_eq!(
    Node::from(article).render(),
    r#"<div class="prose"><p>Rendered <em>Markdown</em></p></div>"#
);
```

`.html(...)` remplace les enfants ajoutés auparavant, et un `.child(...)` ajouté ensuite remplace le HTML. Utilisez l'un ou l'autre sur un même élément.

> **Attention :** Le HTML brut n'est pas échappé. Ne passez que du HTML que vous avez produit ou assaini vous-même, jamais du texte saisi par des utilisateurs.

## SVG

Les éléments SVG fonctionnent comme les éléments HTML, et dans le navigateur ils sont automatiquement créés dans l'espace de noms SVG :

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

## Rendu

- `Node::render()` renvoie le HTML d'une vue sous forme de `String`, et `render_into(&mut out)` l'ajoute à la fin d'un tampon existant.
- `Document::new(view).render()` renvoie une page complète avec le `<head>` et les balises SEO ; voir [Documents](../documents/).
- Dans le navigateur, les mêmes vues sont [hydratées](../islands/) ou [montées](../client-app/).

Les éléments vides (`img`, `input`, `br`, `meta`…) sont écrits sous la forme `<img …/>` et ignorent leurs enfants.

Une vue peut être rendue autant de fois que vous le souhaitez, et afficher un `Node` avec `{:?}` montre son HTML. Sur le serveur, les [états](../state/#cote-serveur) qu'elle utilise sont libérés automatiquement dès que les rendus suivants n'en ont plus besoin.

## À savoir

- **`main` dans un binaire.** Dans un fichier qui définit `fn main`, cette fonction masque la fonction de balise `main()` du prélude. Écrivez-y `active::html::main()` ou `el("main")`. Dans un fichier de bibliothèque, `main()` fonctionne normalement.
- **Noms de balise courts.** Le prélude contient des balises d'une lettre (`a`, `b`, `i`, `p`, `q`, `s`, `u`). Une variable locale du même nom masque la fonction : `for (i, item) in …` suivi de `i()` ne compile pas. Renommez la variable ou appelez `active::html::i()`.
- **Les noms d'attributs sont écrits tels quels.** Les valeurs sont échappées, pas les noms : ne construisez jamais un nom d'attribut à partir d'une saisie utilisateur.
