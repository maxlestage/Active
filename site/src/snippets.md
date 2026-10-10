```tsx
// src/seo.ts — your site, declared once
import { createSeo } from "active-react";

export const Seo = createSeo({
  siteName: "Acme",
  titleTemplate: "%s · Acme",
  baseUrl: "https://acme.com",
  image: "/og.png",
});

// any page — React 19 hoists every tag into <head>
<Seo
  title="Oak chair"
  description="Solid oak, built to last a lifetime."
  canonical="/products/oak-chair"
  jsonLd={{ "@type": "Product", name: "Oak chair" }}
/>
```

```tsx
import { wasm, useWasm } from "active-react";
import url from "./pkg/stats_bg.wasm?url";

// Declared once, fetched on first use, preloaded by SSR.
const stats = wasm(() => import("./pkg/stats.js"), { url });

function Average({ data }: { data: Float64Array }) {
  const rust = useWasm(stats); // undefined while loading
  if (!rust) return <Spinner />;
  return <p>{rust.mean(data)}</p>; // typed from Rust
}
```

```tsx
// React renders <html>? Nothing to do: tags land in <head>.
// HTML template (Vite SSR, static generation)?
import { renderWithHead } from "active-react/server";

const { head, html } = renderWithHead(<App url={url} />);

const page = template
  .replace("<!--app-head-->", head)
  .replace("<!--app-html-->", html);
```

```rust
// Cargo.toml: active = "0.1" — the same tags, from Rust.
use active::Seo;

let site = Seo::new()
    .site_name("Acme")
    .title_template("%s · Acme")
    .base_url("https://acme.com");

let head = site.clone()
    .title("Oak chair")
    .description("Solid oak, built to last a lifetime.")
    .canonical("/products/oak-chair")
    .to_html();
```
