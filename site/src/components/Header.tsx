import { useState } from "react";
import { t } from "../i18n";
import { LANGS, translate, type Route } from "../routes";
import { GITHUB } from "../seo";
import { GitHubIcon, Logo } from "./Logo";

export function LangSwitch({ route }: { route: Route }) {
  return (
    <nav className="langs" aria-label={t(route.lang).footer.language}>
      {LANGS.map((l) => (
        <a key={l} href={translate(route, l)} hrefLang={l} lang={l} aria-current={l === route.lang ? "true" : undefined} title={t(l).langName}>
          {l}
        </a>
      ))}
    </nav>
  );
}

export function Header({ route }: { route: Route }) {
  const d = t(route.lang);
  const [open, setOpen] = useState(false);
  const home = `/${route.lang}/`;
  const links = [
    { href: `/${route.lang}/docs/`, label: d.nav.docs },
    { href: `${home}#features`, label: d.nav.features },
    { href: `${home}#playground`, label: d.nav.playground },
    { href: `${home}#benchmarks`, label: d.nav.benchmarks },
  ];
  return (
    <header id="top" className={`header${open ? " open" : ""}`}>
      <a className="skip" href="#main">
        {d.skip}
      </a>
      <a href={home} className="brand" aria-label="active" data-magnetic>
        <Logo />
      </a>
      <nav className="nav" aria-label="Main">
        {links.map((l) => (
          <a key={l.href} href={l.href} className="link" onClick={() => setOpen(false)}>
            <span data-text={l.label}>{l.label}</span>
          </a>
        ))}
      </nav>
      <div className="actions">
        <LangSwitch route={route} />
        <a className="icon" href={GITHUB} aria-label="GitHub" target="_blank" rel="noopener noreferrer">
          <GitHubIcon />
        </a>
        <a className="btn small" href={`/${route.lang}/docs/quick-start/`} data-magnetic>
          {d.nav.start}
        </a>
        <button className="burger" type="button" aria-expanded={open} aria-label={open ? d.nav.close : d.nav.menu} onClick={() => setOpen(!open)}>
          <span />
          <span />
        </button>
      </div>
    </header>
  );
}
