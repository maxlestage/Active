# active-react

[English](README.md) · [Français](README.fr.md) · **Español**

**SEO + Rust/WebAssembly para React 19, en ~1,3 kB gzip.** Cero dependencias, cero efectos. Funciona con SSR, generación estática y Server Components. Un gemelo en Rust, el crate [`active`](crates/active), genera las mismas etiquetas desde Rust.

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="Acme's website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

📚 **Documentación completa (inglés, francés y español):** el sitio de presentación en [`site/`](site). Despliega tu propia copia con el botón [Deploy to Heroku](#desplegar-el-sitio-en-heroku-desde-un-teléfono), o lee las páginas directamente en [`site/docs/`](site/docs).

## Por qué es rápido

El diseño se basa en los benchmarks. Los detalles y el método están en [`bench/RESULTS.md`](bench/RESULTS.md).

- **SEO sin gestor de `<head>`.** Desde React 19, los elementos `<title>`, `<meta>` y `<link>` renderizados en cualquier parte son trasladados al `<head>` por el propio React, en el servidor y en el navegador. `<Seo>` solo renderiza las etiquetas correctas: ni contexto, ni store, ni efecto, ni segunda pasada. El costo se queda a pocos puntos porcentuales de las etiquetas escritas a mano, y el SSR es 1,5× más rápido que con react-helmet.
- **WASM: un único fetch, compartido y precargado.** `wasm()` declara un módulo sin descargar nada. `useWasm()` lo carga una sola vez (`instantiateStreaming`), sin importar cuántos componentes lo usen. Cuando se conoce la URL del `.wasm`, el renderizado en servidor emite `<link rel="preload">`, así que el navegador descarga el binario **en paralelo** con el JavaScript. Con 100 ms de latencia de red, el primer resultado de Rust llega **un 32 % antes**.

| SEO (misma página: 11 etiquetas + JSON-LD) | JS del cliente (gzip) | SSR (µs/página) | Latencia de navegación (mediana) |
| --- | ---: | ---: | ---: |
| React 19, sin biblioteca (referencia) | 0 | 39,7 | 0,3 ms |
| **active-react** | **0,74 kB** | **42,4** | **0,3 ms** |
| @unhead/react 3.4.2 | 6,07 kB | 44,9 | 0,4 ms |
| react-head 3.4.2 | 1,35 kB | 60,7 | 1,4 ms |
| react-helmet-async 3.0.0 | 6,29 kB | 64,9 | 0,3 ms |
| react-helmet 6.1.0 | 6,76 kB | 66,5 | 16,7 ms |

| WASM (crate con wasm-bindgen) | JS (gzip) | Resultado de Rust en pantalla, RTT de 100 ms |
| --- | ---: | ---: |
| **active-react**, SSR + preload automático | **0,60 kB** | **224 ms** |
| active-react, SSR sin preload | 0,60 kB | 330 ms |
| active-react, renderizado en el cliente | 0,60 kB | 328 ms |
| react-wasm 1.0.1, renderizado en el cliente (`.wasm` puro) | 2,07 kB | 327 ms |
| `useEffect` escrito a mano, renderizado en el cliente | 0 | 333 ms |

Intel Xeon 2,8 GHz, Node 22, Chromium 141, React 19.3. Las cifras de SSR varían en torno a ±5 % entre ejecuciones.

## Instalación

```sh
npm i github:maxlestage/Active   # until the first npm release, then: npm i active-react
cargo add active                 # optional: the Rust crate, once published
```

Peer dependencies: `react >= 19`, `react-dom >= 19`.

## SEO

```tsx
// seo.ts — your site's defaults, written once
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com", // relative URLs (canonical, image, alternates) are resolved against it
  image: "/og.png",
  locale: "en_US",
  twitter: "@acme",
});
```

```tsx
// any page
<Seo
  title="Oak chair"
  description="Solid oak, delivered in 48 hours."
  canonical="/products/oak-chair"
  type="product"
  alternates={{ en: "/products/oak-chair", fr: "/fr/produits/chaise-chene" }}
  jsonLd={{ "@context": "https://schema.org", "@type": "Product", name: "Oak chair" }}
/>
```

Esto renderiza `<title>`, la descripción, los enlaces canonical y hreflang, robots y las etiquetas Open Graph y Twitter. El JSON-LD se escapa contra la inyección de `</script>`. `<Seo>` no usa ningún hook, así que también funciona en Server Components. Renderiza un solo `<Seo>` por página.

**Renderizado en servidor.** Si React renderiza `<html>` (React Router, TanStack Start, Waku, un servidor propio), no hay nada que hacer. Para las plantillas HTML (Vite SSR, generación estática), usa `active-react/server`:

```tsx
import { renderWithHead } from "active-react/server";

const { head, html } = renderWithHead(<App url={url} />);
const page = template.replace("<!--app-head-->", head).replace("<!--app-html-->", html);
```

## WebAssembly (Rust)

```sh
wasm-pack build --target web --out-dir ../src/pkg
```

```tsx
import { wasm, useWasm } from "active-react";
import url from "./pkg/my_crate_bg.wasm?url"; // Vite; optional, enables the SSR preload

export const myCrate = wasm(() => import("./pkg/my_crate.js"), { url });

function Stats({ data }: { data: Float64Array }) {
  const rust = useWasm(myCrate);
  if (!rust) return <Spinner />;
  return <p>Mean: {rust.mean(data)}</p>; // typed from the generated .d.ts
}
```

- `useWasm(mod)` devuelve las exportaciones, o `undefined` mientras se carga. Siempre devuelve `undefined` durante el SSR y la hidratación, para que el HTML del servidor y el del cliente coincidan.
- Un error de carga se lanza al error boundary más cercano, y `mod.load()` reintenta la carga.
- Para usar Suspense, llama a `use(mod.load())` en un componente cliente. Fuera de React, `await mod.load()`.
- Un módulo `extern "C"` puro funciona sin ningún glue: `wasm<{ add(a: number, b: number): number }>("/math.wasm")`.

Perfil de Cargo recomendado (validado por los benchmarks): `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.

## El crate de Rust `active`

```rust
use active::Seo;

let site = Seo::new().site_name("Acme").title_template("%s · Acme").base_url("https://acme.com");
let head = site.clone().title("Contact").description("Write to us.").canonical("/contact").to_html();
```

Produce las mismas etiquetas que `<Seo>`, para Axum, Actix, generadores estáticos o WebAssembly. No tiene ninguna dependencia. Consulta [`crates/active`](crates/active).

## Repositorio

| Carpeta | Contenido |
| --- | --- |
| `src/`, `test/` | La biblioteca `active-react` y sus tests (vitest) |
| `crates/active/` | El crate de Rust `active` (crates.io) |
| `site/` | Sitio de presentación + documentación (en/fr/es): Vite, React 19, active-react, hero y playground con Rust/WASM |
| `bench/` | Benchmarks frente a react-helmet, react-helmet-async, @unhead/react, react-head y react-wasm |
| `examples/vite-ssg/` | Ejemplo mínimo con Vite + Rust + generación estática |

## Desplegar el sitio en Heroku desde un teléfono

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/maxlestage/Active)

1. Abre este README en tu teléfono y toca **Deploy to Heroku**. Inicia sesión, elige un nombre para la app y después toca **Deploy app**.
2. Para volver a desplegar con cada cambio: en el panel de Heroku, abre **Deploy → GitHub**, conecta el repositorio y después activa **Enable Automatic Deploys** desde `master`.

Heroku no tiene plan gratuito. Un dyno Eco cuesta unos 5 USD al mes. El build solo usa Node: el módulo WASM del sitio está incluido en el repositorio. Guía paso a paso: `site/docs/es/deploy-heroku.md`.

## Publicar en npm y crates.io desde un teléfono

1. Añade los secretos del repositorio `NPM_TOKEN` y `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).
2. Abre **Releases → Draft a new release**, crea una etiqueta como `v0.1.0` y después toca **Publish release**.

El workflow `release.yml` toma la versión de la etiqueta, ejecuta los tests y publica el paquete `active-react` y el crate `active`. Guía: `site/docs/es/publishing.md`.

## Desarrollo

```sh
npm install && npm test && npm run build      # library
cd crates/active && cargo test                # Rust crate
cd site && npm install && npm run dev         # site (npm run build && npm start for production)
cd bench && npm run setup && npm run bench    # benchmarks (needs Rust, wasm-bindgen-cli, Chromium)
```

Cada rama `claude/*` que pasa la CI se fusiona automáticamente en `master` (`.github/workflows/ci.yml`).

## Licencia

[MIT](LICENSE)
