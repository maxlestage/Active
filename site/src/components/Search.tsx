import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";
import type { Lang } from "../routes";

interface Entry {
  s: string; // slug
  t: string; // title
  d: string; // description
  c: string; // category title
  h: Array<[string, string]>; // [id, heading]
}
interface Hit {
  href: string;
  title: string;
  sub: string;
}

const norm = (s: string) => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");

function find(index: Entry[], query: string, lang: Lang): Hit[] {
  const words = norm(query).split(/\s+/).filter(Boolean);
  if (!words.length) return index.slice(0, 8).map((e) => ({ href: `/${lang}/docs/${e.s}/`, title: e.t, sub: e.c }));
  const hits: Array<Hit & { score: number }> = [];
  for (const e of index) {
    const page = norm(`${e.t} ${e.d} ${e.c}`);
    if (words.every((w) => page.includes(w))) hits.push({ href: `/${lang}/docs/${e.s}/`, title: e.t, sub: e.c, score: norm(e.t).includes(words[0]!) ? 3 : 2 });
    for (const [id, h] of e.h)
      if (words.every((w) => norm(`${h} ${e.t}`).includes(w))) hits.push({ href: `/${lang}/docs/${e.s}/#${id}`, title: h, sub: e.t, score: 1 });
  }
  return hits.sort((a, b) => b.score - a.score).slice(0, 12);
}

/** Docs search: a dialog opened with the button, "/" or Ctrl/⌘+K. The index is fetched on first open. */
export function Search({ lang }: { lang: Lang }) {
  const d = t(lang).docs;
  const dialog = useRef<HTMLDialogElement>(null);
  const [index, setIndex] = useState<Entry[]>();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const hits = index ? find(index, query, lang) : [];

  const open = () => {
    dialog.current?.showModal();
    if (!index) fetch(`/search-${lang}.json`).then((r) => r.json()).then(setIndex, () => setIndex([]));
  };

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      const typing = (e.target as HTMLElement).closest("input, textarea");
      if ((e.key === "k" && (e.metaKey || e.ctrlKey)) || (e.key === "/" && !typing)) {
        e.preventDefault();
        open();
      }
    };
    addEventListener("keydown", key);
    return () => removeEventListener("keydown", key);
  });

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") setActive((a) => Math.min(a + 1, hits.length - 1));
    else if (e.key === "ArrowUp") setActive((a) => Math.max(a - 1, 0));
    else if (e.key === "Enter" && hits[active]) location.href = hits[active].href;
    else return;
    e.preventDefault();
  };

  return (
    <>
      <button type="button" className="search-btn" onClick={open}>
        <svg viewBox="0 0 24 24" aria-hidden="true" width="16" height="16">
          <circle cx="11" cy="11" r="7" fill="none" stroke="currentColor" strokeWidth="2" />
          <path d="m20 20-4-4" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
        <span>{d.search}</span>
        <kbd>/</kbd>
      </button>
      <dialog ref={dialog} className="search" aria-label={d.search} onClick={(e) => e.target === dialog.current && dialog.current.close()}>
        <div className="search-box">
          <input
            autoFocus
            type="search"
            placeholder={d.searchHint}
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKey}
            aria-label={d.search}
          />
          <ul role="listbox">
            {index && !hits.length && <li className="empty">{d.noResults}</li>}
            {hits.map((h, i) => (
              <li key={h.href} role="option" aria-selected={i === active}>
                <a href={h.href} onMouseEnter={() => setActive(i)} onClick={() => dialog.current?.close()}>
                  <strong>{h.title}</strong>
                  <span>{h.sub}</span>
                </a>
              </li>
            ))}
          </ul>
        </div>
      </dialog>
    </>
  );
}
