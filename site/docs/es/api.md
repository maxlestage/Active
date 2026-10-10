---
title: Referencia de la API
description: Todas las exportaciones de active-react con su firma TypeScript, props, valores por defecto y comportamiento, más un resumen del crate de Rust active.
---

active-react tiene dos puntos de entrada:

| Punto de entrada | Exportaciones |
| --- | --- |
| `active-react` | `Seo`, `createSeo`, `SeoProps`, `wasm`, `useWasm`, `Wasm` |
| `active-react/server` | `renderWithHead` |

## Seo

```ts
function Seo(props: SeoProps): JSX.Element;
```

Renderiza las etiquetas SEO de una página: `<title>`, meta description, robots, canonical, alternativas hreflang, Open Graph, Twitter y JSON-LD. React 19 traslada las etiquetas `<title>`, `<meta>` y `<link>` al `<head>`, en el servidor y en el navegador. El `<script>` JSON-LD se queda donde se renderiza el componente.

No usa ningún hook, ni contexto, ni efecto, así que puede renderizarse en Server Components. Renderiza un solo `<Seo>` por página. Guía: [el componente `<Seo>`](../seo-component/).

## SeoProps

```ts
interface SeoProps {
  title?: string;
  titleTemplate?: string;
  description?: string;
  canonical?: string;
  image?: string;
  type?: string;
  siteName?: string;
  locale?: string;
  twitter?: string;
  baseUrl?: string;
  noindex?: boolean;
  nofollow?: boolean;
  alternates?: Record<string, string>;
  jsonLd?: object | object[];
  meta?: Array<{ name?: string; property?: string; content: string }>;
}
```

| Prop | Valor por defecto | Descripción |
| --- | --- | --- |
| `title` | ninguno | Título de la página. Se le aplica `titleTemplate` para `<title>`; se usa tal cual para `og:title`. Si falta, se usa `siteName` |
| `titleTemplate` | ninguno | Plantilla en la que el primer `%s` se reemplaza por `title`, por ejemplo `"%s · Acme"`. Sin uso si no hay `title` |
| `description` | ninguno | `<meta name="description">` y `og:description` |
| `canonical` | ninguno | `<link rel="canonical">` y `og:url`. Las rutas relativas se resuelven con `baseUrl` |
| `image` | ninguno | `og:image`. Las rutas relativas se resuelven con `baseUrl`. Cambia `twitter:card` a `summary_large_image` |
| `type` | `"website"` | `og:type` |
| `siteName` | ninguno | `og:site_name`, y el título cuando falta `title` |
| `locale` | ninguno | `og:locale`, por ejemplo `"en_US"` |
| `twitter` | ninguno | `twitter:site`, por ejemplo `"@acme"` |
| `baseUrl` | ninguno | Origen (o prefijo) para las URLs relativas de `canonical`, `image` y `alternates`, por ejemplo `"https://acme.com"` |
| `noindex` | `false` | Añade `noindex` a `<meta name="robots">` |
| `nofollow` | `false` | Añade `nofollow` a `<meta name="robots">` |
| `alternates` | ninguno | Alternativas hreflang: `{ en: "/en", fr: "/fr", "x-default": "/en" }` |
| `jsonLd` | ninguno | Datos estructurados JSON-LD, un objeto o un array, serializados con `<` escapado |
| `meta` | ninguno | Etiquetas meta adicionales: `[{ name: "theme-color", content: "#000" }]` |

Siempre se renderizan: `og:type` y `twitter:card` (`summary`, o `summary_large_image` si hay imagen). `<meta name="robots">` solo aparece cuando `noindex` o `nofollow` es true.

Resolución de URLs: las URLs con esquema (`https:`, `mailto:`…) o que empiezan por `//` se conservan; en los demás casos se antepone `baseUrl`, sin su posible barra final, con una `/` entre ambos cuando la ruta no empieza por una.

## createSeo

```ts
function createSeo(defaults: SeoProps): (props: SeoProps) => ReactNode;
```

Devuelve un componente `<Seo>` con valores por defecto para todo el sitio. Las props de la página se fusionan sobre los valores por defecto con un spread superficial (`{ ...defaults, ...props }`): las props de tipo objeto y array reemplazan el valor por defecto, y un `undefined` explícito lo elimina.

```ts
export const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
```

Guía: [valores por defecto del sitio](../site-defaults/).

## wasm

```ts
function wasm<T>(source: () => Promise<T>, options?: { url?: string }): Wasm<T>;

function wasm<T = WebAssembly.Exports>(
  source: string | URL,
  options?: { imports?: WebAssembly.Imports },
): Wasm<T>;
```

Declara un módulo WebAssembly. **No se descarga nada** hasta `load()` o `useWasm()`.

- **Sobrecarga con loader**, para la salida de wasm-bindgen o wasm-pack: `wasm(() => import("./pkg/my_crate.js"), { url })`. Si el módulo tiene una función como exportación por defecto (`--target web`), se llama una vez con `{ module_or_path: url }`, o sin argumento cuando falta `url`. En caso contrario (`--target bundler`, o un objeto simple), el módulo se usa tal cual. `url` es la URL pública del binario `.wasm` y habilita el preload.
- **Sobrecarga con URL**, para un módulo puro sin glue: `wasm<Exports>("/my_crate.wasm", { imports })`. El archivo se descarga y se compila con `WebAssembly.instantiateStreaming` cuando la respuesta tiene el content type `application/wasm`, o con `WebAssembly.instantiate` sobre sus bytes en caso contrario. Una respuesta que no sea 2xx rechaza la promesa con `wasm: <url> responded <status>`. Se resuelve con las exportaciones de la instancia.

Guías: [cargar WebAssembly](../loading-wasm/), [WebAssembly puro](../raw-wasm/).

## Wasm

```ts
interface Wasm<T> {
  /** URL of the `.wasm` binary when known; preloaded from `<head>` by `useWasm`. */
  readonly url?: string;
  /** Starts loading once and resolves with the exports. Calling it again after a failure retries. */
  load(): Promise<T>;
  /** The exports once loaded, `undefined` before; throws if loading failed. */
  get(): T | undefined;
  /** Notifies `listener` when loading settles; starts loading. */
  subscribe(listener: () => void): () => void;
}
```

| Miembro | Comportamiento |
| --- | --- |
| `url` | La opción `url` de la sobrecarga con loader, o la URL que se pasa a la sobrecarga con URL |
| `load()` | Inicia la carga en la primera llamada. Las llamadas posteriores devuelven la misma promesa. Después de un fallo, la siguiente llamada inicia un nuevo intento |
| `get()` | Lectura síncrona: las exportaciones, `undefined` mientras no se ha cargado, o lanza el error de carga |
| `subscribe(listener)` | Llama a `listener` cuando la carga tiene éxito o falla, e inicia la carga si hace falta. Devuelve una función para cancelar la suscripción |

## useWasm

```ts
function useWasm<T>(mod: Wasm<T>): T | undefined;
```

Devuelve las exportaciones del módulo, o `undefined` mientras se carga. Se basa en `useSyncExternalStore`:

- el primer componente que usa un módulo inicia su carga; todos los componentes comparten la misma carga;
- siempre devuelve `undefined` durante el renderizado en servidor y la hidratación, para que el marcado del servidor y el del cliente coincidan;
- cuando la carga falla, el error se lanza durante el renderizado, hacia el error boundary más cercano; llama a `mod.load()` para reintentar;
- mientras el módulo no está cargado y se conoce `mod.url`, llama a `preload(url, { as: "fetch", crossOrigin: "anonymous", type: "application/wasm" })` de React DOM. Durante el renderizado en servidor, esto emite `<link rel="preload" as="fetch" crossorigin type="application/wasm">` en el `<head>`.

Para usar Suspense, usa `use(mod.load())` de React en un componente que solo se renderice en el navegador.

## renderWithHead

```ts
import { renderWithHead } from "active-react/server";

function renderWithHead(node: ReactNode): { head: string; html: string };
```

Para plantillas HTML en las que React no renderiza `<html>` por sí mismo, como Vite SSR con los marcadores `<!--app-head-->` y `<!--app-html-->`, o la generación estática. Renderiza `node` con `renderToString` y divide el resultado:

- `head`: las etiquetas que React trasladó (title, meta, link, preloads), para insertarlas en el `<head>` de la plantilla;
- `html`: el resto del marcado, para insertarlo en el contenedor de la app.

```tsx
const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

Síncrono: los límites de Suspense que todavía están pendientes renderizan su fallback. Si tu árbol renderiza `<html>`, usa directamente `renderToString` o `renderToPipeableStream` de `react-dom/server`. Guía: [renderizado en servidor](../server-rendering/).

## Crate de Rust

El crate `active` ([crates.io](https://crates.io/crates/active)) genera las mismas etiquetas desde Rust:

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head: String = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

| Elemento | Descripción |
| --- | --- |
| `Seo::new()` | Builder vacío; `Seo` también implementa `Default`, `Clone`, `Debug`, `PartialEq`, `Eq` y `Hash` |
| `title`, `title_template`, `description`, `canonical`, `image`, `og_type`, `site_name`, `locale`, `twitter`, `base_url` | Setters que reciben `impl Into<String>` |
| `noindex(bool)`, `nofollow(bool)` | Indicadores de robots |
| `alternate(lang, url)` | Añade o reemplaza una alternativa hreflang |
| `json_ld(json)` | Añade un script JSON-LD a partir de una cadena JSON serializada |
| `meta_name(name, content)`, `meta_property(property, content)` | Etiquetas meta adicionales |
| `to_html(&self) -> String` | Renderiza las etiquetas |
| `write_html(&self, out: &mut String)` | Añade las etiquetas al final de `out` |
| `impl Display` | `format!("{seo}")` renderiza las etiquetas |

Guía: [el crate de Rust](../rust-crate/). Documentación completa: [docs.rs/active](https://docs.rs/active).
