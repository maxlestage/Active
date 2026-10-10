// Static site generation: one HTML file per route, with <head> filled for crawlers.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { render, urls } from "./dist-server/entry-server.js";

const template = readFileSync("dist/index.html", "utf8");
for (const url of urls) {
  const { head, html } = render(url);
  const file = `dist${url}index.html`;
  mkdirSync(file.slice(0, file.lastIndexOf("/")), { recursive: true });
  writeFileSync(file, template.replace("<!--app-head-->", head).replace("<!--app-html-->", html));
  console.log("prerendered", file);
}
rmSync("dist-server", { recursive: true });
