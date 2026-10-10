// Production server for Heroku (or any Node host): serves the prerendered site from dist/.
// No dependency. Picks the language on "/", fills in the site origin, compresses and caches.
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const DIST = fileURLToPath(new URL("./dist/", import.meta.url));
const LANGS = ["en", "fr", "es"];
const SITE_URL = process.env.SITE_URL?.replace(/\/$/, "");
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
  ".wasm": "application/wasm",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
};
const COMPRESSIBLE = new Set([".html", ".js", ".css", ".json", ".xml", ".txt", ".svg", ".wasm"]);
const TEMPLATED = new Set([".html", ".xml", ".txt"]); // contain the "__ORIGIN__" placeholder
const cache = new Map();

/** Origin used in canonical/Open Graph URLs: SITE_URL, else the request's own host. */
function originOf(req) {
  if (SITE_URL) return SITE_URL;
  const host = String(req.headers["x-forwarded-host"] ?? req.headers.host ?? "localhost").split(",")[0].trim();
  if (!/^[a-z0-9.-]+(:\d+)?$/i.test(host)) return "http://localhost";
  const proto = String(req.headers["x-forwarded-proto"] ?? "http").split(",")[0].trim();
  return `${proto === "https" ? "https" : "http"}://${host}`;
}

/** Preferred supported language from Accept-Language, English by default. */
export function pickLang(header = "") {
  const ranked = header
    .split(",")
    .map((part) => {
      const [tag, q] = part.trim().split(";q=");
      return { lang: tag.slice(0, 2).toLowerCase(), q: q === undefined ? 1 : Number(q) };
    })
    .filter((x) => LANGS.includes(x.lang) && x.q > 0)
    .sort((a, b) => b.q - a.q);
  return ranked[0]?.lang ?? "en";
}

async function exists(file) {
  try {
    return await stat(file);
  } catch {
    return undefined;
  }
}

/** File content for a request: origin filled in, compressed once, cached. */
async function load(file, origin, encoding) {
  const ext = extname(file);
  const key = `${file}|${TEMPLATED.has(ext) ? origin : ""}|${encoding}`;
  let entry = cache.get(key);
  if (!entry) {
    let body = await readFile(file);
    if (TEMPLATED.has(ext)) body = Buffer.from(body.toString("utf8").replaceAll("__ORIGIN__", origin));
    const etag = `"${createHash("sha1").update(body).digest("base64url").slice(0, 20)}"`;
    if (encoding === "br") body = brotliCompressSync(body, { params: { [constants.BROTLI_PARAM_QUALITY]: 10 } });
    else if (encoding === "gzip") body = gzipSync(body, { level: 9 });
    entry = { body, etag };
    if (cache.size > 2000) cache.clear();
    cache.set(key, entry);
  }
  return entry;
}

export async function handler(req, res) {
  const security = {
    "x-content-type-options": "nosniff",
    "referrer-policy": "strict-origin-when-cross-origin",
    "x-frame-options": "SAMEORIGIN",
  };
  if (req.headers["x-forwarded-proto"] === "https") security["strict-transport-security"] = "max-age=31536000";
  if (req.method !== "GET" && req.method !== "HEAD") {
    res.writeHead(405, { allow: "GET, HEAD", ...security }).end();
    return;
  }

  const url = new URL(req.url ?? "/", "http://local");
  let pathname;
  try {
    pathname = decodeURIComponent(url.pathname);
  } catch {
    pathname = "/";
  }
  pathname = normalize(pathname).replace(/\\/g, "/");

  if (pathname === "/") {
    res.writeHead(302, { location: `/${pickLang(req.headers["accept-language"])}/`, vary: "Accept-Language", "cache-control": "no-cache", ...security });
    res.end();
    return;
  }

  let file = join(DIST, pathname.endsWith("/") ? `${pathname}index.html` : pathname);
  let status = 200;
  let info = file.startsWith(DIST) ? await exists(file) : undefined;
  if (info?.isDirectory()) {
    // "/en/docs" → "/en/docs/"
    res.writeHead(301, { location: `${pathname}/${url.search}`, ...security }).end();
    return;
  }
  if (!info) {
    const first = pathname.split("/")[1];
    const lang = LANGS.includes(first) ? first : pickLang(req.headers["accept-language"]);
    file = join(DIST, lang, "404.html");
    status = 404;
    info = await exists(file);
    if (!info) {
      res.writeHead(404, { "content-type": "text/plain", ...security }).end("Not found");
      return;
    }
  }

  const ext = extname(file);
  const accept = String(req.headers["accept-encoding"] ?? "");
  const encoding = !COMPRESSIBLE.has(ext) ? "" : /\bbr\b/.test(accept) ? "br" : /\bgzip\b/.test(accept) ? "gzip" : "";
  const { body, etag } = await load(file, originOf(req), encoding);
  const headers = {
    "content-type": TYPES[ext] ?? "application/octet-stream",
    "cache-control": pathname.startsWith("/assets/")
      ? "public, max-age=31536000, immutable"
      : ext === ".html"
        ? "no-cache"
        : "public, max-age=3600",
    etag,
    vary: "Accept-Encoding",
    ...(encoding ? { "content-encoding": encoding } : {}),
    ...security,
  };
  if (status === 200 && req.headers["if-none-match"] === etag) {
    res.writeHead(304, headers).end();
    return;
  }
  headers["content-length"] = body.length;
  res.writeHead(status, headers);
  res.end(req.method === "HEAD" ? undefined : body);
}

export const createApp = () =>
  createServer((req, res) =>
    handler(req, res).catch((error) => {
      console.error(error);
      if (!res.headersSent) res.writeHead(500, { "content-type": "text/plain" });
      res.end("Internal error");
    }),
  );

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.env.PORT) || 3000;
  createApp().listen(port, () => console.log(`active site on http://localhost:${port}`));
}
