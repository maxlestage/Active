// Benchmark runner: `node run.mjs [seo|wasm]` (both by default).
// Builds every scenario with esbuild, measures bundle size, SSR in Node (one process per
// library) and client behaviour in headless Chromium, then rewrites RESULTS.md.
import { build } from "esbuild";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { cpus } from "node:os";
import { extname, join, resolve } from "node:path";
import { brotliCompressSync, gzipSync, constants } from "node:zlib";
import { chromium } from "playwright-core";

const dir = import.meta.dirname;
const out = join(dir, ".build");
mkdirSync(out, { recursive: true });
mkdirSync(join(dir, "results"), { recursive: true });
const only = process.argv[2];

export const SEO_LIBS = {
  native: { label: "React 19 natif (sans lib)", imports: "" },
  active: { label: "**active-react**", imports: `export { createSeo } from "active-react";` },
  "helmet-async": {
    label: "react-helmet-async 3.0.0",
    imports: `export { Helmet, HelmetProvider } from "react-helmet-async";`,
  },
  unhead: {
    label: "@unhead/react 3.4.2",
    imports: `export { useHead } from "@unhead/react"; export { UnheadProvider, createHead } from "@unhead/react/client";`,
  },
  "react-head": { label: "react-head 3.4.2", imports: `export { HeadProvider, Title, Meta, Link } from "react-head";` },
  helmet: { label: "react-helmet 6.1.0", imports: `export { Helmet } from "react-helmet";` },
};
const SSR_ONLY = { "active-template": { label: "**active-react** (`renderWithHead`, template HTML)" } };

const browserOpts = {
  bundle: true,
  minify: true,
  format: "esm",
  target: "es2022",
  jsx: "automatic",
  define: { "process.env.NODE_ENV": '"production"' },
  // active-react is linked from the repo (file:..): resolve React from bench/ so there is one copy.
  alias: { react: "react", "react-dom": "react-dom" },
  absWorkingDir: dir,
  logLevel: "error",
};
// SSR bundles keep npm packages external (like a real Node server), except the linked library.
const SSR_EXTERNAL = ["react", "react-dom", "react/*", "react-dom/*", "react-helmet", "react-helmet-async", "@unhead/*", "react-head"];
const REACT_EXTERNAL = ["react", "react-dom", "react/*", "react-dom/*", "scheduler"];

export async function size(code, name, external = REACT_EXTERNAL) {
  if (!code) return { min: 0, gzip: 0, brotli: 0 };
  const file = join(out, `size-${name}.ts`);
  writeFileSync(file, code);
  const r = await build({ ...browserOpts, entryPoints: [file], external, write: false });
  const buf = Buffer.from(r.outputFiles[0].contents);
  return {
    min: buf.length,
    gzip: gzipSync(buf, { level: 9 }).length,
    brotli: brotliCompressSync(buf, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length,
  };
}

export function serve(root) {
  const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
  const server = createServer((req, res) => {
    const path = join(root, decodeURIComponent(new URL(req.url, "http://x").pathname));
    if (!path.startsWith(root) || !existsSync(path)) return res.writeHead(404).end();
    res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream", "cache-control": "no-store" });
    res.end(readFileSync(path));
  });
  return new Promise((ok) => server.listen(0, "127.0.0.1", () => ok(server)));
}

const median = (a) => [...a].sort((x, y) => x - y)[a.length >> 1];
const html = (script) =>
  `<!doctype html><html><head><meta charset="utf-8"><title>Bench</title></head><body><div id="root"></div><script type="module" src="${script}"></script></body></html>`;

async function seo() {
  const result = { size: {}, ssr: {}, client: {} };
  const names = Object.keys(SEO_LIBS);

  for (const name of names) result.size[name] = await size(SEO_LIBS[name].imports, name);

  // SSR: one Node process per library so JIT state never leaks between them.
  for (const name of [...names, ...Object.keys(SSR_ONLY)]) {
    const entry = join(out, `ssr-${name}.tsx`);
    writeFileSync(
      entry,
      `import { ssr } from ${JSON.stringify(join(dir, "seo/ssr", name + ".tsx"))};\n` +
        `import { measure } from ${JSON.stringify(join(dir, "seo/measure.ts"))};\nmeasure(ssr);\n`,
    );
    const file = join(out, `ssr-${name}.mjs`);
    await build({
      entryPoints: [entry],
      outfile: file,
      bundle: true,
      platform: "node",
      format: "esm",
      jsx: "automatic",
      external: SSR_EXTERNAL,
      logLevel: "error",
    });
    const run = () =>
      JSON.parse(execFileSync("node", [file], { cwd: dir, env: { ...process.env, NODE_ENV: "production" } }).toString());
    const runs = [run(), run(), run()];
    result.ssr[name] = { ...runs[0], us: median(runs.map((r) => r.us)) };
    result.ssr[name].opsPerSec = 1e6 / result.ssr[name].us;
    console.log(`ssr ${name}: ${result.ssr[name].us.toFixed(1)} µs`, runs[0].check);
  }

  // Client: real Chromium, production React, one fresh page per library.
  for (const name of names) {
    const entry = join(out, `client-${name}.tsx`);
    writeFileSync(
      entry,
      `import { Root } from ${JSON.stringify(join(dir, "seo/libs", name + ".tsx"))};\n` +
        `import { start } from ${JSON.stringify(join(dir, "seo/client.tsx"))};\nstart(Root);\n`,
    );
    await build({ ...browserOpts, entryPoints: [entry], outfile: join(out, `client-${name}.js`) });
    writeFileSync(join(out, `client-${name}.html`), html(`/client-${name}.js`));
  }
  const server = await serve(out);
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  result.browser = browser.version();
  for (const name of names) {
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const ready = [];
    for (let k = 0; k < 15; k++) {
      await page.goto(`${origin}/client-${name}.html`);
      await page.waitForFunction(() => window.readyAt !== undefined || window.readyError);
      const r = await page.evaluate(() => window.readyAt ?? window.readyError);
      if (typeof r !== "number") throw new Error(`${name}: ${r}`);
      ready.push(r);
    }
    await page.evaluate(() => window.bench.throughput(200)); // warm-up
    const through = [];
    for (let k = 0; k < 5; k++) through.push(await page.evaluate(() => window.bench.throughput(1000)));
    const lat = await page.evaluate(() => window.bench.latency(300));
    const audit = await page.evaluate(() => window.bench.audit());
    result.client[name] = {
      readyMs: median(ready),
      navPerSec: 1000 / (median(through) / 1000),
      latencyMedianMs: median(lat),
      latencyP95Ms: lat[Math.floor(lat.length * 0.95)],
      audit,
      errors,
    };
    console.log(`client ${name}:`, JSON.stringify(result.client[name]));
    await page.close();
  }
  await browser.close();
  server.close();
  writeFileSync(join(dir, "results/seo.json"), JSON.stringify(result, null, 2));
}

async function wasm() {
  const { runWasm } = await import("./wasm/run.mjs");
  writeFileSync(join(dir, "results/wasm.json"), JSON.stringify(await runWasm({ out, size, serve, browserOpts }), null, 2));
}

if (!only || only === "seo") await seo();
if (!only || only === "wasm") await wasm();

const { report } = await import("./report.mjs");
writeFileSync(
  join(dir, "RESULTS.md"),
  report({
    seo: existsSync(join(dir, "results/seo.json")) && JSON.parse(readFileSync(join(dir, "results/seo.json"))),
    wasm: existsSync(join(dir, "results/wasm.json")) && JSON.parse(readFileSync(join(dir, "results/wasm.json"))),
    env: { node: process.version, cpu: cpus()[0].model, cores: cpus().length },
    labels: { ...SEO_LIBS, ...SSR_ONLY },
  }),
);
console.log("→ RESULTS.md");
