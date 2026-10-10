// Smoke tests of the production server against the built site (run `npm run build` first).
import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { createApp, pickLang } from "../server.js";

let base;
let server;
before(async () => {
  server = createApp();
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  base = `http://127.0.0.1:${server.address().port}`;
});
after(() => server.close());

const get = (path, headers = {}) => fetch(base + path, { headers, redirect: "manual" });

test("Accept-Language picks the language", () => {
  assert.equal(pickLang("fr-FR,fr;q=0.9,en;q=0.8"), "fr");
  assert.equal(pickLang("de-DE,es;q=0.5"), "es");
  assert.equal(pickLang("en;q=0.1,es;q=0.9"), "es");
  assert.equal(pickLang(""), "en");
});

test("/ redirects to the visitor's language", async () => {
  const res = await get("/", { "accept-language": "es-ES,es;q=0.9" });
  assert.equal(res.status, 302);
  assert.equal(res.headers.get("location"), "/es/");
});

for (const lang of ["en", "fr", "es"]) {
  test(`home page in ${lang} has complete SEO tags`, async () => {
    const res = await get(`/${lang}/`);
    assert.equal(res.status, 200);
    const html = await res.text();
    assert.match(html, new RegExp(`<html lang="${lang}">`));
    assert.match(html, /<title>active — [^<]+<\/title>/);
    assert.match(html, new RegExp(`<link rel="canonical" href="${base}/${lang}/"/>`));
    for (const l of ["en", "fr", "es", "x-default"]) assert.match(html, new RegExp(`hrefLang="${l}"`));
    assert.match(html, /<meta property="og:image" content="http:\/\/127\.0\.0\.1:\d+\/og\.png"\/>/);
    assert.match(html, /<link rel="preload" href="\/assets\/active_site_bg-[^"]+\.wasm" as="fetch"/);
    assert.ok(!html.includes("__ORIGIN__"));
  });
}

test("docs pages are prerendered with their content", async () => {
  const res = await get("/fr/docs/seo-component/");
  assert.equal(res.status, 200);
  const html = await res.text();
  assert.match(html, /<h1 class="doc-title">/);
  assert.match(html, /"@type":"BreadcrumbList"/);
  assert.match(html, /<meta property="og:type" content="article"\/>/);
});

test("directories get a trailing slash", async () => {
  const res = await get("/en/docs?x=1");
  assert.equal(res.status, 301);
  assert.equal(res.headers.get("location"), "/en/docs/?x=1");
});

test("unknown pages answer 404 in the right language, with noindex", async () => {
  const res = await get("/es/nope");
  assert.equal(res.status, 404);
  const html = await res.text();
  assert.match(html, /<html lang="es">/);
  assert.match(html, /<meta name="robots" content="noindex,follow"\/>/);
});

test("assets are compressed and cached forever", async () => {
  const page = await (await get("/en/")).text();
  const wasm = /href="(\/assets\/active_site_bg-[^"]+\.wasm)"/.exec(page)[1];
  const res = await get(wasm, { "accept-encoding": "br, gzip" });
  assert.equal(res.status, 200);
  assert.equal(res.headers.get("content-type"), "application/wasm");
  assert.equal(res.headers.get("content-encoding"), "br");
  assert.match(res.headers.get("cache-control"), /immutable/);
});

test("HTML is revalidated with ETag", async () => {
  const first = await get("/en/docs/");
  const etag = first.headers.get("etag");
  assert.ok(etag);
  const second = await get("/en/docs/", { "if-none-match": etag });
  assert.equal(second.status, 304);
});

test("sitemap and robots use absolute URLs", async () => {
  const sitemap = await (await get("/sitemap.xml")).text();
  assert.match(sitemap, new RegExp(`<loc>${base}/fr/docs/api/</loc>`));
  assert.match(sitemap, /hreflang="x-default"/);
  const robots = await (await get("/robots.txt")).text();
  assert.match(robots, new RegExp(`Sitemap: ${base}/sitemap.xml`));
});

test("search index lists every docs page", async () => {
  const index = await (await get("/search-en.json")).json();
  assert.equal(index.length, 17);
  assert.ok(index.every((e) => e.t && e.s));
});

test("path traversal stays inside dist", async () => {
  const res = await get("/..%2f..%2fpackage.json");
  assert.equal(res.status, 404);
});
