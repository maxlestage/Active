// WASM part of the benchmark (called by ../run.mjs).
import { build } from "esbuild";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";
import { chromium } from "playwright-core";

const dir = import.meta.dirname;
const rust = join(dir, "rust");
const pages = join(dir, "pages");
const median = (a) => [...a].sort((x, y) => x - y)[a.length >> 1];
const gz = (file) => gzipSync(readFileSync(file), { level: 9 }).length;

// Cargo profiles (+ optional wasm-opt pass) compared for size and speed.
const PROFILES = [
  { name: 'opt-level = 3', opt: "3" },
  { name: "opt-level = 3 + wasm-opt -O3", opt: "3", wasmOpt: "-O3" },
  { name: 'opt-level = "s"', opt: "s" },
  { name: 'opt-level = "z"', opt: "z" },
  { name: 'opt-level = "z" + wasm-opt -Oz', opt: "z", wasmOpt: "-Oz" },
];

function fibNode(file) {
  const { exports } = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(file)));
  const t = [];
  for (let k = 0; k < 9; k++) {
    const t0 = performance.now();
    exports.fib(30);
    t.push(performance.now() - t0);
  }
  return median(t);
}

export async function runWasm({ out, size, serve, browserOpts }) {
  const w = join(out, "wasm");
  mkdirSync(w, { recursive: true });
  const result = { profiles: [], size: {}, load: {}, calls: {} };

  // 1. Rust → wasm for each profile.
  const built = {};
  for (const opt of new Set(PROFILES.map((p) => p.opt))) {
    const target = join(out, `cargo-${opt}`);
    execFileSync("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown", "--target-dir", target], {
      cwd: rust,
      env: { ...process.env, CARGO_PROFILE_RELEASE_OPT_LEVEL: opt },
      stdio: "inherit",
    });
    const rel = join(target, "wasm32-unknown-unknown/release");
    execFileSync("wasm-bindgen", ["--target", "web", "--out-dir", join(w, `bg-${opt}`), join(rel, "bindgen.wasm")]);
    built[opt] = { raw: join(rel, "raw.wasm"), bg: join(w, `bg-${opt}`, "bindgen_bg.wasm") };
  }
  for (const p of PROFILES) {
    let { raw, bg } = built[p.opt];
    if (p.wasmOpt) {
      const r = join(w, `raw${p.wasmOpt}.wasm`), b = join(w, `bg${p.wasmOpt}.wasm`);
      for (const [src, dst] of [[raw, r], [bg, b]])
        execFileSync(join(dir, "../node_modules/.bin/wasm-opt"), [p.wasmOpt, "--all-features", src, "-o", dst]);
      raw = r;
      bg = b;
    }
    result.profiles.push({
      name: p.name,
      rawBytes: readFileSync(raw).length,
      bindgenBytes: readFileSync(bg).length,
      bindgenGzip: gz(bg),
      fib30Ms: fibNode(raw),
    });
  }
  console.log(result.profiles);

  // Artifacts used by the pages: the default profile (opt-level = 3).
  copyFileSync(built["3"].raw, join(out, "raw.wasm"));
  copyFileSync(built["3"].bg, join(out, "bindgen_bg.wasm"));
  copyFileSync(join(w, "bg-3/bindgen.js"), join(w, "bindgen.js"));

  // 2. Sizes: JS needed to use wasm from React (react/react-dom excluded) + binaries.
  result.size = {
    active: await size(`export { wasm, useWasm } from "active-react";`, "wasm-active"),
    "react-wasm": await size(`export { useWasm } from "react-wasm";`, "wasm-react-wasm"),
    glue: await size(`export * from ${JSON.stringify(join(w, "bindgen.js"))};`, "wasm-glue"),
    raw: { bytes: readFileSync(join(out, "raw.wasm")).length, gzip: gz(join(out, "raw.wasm")) },
    bindgen: { bytes: readFileSync(join(out, "bindgen_bg.wasm")).length, gzip: gz(join(out, "bindgen_bg.wasm")) },
  };

  // 3. Pages.
  const html = (head, body, script) =>
    `<!doctype html><html><head><meta charset="utf-8"><title>Bench</title>${head}</head><body><div id="root">${body}</div><script type="module" src="${script}"></script></body></html>`;
  const LOAD = ["active-raw", "react-wasm", "effect-raw", "active-bindgen", "effect-bindgen"];
  for (const name of [...LOAD, "calls"]) {
    await build({ ...browserOpts, entryPoints: [join(pages, `${name}.tsx`)], outfile: join(out, `w-${name}.js`) });
    writeFileSync(join(out, `w-${name}.html`), html("", "", `/w-${name}.js`));
  }
  for (const preload of [true, false]) {
    const name = preload ? "ssr-preload" : "ssr-nopreload";
    const define = { ...browserOpts.define, PRELOAD: String(preload) };
    await build({ ...browserOpts, define, entryPoints: [join(pages, "ssr-client.tsx")], outfile: join(out, `w-${name}.js`) });
    const server = join(out, `w-${name}-server.mjs`);
    await build({
      ...browserOpts,
      define,
      minify: false,
      platform: "node",
      external: ["react", "react-dom", "react/*", "react-dom/*"],
      entryPoints: [join(pages, "ssr-server.tsx")],
      outfile: server,
    });
    const { head, html: body } = JSON.parse(execFileSync("node", [server], { cwd: dir }).toString());
    writeFileSync(join(out, `w-${name}.html`), html(head, body, `/w-${name}.js`));
    result.load[name] = { head };
  }

  // 4. Browser: time until the wasm result is rendered, on localhost and with 100 ms RTT.
  const srv = await serve(out);
  const origin = `http://127.0.0.1:${srv.address().port}`;
  const browser = await chromium.launch();
  for (const name of [...LOAD, "ssr-preload", "ssr-nopreload"]) {
    const entry = (result.load[name] ??= {});
    for (const [cond, latency] of [["local", 0], ["rtt100", 100]]) {
      const page = await browser.newPage();
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      if (latency) {
        const cdp = await page.context().newCDPSession(page);
        await cdp.send("Network.enable");
        await cdp.send("Network.emulateNetworkConditions", {
          offline: false,
          latency,
          downloadThroughput: -1,
          uploadThroughput: -1,
        });
      }
      const t = [];
      for (let k = 0; k < (latency ? 7 : 15); k++) {
        await page.goto(`${origin}/w-${name}.html`);
        await page.waitForFunction(() => window.readyAt !== undefined);
        t.push(await page.evaluate(() => window.readyAt));
      }
      if (errors.length) throw new Error(`${name}: ${errors[0]}`);
      entry[cond] = median(t);
      await page.close();
    }
    console.log(`load ${name}:`, entry.local.toFixed(1), entry.rtt100.toFixed(1));
  }
  const page = await browser.newPage();
  await page.goto(`${origin}/w-calls.html`);
  result.calls = await page.evaluate(() => window.run());
  console.log(result.calls);
  await browser.close();
  srv.close();
  return result;
}
