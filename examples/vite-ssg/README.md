# Exemple : Vite + Rust + génération statique

```sh
npm install
npm run wasm    # crate/ → src/pkg (cargo + wasm-bindgen-cli 0.2.129, ou wasm-pack)
npm run dev     # développement
npm run build   # dist/ : une page HTML par route, <head> rempli, preload du .wasm
npm run preview
```

- `src/seo.ts` : valeurs SEO du site (`createSeo`).
- `src/wasm.ts` : déclaration du module Rust ; l'URL `?url` active le preload.
- `src/App.tsx` : pages avec `<Seo>` et `useWasm`.
- `prerender.js` : `renderWithHead` pour chaque route.
