// Static generation: one HTML file per route and language, plus sitemap, robots and search index.
// URLs use the "__ORIGIN__" placeholder, replaced by server.js at request time — or here when
// SITE_URL is set (for static hosts).
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { LANGS, href, render, routes, searchIndex } from "./dist-server/entry-server.js";

const SITE_URL = process.env.SITE_URL?.replace(/\/$/, "");
const template = readFileSync("dist/index.html", "utf8");
const finish = (s) => (SITE_URL ? s.replaceAll("__ORIGIN__", SITE_URL) : s);
const write = (file, content) => {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, finish(content));
};

for (const route of routes) {
  const { head, html } = render(route);
  const page = template
    .replace('lang="__LANG__"', `lang="${route.lang}"`)
    .replace("<!--app-head-->", head)
    .replace("<!--app-html-->", html);
  write(route.page === "404" ? `dist/${route.lang}/404.html` : `dist${href(route)}index.html`, page);
}

// "/" picks a language (server.js does it with Accept-Language; this file serves static hosts).
write(
  "dist/index.html",
  `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>active</title>` +
    LANGS.map((l) => `<link rel="alternate" hreflang="${l}" href="__ORIGIN__/${l}/">`).join("") +
    `<link rel="alternate" hreflang="x-default" href="__ORIGIN__/en/">` +
    `<script>var l=(navigator.languages||[navigator.language]).map(function(x){return(x||"").slice(0,2)}).filter(function(x){return["en","fr","es"].indexOf(x)>=0})[0]||"en";location.replace("/"+l+"/")</script>` +
    `<noscript><meta http-equiv="refresh" content="0;url=/en/"></noscript></head>` +
    `<body style="background:#0b0b0c;color:#ede9e3;font-family:system-ui"><p>${LANGS.map((l) => `<a style="color:inherit" href="/${l}/">${l}</a>`).join(" · ")}</p></body></html>`,
);

const pages = routes.filter((r) => r.page !== "404");
write(
  "dist/sitemap.xml",
  `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n` +
    pages
      .map(
        (r) =>
          `<url><loc>__ORIGIN__${href(r)}</loc>` +
          LANGS.map((l) => `<xhtml:link rel="alternate" hreflang="${l}" href="__ORIGIN__${href({ ...r, lang: l })}"/>`).join("") +
          `<xhtml:link rel="alternate" hreflang="x-default" href="__ORIGIN__${href({ ...r, lang: "en" })}"/></url>`,
      )
      .join("\n") +
    `\n</urlset>\n`,
);
write("dist/robots.txt", `User-agent: *\nAllow: /\n\nSitemap: __ORIGIN__/sitemap.xml\n`);
for (const lang of LANGS) write(`dist/search-${lang}.json`, JSON.stringify(searchIndex(lang)));

rmSync("dist-server", { recursive: true });
console.log(`prerendered ${routes.length} pages in ${LANGS.length} languages`);
