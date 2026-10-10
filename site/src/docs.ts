import type { Lang } from "./routes";
import type { TocItem } from "../plugins/markdown";
// Titles and descriptions of every page, generated from the Markdown frontmatter.
import manifest from "virtual:docs-manifest";

export interface Doc {
  title: string;
  description: string;
  html: string;
  toc: TocItem[];
}

export const CATEGORIES = [
  { id: "start", pages: ["introduction", "installation", "quick-start"] },
  { id: "seo", pages: ["seo-component", "site-defaults", "structured-data", "server-rendering"] },
  { id: "wasm", pages: ["rust-setup", "loading-wasm", "raw-wasm", "performance"] },
  { id: "rust", pages: ["rust-crate"] },
  { id: "ship", pages: ["deploy-heroku", "publishing"] },
  { id: "reference", pages: ["api", "benchmarks", "faq"] },
] as const;
export type CategoryId = (typeof CATEGORIES)[number]["id"];

export const SLUGS: readonly string[] = CATEGORIES.flatMap((c) => c.pages);

/** Title/description of a page, falling back to English when a translation is missing. */
export const meta = (lang: Lang, slug: string): { title: string; description: string } =>
  manifest[lang]?.[slug] ?? manifest.en?.[slug] ?? { title: slug, description: "" };

export const neighbours = (slug: string) => {
  const i = SLUGS.indexOf(slug);
  return { prev: SLUGS[i - 1], next: SLUGS[i + 1] };
};

export const categoryOf = (slug: string) => CATEGORIES.find((c) => (c.pages as readonly string[]).includes(slug))!;

// Lazy on the client (one chunk per page), eager on the server.
const lazy = import.meta.glob<{ default: Doc }>("../docs/*/*.md");
export async function loadDoc(lang: Lang, slug: string): Promise<Doc> {
  const load = lazy[`../docs/${lang}/${slug}.md`] ?? lazy[`../docs/en/${slug}.md`];
  return (await load!()).default;
}
