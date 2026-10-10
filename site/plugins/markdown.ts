// Vite plugin: `import doc from "./page.md"` → { title, description, html, toc, blocks }.
// Markdown is rendered at build time (marked + shiki), so no parser ships to the browser.
import { Marked, Renderer, type Tokens } from "marked";
import { createHighlighter, type Highlighter } from "shiki";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { Plugin } from "vite";

export interface TocItem {
  depth: number;
  id: string;
  text: string;
}

const LANGS = ["tsx", "ts", "js", "jsx", "rust", "toml", "sh", "bash", "html", "json", "text", "css", "yaml"];
let highlighter: Promise<Highlighter> | undefined;
const getHighlighter = () => (highlighter ??= createHighlighter({ themes: ["vesper"], langs: LANGS }));

export const slugify = (s: string) =>
  s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/<[^>]+>/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

const escape = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

function frontmatter(src: string): { data: Record<string, string>; body: string } {
  const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(src);
  if (!m) return { data: {}, body: src };
  const data: Record<string, string> = {};
  for (const line of m[1]!.split(/\r?\n/)) {
    const i = line.indexOf(":");
    if (i > 0) data[line.slice(0, i).trim()] = line.slice(i + 1).trim().replace(/^["']|["']$/g, "");
  }
  return { data, body: src.slice(m[0].length) };
}

export async function renderMarkdown(src: string) {
  const { data, body } = frontmatter(src);
  const hl = await getHighlighter();
  const toc: TocItem[] = [];
  const blocks: string[] = [];
  const seen = new Map<string, number>();
  const marked = new Marked({ gfm: true });

  const code = (text: string, lang = "text") => {
    const l = LANGS.includes(lang) ? lang : "text";
    const html = hl.codeToHtml(text, { lang: l, theme: "vesper" });
    blocks.push(html);
    const label = { sh: "terminal", bash: "terminal", text: "" }[l] ?? l;
    return `<div class="code" data-lang="${label}"><button class="copy" type="button" aria-label="Copy">Copy</button>${html}</div>`;
  };

  marked.use({
    renderer: {
      heading({ tokens, depth, text }: Tokens.Heading) {
        const inner = this.parser.parseInline(tokens);
        const plain = text.replace(/`/g, "");
        let id = slugify(plain) || "section";
        const n = seen.get(id) ?? 0;
        seen.set(id, n + 1);
        if (n) id += `-${n}`;
        if (depth === 2 || depth === 3) toc.push({ depth, id, text: plain });
        return `<h${depth} id="${id}"><a class="anchor" href="#${id}" aria-hidden="true" tabindex="-1">#</a>${inner}</h${depth}>`;
      },
      code({ text, lang }: Tokens.Code) {
        return code(text, (lang ?? "").split(/\s/)[0] || "text");
      },
      table(token: Tokens.Table) {
        return `<div class="table">${Renderer.prototype.table.call(this, token)}</div>`;
      },
      blockquote({ tokens }: Tokens.Blockquote) {
        const inner = this.parser.parse(tokens);
        const kind = /^<p><strong>(Note|Tip|Warning|Remarque|Astuce|Attention|Nota|Consejo|Advertencia)\b/i.exec(inner)?.[1]?.toLowerCase() ?? "note";
        const cls = { tip: "tip", astuce: "tip", consejo: "tip", warning: "warning", attention: "warning", advertencia: "warning" }[kind] ?? "note";
        return `<blockquote class="callout ${cls}">${inner}</blockquote>`;
      },
      link({ href, title, tokens }: Tokens.Link) {
        const inner = this.parser.parseInline(tokens);
        const external = /^https?:\/\//.test(href);
        return `<a href="${escape(href)}"${title ? ` title="${escape(title)}"` : ""}${external ? ' target="_blank" rel="noopener noreferrer"' : ""}>${inner}</a>`;
      },
    },
  });

  const html = await marked.parse(body);
  return { title: data.title ?? "", description: data.description ?? "", html, toc, blocks };
}

/** { lang: { slug: { title, description } } } read from every docs/<lang>/<slug>.md frontmatter. */
export function manifest(dir: string) {
  const out: Record<string, Record<string, { title: string; description: string }>> = {};
  for (const lang of readdirSync(dir)) {
    for (const file of readdirSync(join(dir, lang)).filter((f) => f.endsWith(".md"))) {
      const { data } = frontmatter(readFileSync(join(dir, lang, file), "utf8"));
      (out[lang] ??= {})[file.slice(0, -3)] = { title: data.title ?? "", description: data.description ?? "" };
    }
  }
  return out;
}

const VIRTUAL = "virtual:docs-manifest";

export default function markdown(docsDir: string): Plugin {
  return {
    name: "active-markdown",
    enforce: "pre",
    resolveId: (id) => (id === VIRTUAL ? "\0" + VIRTUAL : undefined),
    load(id) {
      if (id === "\0" + VIRTUAL) return `export default ${JSON.stringify(manifest(docsDir))};`;
    },
    configureServer(server) {
      // Dev: rebuild the manifest when a docs page changes.
      server.watcher.add(docsDir);
      server.watcher.on("all", (_event, file) => {
        const mod = file.startsWith(docsDir) && server.moduleGraph.getModuleById("\0" + VIRTUAL);
        if (mod) server.moduleGraph.invalidateModule(mod);
      });
    },
    async transform(src, id) {
      if (!id.endsWith(".md")) return;
      return { code: `export default ${JSON.stringify(await renderMarkdown(src))};`, map: null };
    },
  };
}
