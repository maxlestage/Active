# Example: Vite + Rust + static generation

```sh
npm install
npm run wasm    # crate/ → src/pkg (cargo + wasm-bindgen-cli 0.2.129, or wasm-pack)
npm run dev     # development
npm run build   # dist/: one HTML page per route, <head> filled in, .wasm preloaded
npm run preview
```

- `src/seo.ts`: the site's SEO defaults (`createSeo`).
- `src/wasm.ts`: declares the Rust module; the `?url` import enables the preload.
- `src/App.tsx`: pages using `<Seo>` and `useWasm`.
- `prerender.js`: `renderWithHead` for every route.

Documentation (English, French, Spanish): [`site/docs`](../../site/docs).
