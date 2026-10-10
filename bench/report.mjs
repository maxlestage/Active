// Turns results/*.json into RESULTS.md.
const f = (n, d = 1) => n.toLocaleString("fr-FR", { minimumFractionDigits: d, maximumFractionDigits: d });
const kb = (n) => (n ? `${f(n / 1000, 2)} kB` : "0");
const row = (cells) => `| ${cells.join(" | ")} |`;
const table = (head, rows) =>
  [row(head), row(head.map((_, i) => (i ? "---:" : "---"))), ...rows.map(row)].join("\n");

const WASM_LABELS = {
  "active-raw": "active-react `wasm(\"/x.wasm\")` (brut)",
  "react-wasm": "react-wasm 1.0.1 (brut)",
  "effect-raw": "`useEffect` écrit à la main (brut)",
  "active-bindgen": "active-react + wasm-bindgen",
  "effect-bindgen": "`useEffect` écrit à la main + wasm-bindgen",
  "ssr-preload": "SSR + hydratation, **avec** preload auto (`url`)",
  "ssr-nopreload": "SSR + hydratation, sans preload",
};

export function report({ seo, wasm, env, labels }) {
  const out = [
    "# Résultats des benchmarks",
    "",
    "Générés par `npm run bench` (voir `bench/README.md`). Toutes les libs sont rendues en mode production, " +
      "avec exactement la même page : `<title>` avec template, description, canonical, 6 balises Open Graph, " +
      "2 balises Twitter, un JSON-LD `Product` et un corps de 30 éléments.",
    "",
    `Machine : ${env.cpu} (${env.cores} cœurs), Node ${env.node}${seo ? `, Chromium ${seo.browser}` : ""}.`,
    "",
  ];

  if (seo) {
    const names = Object.keys(seo.size);
    const native = seo.ssr.native.us;
    out.push(
      "## SEO",
      "",
      "### Poids côté client",
      "",
      "JS ajouté au bundle pour les imports réellement utilisés (minifié, react/react-dom exclus).",
      "",
      table(
        ["Lib", "min", "gzip", "brotli"],
        names.map((n) => [labels[n].label, kb(seo.size[n].min), kb(seo.size[n].gzip), kb(seo.size[n].brotli)]),
      ),
      "",
      "### Rendu serveur (Node)",
      "",
      "Temps pour produire le document HTML complet (head + body), médiane de 3 processus × ~2,5 s chacun.",
      "",
      table(
        ["Lib", "µs / page", "pages / s", "vs React natif"],
        Object.entries(seo.ssr)
          .sort((a, b) => a[1].us - b[1].us)
          .map(([n, r]) => [labels[n].label, f(r.us), f(r.opsPerSec, 0), `${r.us <= native ? "" : "+"}${f((r.us / native - 1) * 100, 0)} %`]),
      ),
      "",
      "### Navigateur (Chromium headless)",
      "",
      "- **Head prêt** : du début de la navigation jusqu'à ce que title/description/canonical soient corrects (médiane de 15 chargements).",
      "- **Latence** : entre le rendu d'une nouvelle page (changement de route) et le `<head>` à jour (300 navigations).",
      "- **Rafale** : 1000 changements de route d'affilée, jusqu'au `<head>` final correct.",
      "",
      table(
        ["Lib", "Head prêt", "Latence médiane", "Latence p95", "Rafale (nav/s)", "`<head>` final"],
        names.map((n) => {
          const c = seo.client[n];
          const a = c.audit;
          const dup = a.description > 1 || a.canonical > 1 || a.ogTitle > 1;
          return [
            labels[n].label,
            `${f(c.readyMs)} ms`,
            `${f(c.latencyMedianMs)} ms`,
            `${f(c.latencyP95Ms)} ms`,
            f(c.navPerSec, 0),
            a.correct && !dup ? "✅ correct" : "❌",
          ];
        }),
      ),
      "",
      "> Les approches basées sur le hoisting natif de React 19 (natif, active-react, react-helmet-async 3, react-head) " +
        "laissent le `<title>` statique de `index.html` en place : React insère le sien **avant**, donc `document.title` est correct. " +
        "Supprimez le `<title>` statique de `index.html` si vous voulez un seul élément.",
      "",
    );
  }

  if (wasm) {
    out.push(
      "## WASM (Rust)",
      "",
      "### Profils Cargo",
      "",
      "Même crate (`bench/wasm/rust`), `lto = true`, `codegen-units = 1`, `panic = \"abort\"`, `strip = true`. " +
        "`fib(30)` récursif mesuré dans Node (V8).",
      "",
      table(
        ["Profil", "wasm brut", "wasm-bindgen", "wasm-bindgen gzip", "fib(30)"],
        wasm.profiles.map((p) => [p.name, `${p.rawBytes} o`, kb(p.bindgenBytes), kb(p.bindgenGzip), `${f(p.fib30Ms, 2)} ms`]),
      ),
      "",
      "### Poids",
      "",
      table(
        ["Élément", "min", "gzip"],
        [
          ["**active-react** `wasm` + `useWasm`", kb(wasm.size.active.min), kb(wasm.size.active.gzip)],
          ["react-wasm 1.0.1 `useWasm`", kb(wasm.size["react-wasm"].min), kb(wasm.size["react-wasm"].gzip)],
          ["Glue JS wasm-bindgen (3 fonctions)", kb(wasm.size.glue.min), kb(wasm.size.glue.gzip)],
          ["Binaire brut `extern \"C\"` (add + fib)", `${wasm.size.raw.bytes} o`, `${wasm.size.raw.gzip} o`],
          ["Binaire wasm-bindgen (add + fib + greet)", kb(wasm.size.bindgen.bytes), kb(wasm.size.bindgen.gzip)],
        ],
      ),
      "",
      "### Temps jusqu'au résultat WASM affiché",
      "",
      "Du début de la navigation jusqu'au rendu du résultat d'un appel Rust (médiane). " +
        "« RTT 100 ms » émule une latence réseau réaliste (mobile/4G) via Chrome DevTools Protocol.",
      "",
      table(
        ["Scénario", "localhost", "RTT 100 ms"],
        Object.entries(WASM_LABELS).map(([n, label]) => [label, `${f(wasm.load[n].local)} ms`, `${f(wasm.load[n].rtt100)} ms`]),
      ),
      "",
      "### Coût des appels JS → WASM (Chromium)",
      "",
      table(
        ["Mesure", "JS pur", "wasm brut", "wasm-bindgen"],
        [
          ["`add(a, b)` par appel", `${f(wasm.calls.addNs.js, 2)} ns`, `${f(wasm.calls.addNs.raw, 2)} ns`, `${f(wasm.calls.addNs.bindgen, 2)} ns`],
          ["`fib(30)` récursif", `${f(wasm.calls.fib30Ms.js)} ms`, `${f(wasm.calls.fib30Ms.raw)} ms`, `${f(wasm.calls.fib30Ms.bindgen)} ms`],
          ["`greet(str) -> String` par appel", `${f(wasm.calls.greetNs.js)} ns`, "—", `${f(wasm.calls.greetNs.bindgen)} ns`],
        ],
      ),
      "",
      "> `performance.now()` est arrondi à 0,1 ms dans Chromium hors isolation cross-origin : les petites durées sont approximatives.",
      "",
    );
  }
  return out.join("\n");
}
