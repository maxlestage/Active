---
title: Apps instalables (PWA)
description: Haz que un sitio active se pueda instalar y usar sin conexión con active::pwa: el manifiesto de la app web, el service worker y su estrategia de caché, los métodos de Document, servir y exportar, iconos y pruebas sin conexión.
---

Una Progressive Web App es un sitio web que los teléfonos y los ordenadores pueden instalar: tiene un icono en la pantalla de inicio, se abre a pantalla completa sin la barra del navegador y sigue funcionando cuando no hay red. Convertir un sitio active en una solo requiere dos archivos y unas pocas etiquetas, todo escrito en Rust por el módulo `active::pwa`:

- el **manifiesto de la app web** (`manifest.webmanifest`): el nombre, los iconos y los colores de la app instalada, escrito por `Manifest`;
- el **service worker** (`sw.js`): un pequeño script que el navegador ejecuta junto a las páginas, que guarda el sitio en el dispositivo y responde desde ahí cuando no hay conexión, generado por `ServiceWorker`;
- en cada página, `Document::manifest` y `Document::service_worker` añaden el enlace al manifiesto, el color del tema, el título y el icono de la pantalla de inicio, y la línea que registra el worker.

No hay que escribir JavaScript: los navegadores solo ejecutan service workers en forma de script, así que active genera el worker a partir de tu configuración en Rust. Este sitio es una PWA hecha así: instálalo desde el menú de tu navegador, o activa el modo avión y recarga la página. La app `hello` de la carpeta `examples` del repositorio es la más pequeña: una isla contador en una página, servida solo con la biblioteca estándar, instalable y sin conexión en cuarenta líneas.

Los service workers solo funcionan en un contexto seguro: HTTPS, o `http://localhost` mientras desarrollas.

## El manifiesto

```rust
use active::pwa::Manifest;

fn manifest() -> Manifest {
    Manifest::new("Acme")
        .short_name("Acme")
        .description("Las recetas de la cocina Acme, incluso sin conexión.")
        .start_url("/")
        .scope("/")
        .id("/")
        .theme_color("#6d4aff")
        .background_color("#0b0b10")
        .lang("es")
        .categories(["food"])
        .icon("/icon-192.png", "192x192", "image/png")
        .icon("/icon-512.png", "512x512", "image/png")
        .maskable_icon("/icon-maskable-512.png", "512x512", "image/png")
        .icon("/favicon.svg", "any", "image/svg+xml")
}

let json = manifest().to_json();
assert!(json.starts_with(r#"{"name":"Acme","short_name":"Acme""#));
assert!(json.contains(r#""start_url":"/","scope":"/","display":"standalone""#));
```

| Método | Efecto |
| --- | --- |
| `Manifest::new(name)` | un manifiesto llamado `name`, que se abre en `./` en modo `standalone` (pantalla completa, sin barra del navegador) |
| `.short_name(name)` | el nombre bajo el icono; con 12 caracteres como máximo queda bien |
| `.description(text)` | lo que hace la app |
| `.start_url(url)` | la página que abre el icono (`./` por defecto: relativa al manifiesto) |
| `.scope(url)` | las URL que forman parte de la app (por defecto la carpeta del manifiesto) |
| `.id(url)` | la identidad de la app (por defecto `start_url`); no la cambies una vez publicada |
| `.display(mode)` | `standalone` (por defecto), `fullscreen`, `minimal-ui` o `browser` |
| `.orientation(value)` | `portrait`, `landscape`, `any`… |
| `.theme_color(color)` | el color de la barra de estado y del título de la ventana (`#000000` por defecto) |
| `.background_color(color)` | el color de la pantalla de inicio mientras arranca la app (`#ffffff` por defecto) |
| `.lang(code)` | el idioma de los textos: `es`, `en`, `es-MX`… |
| `.categories(list)` | categorías de las tiendas: `productivity`, `education`… |
| `.icon(src, sizes, mime)` | un icono: `"192x192"`, `"512x512"`, o `"any"` para un SVG |
| `.maskable_icon(src, sizes, mime)` | un icono dibujado para máscaras redondas o redondeadas (ver [Iconos](#iconos)) |
| `.to_json()` | el manifiesto en JSON, para servirlo en `/manifest.webmanifest` |

`name()`, `theme()` e `icons()` devuelven los valores. Todas las cadenas se escapan, así que los nombres y descripciones pueden contener comillas o `<`.

La URL de inicio debe ser una página que responda **200** y que el worker guarde: una redirección no se puede reproducir desde la caché, así que una app que arranque en un `/` que redirige se abriría en la página sin conexión. Por eso este sitio arranca en `/en/`: su `/` redirige al idioma del visitante.

## El service worker

```rust
use active::pwa::ServiceWorker;

fn worker() -> ServiceWorker {
    ServiceWorker::new("acme", "2026-10-10.1")
        .precache([
            "/",
            "/static/app.css",
            "/pkg/acme.js",
            "/pkg/acme_bg.wasm",
            "/icon-192.png",
            "/icon-512.png",
        ])
        .offline_page("/offline/")
        .network_only("/api/")
}

let js = worker().to_js();
assert!(js.contains(r#"const CACHE = "acme-2026-10-10.1";"#));
assert!(js.contains(r#"const OFFLINE = "/offline/";"#));
assert!(js.contains(r#"const NETWORK_ONLY = ["/api/"];"#));
```

| Método | Efecto |
| --- | --- |
| `ServiceWorker::new(name, version)` | un worker cuya caché se llama `{name}-{version}` |
| `.precache(urls)` | las URL que se guardan al instalar el worker, disponibles sin conexión desde la primera visita; llámalo tantas veces como haga falta |
| `.offline_page(url)` | la página que se muestra cuando una página nunca se guardó y no hay red; también se guarda de antemano |
| `.network_only(prefix)` | las URL cuya ruta empieza por `prefix` siempre van a la red (una API, un feed en directo…) |
| `.cache_name()` | `{name}-{version}` |
| `.to_js()` | el script, para servirlo en la raíz del sitio |

### Estrategia

El worker generado atiende las peticiones `GET` de su propio sitio y deja todo lo demás al navegador (los formularios que hacen `POST`, otros sitios):

- **Las páginas** (navegaciones) vienen **primero de la red**, así que un visitante con conexión siempre ve la última versión, y cada página recibida se guarda. Sin conexión se muestra la copia guardada, o si no, la página sin conexión.
- **Los demás archivos** del sitio (hoja de estilos, bundle de WebAssembly, imágenes, fuentes, datos que pide la página) vienen **al instante de la caché**, y se actualizan en segundo plano para la próxima visita. Un archivo que aún no está guardado viene de la red y se guarda.
- **Los prefijos `network_only`** nunca se guardan en caché. El prefijo se compara con la ruta de la URL, así que incluye la ruta base del sitio si tiene una.
- **Una nueva versión** (otra cadena `version`) se instala, descarga su lista de archivos, toma el control de las páginas abiertas de inmediato y borra las cachés de versiones anteriores con el mismo `name`.

### Qué guardar de antemano

Guarda lo que un visitante necesita para abrir la app sin conexión: la página de inicio y las páginas principales, la hoja de estilos, el bundle de WebAssembly (el cargador JavaScript y el archivo `.wasm`), las fuentes y los iconos. Usa **exactamente las URL que usan las páginas**: la caché tiene en cuenta la query string, así que si una página carga `/static/app.css?v=3f2a…`, guarda esa URL y no `/static/app.css`. Deja fuera los archivos que no existen: una sola descarga fallida cancela la instalación. Una URL dada dos veces se guarda una sola vez.

### Elegir la versión

Cambia `version` en cada despliegue que modifique algo que el worker guarda; si no, los navegadores conservan el worker antiguo y sus archivos antiguos. `active::pwa::version` lo hace automáticamente: un hash FNV-1a corto de lo que le des, el mismo que este sitio usa para sus URL con `?v=` (ver [Documentos, Caché a largo plazo](../documents/#cache-a-largo-plazo)):

```rust
use active::pwa::version;

let v1 = version([b"body{color:red}".as_slice(), b"\0asm".as_slice()]);
assert_eq!(v1.len(), 10);
assert_ne!(v1, version([b"body{color:blue}".as_slice(), b"\0asm".as_slice()]));
```

Calcula el hash de lo que guarda el worker: las páginas guardadas tal como se renderizan, el manifiesto y los archivos estáticos. Es exactamente lo que hace este sitio: cualquier cambio publica un worker nuevo.

## Las etiquetas de cada página

```rust
use active::pwa::Manifest;
use active::prelude::*;

let manifest = Manifest::new("Acme")
    .short_name("Acme")
    .theme_color("#6d4aff")
    .icon("/icon-192.png", "192x192", "image/png")
    .icon("/icon-512.png", "512x512", "image/png");
let html = Document::new(h1().text("Recetas"))
    .manifest("/manifest.webmanifest", &manifest)
    .service_worker("/sw.js")
    .render();
assert!(html.contains(r#"<link rel="manifest" href="/manifest.webmanifest"/>"#));
assert!(html.contains(r##"<meta name="theme-color" content="#6d4aff"/>"##));
assert!(html.contains(r#"<link rel="apple-touch-icon" href="/icon-512.png"/>"#));
assert!(html.contains(r#"navigator.serviceWorker.register("/sw.js")"#));
```

`.manifest(href, &manifest)` escribe en el `<head>`:

- `<link rel="manifest">` hacia `href`, donde sirves `manifest.to_json()`;
- `<meta name="theme-color">` con el color del tema del manifiesto (no escribas el tuyo);
- `mobile-web-app-capable`, `apple-mobile-web-app-capable` y `apple-mobile-web-app-title` (el nombre corto, o si no, el nombre), para los navegadores que leen estas etiquetas en lugar del manifiesto;
- `<link rel="apple-touch-icon">` con el icono PNG más grande de tipo `any`: iOS lo usa para la pantalla de inicio e ignora los iconos del manifiesto.

`.service_worker(url)` añade un pequeño script en línea que registra el worker cuando la página ha terminado de cargar, para no retrasar nunca la primera visualización:

```html
<script>if("serviceWorker"in navigator)addEventListener("load",()=>navigator.serviceWorker.register("/sw.js"))</script>
```

Llama a los dos en **todas** las páginas, para que la app se pueda instalar desde cualquiera. Son atajos de [`.head(...)`](../documents/#el-builder); la URL se escapa de forma segura.

### Content-Security-Policy

Si tu servidor envía una Content-Security-Policy sin `'unsafe-inline'`, autoriza el script de registro por su hash: calcula el SHA-256 de `active::pwa::register_script(url)` (el texto exacto entre las etiquetas `<script>`), codifícalo en base64 y añade `'sha256-…'` a `script-src`, como haces con otros scripts en línea. El worker y el manifiesto son archivos de tu propio sitio: mantén `'self'` en `script-src` (o `worker-src`) y en `default-src` (o `manifest-src`).

## Servir los archivos

El manifiesto debe servirse como `application/manifest+json` y el worker como JavaScript. Sirve el worker con `Cache-Control: no-cache`, para que los navegadores vean una versión nueva en cuanto se despliega. Con [Vitesse](../vitesse/):

```rust
use active::pwa::{Manifest, ServiceWorker};
use vitesse::prelude::*;

fn manifest() -> Manifest {
    Manifest::new("Acme").start_url("/").icon("/icon-512.png", "512x512", "image/png")
}

fn worker() -> ServiceWorker {
    ServiceWorker::new("acme", env!("CARGO_PKG_VERSION")).precache(["/", "/icon-512.png"])
}

async fn manifest_file(_: Request) -> Response {
    res::send(manifest().to_json())
        .header("content-type", "application/manifest+json")
        .header("cache-control", "no-cache")
}

async fn worker_file(_: Request) -> Response {
    res::send(worker().to_js())
        .header("content-type", "text/javascript; charset=utf-8")
        .header("cache-control", "no-cache")
}

let mut app = App::new();
app.get("/manifest.webmanifest", manifest_file)
    .get("/sw.js", worker_file);
```

Un worker controla las páginas que están bajo su propia carpeta: sirve `sw.js` en la raíz del sitio (`/sw.js`), no en `/static/` ni en `/pkg/`, o solo atendería las URL de esa carpeta.

## Sitios estáticos y ruta base

Un [sitio estático](../static-site/) escribe los dos archivos junto a sus páginas:

```rust
use std::path::Path;

use active::pwa::{Manifest, ServiceWorker};

/// "" en la raíz de un dominio, "/my-blog" para un sitio de proyecto en GitHub Pages.
const BASE: &str = "/my-blog";

fn write_pwa(dist: &Path) -> std::io::Result<()> {
    let manifest = Manifest::new("Mi blog")
        .start_url(format!("{BASE}/"))
        .scope(format!("{BASE}/"))
        .id(format!("{BASE}/"))
        .icon(format!("{BASE}/icon-512.png"), "512x512", "image/png");
    let worker = ServiceWorker::new("my-blog", "1")
        .precache([format!("{BASE}/"), format!("{BASE}/app.css")])
        .offline_page(format!("{BASE}/offline/"));
    std::fs::write(dist.join("manifest.webmanifest"), manifest.to_json())?;
    std::fs::write(dist.join("sw.js"), worker.to_js())
}
```

Todas las URL del manifiesto, del worker y de las etiquetas de las páginas (`.manifest(&format!("{BASE}/manifest.webmanifest"), …)`, `.service_worker(&format!("{BASE}/sw.js"))`) llevan la ruta base. Escrito en la raíz de la exportación, `sw.js` se sirve en `/my-blog/sw.js`, así que su alcance es `/my-blog/`. GitHub Pages sirve los dos archivos con los tipos correctos, por HTTPS (ver [GitHub Pages](../deploy-pages/)).

## Iconos

Los teléfonos piden al menos un PNG de **192×192** y otro de **512×512**. Añade también un icono **maskable** de 512×512: Android recorta los iconos en círculos o en cuadrados redondeados, así que un icono maskable llena todo el cuadrado con el color de fondo y mantiene el logo dentro del círculo central, el 80 % del lado. Conserva el logo SVG como icono adicional de tamaño `any` para los ordenadores. El icono PNG más grande de tipo `any` se convierte en el icono de Apple.

Dibuja los PNG una sola vez a partir de tu logo SVG con cualquier renderizador de SVG (resvg, Inkscape, o un navegador sin interfaz que haga una captura del SVG en una página del tamaño exacto) y guárdalos en el repositorio con el sitio: no hay nada que ejecutar al compilar. Los iconos de este sitio se generan así a partir de `site/static/favicon.svg`.

## Probar sin conexión

- **En Chrome**, abre DevTools → **Application**: *Manifest* muestra el manifiesto leído, sus iconos y los errores de instalación; *Service workers* el estado del worker, con **Update on reload** para el desarrollo; *Cache storage* la caché `{name}-{version}` y las URL que contiene. Después marca **Offline** en la pestaña *Network*, recarga y abre una página que nunca hayas visitado: verás la página sin conexión.
- **De forma automática**, con un navegador sin interfaz como Playwright: carga una página, espera a `navigator.serviceWorker.ready`, luego **detén el servidor** y navega: las páginas guardadas de antemano y las ya visitadas se muestran, con sus islas; cualquier otra página muestra la página sin conexión. Detener el servidor es más fiel que la emulación sin conexión del navegador, que puede dejar pasar las peticiones del propio worker.
- **En las pruebas de Rust**, comprueba lo que sirves: las URL del manifiesto y la lista del worker.

```rust
use active::pwa::ServiceWorker;

let js = ServiceWorker::new("acme", "1")
    .precache(["/", "/static/app.css?v=3f2a"])
    .offline_page("/offline/")
    .to_js();
assert!(js.contains(r#"const PRECACHE = ["/","/static/app.css?v=3f2a","/offline/"];"#));
```

## Este sitio

El sitio que estás leyendo hace todo esto, en `site/src/server/pwa.rs`:

- `/manifest.webmanifest` abre la app en `/en/`, con los iconos PNG de 192, 512 y maskable, y el logo SVG;
- `/sw.js` guarda de antemano las tres páginas de inicio, la hoja de estilos y el bundle de WebAssembly con sus URL con `?v=`, las fuentes y los iconos; su versión es un hash de todo ello;
- `/offline/` es la página sin conexión, en los tres idiomas porque el worker solo tiene una;
- cada página llama a `.manifest(...)` y `.service_worker(...)`, y la exportación estática para GitHub Pages escribe `manifest.webmanifest`, `sw.js` y `offline/index.html` con `/Active` en cada URL.
