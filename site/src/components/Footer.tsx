import { t } from "../i18n";
import { LANGS, translate, type Route } from "../routes";
import { GITHUB } from "../seo";
import { Arrow, Logo } from "./Logo";

export function Footer({ route }: { route: Route }) {
  const d = t(route.lang);
  const f = d.footer;
  const docs = (slug: string) => `/${route.lang}/docs/${slug}/`;
  const columns = [
    {
      title: f.product,
      links: [
        { href: `/${route.lang}/docs/`, label: d.nav.docs },
        { href: docs("quick-start"), label: f.links.quickStart },
        { href: docs("api"), label: f.links.api },
        { href: docs("benchmarks"), label: f.links.benchmarks },
      ],
    },
    {
      title: f.learn,
      links: [
        { href: docs("seo-component"), label: f.links.seo },
        { href: docs("loading-wasm"), label: f.links.wasm },
        { href: docs("rust-crate"), label: f.links.rust },
        { href: docs("faq"), label: f.links.faq },
      ],
    },
    {
      title: f.project,
      links: [
        { href: GITHUB, label: "GitHub", external: true },
        { href: "https://www.npmjs.com/package/active-react", label: "npm", external: true },
        { href: "https://crates.io/crates/active", label: "crates.io", external: true },
        { href: docs("deploy-heroku"), label: f.links.deploy },
      ],
    },
  ];

  return (
    <footer className="footer">
      <a className="footer-cta" href={docs("quick-start")} data-reveal data-cursor>
        <span>{f.start}</span>
        <Arrow />
      </a>
      <div className="footer-grid">
        <div className="footer-brand" data-reveal>
          <Logo />
          <p>{f.tagline}</p>
        </div>
        {columns.map((c, i) => (
          <nav key={c.title} aria-label={c.title} data-reveal style={{ "--d": `${(i + 1) * 80}ms` } as React.CSSProperties}>
            <h2>{c.title}</h2>
            <ul>
              {c.links.map((l) => (
                <li key={l.href}>
                  <a href={l.href} {...("external" in l ? { target: "_blank", rel: "noopener noreferrer" } : {})}>
                    {l.label}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
        <nav aria-label={f.language} data-reveal style={{ "--d": "320ms" } as React.CSSProperties}>
          <h2>{f.language}</h2>
          <ul>
            {LANGS.map((l) => (
              <li key={l}>
                <a href={translate(route, l)} hrefLang={l} lang={l} aria-current={l === route.lang ? "true" : undefined}>
                  {t(l).langName}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      </div>
      <div className="wordmark" aria-hidden="true" data-reveal>
        {"active".split("").map((c, i) => (
          <span key={i} style={{ "--i": i } as React.CSSProperties}>
            {c}
          </span>
        ))}
      </div>
      <div className="footer-bottom">
        <span>© 2026 active · {f.license}</span>
        <span>{f.built}</span>
        <a href="#top" className="top">
          {f.top} ↑
        </a>
      </div>
    </footer>
  );
}
