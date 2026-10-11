---
title: Datos estructurados
description: Añade datos estructurados JSON-LD a tus páginas con Seo::json_ld: organizaciones, migas de pan, artículos, productos y FAQ, con serde_json o a mano.
---

Los datos estructurados indican a los buscadores de qué trata una página en un formato legible por máquinas: esta página es un producto con un precio, esta otra un artículo de un autor, estos enlaces forman unas migas de pan. Google los usa para los resultados enriquecidos (estrellas, precios, migas de pan, entradas de preguntas frecuentes). El formato estándar es [JSON-LD](https://json-ld.org) con el vocabulario de [schema.org](https://schema.org), en una etiqueta `<script type="application/ld+json">`.

`active` los añade con el mismo builder `Seo` que las demás etiquetas.

## Seo::json_ld

`.json_ld(json)` recibe una cadena de JSON ya serializada y añade una etiqueta `<script type="application/ld+json">` por llamada, después de las demás etiquetas del head:

```rust
use active::Seo;

let seo = Seo::new().json_ld(
    r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}"#,
);
assert!(seo.to_html().ends_with(
    r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Organization","name":"Acme","url":"https://acme.com"}</script>"#
));
```

El JSON se escribe tal cual, con un cambio: cada `<` se convierte en `\u003c`. Sigue siendo el mismo valor JSON, pero una cadena como `</script>` en tus datos nunca puede cerrar la etiqueta antes de tiempo e inyectar HTML. Una cadena vacía no añade nada.

## Construir el JSON con serde_json

La forma más segura de producir el JSON es [`serde_json`](https://docs.rs/serde_json), que escapa correctamente cada valor:

```toml
[dependencies]
serde_json = "1"
```

```rust
use active::Seo;
use serde_json::json;

fn product_seo(name: &str, price: &str, image: &str) -> Seo {
    let data = json!({
        "@context": "https://schema.org",
        "@type": "Product",
        "name": name,
        "image": image,
        "offers": {
            "@type": "Offer",
            "price": price,
            "priceCurrency": "EUR",
            "availability": "https://schema.org/InStock"
        }
    });
    Seo::new().title(name.to_string()).og_type("product").json_ld(data.to_string())
}

let html = product_seo("Oak chair", "249.00", "https://acme.com/oak.jpg").to_html();
assert!(html.contains(r#""@type":"Product""#));
```

`json!` construye un `serde_json::Value`, y `to_string()` lo serializa sin espacios. Con tus propios tipos, deriva `serde::Serialize` y llama a `serde_json::to_string(&value)`.

## Sin dependencias

Para unos pocos campos sirve `format!`, siempre que cada valor de texto se escape como una cadena JSON:

```rust
use active::Seo;

/// `s` como literal de cadena JSON, comillas incluidas.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

let title = r#"Say "hello" to Rust"#;
let seo = Seo::new().json_ld(format!(
    r#"{{"@context":"https://schema.org","@type":"Article","headline":{}}}"#,
    json_string(title)
));
assert!(seo.to_html().contains(r#""headline":"Say \"hello\" to Rust""#));
```

Recuerda duplicar las llaves (`{{` y `}}`) del propio JSON dentro de `format!`.

## Tipos habituales

Cada ejemplo de abajo es un objeto JSON-LD; pásalo a `.json_ld(...)` como se muestra arriba.

### Organización y sitio web

Una vez por sitio, normalmente en el layout o en la página de inicio:

```text
{
  "@context": "https://schema.org",
  "@type": "Organization",
  "name": "Acme",
  "url": "https://acme.com",
  "logo": "https://acme.com/logo.png",
  "sameAs": ["https://acme.example"]
}
```

### Migas de pan

Muestran la posición de la página dentro del sitio en los resultados de búsqueda. Este sitio añade unas a cada página de la documentación:

```text
{
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    { "@type": "ListItem", "position": 1, "name": "Docs", "item": "https://acme.com/docs/" },
    { "@type": "ListItem", "position": 2, "name": "SEO", "item": "https://acme.com/docs/seo/" }
  ]
}
```

### Artículo

Para entradas de blog y noticias. Combínalo con `.og_type("article")`:

```text
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Shipping a Rust website",
  "datePublished": "2026-03-01",
  "author": { "@type": "Person", "name": "Ada" },
  "image": "https://acme.com/posts/rust-site.png"
}
```

### Preguntas frecuentes

```text
{
  "@context": "https://schema.org",
  "@type": "FAQPage",
  "mainEntity": [
    {
      "@type": "Question",
      "name": "Do you ship abroad?",
      "acceptedAnswer": { "@type": "Answer", "text": "Yes, to the whole European Union." }
    }
  ]
}
```

Aquí el JSON se muestra en varias líneas para facilitar la lectura; `serde_json` produce una sola línea, y ambas formas son válidas.

## Varios bloques en una página

Cada llamada a `json_ld` añade su propia etiqueta `<script>`, y cuando [los valores `Seo` se fusionan](../seo/#fusion), los bloques JSON-LD se **acumulan** en lugar de reemplazarse. Un layout puede declarar la organización mientras cada página añade sus migas de pan:

```rust
use active::prelude::*;

let organization = r#"{"@context":"https://schema.org","@type":"Organization","name":"Acme"}"#;
let breadcrumbs = r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#;

let page = div()
    .child(Seo::new().json_ld(organization))
    .child(article().child(Seo::new().title("Chairs").json_ld(breadcrumbs)));

let html = Document::new(page).render();
assert_eq!(html.matches(r#"<script type="application/ld+json">"#).count(), 2);
```

> **Advertencia:** Como los bloques se acumulan, un bloque declarado tanto por el layout como por una página aparece dos veces. Declara cada bloque común a todo el sitio en un solo lugar.

## Comprueba tu marcado

- Prueba una página pública con la [Prueba de resultados enriquecidos](https://search.google.com/test/rich-results) de Google o con el [Schema Markup Validator](https://validator.schema.org).
- En tus propias pruebas, renderiza la página y examina las etiquetas fusionadas con `Document::final_seo()`:

```rust
use active::prelude::*;

let page = main().child(Seo::new().json_ld(r#"{"@type":"Thing"}"#));
let seo = Document::new(page).final_seo();
assert!(seo.to_html().contains(r#"{"@type":"Thing"}"#));
```
