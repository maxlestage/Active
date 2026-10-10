import type { ReactNode } from "react";

export interface SeoProps {
  /** Page title. `titleTemplate` is applied to it; falls back to `siteName`. */
  title?: string;
  /** Title template, `%s` is replaced by `title` (e.g. `"%s · Acme"`). */
  titleTemplate?: string;
  description?: string;
  /** Canonical URL (also og:url). Relative paths are resolved against `baseUrl`. */
  canonical?: string;
  /** Share image (og:image). Relative paths are resolved against `baseUrl`. */
  image?: string;
  /** og:type, defaults to `"website"`. */
  type?: string;
  /** og:site_name, also the fallback title. */
  siteName?: string;
  /** og:locale, e.g. `"fr_FR"`. */
  locale?: string;
  /** Twitter/X handle, e.g. `"@acme"`. */
  twitter?: string;
  /** Origin used to resolve relative URLs, e.g. `"https://acme.com"`. */
  baseUrl?: string;
  noindex?: boolean;
  nofollow?: boolean;
  /** hreflang alternates: `{ en: "/en", fr: "/fr" }`. */
  alternates?: Record<string, string>;
  /** JSON-LD structured data: one object or a list. */
  jsonLd?: object | object[];
  /** Extra meta tags (escape hatch). */
  meta?: Array<{ name?: string; property?: string; content: string }>;
}

const ABSOLUTE = /^[a-z][a-z\d+.-]*:|^\/\//i;

function abs(url: string | undefined, base: string | undefined): string | undefined {
  if (!url || !base || ABSOLUTE.test(url)) return url;
  if (base.endsWith("/")) base = base.slice(0, -1);
  return url[0] === "/" ? base + url : base + "/" + url;
}

/**
 * Renders title, meta, canonical, Open Graph, Twitter and JSON-LD tags.
 * React 19 hoists them into `<head>` (server and client): no hook, no effect, no store,
 * so it also works in Server Components. Render one `<Seo>` per page.
 */
export function Seo(p: SeoProps) {
  const base = p.baseUrl;
  const name = p.title ?? p.siteName;
  const title = p.title && p.titleTemplate ? p.titleTemplate.replace("%s", p.title) : name;
  const url = abs(p.canonical, base);
  const image = abs(p.image, base);
  const robots = p.noindex || p.nofollow;

  return (
    <>
      {title && <title>{title}</title>}
      {p.description && <meta name="description" content={p.description} />}
      {robots && <meta name="robots" content={(p.noindex ? "noindex" : "index") + "," + (p.nofollow ? "nofollow" : "follow")} />}
      {url && <link rel="canonical" href={url} />}
      {p.alternates &&
        Object.keys(p.alternates).map((lang) => (
          <link key={lang} rel="alternate" hrefLang={lang} href={abs(p.alternates![lang], base)} />
        ))}
      {name && <meta property="og:title" content={name} />}
      {p.description && <meta property="og:description" content={p.description} />}
      <meta property="og:type" content={p.type ?? "website"} />
      {url && <meta property="og:url" content={url} />}
      {image && <meta property="og:image" content={image} />}
      {p.siteName && <meta property="og:site_name" content={p.siteName} />}
      {p.locale && <meta property="og:locale" content={p.locale} />}
      <meta name="twitter:card" content={image ? "summary_large_image" : "summary"} />
      {p.twitter && <meta name="twitter:site" content={p.twitter} />}
      {p.meta?.map((m, i) => <meta key={i} {...m} />)}
      {p.jsonLd && (
        <script
          type="application/ld+json"
          // `<` is escaped so the payload can never close the script tag.
          dangerouslySetInnerHTML={{ __html: JSON.stringify(p.jsonLd).replace(/</g, "\\u003c") }}
        />
      )}
    </>
  );
}

/** Binds site-wide defaults: `export const Seo = createSeo({ siteName, titleTemplate, baseUrl })`. */
export function createSeo(defaults: SeoProps): (props: SeoProps) => ReactNode {
  return function SiteSeo(props) {
    return Seo({ ...defaults, ...props });
  };
}
