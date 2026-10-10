export const LANGS = ["en", "fr", "es"] as const;
export type Lang = (typeof LANGS)[number];

export type Route =
  | { lang: Lang; page: "home" }
  | { lang: Lang; page: "docs" }
  | { lang: Lang; page: "doc"; slug: string }
  | { lang: Lang; page: "404" };

export const isLang = (s: string | undefined): s is Lang => LANGS.includes(s as Lang);

/** `/fr/docs/seo-component/` → { lang: "fr", page: "doc", slug: "seo-component" }. */
export function parse(pathname: string, slugs: readonly string[]): Route {
  const [first, second, third, ...rest] = pathname.split("/").filter(Boolean);
  const lang = isLang(first) ? first : "en";
  if (!isLang(first)) return { lang, page: "404" };
  if (second === undefined) return { lang, page: "home" };
  if (second === "docs" && third === undefined) return { lang, page: "docs" };
  if (second === "docs" && third && !rest.length && slugs.includes(third)) return { lang, page: "doc", slug: third };
  return { lang, page: "404" };
}

export function href(route: Route): string {
  switch (route.page) {
    case "home":
      return `/${route.lang}/`;
    case "docs":
      return `/${route.lang}/docs/`;
    case "doc":
      return `/${route.lang}/docs/${route.slug}/`;
    case "404":
      return `/${route.lang}/404/`;
  }
}

/** Same page in another language. */
export const translate = (route: Route, lang: Lang): string => href({ ...route, lang } as Route);
