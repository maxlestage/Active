import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
import { Seo, createSeo } from "../src/index.js";
import { renderWithHead } from "../src/server.js";

const doc = (node: React.ReactNode) =>
  renderToString(
    <html>
      <head />
      <body>{node}</body>
    </html>,
  );
const head = (html: string) => html.slice(html.indexOf("<head>") + 6, html.indexOf("</head>"));

describe("Seo (SSR)", () => {
  it("hoists every tag into <head>", () => {
    const html = doc(
      <main>
        <Seo
          title="Produit"
          description="Un super produit"
          canonical="https://acme.com/p"
          image="https://acme.com/p.png"
          type="product"
          locale="fr_FR"
          twitter="@acme"
          siteName="Acme"
        />
      </main>,
    );
    expect(head(html)).toBe(
      "<title>Produit</title>" +
        '<meta name="description" content="Un super produit"/>' +
        '<link rel="canonical" href="https://acme.com/p"/>' +
        '<meta property="og:title" content="Produit"/>' +
        '<meta property="og:description" content="Un super produit"/>' +
        '<meta property="og:type" content="product"/>' +
        '<meta property="og:url" content="https://acme.com/p"/>' +
        '<meta property="og:image" content="https://acme.com/p.png"/>' +
        '<meta property="og:site_name" content="Acme"/>' +
        '<meta property="og:locale" content="fr_FR"/>' +
        '<meta name="twitter:card" content="summary_large_image"/>' +
        '<meta name="twitter:site" content="@acme"/>',
    );
    expect(html).toContain("<main></main>");
  });

  it("merges createSeo defaults, applies the template and resolves URLs", () => {
    const SiteSeo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com/", image: "/og.png" });
    const html = doc(
      <SiteSeo twitter="@acme" title="Contact" canonical="/contact" alternates={{ en: "/en/contact", fr: "contact" }} />,
    );
    const h = head(html);
    expect(h).toContain("<title>Contact · Acme</title>");
    expect(h).toContain('<meta property="og:title" content="Contact"/>');
    expect(h).toContain('<link rel="canonical" href="https://acme.com/contact"/>');
    expect(h).toContain('<link rel="alternate" hrefLang="en" href="https://acme.com/en/contact"/>');
    expect(h).toContain('<link rel="alternate" hrefLang="fr" href="https://acme.com/contact"/>');
    expect(h).toContain('<meta property="og:image" content="https://acme.com/og.png"/>');
    expect(h).toContain('<meta name="twitter:site" content="@acme"/>');
  });

  it("falls back to siteName and lets a page override defaults", () => {
    const SiteSeo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", noindex: true });
    const h = head(doc(<SiteSeo noindex={false} nofollow />));
    expect(h).toContain("<title>Acme</title>");
    expect(h).toContain('<meta name="robots" content="index,nofollow"/>');
  });

  it("inserts the title literally into the template", () => {
    expect(head(doc(<Seo title="Save $& and $1" titleTemplate="%s · Acme" />))).toContain("<title>Save $&amp; and $1 · Acme</title>");
  });

  it("keeps absolute and protocol-relative URLs untouched", () => {
    const h = head(doc(<Seo baseUrl="https://acme.com" image="//cdn.acme.com/a.png" canonical="https://x.dev/" />));
    expect(h).toContain('content="//cdn.acme.com/a.png"');
    expect(h).toContain('href="https://x.dev/"');
  });

  it("serialises JSON-LD safely", () => {
    const html = doc(<Seo jsonLd={{ "@type": "Thing", name: "</script><script>alert(1)</script>" }} />);
    expect(html).toContain(
      '<script type="application/ld+json">{"@type":"Thing","name":"\\u003c/script>\\u003cscript>alert(1)\\u003c/script>"}</script>',
    );
    expect(html.match(/<script/g)).toHaveLength(1);
  });

  it("uses no hook, so it can run as a Server Component", () => {
    expect(() => Seo({ title: "x" })).not.toThrow();
    expect(() => createSeo({ siteName: "Acme" })({})).not.toThrow();
  });

  it("supports extra meta tags", () => {
    const h = head(doc(<Seo meta={[{ name: "theme-color", content: "#000" }]} />));
    expect(h).toContain('<meta name="theme-color" content="#000"/>');
  });
});

describe("renderWithHead", () => {
  it("splits hoisted tags from the body", () => {
    const { head, html } = renderWithHead(
      <div id="app">
        <Seo title="Hello" description="World" />
        <p>body</p>
      </div>,
    );
    expect(head).toMatch(/^<title>Hello<\/title><meta name="description" content="World"\/>/);
    expect(head).not.toContain("<div");
    expect(html).toBe('<div id="app"><p>body</p></div>');
  });
});
