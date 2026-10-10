import type { CSSProperties } from "react";
import { useState } from "react";
import { BENCH } from "../data";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { HEROKU_DEPLOY } from "../seo";
import snippets from "../snippets.md";
import { Arrow } from "./Logo";
import { Split } from "./Split";

const css = (vars: Record<string, string | number>) => vars as CSSProperties;

export function Marquee({ lang }: { lang: Lang }) {
  const words = t(lang).home.marquee;
  const row = (hidden: boolean) => (
    <ul aria-hidden={hidden || undefined}>
      {words.map((w) => (
        <li key={w}>
          {w}
          <span className="star" aria-hidden="true">
            ✦
          </span>
        </li>
      ))}
    </ul>
  );
  return (
    <div className="marquee" data-reveal>
      <div className="track">
        {row(false)}
        {row(true)}
      </div>
    </div>
  );
}

export function Manifesto({ lang }: { lang: Lang }) {
  const text = t(lang).home.manifesto;
  // Words light up one by one while the section scrolls by (--p is set by motion.ts).
  return (
    <section id="manifesto" className="manifesto" data-scrub style={css({ "--n": text.split(/\s+/).length })}>
      <Split as="p" text={text} className="big" reveal={false} />
    </section>
  );
}

export function Features({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  return (
    <section id="features" className="section features">
      <div className="section-head">
        <span className="kicker" data-reveal>
          01 — {t(lang).nav.features}
        </span>
        <Split as="h2" text={d.featuresTitle} className="title" />
      </div>
      <div className="cards">
        {d.features.map((f, i) => (
          <article key={f.title} className="card" data-reveal data-spot style={css({ "--d": `${i * 90}ms` })}>
            <span className="num">0{i + 1}</span>
            <h3>{f.title}</h3>
            <p>{f.text}</p>
            <span className="rule" />
          </article>
        ))}
      </div>
      <div className="stats">
        {d.stats.map((s) => (
          <div key={s.label} className="stat" data-reveal>
            <strong>
              {"prefix" in s ? s.prefix : ""}
              <span data-count={s.value} data-decimals={s.decimals}>
                {s.value.toLocaleString(lang, { minimumFractionDigits: s.decimals })}
              </span>
              {s.suffix}
            </strong>
            <span>{s.label}</span>
          </div>
        ))}
      </div>
    </section>
  );
}

export function Code({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  const [tab, setTab] = useState(0);
  return (
    <section className="section code-section">
      <div className="section-head">
        <span className="kicker" data-reveal>
          02 — API
        </span>
        <Split as="h2" text={d.codeTitle} className="title" />
        <p className="sub" data-reveal>
          {d.codeText}
        </p>
      </div>
      <div className="window" data-reveal>
        <div className="tabs" role="tablist" style={css({ "--tab": tab })}>
          {d.tabs.map((label, i) => (
            <button key={label} role="tab" type="button" aria-selected={tab === i} onClick={() => setTab(i)}>
              {label}
            </button>
          ))}
          <span className="indicator" aria-hidden="true" />
        </div>
        {snippets.blocks.map((html, i) => (
          <div key={i} role="tabpanel" className="panel code" hidden={tab !== i}>
            <button className="copy" type="button" aria-label="Copy">
              Copy
            </button>
            <div dangerouslySetInnerHTML={{ __html: html }} />
          </div>
        ))}
      </div>
    </section>
  );
}

function Bars({ title, rows, unit, lang }: { title: string; rows: Array<{ name: string; value: number; us?: boolean; base?: boolean }>; unit: string; lang: Lang }) {
  const max = Math.max(...rows.map((r) => r.value));
  return (
    <figure className="bars" data-reveal>
      <figcaption>{title}</figcaption>
      {rows.map((r, i) => (
        <div key={r.name} className={`bar${r.us ? " us" : ""}${r.base ? " base" : ""}`} style={css({ "--w": r.value / max, "--d": `${i * 90}ms` })}>
          <span className="name">{r.name}</span>
          <span className="track">
            <span className="fill" />
          </span>
          <span className="value">
            {r.value.toLocaleString(lang)} {unit}
          </span>
        </div>
      ))}
    </figure>
  );
}

export function Benchmarks({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  return (
    <section id="benchmarks" className="section bench">
      <div className="section-head">
        <span className="kicker" data-reveal>
          04 — {t(lang).nav.benchmarks}
        </span>
        <Split as="h2" text={d.benchTitle} className="title" />
        <p className="sub" data-reveal>
          {d.benchText}
        </p>
      </div>
      <div className="bench-grid">
        <Bars title={d.benchSize} rows={BENCH.size} unit="kB" lang={lang} />
        <Bars title={d.benchSsr} rows={BENCH.ssr} unit="µs" lang={lang} />
      </div>
      <a className="more" href={`/${lang}/docs/benchmarks/`} data-reveal>
        {d.benchLink} <Arrow />
      </a>
    </section>
  );
}

export function Steps({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  const code = ["npm i active-react", "wasm-pack build --target web", "useWasm(mod)"];
  return (
    <section className="section steps">
      <div className="section-head">
        <span className="kicker" data-reveal>
          05 — Start
        </span>
        <Split as="h2" text={d.stepsTitle} className="title" />
      </div>
      <ol>
        {d.steps.map((s, i) => (
          <li key={s.title} data-reveal style={css({ "--d": `${i * 120}ms` })}>
            <span className="num">{i + 1}</span>
            <h3>{s.title}</h3>
            <p>{s.text}</p>
            <code>{code[i]}</code>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function Cta({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  return (
    <section className="cta">
      <Split as="h2" text={d.ctaTitle} className="title huge" />
      <p data-reveal>{d.ctaText}</p>
      <div className="cta-actions" data-reveal>
        <a className="btn" href={`/${lang}/docs/`} data-magnetic>
          {d.ctaDocs} <Arrow />
        </a>
        <a className="btn ghost" href={HEROKU_DEPLOY} target="_blank" rel="noopener noreferrer" data-magnetic>
          {d.ctaDeploy}
        </a>
      </div>
    </section>
  );
}
