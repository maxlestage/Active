import { useEffect, useState } from "react";
import { Arrow } from "../components/Logo";
import { Search } from "../components/Search";
import { Split } from "../components/Split";
import { CATEGORIES, categoryOf, meta, neighbours, type Doc } from "../docs";
import { t } from "../i18n";
import type { Lang } from "../routes";
import { GITHUB, ORIGIN, Seo } from "../seo";
import { alternates } from "./seo-helpers";

function Sidebar({ lang, slug }: { lang: Lang; slug?: string }) {
  const d = t(lang).docs;
  const [open, setOpen] = useState(false); // phones only: the list is folded behind a button
  return (
    <aside className={`sidebar${open ? " open" : ""}`}>
      <Search lang={lang} />
      <div className="sidebar-nav">
        <button type="button" className="sidebar-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
          {d.title}
          <span aria-hidden="true">{open ? "−" : "+"}</span>
        </button>
        <nav aria-label={d.title}>
          {CATEGORIES.map((c) => (
            <div key={c.id} className="group">
              <h2>{d.categories[c.id].title}</h2>
              <ul>
                {c.pages.map((p) => (
                  <li key={p}>
                    <a href={`/${lang}/docs/${p}/`} aria-current={p === slug ? "page" : undefined}>
                      {meta(lang, p).title}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
      </div>
    </aside>
  );
}

export function DocsIndex({ lang }: { lang: Lang }) {
  const d = t(lang).docs;
  return (
    <div className="docs docs-index">
      <Seo
        title={d.title}
        description={d.description}
        canonical={`/${lang}/docs/`}
        alternates={alternates({ lang, page: "docs" })}
        locale={t(lang).locale}
      />
      <header className="docs-hero">
        <span className="kicker">active-react · active</span>
        <Split as="h1" text={d.title} className="title" reveal={false} />
        <p className="lead">{d.intro}</p>
        <Search lang={lang} />
      </header>
      <div className="categories">
        {CATEGORIES.map((c, i) => (
          <section key={c.id} className="category" data-reveal data-spot style={{ "--d": `${i * 70}ms` } as React.CSSProperties}>
            <span className="num">0{i + 1}</span>
            <h2>{d.categories[c.id].title}</h2>
            <p>{d.categories[c.id].text}</p>
            <ul>
              {c.pages.map((p) => (
                <li key={p}>
                  <a href={`/${lang}/docs/${p}/`}>
                    <strong>{meta(lang, p).title}</strong>
                    <span>{meta(lang, p).description}</span>
                    <Arrow />
                  </a>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}

export function DocPage({ lang, slug, doc }: { lang: Lang; slug: string; doc: Doc }) {
  const d = t(lang).docs;
  const { prev, next } = neighbours(slug);
  const category = d.categories[categoryOf(slug).id].title;

  // "On this page": highlight the heading currently read.
  useEffect(() => {
    const links = new Map([...document.querySelectorAll<HTMLAnchorElement>(".toc a")].map((a) => [a.hash.slice(1), a]));
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries)
          if (e.isIntersecting) {
            links.forEach((a) => a.classList.remove("active"));
            links.get(e.target.id)?.classList.add("active");
          }
      },
      { rootMargin: "-15% 0px -70% 0px" },
    );
    document.querySelectorAll(".prose h2[id], .prose h3[id]").forEach((h) => io.observe(h));
    return () => io.disconnect();
  }, [slug]);

  return (
    <div className="docs">
      <Seo
        title={doc.title}
        titleTemplate="%s — active docs"
        description={doc.description}
        canonical={`/${lang}/docs/${slug}/`}
        alternates={alternates({ lang, page: "doc", slug })}
        type="article"
        locale={t(lang).locale}
        jsonLd={[
          { "@context": "https://schema.org", "@type": "TechArticle", headline: doc.title, description: doc.description, inLanguage: lang },
          {
            "@context": "https://schema.org",
            "@type": "BreadcrumbList",
            itemListElement: [
              { "@type": "ListItem", position: 1, name: d.title, item: `${ORIGIN}/${lang}/docs/` },
              { "@type": "ListItem", position: 2, name: doc.title, item: `${ORIGIN}/${lang}/docs/${slug}/` },
            ],
          },
        ]}
      />
      <Sidebar lang={lang} slug={slug} />
      <article className="doc">
        <nav className="crumbs" aria-label="Breadcrumb">
          <a href={`/${lang}/docs/`}>{d.title}</a>
          <span aria-hidden="true">/</span>
          <span>{category}</span>
        </nav>
        <h1 className="doc-title">{doc.title}</h1>
        <p className="doc-lead">{doc.description}</p>
        <div className="prose" dangerouslySetInnerHTML={{ __html: doc.html }} />
        <a className="edit" href={`${GITHUB}/edit/master/site/docs/${lang}/${slug}.md`} target="_blank" rel="noopener noreferrer">
          {d.edit}
        </a>
        <nav className="pager" aria-label="Pagination">
          {prev ? (
            <a href={`/${lang}/docs/${prev}/`} rel="prev">
              <small>← {d.prev}</small>
              <span>{meta(lang, prev).title}</span>
            </a>
          ) : (
            <span />
          )}
          {next && (
            <a href={`/${lang}/docs/${next}/`} rel="next" className="next">
              <small>{d.next} →</small>
              <span>{meta(lang, next).title}</span>
            </a>
          )}
        </nav>
      </article>
      <nav className="toc" aria-label={d.onThisPage}>
        {doc.toc.length > 0 && (
          <>
            <h2>{d.onThisPage}</h2>
            <ul>
              {doc.toc.map((h) => (
                <li key={h.id} className={`d${h.depth}`}>
                  <a href={`#${h.id}`}>{h.text}</a>
                </li>
              ))}
            </ul>
          </>
        )}
      </nav>
    </div>
  );
}
