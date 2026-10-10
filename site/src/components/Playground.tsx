import { useWasm } from "active-react";
import { useMemo, useState } from "react";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { site } from "../wasm";
import { Split } from "./Split";

const ORIGIN = "https://acme.com";

/** One tag per line, with light syntax colouring (input is already HTML-escaped by Rust). */
function pretty(html: string) {
  return html
    .replace(/></g, ">\n<")
    .split("\n")
    .map((line) =>
      line
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        // Attributes first: the tag markup inserted next must not be matched as an attribute.
        .replace(/ ([a-z:-]+)=("[^"]*")/g, ' <i class="at">$1</i>=<u class="st">$2</u>')
        .replace(/&lt;(\/?)([a-z]+)/g, '&lt;$1<b class="tg">$2</b>'),
    )
    .join("\n");
}

export function Playground({ lang }: { lang: Lang }) {
  const d = t(lang).home;
  const p = d.play;
  const rust = useWasm(site);
  const [title, setTitle] = useState(p.titleValue);
  const [description, setDescription] = useState(p.descriptionValue);
  const [path, setPath] = useState(p.pathValue);

  const result = useMemo(() => {
    if (!rust) return undefined;
    const start = performance.now();
    const html = rust.seo_head(title, description, path, ORIGIN);
    return { html, ms: performance.now() - start };
  }, [rust, title, description, path]);

  const shown = title ? `${title} · Acme` : "Acme";
  const url = ORIGIN + (path.startsWith("/") ? path : `/${path}`);

  return (
    <section id="playground" className="section playground">
      <div className="section-head">
        <span className="kicker" data-reveal>
          03 — {t(lang).nav.playground}
        </span>
        <Split as="h2" text={d.playTitle} className="title" />
        <p className="sub" data-reveal>
          {d.playText}
        </p>
      </div>
      <div className="play" data-reveal>
        <form className="play-form" onSubmit={(e) => e.preventDefault()}>
          <label>
            <span>{p.title}</span>
            <input value={title} maxLength={90} onChange={(e) => setTitle(e.target.value)} />
          </label>
          <label>
            <span>{p.description}</span>
            <textarea value={description} rows={3} maxLength={220} onChange={(e) => setDescription(e.target.value)} />
          </label>
          <label>
            <span>{p.path}</span>
            <input value={path} maxLength={80} onChange={(e) => setPath(e.target.value)} />
          </label>
          <div className="previews">
            <div className="serp" aria-label={p.google}>
              <small>{p.google}</small>
              <span className="serp-url">acme.com › {path.replace(/^\//, "").split("/").join(" › ")}</span>
              <span className="serp-title">{shown.length > 60 ? `${shown.slice(0, 58)}…` : shown}</span>
              <span className="serp-desc">{description.length > 155 ? `${description.slice(0, 152)}…` : description}</span>
            </div>
            <div className="social" aria-label={p.social}>
              <small>{p.social}</small>
              <div className="social-img" aria-hidden="true">
                <span>active</span>
              </div>
              <span className="social-domain">acme.com</span>
              <span className="social-title">{title || "Acme"}</span>
            </div>
          </div>
        </form>
        <div className="play-out">
          <div className="play-bar">
            <span>{p.output}</span>
            {result && (
              <span className="took">
                <span className="tag">Rust</span> {p.took} {result.ms < 0.1 ? "<0.1" : result.ms.toFixed(2)} ms
              </span>
            )}
          </div>
          <pre aria-live="polite">
            {result ? <code dangerouslySetInnerHTML={{ __html: pretty(result.html) }} /> : <code className="muted">{p.loading}</code>}
          </pre>
          <span className="sr-only">{url}</span>
        </div>
      </div>
    </section>
  );
}
