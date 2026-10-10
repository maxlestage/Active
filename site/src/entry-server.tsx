import { renderWithHead } from "active-react/server";
import { App } from "./App";
import { CATEGORIES, SLUGS, type Doc } from "./docs";
import { t } from "./i18n";
import { LANGS, type Lang, type Route } from "./routes";

const docs = import.meta.glob<{ default: Doc }>("../docs/*/*.md", { eager: true });
const doc = (lang: Lang, slug: string) => (docs[`../docs/${lang}/${slug}.md`] ?? docs[`../docs/en/${slug}.md`])!.default;

/** Every prerendered page. */
export const routes: Route[] = LANGS.flatMap((lang) => [
  { lang, page: "home" as const },
  { lang, page: "docs" as const },
  ...SLUGS.map((slug) => ({ lang, page: "doc" as const, slug })),
  { lang, page: "404" as const },
]);

export const render = (route: Route) =>
  renderWithHead(<App route={route} doc={route.page === "doc" ? doc(route.lang, route.slug) : undefined} />);

/** Search index per language: pages and their headings. */
export const searchIndex = (lang: Lang) =>
  CATEGORIES.flatMap((c) =>
    c.pages.map((slug) => {
      const d = doc(lang, slug);
      return { s: slug, t: d.title, d: d.description, c: t(lang).docs.categories[c.id].title, h: d.toc.map((h) => [h.id, h.text]) };
    }),
  );

export { href } from "./routes";
export { LANGS };
