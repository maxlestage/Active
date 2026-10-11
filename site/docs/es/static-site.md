---
title: Sitios estáticos
description: Genera cada página de un sitio active como archivo HTML al compilar, con sitemap y robots.txt, alójala gratis donde quieras y mantén las islas interactivas.
---

Cuando tus páginas no dependen de la petición (un blog, documentación, un portafolio, una landing page), no necesitas ningún servidor. Un pequeño programa en Rust renderiza cada página a un archivo HTML una sola vez, en tiempo de compilación, y cualquier hosting estático sirve los archivos. Las páginas conservan sus etiquetas SEO completas y las [islas](../islands/) se siguen hidratando en el navegador, porque la hidratación solo necesita el HTML y el bundle.

## Cómo funciona

```text
cargo build --lib --target wasm32-unknown-unknown …   →  pkg/            el bundle de las islas
cargo run --release                                    →  dist/index.html
                                                          dist/posts/hello-rust/index.html
                                                          dist/pkg/…, dist/sitemap.xml, dist/robots.txt
```

La estructura del crate es la habitual ([Instalación](../installation/#un-crate-dos-destinos)): las páginas y las islas en `src/lib.rs`, y `src/main.rs` pasa a ser un generador en lugar de un servidor. No necesita más dependencias que `active` (y `wasm-bindgen` para el navegador).

## Las páginas

```rust
use active::prelude::*;

/// Dirección pública del sitio, sin barra final.
pub const ORIGIN: &str = "https://example.com";
/// Ruta bajo la que se publica el sitio: "" en la raíz de un dominio, "/my-blog" para
/// el sitio de un proyecto en GitHub Pages (https://<user>.github.io/my-blog/).
pub const BASE: &str = "";

pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

pub static POSTS: [Post; 2] = [
    Post { slug: "hello-rust", title: "Hello, Rust", summary: "Why this blog is written in Rust." },
    Post { slug: "islands", title: "Islands", summary: "Interactive parts on a static page." },
];

/// Un enlace dentro del sitio, con la ruta base.
fn url(path: &str) -> String {
    format!("{BASE}{path}")
}

/// Isla: un botón de "me gusta", interactivo una vez cargado el bundle.
pub fn likes(props: &str) -> Node {
    let n = use_state(props.parse::<u32>().unwrap_or(0));
    button().on_click(move |_| n.update(|n| *n += 1)).text_dyn(move || format!("♥ {}", n.get())).into()
}

fn document(content: Element) -> String {
    let body = div()
        .child(header().child(a().href(url("/")).text("My blog")))
        .child(main().child(content));
    Document::new(body)
        .lang("en")
        .seo(
            Seo::new()
                .site_name("My blog")
                .title_template("%s · My blog")
                .base_url(format!("{ORIGIN}{BASE}")),
        )
        .stylesheet(url("/static/app.css"))
        .client(url("/pkg/my_blog.js"), url("/pkg/my_blog_bg.wasm"))
        .render()
}

fn home() -> String {
    document(
        section()
            .child(Seo::new().title("Home").description("Notes about Rust and the web.").canonical("/"))
            .child(h1().text("My blog"))
            .child(ul().children(POSTS.iter().map(|post| {
                li().child(a().href(url(&format!("/posts/{}/", post.slug))).text(post.title))
            }))),
    )
}

fn post_page(post: &Post) -> String {
    document(
        article()
            .child(
                Seo::new()
                    .title(post.title)
                    .description(post.summary)
                    .canonical(format!("/posts/{}/", post.slug))
                    .og_type("article"),
            )
            .child(h1().text(post.title))
            .child(p().text(post.summary))
            .child(island("likes", "0", likes)),
    )
}

/// La página 404 que muestran los hostings estáticos para las URL desconocidas.
pub fn not_found() -> String {
    document(
        section()
            .child(Seo::new().title("Page not found").noindex(true))
            .child(h1().text("Page not found")),
    )
}

/// Todas las páginas del sitio, como (ruta, HTML).
pub fn pages() -> Vec<(String, String)> {
    let mut pages = vec![("/".to_string(), home())];
    for post in &POSTS {
        pages.push((format!("/posts/{}/", post.slug), post_page(post)));
    }
    pages
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("likes", likes)]);
}
```

Cada página se renderiza con su propio `Document`, así que cada una recibe su propio título, su descripción y su URL canónica. Las rutas terminan en `/` y se escriben como archivos `index.html` en una carpeta con el mismo nombre, que cualquier hosting estático sirve en esa URL limpia.

## El generador

`src/main.rs` escribe las páginas, copia el bundle y los archivos estáticos, y añade el sitemap y el `robots.txt`:

```rust
use std::fs;
use std::io;
use std::path::Path;

use my_blog::{BASE, ORIGIN};

fn main() -> io::Result<()> {
    let out = Path::new("dist");
    if out.exists() {
        fs::remove_dir_all(out)?;
    }

    // 1. Un index.html por página.
    let pages = my_blog::pages();
    for (path, html) in &pages {
        let dir = out.join(path.trim_start_matches('/'));
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }
    fs::write(out.join("404.html"), my_blog::not_found())?;

    // 2. El bundle de las islas y los archivos estáticos.
    copy_dir(Path::new("pkg"), &out.join("pkg"))?;
    copy_dir(Path::new("static"), &out.join("static"))?;

    // 3. Sitemap y robots.txt, con URL absolutas.
    let mut sitemap = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for (path, _) in &pages {
        sitemap.push_str(&format!("<url><loc>{ORIGIN}{BASE}{path}</loc></url>"));
    }
    sitemap.push_str("</urlset>\n");
    fs::write(out.join("sitemap.xml"), sitemap)?;
    fs::write(
        out.join("robots.txt"),
        format!("User-agent: *\nAllow: /\n\nSitemap: {ORIGIN}{BASE}/sitemap.xml\n"),
    )?;

    println!("{} pages written to dist/", pages.len());
    Ok(())
}

/// Copia los archivos de `from` (de forma recursiva) en `to`. Si `from` no existe, no hace nada.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(from) else {
        return Ok(());
    };
    fs::create_dir_all(to)?;
    for entry in entries {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
```

## Compilación

Compila primero el bundle y después ejecuta el generador:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
cargo run --release
```

Ahora `dist/` contiene todo el sitio. Para verlo en local, sirve la carpeta con cualquier servidor de archivos estáticos, por ejemplo:

```sh
python3 -m http.server --directory dist 8000
```

y abre [http://localhost:8000](http://localhost:8000). (Abrir los archivos directamente con `file://` no funciona: los navegadores se niegan a cargar módulos WebAssembly desde archivos locales).

Un sitio sin islas no necesita bundle: elimina `.client(...)`, la función `start` y los dos primeros comandos.

## Sitemap y robots.txt

Los buscadores descubren las páginas a través de `sitemap.xml`, que enumera cada URL con su dirección absoluta, y encuentran el sitemap a través de `robots.txt`. El generador anterior escribe ambos a partir de la misma lista de páginas, así que nunca se desincronizan. Envía la dirección del sitemap en [Google Search Console](https://search.google.com/search-console) y en Bing Webmaster Tools para acelerar la indexación.

Para un sitio multilingüe, añade un elemento `<xhtml:link rel="alternate" hreflang="…">` por idioma dentro de cada `<url>`, como hace el servidor de este sitio en `site/src/server/mod.rs`.

## Alojamiento

`dist/` son archivos normales: sirve cualquier hosting estático, a menudo gratis.

### GitHub Pages

1. En el repositorio, abre **Settings**, **Pages**, y en **Source** elige **GitHub Actions**.
2. Define `BASE` como `"/<repository>"` (o `""` con un dominio propio o un repositorio `<user>.github.io`), y `ORIGIN` como `https://<user>.github.io`.
3. Añade este workflow como `.github/workflows/pages.yml`:

```yaml
name: Pages

on:
  push:
    branches: [master]

permissions:
  contents: read
  pages: write
  id-token: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
      - run: |
          cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
          wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_blog.wasm
          cargo run --release
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Cada push a `master` vuelve a compilar y publicar el sitio. También funciona desde la aplicación de GitHub: edita un archivo, haz commit y el sitio se actualiza unos minutos después.

### Netlify, Cloudflare Pages y otros

Genera `dist/` en CI (como arriba) y despliega la carpeta con la action o la herramienta de línea de comandos del hosting, o arrastra y suelta `dist/` en el panel del hosting para un despliegue puntual. Estos hostings sirven `404.html` para las URL desconocidas y definen automáticamente el tipo de contenido `application/wasm`.

## Instalable y sin conexión

Un sitio estático también se puede instalar en la pantalla de inicio y seguir funcionando sin conexión: escribe `manifest.webmanifest` y `sw.js` junto a las páginas con `active::pwa`, y añade `.manifest(...)` y `.service_worker(...)` a cada `Document`. Ver [Apps instalables (PWA)](../pwa/#sitios-estaticos-y-ruta-base).

## Actualizar el contenido

El sitio solo cambia cuando lo vuelves a generar. El contenido suele vivir en el repositorio (datos en Rust como arriba, o archivos Markdown que el generador lee con un crate como [`pulldown-cmark`](https://docs.rs/pulldown-cmark) e inserta con `.html(...)`), así que un commit desencadena una nueva compilación a través de CI. Cuando las páginas deben cambiar en cada petición o para cada visitante, usa un servidor: consulta [Vitesse](../vitesse/).
