---
title: Etiquetas SEO
description: Describe cada página para buscadores y redes sociales con Seo: título, descripción, canonical, hreflang, robots, Open Graph, Twitter y JSON-LD.
---

Los buscadores y las redes sociales leen el `<head>` de tus páginas: el título y la descripción que aparecen en los resultados, la dirección canónica, las versiones en otros idiomas, la imagen de vista previa de un enlace compartido. `active::Seo` escribe todas estas etiquetas desde un único builder, y una página puede contener varios valores `Seo` que se [fusionan](#fusion) en un solo `<head>` sin duplicados.

## Un ejemplo completo

```rust
use active::Seo;

let head = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com")
    .title("Oak chair")
    .description("Solid oak chair, built to last and delivered in 48 hours.")
    .canonical("/products/oak-chair")
    .image("/images/oak-chair.jpg")
    .og_type("product")
    .locale("en_US")
    .twitter("@acme")
    .alternate("en", "/products/oak-chair")
    .alternate("fr", "/fr/produits/chaise-chene")
    .to_html();

assert!(head.starts_with("<title>Oak chair · Acme</title>"));
assert!(head.contains(r#"<link rel="canonical" href="https://acme.com/products/oak-chair"/>"#));
```

La salida, aquí con una etiqueta por línea para facilitar la lectura (la salida real no tiene saltos de línea):

```html
<title>Oak chair · Acme</title>
<meta name="description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<link rel="canonical" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="en" href="https://acme.com/products/oak-chair"/>
<link rel="alternate" hreflang="fr" href="https://acme.com/fr/produits/chaise-chene"/>
<meta property="og:title" content="Oak chair"/>
<meta property="og:description" content="Solid oak chair, built to last and delivered in 48 hours."/>
<meta property="og:type" content="product"/>
<meta property="og:url" content="https://acme.com/products/oak-chair"/>
<meta property="og:image" content="https://acme.com/images/oak-chair.jpg"/>
<meta property="og:site_name" content="Acme"/>
<meta property="og:locale" content="en_US"/>
<meta name="twitter:card" content="summary_large_image"/>
<meta name="twitter:site" content="@acme"/>
```

Dentro de una página, rara vez llamarás tú mismo a `to_html()`: pon el `Seo` en la vista y deja que [`Document`](../documents/) lo renderice en el `<head>`.

## El builder

`Seo::new()` (o `Seo::default()`) empieza vacío. Cada setter recibe el valor como `impl Into<String>` (un `bool` para `noindex` y `nofollow`) y devuelve el `Seo` actualizado, así que las llamadas se encadenan.

| Método | Etiquetas que produce |
| --- | --- |
| `.title(title)` | `<title>` y `og:title` |
| `.title_template(template)` | solo `<title>`: el primer `%s` se reemplaza por el título, p. ej. `"%s · Acme"` |
| `.site_name(name)` | `og:site_name`, y el título cuando no se define ninguno |
| `.description(text)` | `<meta name="description">` y `og:description` |
| `.canonical(url)` | `<link rel="canonical">` y `og:url` |
| `.image(url)` | `og:image`; además cambia `twitter:card` a `summary_large_image` |
| `.og_type(type)` | `og:type`: `website` si no se define, `article`, `product`… |
| `.locale(locale)` | `og:locale`, p. ej. `en_US` |
| `.twitter(handle)` | `twitter:site`, p. ej. `@acme` |
| `.base_url(origin)` | ninguna etiqueta: resuelve las URL relativas de `canonical`, `image` y `alternate` |
| `.noindex(bool)`, `.nofollow(bool)` | `<meta name="robots">` |
| `.alternate(lang, url)` | `<link rel="alternate" hreflang="…">`, una por llamada |
| `.json_ld(json)` | `<script type="application/ld+json">`, una por llamada (consulta [Datos estructurados](../structured-data/)) |
| `.meta_name(name, content)` | cualquier otra `<meta name="…" content="…">` |
| `.meta_property(property, content)` | cualquier otra `<meta property="…" content="…">` |

Para renderizar: `.to_html()` devuelve un `String` nuevo, `.write_html(&mut out)` agrega las etiquetas a uno existente, y `Seo` implementa `Display`, así que `format!("<head>{seo}</head>")` también funciona.

## Reglas de renderizado

Estas reglas hacen que la salida sea predecible:

- **Orden fijo.** Las etiquetas siempre aparecen en el orden del ejemplo anterior, después las etiquetas adicionales `meta_name`/`meta_property` en el orden de las llamadas, y al final el JSON-LD.
- **Vacío significa sin definir.** Un valor ausente o una cadena vacía no produce ninguna etiqueta. `og:type` y `twitter:card` siempre están presentes.
- **Título y plantilla.** Con un título y una plantilla, `<title>` es la plantilla con `%s` reemplazado; `og:title` es el título solo, que se lee mejor en las vistas previas de enlaces. Sin título, se usa el nombre del sitio tal cual, sin la plantilla. Un título vacío explícito (`.title("")`) no genera ningún `<title>`.
- **Robots.** La etiqueta robots solo aparece cuando `noindex` o `nofollow` es `true`: `noindex,follow`, `index,nofollow` o `noindex,nofollow`. Las páginas son indexables por defecto, así que no hay nada que escribir en las páginas normales.
- **URL.** Con `base_url("https://acme.com")`, una URL relativa como `/contact` o `contact` se convierte en `https://acme.com/contact`. Las URL con esquema (`https:`, `mailto:`…) o que empiezan por `//` se dejan tal cual. Se ignora una `/` final en la base.
- **Alternativas.** Volver a llamar a `.alternate` con el mismo idioma reemplaza su URL en el mismo lugar. Una URL vacía se omite.
- **Escapado.** Todos los valores se escapan para HTML (`&`, `<`, `>`, `"`, `'`). En el JSON-LD, `<` se escribe `\u003c` para que los datos nunca puedan cerrar su etiqueta `<script>`.

## Valores por defecto del sitio

La mayoría de los valores son iguales en todas las páginas: nombre del sitio, plantilla de título, URL base, imagen para compartir por defecto, locale, usuario de Twitter. Constrúyelos una vez y clónalos, o pásalos al documento:

```rust
use std::sync::LazyLock;

use active::Seo;

static SITE: LazyLock<Seo> = LazyLock::new(|| {
    Seo::new()
        .site_name("Acme")
        .title_template("%s · Acme")
        .base_url("https://acme.com")
        .image("/og.png")
        .locale("en_US")
        .twitter("@acme")
});

fn contact_head() -> String {
    SITE.clone()
        .title("Contact")
        .description("Write to us, we answer within a day.")
        .canonical("/contact")
        .to_html()
}
```

Cuando la dirección del sitio solo se conoce en tiempo de ejecución (un ajuste `SITE_URL`, la cabecera `Host` de la petición), construye los valores por defecto en una función que reciba el origen, como hace este sitio en [`site/src/server/pages.rs`](https://github.com/maxlestage/Active/blob/master/site/src/server/pages.rs).

## SEO en las vistas

Un `Seo` se convierte en un `Node`, así que puede ser un hijo en cualquier parte de una vista. No renderiza nada en el body: [`Document`](../documents/) lo recoge y escribe las etiquetas en el `<head>`.

```rust
use active::prelude::*;

fn product_page(name: &str, slug: &str) -> Element {
    article()
        .child(
            Seo::new()
                .title(name.to_string())
                .description(format!("{name}: solid oak, delivered in 48 hours."))
                .canonical(format!("/products/{slug}"))
                .og_type("product"),
        )
        .child(h1().text(name.to_string()))
}

let html = Document::new(product_page("Oak chair", "oak-chair"))
    .lang("en")
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com"))
    .render();

assert!(html.contains("<title>Oak chair · Acme</title>"));
assert!(html.contains(r#"<meta property="og:type" content="product"/>"#));
```

Así la descripción de una página queda junto a su contenido: el componente que conoce el producto también lo describe.

## Fusión

Una página puede contener varios valores `Seo`: los valores por defecto del sitio pasados a `Document::seo`, la descripción de un layout, el título de la página, el JSON-LD de un componente. `Document` los fusiona en un solo conjunto de etiquetas:

1. Parte del `Seo` pasado a `Document::seo` (vacío si no hay ninguno).
2. Recorre el body en orden de documento (el orden en que aparecen las etiquetas en el HTML) y aplica cada `Seo` que encuentra sobre el resultado anterior.
3. Un valor definido en un `Seo` posterior reemplaza al anterior. Los valores que un `Seo` posterior no define se conservan.
4. Las alternativas se reemplazan por idioma, las metaetiquetas adicionales por nombre o propiedad, y los bloques JSON-LD se acumulan.

En un árbol típico, el layout va primero y la página está anidada dentro, así que **gana el valor más profundo y más específico**:

```rust
use active::prelude::*;

fn layout(content: Element) -> Element {
    div()
        .child(Seo::new().description("Solid oak furniture.").noindex(true))
        .child(content)
}

let page = layout(section().child(Seo::new().title("Chairs").noindex(false)));
let html = Document::new(page)
    .seo(Seo::new().site_name("Acme").title_template("%s · Acme"))
    .render();

assert_eq!(html.matches("<title>").count(), 1);
assert!(html.contains("<title>Chairs · Acme</title>"));
assert!(html.contains(r#"content="Solid oak furniture.""#));
assert!(!html.contains("robots")); // the page's noindex(false) wins
```

Dónde busca la fusión:

- en los hijos de cada elemento, en los fragmentos y en las islas;
- en el contenido actual de los hijos dinámicos (`children_dyn`, `fragment_dyn`), así que una página elegida por un enrutador se describe a sí misma como cualquier otra;
- no dentro de las cadenas de HTML sin escapar (`.html(…)`), que para `active` son texto plano.

> **Consejo:** Como gana el último, coloca el `Seo` de un layout antes del contenido que envuelve, como arriba. Un `Seo` añadido después del contenido sobrescribiría los valores de la página.

También puedes fusionar a mano: `a.merge(&b)` aplica `b` sobre `a`, y `a.merged(&b)` devuelve un valor nuevo. `Document::final_seo()` devuelve el resultado fusionado de una página entera, algo práctico en las pruebas.

### En el navegador

- Las **islas** nunca tocan el `<head>`: el servidor ya lo escribió.
- Las **aplicaciones del lado del cliente** lo mantienen actualizado. Cuando se monta una vista con `active::mount_to` o `active::mount`, sus valores `Seo` se fusionan y se escriben en el head; cada vez que se reconstruye una región dinámica (por ejemplo, cuando el enrutador muestra otra página), los valores `Seo` de la región se fusionan sobre los de sus ancestros y el head se actualiza. Pon los valores por defecto del sitio en un `Seo` de la vista raíz. Consulta [Aplicaciones del lado del cliente, Títulos y metaetiquetas](../client-app/#titulos-y-metaetiquetas).
- `active::update_head(&seo)` te permite escribir tú mismo un `Seo` en el head, por ejemplo desde un manejador de eventos. Un `Seo` sin título ni nombre de sitio vacía el título de la página.

## Idiomas y hreflang

Para un sitio en varios idiomas, da a cada versión la misma lista de alternativas, más `x-default` para los visitantes cuyo idioma no ofreces:

```rust
use active::Seo;

fn alternates(seo: Seo, path: &str) -> Seo {
    ["en", "fr", "es"]
        .into_iter()
        .fold(seo, |seo, lang| seo.alternate(lang, format!("/{lang}{path}")))
        .alternate("x-default", format!("/en{path}"))
}

let seo = alternates(Seo::new().base_url("https://acme.com").title("Contact"), "/contact/");
assert!(seo.to_html().contains(r#"hreflang="x-default" href="https://acme.com/en/contact/""#));
```

Define también `Document::lang` con el idioma de la página, y `locale` con el locale de Open Graph correspondiente (`en_US`, `fr_FR`…).

## Páginas que no deben indexarse

Las páginas de error, los resultados de búsqueda y las áreas privadas deberían quedar fuera de los resultados de los buscadores:

```rust
use active::prelude::*;

fn not_found() -> Element {
    section()
        .child(Seo::new().title("Page not found").noindex(true))
        .child(h1().text("Page not found"))
}
```

Devuelve además el código de estado HTTP correcto (404) desde tu servidor; consulta [Axum](../axum/#errores-y-404).

## Usar Seo sin vistas

`Seo` no necesita el resto de la biblioteca. Funciona con cualquier motor de plantillas, framework web o generador estático, en el servidor o en WebAssembly:

```rust
use active::Seo;

let seo = Seo::new().title("Hello").description("A page.");
let page = format!(
    r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>{seo}</head><body>…</body></html>"#
);
assert!(page.contains("<title>Hello</title>"));
```

## Lista de comprobación

- Un título claro por página, de unos 50 a 60 caracteres, y una descripción de 120 a 160 caracteres.
- Una URL canónica y absoluta en cada página (define `base_url`).
- Una imagen para compartir (`image`) de 1200 × 630 píxeles, para que las vistas previas de enlaces se vean bien.
- Alternativas `hreflang` en cada versión de idioma, incluida `x-default`.
- `noindex` en las páginas de error, de búsqueda y privadas.
- Un `sitemap.xml` y un `robots.txt` (consulta [Axum](../axum/) y [Sitios estáticos](../static-site/#sitemap-y-robots-txt)).
- [Datos estructurados](../structured-data/) donde ayuden: organización, migas de pan, artículos, productos.
